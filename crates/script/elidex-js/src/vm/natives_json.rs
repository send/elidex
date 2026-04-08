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
            if let Some(JsValue::Object(to_json_obj)) =
                ctx.try_get_property_value(obj_id, to_json_pk)?
            {
                if ctx.get_object(to_json_obj).kind.is_callable() {
                    val = ctx.call_function(to_json_obj, val, &[key])?;
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

            // Only intern the index string when toJSON or replacer needs it.
            // This avoids permanently growing the StringPool for large arrays.
            let needs_key = self.replacer_fn.is_some() || matches!(elem, JsValue::Object(_));
            let key = if needs_key {
                self.index_buf.clear();
                let _ = write!(self.index_buf, "{i}");
                JsValue::String(ctx.intern(&self.index_buf))
            } else {
                JsValue::Undefined
            };
            let wrote = self.serialize_property(ctx, elem, obj_id, key)?;
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
            // Replacer array: only include keys that are own + enumerable (§24.5.2 step 4.b.iii).
            pl.iter()
                .copied()
                .filter(|&sid| {
                    let pk = PropertyKey::String(sid);
                    match ctx.get_object(obj_id).storage.get(pk, &ctx.vm.shapes) {
                        Some((_, attrs)) => attrs.enumerable,
                        None => false,
                    }
                })
                .collect()
        } else {
            // §24.5.2.3 step 5: EnumerableOwnPropertyNames / OrdinaryOwnPropertyKeys.
            // Array-index keys come first in ascending numeric order, then
            // other string keys in insertion order.
            collect_own_keys_es_order(ctx, obj_id)
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

/// Collect own enumerable string keys in ES spec order (§9.1.11.1):
/// array-index keys in ascending numeric order, then other string keys
/// in insertion order.
fn collect_own_keys_es_order(ctx: &NativeContext<'_>, obj_id: ObjectId) -> Vec<StringId> {
    let mut index_keys: Vec<(u32, StringId)> = Vec::new();
    let mut other_keys: Vec<StringId> = Vec::new();

    for (k, attrs) in ctx.get_object(obj_id).storage.iter_keys(&ctx.vm.shapes) {
        if !attrs.enumerable {
            continue;
        }
        let sid = match k {
            PropertyKey::String(s) => s,
            PropertyKey::Symbol(_) => continue,
        };
        match parse_array_index(ctx.get_u16(sid)) {
            Some(idx) => index_keys.push((idx, sid)),
            None => other_keys.push(sid),
        }
    }

    index_keys.sort_by_key(|(idx, _)| *idx);

    let mut keys = Vec::with_capacity(index_keys.len() + other_keys.len());
    keys.extend(index_keys.into_iter().map(|(_, sid)| sid));
    keys.extend(other_keys);
    keys
}

/// Parse a string as an ES array index (0..2^32-2). Returns `None` for
/// non-index strings, leading-zero forms like "01", and out-of-range values.
fn parse_array_index(units: &[u16]) -> Option<u32> {
    if units.is_empty() || units.len() > 10 {
        return None;
    }
    // Reject leading zeros ("00", "01", etc.) but allow "0".
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut val: u64 = 0;
    for &u in units {
        let d = u.wrapping_sub(u16::from(b'0'));
        if d > 9 {
            return None;
        }
        val = val * 10 + u64::from(d);
        if val > u64::from(u32::MAX) - 1 {
            return None;
        }
    }
    Some(val as u32)
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

/// Write a finite f64 to the output buffer in ES Number::toString format
/// (§7.1.12.1).
///
/// ES rules:
/// - Integers with ≤ 20 digits: decimal without exponent
/// - Integers with ≥ 21 digits: exponent form with `e+`
/// - Decimals with fraction: shortest representation, exponent form when
///   the decimal point position falls outside the significant digits
fn write_number(n: f64, output: &mut String) {
    debug_assert!(n.is_finite());
    // -0.0 == 0.0 in IEEE 754; ES §7.1.12.1 step 2 says "0".
    if n == 0.0 {
        output.push('0');
        return;
    }
    if n < 0.0 {
        output.push('-');
        write_number(-n, output);
        return;
    }
    // Use Rust Display for the shortest decimal representation, then
    // re-format according to ES rules.
    let s = format!("{n}");
    // If Rust already chose exponent form, fix the sign format.
    if let Some(e_pos) = s.find('e') {
        let (mantissa, exp_part) = s.split_at(e_pos);
        output.push_str(mantissa);
        output.push('e');
        let exp_rest = &exp_part[1..];
        if !exp_rest.starts_with('-') {
            output.push('+');
        }
        output.push_str(exp_rest);
        return;
    }
    // Rust chose decimal form. Check if ES would use exponent form.
    if s.contains('.') {
        // Has decimal point — Rust already used shortest form. ES uses the
        // same representation for these cases.
        output.push_str(&s);
    } else {
        // Pure integer string from Rust. ES uses exponent form for ≥ 21 digits.
        let digits = s.len();
        // ES §7.1.12.1 step 6: integer form when n ≤ 21.
        // `digits` equals n (number of digits = floor(log10(x)) + 1).
        if digits <= 21 {
            output.push_str(&s);
        } else {
            // e.g. "1000000000000000000000" → "1e+21"
            // Trim trailing zeros to find significant digits.
            let trimmed = s.trim_end_matches('0');
            let sig_len = trimmed.len();
            if sig_len == 1 {
                output.push_str(trimmed);
            } else {
                output.push(trimmed.as_bytes()[0] as char);
                output.push('.');
                output.push_str(&trimmed[1..]);
            }
            let _ = write!(output, "e+{}", digits - 1);
        }
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

    // Build wrapper object: { "": value } (§24.5.2 step 9)
    let wrapper_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
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

/// Recursive-descent JSON parser operating on WTF-16 code units.
///
/// JSON structural characters and keywords are all ASCII, so each `u16`
/// in the range 0-127 is compared directly.  Non-ASCII code units only
/// appear inside string values and are preserved as-is (including lone
/// surrogates), matching JS `JSON.parse` semantics.
struct JsonParser<'a> {
    input: &'a [u16],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u16]) -> Self {
        Self { input, pos: 0 }
    }

    fn err(&self, msg: &str) -> VmError {
        VmError::syntax_error(format!("JSON.parse: {msg} at position {}", self.pos))
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                0x20 | 0x09 | 0x0A | 0x0D => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Peek the current code unit as a `u16`.
    fn peek_u16(&self) -> Option<u16> {
        self.input.get(self.pos).copied()
    }

    /// Advance and return the current code unit.
    fn advance_u16(&mut self) -> Option<u16> {
        let c = self.input.get(self.pos).copied()?;
        self.pos += 1;
        Some(c)
    }

    /// Peek as ASCII byte (returns `None` for non-ASCII code units).
    fn peek(&self) -> Option<u8> {
        let c = self.peek_u16()?;
        if c <= 0x7F {
            Some(c as u8)
        } else {
            None
        }
    }

    /// Advance and return as ASCII byte (`None` for non-ASCII).
    fn advance(&mut self) -> Option<u8> {
        let c = self.peek_u16()?;
        self.pos += 1;
        if c <= 0x7F {
            Some(c as u8)
        } else {
            None
        }
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
            Some(_) | None => {
                // Non-ASCII code unit or end of input.
                if self.pos < self.input.len() {
                    Err(self.err("unexpected character"))
                } else {
                    Err(self.err("unexpected end of input"))
                }
            }
        }
    }

    /// Parse a JSON string into WTF-16 code units, preserving lone surrogates.
    fn parse_string_u16(&mut self) -> Result<Vec<u16>, VmError> {
        self.expect(b'"')?;
        let mut s: Vec<u16> = Vec::new();
        loop {
            match self.advance_u16() {
                None => return Err(self.err("unterminated string")),
                Some(0x22) => return Ok(s), // '"'
                Some(0x5C) => {
                    // '\\'
                    match self.advance() {
                        Some(b'"') => s.push(0x22),
                        Some(b'\\') => s.push(0x5C),
                        Some(b'/') => s.push(0x2F),
                        Some(b'b') => s.push(0x08),
                        Some(b'f') => s.push(0x0C),
                        Some(b'n') => s.push(0x0A),
                        Some(b'r') => s.push(0x0D),
                        Some(b't') => s.push(0x09),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            // Preserve raw UTF-16 code units (including lone surrogates).
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // High surrogate — check for trailing low surrogate.
                                if self.peek() == Some(b'\\')
                                    && self.input.get(self.pos + 1) == Some(&u16::from(b'u'))
                                {
                                    let saved = self.pos;
                                    self.pos += 2; // skip \u
                                    let lo = self.parse_hex4()?;
                                    if (0xDC00..=0xDFFF).contains(&lo) {
                                        s.push(cp);
                                        s.push(lo);
                                    } else {
                                        // Not a valid pair — rewind, keep high surrogate.
                                        self.pos = saved;
                                        s.push(cp);
                                    }
                                } else {
                                    s.push(cp);
                                }
                            } else {
                                // BMP scalar or lone low surrogate — preserve as-is.
                                s.push(cp);
                            }
                        }
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                Some(c) if c < 0x20 => {
                    return Err(self.err("control character in string"));
                }
                Some(c) => {
                    // Regular character (including non-ASCII / surrogates from input).
                    s.push(c);
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
        let units = self.parse_string_u16()?;
        let id = ctx.intern_utf16(&units);
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

        // Number tokens are pure ASCII — collect into a small buffer.
        let num_str: String = self.input[start..self.pos]
            .iter()
            .map(|&c| char::from(c as u8))
            .collect();
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
            let key_units = self.parse_string_u16()?;
            let key_sid = ctx.intern_utf16(&key_units);

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
    index_buf: &mut String,
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
                        let new_val = internalize(ctx, elem, reviver, index_buf)?;
                        index_buf.clear();
                        let _ = write!(index_buf, "{i}");
                        let key_str = ctx.intern(index_buf);
                        let result = ctx.call_function(
                            reviver,
                            JsValue::Object(obj_id),
                            &[JsValue::String(key_str), new_val],
                        )?;
                        // Spec says [[Delete]] for undefined, but our dense Vec<JsValue>
                        // arrays don't support holes. Assign undefined instead.
                        // This is observable via `in` but matches practical behavior.
                        if let ObjectKind::Array { elements } = &mut ctx.get_object_mut(obj_id).kind
                        {
                            if i < elements.len() {
                                elements[i] = result;
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
                        let new_val = internalize(ctx, child, reviver, index_buf)?;
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
    let text: Vec<u16> = ctx.get_u16(text_sid).to_vec();

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
            // Build wrapper { "": result } (§24.5.1 step 7)
            let wrapper_id = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(ROOT_SHAPE),
                prototype: ctx.vm.object_prototype,
            });
            let empty_key = ctx.vm.well_known.empty;
            ctx.vm.define_shaped_property(
                wrapper_id,
                PropertyKey::String(empty_key),
                PropertyValue::Data(result),
                PropertyAttrs::DATA,
            );

            let mut index_buf = String::with_capacity(8);
            let internalized = internalize(ctx, result, rev_id, &mut index_buf)?;
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
