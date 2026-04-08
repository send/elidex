//! JSON.stringify and JSON.parse (ES2020 §24.5).

use std::fmt::Write;

use super::shape::{PropertyAttrs, ROOT_SHAPE};
use super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};

// ============================================================================
// JSON.stringify (§24.5.2)
// ============================================================================

/// Serialization state for `JSON.stringify`.
struct JsonSerializer {
    output: String,
    /// Object stack for circular reference detection.
    stack: Vec<ObjectId>,
    /// Current indentation prefix.
    indent: String,
    /// One level of indentation (derived from `space` argument).
    gap: String,
    /// Replacer function, if any.
    replacer_fn: Option<ObjectId>,
    /// Replacer property list (when replacer is an Array).
    property_list: Option<Vec<StringId>>,
    /// Cached `StringId` for `"toJSON"` (interned once at construction).
    to_json_key: StringId,
    /// Reusable buffer for array index → string conversion.
    index_buf: String,
}

impl JsonSerializer {
    /// §24.5.2.1 SerializeJSONProperty — returns `true` if a value was written,
    /// `false` if the value is `undefined`/Symbol/Function (i.e. should be skipped).
    fn serialize_property(
        &mut self,
        ctx: &mut NativeContext<'_>,
        value: JsValue,
        holder: ObjectId,
        key: JsValue,
    ) -> Result<bool, VmError> {
        let mut val = value;

        // Step 2: If value is Object, check for toJSON.
        if let JsValue::Object(obj_id) = val {
            let to_json_pk = PropertyKey::String(self.to_json_key);
            if let Some(to_json_val) = ctx.try_get_property_value(obj_id, to_json_pk)? {
                if matches!(to_json_val, JsValue::Object(_)) {
                    val = ctx.call_value(to_json_val, val, &[key])?;
                }
            }
        }

        // Step 3: If replacer function, call it.
        if let Some(replacer) = self.replacer_fn {
            val = ctx.call_function(replacer, JsValue::Object(holder), &[key, val])?;
        }

        // Step 4: Unwrap wrapper objects.
        if let JsValue::Object(obj_id) = val {
            val = match ctx.get_object(obj_id).kind {
                ObjectKind::NumberWrapper(n) => JsValue::Number(n),
                ObjectKind::StringWrapper(s) => JsValue::String(s),
                ObjectKind::BooleanWrapper(b) => JsValue::Boolean(b),
                ObjectKind::BigIntWrapper(id) => JsValue::BigInt(id),
                _ => val,
            };
        }

        // Step 5+: Type-specific serialization.
        match val {
            JsValue::Null => {
                self.output.push_str("null");
                Ok(true)
            }
            JsValue::Boolean(true) => {
                self.output.push_str("true");
                Ok(true)
            }
            JsValue::Boolean(false) => {
                self.output.push_str("false");
                Ok(true)
            }
            JsValue::Number(n) => {
                if n.is_finite() {
                    write_number(n, &mut self.output);
                } else {
                    // NaN, Infinity, -Infinity → "null"
                    self.output.push_str("null");
                }
                Ok(true)
            }
            JsValue::String(id) => {
                let units = ctx.get_u16(id);
                quote(units, &mut self.output);
                Ok(true)
            }
            JsValue::BigInt(_) => Err(VmError::type_error("Do not know how to serialize a BigInt")),
            JsValue::Object(obj_id) => {
                if ctx.get_object(obj_id).kind.is_callable() {
                    return Ok(false); // skip
                }
                let is_array = matches!(ctx.get_object(obj_id).kind, ObjectKind::Array { .. });
                if is_array {
                    self.serialize_array(ctx, obj_id)
                } else {
                    self.serialize_object(ctx, obj_id)
                }
            }
            // undefined, Symbol → skip
            JsValue::Undefined | JsValue::Symbol(_) => Ok(false),
        }
    }

