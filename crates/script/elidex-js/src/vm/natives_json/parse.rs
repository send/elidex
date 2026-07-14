//! `JSON.parse` (ECMA-262 §25.5.2) — the parser half.

use std::fmt::Write;

use super::super::coerce_format::collect_own_keys_es_order;
use super::super::natives_array::create_array;
use super::super::ops::DENSE_ARRAY_LEN_LIMIT;
use super::super::shape::{PropertyAttrs, ROOT_SHAPE};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::VmInner;
use super::MAX_JSON_DEPTH;

/// Recursive-descent JSON parser operating on WTF-16 code units.
///
/// JSON structural characters and keywords are all ASCII, so each `u16`
/// in the range 0-127 is compared directly.  Non-ASCII code units only
/// appear inside string values and are preserved as-is (including lone
/// surrogates), matching JS `JSON.parse` semantics.
struct JsonParser<'a> {
    input: &'a [u16],
    pos: usize,
    /// Current nesting depth during parse; capped at MAX_JSON_DEPTH to
    /// prevent Rust-stack exhaustion from deeply nested input like
    /// `"[[[[[...]]]]]"`.
    depth: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u16]) -> Self {
        Self {
            input,
            pos: 0,
            depth: 0,
        }
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

        // Number tokens are pure ASCII (guarded by peek() checks above).
        let num_str: String = self.input[start..self.pos]
            .iter()
            .map(|&c| {
                debug_assert!(c <= 0x7F, "non-ASCII in number token");
                char::from(c as u8)
            })
            .collect();
        let n: f64 = num_str.parse().map_err(|_| self.err("invalid number"))?;
        Ok(JsValue::Number(n))
    }

    fn parse_array(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.depth += 1;
        if self.depth > MAX_JSON_DEPTH {
            return Err(VmError::range_error("Maximum JSON nesting depth exceeded"));
        }
        let result = self.parse_array_inner(ctx);
        self.depth -= 1;
        result
    }

    fn parse_array_inner(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.expect(b'[')?;
        self.skip_ws();

        let mut elements = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(create_array(ctx, elements));
        }

        loop {
            let val = self.parse_value(ctx)?;
            if elements.len() >= DENSE_ARRAY_LEN_LIMIT {
                return Err(VmError::range_error("Array allocation failed"));
            }
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

        Ok(create_array(ctx, elements))
    }

    fn parse_object(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.depth += 1;
        if self.depth > MAX_JSON_DEPTH {
            return Err(VmError::range_error("Maximum JSON nesting depth exceeded"));
        }
        let result = self.parse_object_inner(ctx);
        self.depth -= 1;
        result
    }

    fn parse_object_inner(&mut self, ctx: &mut NativeContext<'_>) -> Result<JsValue, VmError> {
        self.expect(b'{')?;
        self.skip_ws();

        let obj_id = ctx.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(ROOT_SHAPE),
            prototype: ctx.vm.object_prototype,
            extensible: true,
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

            // Duplicate keys: last value wins (ECMA-262 §25.5.2 + web compat).
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

/// §25.5.2.4 InternalizeJSONProperty — apply reviver function.
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
                        #[allow(clippy::cast_precision_loss)]
                        let elem = ctx
                            .vm
                            .get_element(JsValue::Object(obj_id), JsValue::Number(i as f64))?;
                        let new_val = internalize(ctx, elem, reviver, index_buf)?;
                        index_buf.clear();
                        let _ = write!(index_buf, "{i}");
                        let key_str = ctx.intern(index_buf);
                        let result = ctx.call_function(
                            reviver,
                            JsValue::Object(obj_id),
                            &[JsValue::String(key_str), new_val],
                        )?;
                        // Spec §25.5.2.4: reviver returning undefined → [[Delete]].
                        // With JsValue::Empty this creates a proper sparse hole.
                        // JS code cannot produce Empty, so only check Undefined.
                        debug_assert!(!result.is_empty(), "reviver should never return Empty");
                        if let ObjectKind::Array { elements } = &mut ctx.get_object_mut(obj_id).kind
                        {
                            if i < elements.len() {
                                elements[i] = if matches!(result, JsValue::Undefined) {
                                    JsValue::Empty
                                } else {
                                    result
                                };
                            }
                        }
                    }
                }
                ObjectKind::Ordinary | ObjectKind::Arguments { .. } => {
                    // Snapshot keys in ES spec order (§25.5.2.4 step 5).
                    let keys = collect_own_keys_es_order(ctx.vm, obj_id)?;

                    for key_sid in keys {
                        let child = ctx.get_property_value(obj_id, PropertyKey::String(key_sid))?;
                        let new_val = internalize(ctx, child, reviver, index_buf)?;
                        let result = ctx.call_function(
                            reviver,
                            JsValue::Object(obj_id),
                            &[JsValue::String(key_sid), new_val],
                        )?;
                        if matches!(result, JsValue::Undefined) {
                            // §25.5.2.4 step 3.d: `? O.[[Delete]](P)`.
                            // Observable only via Proxy deleteProperty trap
                            // (not yet implemented), but propagate for spec
                            // conformance once Proxy lands.
                            ctx.vm
                                .try_delete_property(obj_id, PropertyKey::String(key_sid))?;
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
/// Rust `&str` helper equivalent to `JSON.parse(text)` with **no reviver**
/// (ECMA-262 §25.5.2 parse core only), **without interning the transient source
/// text** into the permanent, deduped `StringPool`. This is *not* the JS-facing
/// `JSON.parse` entry point — that is [`native_json_parse`] below, which takes
/// an interned JS string and supports the optional reviver argument.
///
/// `native_json_parse` requires its input as an already-interned JS string;
/// routing high-cardinality transient payloads (the cross-thread worker
/// `postMessage` JSON blobs, WHATWG HTML §10.2.1.2) through it would intern
/// every distinct blob permanently → unbounded pool growth. Parsed *string
/// values* are still interned by the parser (they become live JS strings, which
/// is correct); only the one-shot wrapper blob is kept out of the pool.
pub(in crate::vm) fn parse_json_str(vm: &mut VmInner, source: &str) -> Result<JsValue, VmError> {
    let text: Vec<u16> = source.encode_utf16().collect();
    let mut parser = JsonParser::new(&text);
    let mut ctx = NativeContext::new_call(vm);
    let result = parser.parse_value(&mut ctx)?;
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        return Err(VmError::syntax_error(format!(
            "JSON.parse: unexpected character at position {}",
            parser.pos
        )));
    }
    Ok(result)
}

pub(in crate::vm) fn native_json_parse(
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
            // Build wrapper { "": result } (§25.5.2 step 7)
            let wrapper_id = ctx.alloc_object(Object {
                kind: ObjectKind::Ordinary,
                storage: PropertyStorage::shaped(ROOT_SHAPE),
                prototype: ctx.vm.object_prototype,
                extensible: true,
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