    /// §24.5.2.4 SerializeJSONArray
    fn serialize_array(
        &mut self,
        ctx: &mut NativeContext<'_>,
        obj_id: ObjectId,
    ) -> Result<bool, VmError> {
        // Circular reference check.
        if self.stack.contains(&obj_id) {
            return Err(VmError::type_error("Converting circular structure to JSON"));
        }
        self.stack.push(obj_id);

        let len = match &ctx.get_object(obj_id).kind {
            ObjectKind::Array { elements } => elements.len(),
            _ => 0,
        };

        self.output.push('[');
        if len == 0 {
            self.output.push(']');
            self.stack.pop();
            return Ok(true);
        }

        let has_gap = !self.gap.is_empty();
        let prev_indent = if has_gap {
            let prev = self.indent.clone();
            self.indent.push_str(&self.gap);
            prev
        } else {
            String::new()
        };

        for i in 0..len {
            if i > 0 {
                self.output.push(',');
            }
            if has_gap {
                self.output.push('\n');
                self.output.push_str(&self.indent);
            }

            // Read element (must re-borrow each iteration due to ctx mutation).
            let elem = match &ctx.get_object(obj_id).kind {
                ObjectKind::Array { elements } => {
                    elements.get(i).copied().unwrap_or(JsValue::Undefined)
                }
                _ => JsValue::Undefined,
            };

            self.index_buf.clear();
            let _ = write!(self.index_buf, "{i}");
            let key_str = ctx.intern(&self.index_buf);
            let wrote = self.serialize_property(ctx, elem, obj_id, JsValue::String(key_str))?;
            if !wrote {
                // undefined/Symbol/Function in array → "null"
                self.output.push_str("null");
            }
        }

        if has_gap {
            self.indent = prev_indent;
            self.output.push('\n');
            self.output.push_str(&self.indent);
        }
        self.output.push(']');

        self.stack.pop();
        Ok(true)
    }

    /// §24.5.2.3 SerializeJSONObject
    fn serialize_object(
        &mut self,
        ctx: &mut NativeContext<'_>,
        obj_id: ObjectId,
    ) -> Result<bool, VmError> {
        // Circular reference check.
        if self.stack.contains(&obj_id) {
            return Err(VmError::type_error("Converting circular structure to JSON"));
        }
        self.stack.push(obj_id);

        // Collect keys.
        let keys: Vec<StringId> = if let Some(ref pl) = self.property_list {
            pl.clone()
        } else {
            ctx.get_object(obj_id)
                .storage
                .iter_keys(&ctx.vm.shapes)
                .filter(|(k, attrs)| attrs.enumerable && matches!(k, PropertyKey::String(_)))
                .map(|(k, _)| match k {
                    PropertyKey::String(s) => s,
                    PropertyKey::Symbol(_) => unreachable!(),
                })
                .collect()
        };

        self.output.push('{');

        let has_gap = !self.gap.is_empty();
        let prev_indent = if has_gap {
            let prev = self.indent.clone();
            self.indent.push_str(&self.gap);
            prev
        } else {
            String::new()
        };

        let mut first = true;
        for key_sid in keys {
            let key_pk = PropertyKey::String(key_sid);
            let val = ctx.get_property_value(obj_id, key_pk)?;

            // Try to serialize. If skip, don't emit this key at all.
            let before_len = self.output.len();
            if !first {
                self.output.push(',');
            }
            if has_gap {
                self.output.push('\n');
                self.output.push_str(&self.indent);
            }
            // Write key.
            let key_units = ctx.get_u16(key_sid);
            quote(key_units, &mut self.output);
            self.output.push(':');
            if has_gap {
                self.output.push(' ');
            }

            let wrote = self.serialize_property(ctx, val, obj_id, JsValue::String(key_sid))?;
            if wrote {
                first = false;
            } else {
                // Revert: this property produces undefined/Symbol/Function.
                self.output.truncate(before_len);
            }
        }

        if !first && has_gap {
            self.indent = prev_indent;
            self.output.push('\n');
            self.output.push_str(&self.indent);
        } else if first && has_gap {
            self.indent = prev_indent;
        }
        self.output.push('}');

        self.stack.pop();
        Ok(true)
    }
}

/// JSON string escaping: surround with `"` and escape special characters.
/// Operates on WTF-16 code units to correctly handle lone surrogates.
fn quote(units: &[u16], output: &mut String) {
    output.reserve(units.len() + 2);
    output.push('"');
    let mut i = 0;
    while i < units.len() {
        let c = units[i];
        match c {
            0x08 => output.push_str("\\b"),
            0x09 => output.push_str("\\t"),
            0x0A => output.push_str("\\n"),
            0x0C => output.push_str("\\f"),
            0x0D => output.push_str("\\r"),
            0x22 => output.push_str("\\\""),
            0x5C => output.push_str("\\\\"),
            0x00..=0x1F => {
                // Other control characters → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            // Surrogate pair handling
            0xD800..=0xDBFF => {
                if let Some(&lo) = units.get(i + 1) {
                    if (0xDC00..=0xDFFF).contains(&lo) {
                        // Valid surrogate pair → decode to char and emit UTF-8.
                        let cp =
                            0x10000 + ((u32::from(c) - 0xD800) << 10) + (u32::from(lo) - 0xDC00);
                        if let Some(ch) = char::from_u32(cp) {
                            output.push(ch);
                        }
                        i += 2;
                        continue;
                    }
                }
                // Lone high surrogate → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            0xDC00..=0xDFFF => {
                // Lone low surrogate → \uXXXX
                let _ = write!(output, "\\u{c:04x}");
            }
            _ => {
                // BMP character → emit as UTF-8.
                if let Some(ch) = char::from_u32(u32::from(c)) {
                    output.push(ch);
                }
            }
        }
        i += 1;
    }
    output.push('"');
}

/// Determine the length of a UTF-8 character from its leading byte.
#[inline]
fn utf8_char_len(lead: u8) -> usize {
    match lead {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// Write a finite f64 to the output buffer in JS number format.
fn write_number(n: f64, output: &mut String) {
    debug_assert!(n.is_finite());
    if n == 0.0 {
        output.push('0');
        return;
    }
    #[allow(clippy::cast_precision_loss)]
    if n == (n as i64 as f64) && n.abs() < 1e15 {
        let _ = write!(output, "{}", n as i64);
    } else {
        let _ = write!(output, "{n}");
    }
}

/// Entry point for `JSON.stringify(value, replacer?, space?)`.
pub(super) fn native_json_stringify(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let replacer_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let space_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

    // Step 4: Process replacer.
    let mut replacer_fn = None;
    let mut property_list = None;
    if let JsValue::Object(obj_id) = replacer_arg {
        if ctx.get_object(obj_id).kind.is_callable() {
            replacer_fn = Some(obj_id);
        } else if let ObjectKind::Array { elements } = &ctx.get_object(obj_id).kind {
            let elems: Vec<JsValue> = elements.clone();
            let mut list = Vec::new();
            for elem in elems {
                let sid = match elem {
                    JsValue::String(s) => s,
                    JsValue::Number(_) => ctx.to_string_val(elem)?,
                    JsValue::Object(oid) => match ctx.get_object(oid).kind {
                        ObjectKind::NumberWrapper(n) => ctx.to_string_val(JsValue::Number(n))?,
                        ObjectKind::StringWrapper(s) => s,
                        _ => continue,
                    },
                    _ => continue,
                };
                if !list.contains(&sid) {
                    list.push(sid);
                }
            }
            property_list = Some(list);
        }
    }

    // Step 5-8: Process space.
    let gap = compute_gap(ctx, space_arg);

    // Build wrapper object: { "": value }
    let wrapper_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(ROOT_SHAPE),
        prototype: None,
    });
    let empty_key = ctx.vm.well_known.empty;
    ctx.vm.define_shaped_property(
        wrapper_id,
        PropertyKey::String(empty_key),
        PropertyValue::Data(value),
        PropertyAttrs::DATA,
    );

    let to_json_key = ctx.intern("toJSON");
    let mut serializer = JsonSerializer {
        output: String::with_capacity(128),
        stack: Vec::new(),
        indent: String::new(),
        gap,
        replacer_fn,
        property_list,
        to_json_key,
        index_buf: String::with_capacity(8),
    };

    let wrote =
        serializer.serialize_property(ctx, value, wrapper_id, JsValue::String(empty_key))?;

    if wrote {
        let id = ctx.intern(&serializer.output);
        Ok(JsValue::String(id))
    } else {
        Ok(JsValue::Undefined)
    }
}

/// Compute the `gap` string from the `space` argument.
fn compute_gap(ctx: &mut NativeContext<'_>, space: JsValue) -> String {
    // Unwrap wrapper objects first.
    let space = match space {
        JsValue::Object(obj_id) => match ctx.get_object(obj_id).kind {
            ObjectKind::NumberWrapper(n) => JsValue::Number(n),
            ObjectKind::StringWrapper(s) => JsValue::String(s),
            _ => space,
        },
        other => other,
    };

    match space {
        JsValue::Number(n) => {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let count = n.clamp(0.0, 10.0) as usize;
            " ".repeat(count)
        }
        JsValue::String(id) => {
            let units = ctx.get_u16(id);
            let len = units.len().min(10);
            String::from_utf16_lossy(&units[..len])
        }
        _ => String::new(),
    }
}

// ============================================================================
// JSON.parse (§24.5.1)
// ============================================================================

/// Recursive-descent JSON parser operating on UTF-8 bytes.
struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn err(&self, msg: &str) -> VmError {
        VmError::syntax_error(format!("JSON.parse: {msg} at position {}", self.pos))
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    fn expect(&mut self, expected: u8) -> Result<(), VmError> {
        match self.advance() {
            Some(b) if b == expected => Ok(()),
            _ => Err(self.err(&format!("expected '{}'", expected as char))),
        }
    }

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), VmError> {
        for &b in literal {
            if self.advance() != Some(b) {
                return Err(self.err(&format!(
                    "expected '{}'",
                    std::str::from_utf8(literal).unwrap_or("?")
                )));
            }
        }
        Ok(())
    }

    /// Parse a JSON value and produce a `JsValue`.
    fn parse_value(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => self.parse_string(ctx),
            Some(b'{') => self.parse_object(ctx),
            Some(b'[') => self.parse_array(ctx),
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(JsValue::Boolean(true))
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(JsValue::Boolean(false))
            }
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(JsValue::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => Err(self.err("unexpected character")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    /// Parse a JSON string, returning the raw Rust `String` (with escapes resolved).
    fn parse_string_raw(&mut self) -> Result<String, VmError> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => return Ok(s),
                Some(b'\\') => {
                    match self.advance() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'/') => s.push('/'),
                        Some(b'b') => s.push('\u{08}'),
                        Some(b'f') => s.push('\u{0C}'),
                        Some(b'n') => s.push('\n'),
                        Some(b'r') => s.push('\r'),
                        Some(b't') => s.push('\t'),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            // Handle surrogate pairs.
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // High surrogate — expect \uXXXX for low surrogate.
                                if self.peek() == Some(b'\\')
                                    && self.input.get(self.pos + 1) == Some(&b'u')
                                {
                                    self.pos += 2; // skip \u
                                    let lo = self.parse_hex4()?;
                                    if (0xDC00..=0xDFFF).contains(&lo) {
                                        let full = 0x10000
                                            + ((u32::from(cp) - 0xD800) << 10)
                                            + (u32::from(lo) - 0xDC00);
                                        if let Some(ch) = char::from_u32(full) {
                                            s.push(ch);
                                        }
                                    } else {
                                        // Invalid pair — emit both as replacement chars.
                                        s.push(char::REPLACEMENT_CHARACTER);
                                        if let Some(ch) = char::from_u32(u32::from(lo)) {
                                            s.push(ch);
                                        }
                                    }
                                } else {
                                    // Lone high surrogate.
                                    s.push(char::REPLACEMENT_CHARACTER);
                                }
                            } else if (0xDC00..=0xDFFF).contains(&cp) {
                                // Lone low surrogate.
                                s.push(char::REPLACEMENT_CHARACTER);
                            } else if let Some(ch) = char::from_u32(u32::from(cp)) {
                                s.push(ch);
                            }
                        }
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                Some(b) if b < 0x20 => {
                    return Err(self.err("control character in string"));
                }
                Some(b) => {
                    if b < 0x80 {
                        s.push(char::from(b));
                    } else {
                        // Multi-byte UTF-8. Input is guaranteed valid UTF-8
                        // (&str), so decode the leading byte to determine length.
                        self.pos -= 1;
                        let len = utf8_char_len(b);
                        let end = (self.pos + len).min(self.input.len());
                        let slice = &self.input[self.pos..end];
                        // SAFETY: input originates from &str, so valid UTF-8.
                        if let Ok(ch_str) = std::str::from_utf8(slice) {
                            if let Some(ch) = ch_str.chars().next() {
                                s.push(ch);
                            }
                        }
                        self.pos = end;
                    }
                }
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u16, VmError> {
        let mut val: u16 = 0;
        for _ in 0..4 {
            let b = self
                .advance()
                .ok_or_else(|| self.err("unexpected end in \\uXXXX"))?;
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.err("invalid hex digit in \\uXXXX")),
            };
            val = (val << 4) | u16::from(digit);
        }
        Ok(val)
    }

    fn parse_string(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        let s = self.parse_string_raw()?;
        let id = ctx.intern(&s);
        Ok(JsValue::String(id))
    }

    fn parse_number(&mut self) -> Result<JsValue, VmError> {
        let start = self.pos;

        // Optional leading minus.
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        // Integer part.
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                // No leading zeros allowed (except "0" or "0.xxx").
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.err("expected digit")),
        }

        // Fractional part.
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit after '.'"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }

        // Exponent.
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit in exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }

        let num_str = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| self.err("invalid number"))?;
        let n: f64 = num_str.parse().map_err(|_| self.err("invalid number"))?;
        Ok(JsValue::Number(n))
    }

    fn parse_array(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.expect(b'[')?;
        self.skip_ws();

        let mut elements = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsValue::Object(ctx.alloc_object(Object {
                kind: ObjectKind::Array { elements },
                storage: PropertyStorage::shaped(ROOT_SHAPE),
                prototype: ctx.vm.array_prototype,
            })));
        }

        loop {
            let val = self.parse_value(ctx)?;
            elements.push(val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_ws();
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }

        Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: PropertyStorage::shaped(ROOT_SHAPE),
            prototype: ctx.vm.array_prototype,
        })))
    }

    fn parse_object(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.expect(b'{')?;
        self.skip_ws();

        let obj_id = ctx.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(ROOT_SHAPE),
            prototype: ctx.vm.object_prototype,
        });

        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsValue::Object(obj_id));
        }

        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected string key"));
            }
            let key_str = self.parse_string_raw()?;
            let key_sid = ctx.intern(&key_str);

            self.skip_ws();
            self.expect(b':')?;

            let val = self.parse_value(ctx)?;

            // Duplicate keys: last value wins (ES2020 §24.5.1 + web compat).
            let pk = PropertyKey::String(key_sid);
            let existing_idx = {
                let obj = ctx.get_object(obj_id);
                match &obj.storage {
                    PropertyStorage::Shaped { shape, .. } => {
                        ctx.vm.shapes[*shape as usize].lookup(pk).map(|(i, _)| i)
                    }
                    PropertyStorage::Dictionary(_) => None,
                }
            };
            if let Some(idx) = existing_idx {
                ctx.get_object_mut(obj_id)
                    .storage
                    .set_slot_value(idx, PropertyValue::Data(val));
            } else {
                ctx.vm.define_shaped_property(
                    obj_id,
                    pk,
                    PropertyValue::Data(val),
                    PropertyAttrs::DATA,
                );
            }

            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_ws();
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }

        Ok(JsValue::Object(obj_id))
    }
}

/// §24.5.1.1 InternalizeJSONProperty — apply reviver function.
fn internalize(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    reviver: ObjectId,
) -> Result<JsValue, VmError> {
    match val {
        JsValue::Object(obj_id) => {
            match &ctx.get_object(obj_id).kind {
                ObjectKind::Array { elements } => {
                    let len = elements.len();
                    for i in 0..len {
                        let elem = match &ctx.get_object(obj_id).kind {
                            ObjectKind::Array { elements } => {
                                elements.get(i).copied().unwrap_or(JsValue::Undefined)
                            }
                            _ => JsValue::Undefined,
                        };
                        let new_val = internalize(ctx, elem, reviver)?;
                        let key_str = ctx.intern(&i.to_string());
                        let result = ctx.call_function(
                            reviver,
                            JsValue::Object(obj_id),
                            &[JsValue::String(key_str), new_val],
                        )?;
                        // If reviver returns undefined, set element to undefined.
                        // (Arrays use index assignment, not delete.)
                        if let ObjectKind::Array { elements } = &mut ctx.get_object_mut(obj_id).kind
                        {
                            if i < elements.len() {
                                elements[i] = if matches!(result, JsValue::Undefined) {
                                    JsValue::Undefined
                                } else {
                                    result
                                };
                            }
                        }
                    }
                }
                ObjectKind::Ordinary | ObjectKind::Arguments { .. } => {
                    // Snapshot keys.
                    let keys: Vec<StringId> = ctx
                        .get_object(obj_id)
                        .storage
                        .iter_keys(&ctx.vm.shapes)
                        .filter_map(|(k, _)| match k {
                            PropertyKey::String(s) => Some(s),
                            PropertyKey::Symbol(_) => None,
                        })
                        .collect();

                    for key_sid in keys {
                        let child = ctx.get_property_value(obj_id, PropertyKey::String(key_sid))?;
                        let new_val = internalize(ctx, child, reviver)?;
                        let result = ctx.call_function(
                            reviver,
                            JsValue::Object(obj_id),
                            &[JsValue::String(key_sid), new_val],
                        )?;
                        if matches!(result, JsValue::Undefined) {
                            let _ = ctx
                                .vm
                                .try_delete_property(obj_id, PropertyKey::String(key_sid));
                        } else {
                            ctx.vm
                                .set_property_val(JsValue::Object(obj_id), key_sid, result)?;
                        }
                    }
                }
                _ => {}
            }
            Ok(val)
        }
        _ => Ok(val),
    }
}

/// Entry point for `JSON.parse(text, reviver?)`.
pub(super) fn native_json_parse(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let text_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let text_sid = ctx.to_string_val(text_val)?;
    let text = ctx.get_utf8(text_sid);

    let mut parser = JsonParser::new(&text);
    let result = parser.parse_value(ctx)?;

    // Ensure no trailing non-whitespace.
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        return Err(VmError::syntax_error(format!(
            "JSON.parse: unexpected character at position {}",
            parser.pos
        )));
    }

    // Apply reviver if provided.
    let reviver_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(rev_id) = reviver_arg {
        if ctx.get_object(rev_id).kind.is_callable() {
            // Build wrapper { "": result }.
            let wrapper_id = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(ROOT_SHAPE),
                prototype: None,
            });
            let empty_key = ctx.vm.well_known.empty;
            ctx.vm.define_shaped_property(
                wrapper_id,
                PropertyKey::String(empty_key),
                PropertyValue::Data(result),
                PropertyAttrs::DATA,
            );

            let internalized = internalize(ctx, result, rev_id)?;
            let final_val = ctx.call_function(
                rev_id,
                JsValue::Object(wrapper_id),
                &[JsValue::String(empty_key), internalized],
            )?;
            return Ok(final_val);
        }
    }

    Ok(result)
}
