//! Native (Rust-implemented) JS built-in functions.
//!
//! These are free functions with the `NativeFn` signature, referenced by name
//! in `globals.rs` when registering built-in objects.
//!
//! Object-related built-ins are in `natives_object.rs`.

use super::natives_array::create_array;
use super::value::{JsValue, NativeContext, ObjectKind, PropertyKey, VmError};
use super::VmInner;

// -- Global functions -------------------------------------------------------

pub(super) fn native_parse_int(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val)?;
    let s = super::coerce::trim_js(&ctx.get_utf8(s_id)).to_string();

    // ES2020 §18.2.5: strip sign first, then detect 0x prefix.
    let mut negative = false;
    let mut rest = s.as_str();
    if let Some(r) = rest.strip_prefix('-') {
        negative = true;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('+') {
        rest = r;
    }

    let radix_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let (radix, rest) = if matches!(radix_arg, JsValue::Undefined) {
        if let Some(r) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
            (16u32, r)
        } else {
            (10u32, rest)
        }
    } else {
        let r = ctx.to_number(radix_arg)?;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let ri = r as i32;
        // ES2020 §18.2.5: radix 0 (or undefined) → default (10, with 0x prefix detection).
        if ri == 0 {
            if let Some(r2) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
                (16u32, r2)
            } else {
                (10u32, rest)
            }
        } else if !(2..=36).contains(&ri) {
            return Ok(JsValue::Number(f64::NAN));
        } else {
            #[allow(clippy::cast_sign_loss)]
            let radix = ri as u32;
            let rest = if radix == 16 {
                rest.strip_prefix("0x")
                    .or_else(|| rest.strip_prefix("0X"))
                    .unwrap_or(rest)
            } else {
                rest
            };
            (radix, rest)
        }
    };

    // Parse digits in `radix`, stopping at the first invalid char.
    let mut result = 0.0f64;
    let mut any_digit = false;
    for c in rest.chars() {
        let digit = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 10,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        any_digit = true;
        result = result * f64::from(radix) + f64::from(digit);
    }
    if !any_digit {
        return Ok(JsValue::Number(f64::NAN));
    }
    if negative {
        result = -result;
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_parse_float(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val)?;
    let trimmed = super::coerce::trim_js(&ctx.get_utf8(s_id)).to_string();
    let n = parse_float_prefix(&trimmed);
    Ok(JsValue::Number(n))
}

/// Parse the longest valid float prefix from a string (ES2020 `parseFloat` semantics).
///
/// Recognises `[+-]? digits [. digits] [eE [+-] digits]` and the literal
/// `Infinity` / `+Infinity` / `-Infinity`. Rejects Rust-specific literals
/// such as `inf`, `nan`, etc.
fn parse_float_prefix(s: &str) -> f64 {
    if s.is_empty() {
        return f64::NAN;
    }

    // Check for Infinity literals (the only non-numeric token parseFloat accepts).
    if s.strip_prefix("Infinity").is_some() {
        return f64::INFINITY;
    }
    if s.strip_prefix("+Infinity").is_some() {
        return f64::INFINITY;
    }
    if s.starts_with("-Infinity") {
        return f64::NEG_INFINITY;
    }

    // Scan the longest valid numeric prefix: [+-]? digits [. digits] [eE [+-] digits]
    let bytes = s.as_bytes();
    let mut i = 0;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Integer digits
    let mut has_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digit = true;
        i += 1;
    }

    // Decimal point + fraction.  Digits on either side of the dot are
    // sufficient — `.5`, `5.`, and `5.5` are all valid.
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        }
    }

    // Must have consumed at least one digit (a bare "." or sign is invalid).
    if !has_digit {
        return f64::NAN;
    }

    // Exponent part
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let save = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            // No digits after 'e', roll back to before the exponent.
            i = save;
        }
    }

    s[..i].parse::<f64>().unwrap_or(f64::NAN)
}

pub(super) fn native_is_nan(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Boolean(n.is_nan()))
}

pub(super) fn native_is_finite(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Boolean(n.is_finite()))
}

// -- URI encoding/decoding (§18.2.6) ----------------------------------------

/// Characters that encodeURI does NOT encode (by code point value).
fn is_uri_unescaped(cp: u32) -> bool {
    matches!(cp,
        0x41..=0x5A | 0x61..=0x7A | 0x30..=0x39 // A-Z a-z 0-9
        | 0x2D | 0x5F | 0x2E | 0x21 | 0x7E | 0x2A | 0x27 | 0x28 | 0x29) // -_.!~*'()
}

fn is_uri_reserved(cp: u32) -> bool {
    matches!(
        cp,
        0x3B | 0x2F | 0x3F | 0x3A | 0x40 | 0x26 | 0x3D | 0x2B | 0x24 | 0x2C | 0x23
    )
    // ; / ? : @ & = + $ , #
}

pub(super) fn native_encode_uri(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = ctx.to_string_val(val)?;
    let units = ctx.vm.strings.get(sid).to_vec();
    let encoded = encode_uri_wtf16(&units, |cp| is_uri_unescaped(cp) || is_uri_reserved(cp))?;
    let id = ctx.intern(&encoded);
    Ok(JsValue::String(id))
}

pub(super) fn native_decode_uri(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = ctx.to_string_val(val)?;
    let units = ctx.vm.strings.get(sid).to_vec();
    let decoded = decode_uri_wtf16(&units, is_uri_reserved)?;
    let id = ctx.vm.strings.intern_utf16(&decoded);
    Ok(JsValue::String(id))
}

pub(super) fn native_encode_uri_component(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = ctx.to_string_val(val)?;
    let units = ctx.vm.strings.get(sid).to_vec();
    let encoded = encode_uri_wtf16(&units, is_uri_unescaped)?;
    let id = ctx.intern(&encoded);
    Ok(JsValue::String(id))
}

pub(super) fn native_decode_uri_component(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = ctx.to_string_val(val)?;
    let units = ctx.vm.strings.get(sid).to_vec();
    let decoded = decode_uri_wtf16(&units, |_| false)?;
    let id = ctx.vm.strings.intern_utf16(&decoded);
    Ok(JsValue::String(id))
}

/// Encode a WTF-16 string per ES2020 §18.2.6.1. Lone surrogates throw URIError.
fn encode_uri_wtf16(units: &[u16], is_unescaped: impl Fn(u32) -> bool) -> Result<String, VmError> {
    let mut out = String::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let cu = units[i];
        let cp = if (0xD800..=0xDBFF).contains(&cu) {
            // High surrogate — must be followed by low surrogate
            if i + 1 < units.len() && (0xDC00..=0xDFFF).contains(&units[i + 1]) {
                let hi = u32::from(cu);
                let lo = u32::from(units[i + 1]);
                i += 1;
                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
            } else {
                return Err(VmError::uri_error("URI malformed"));
            }
        } else if (0xDC00..=0xDFFF).contains(&cu) {
            // Lone low surrogate
            return Err(VmError::uri_error("URI malformed"));
        } else {
            u32::from(cu)
        };
        i += 1;

        if is_unescaped(cp) {
            // Safe: cp is ASCII when is_unescaped returns true
            out.push(char::from(cp as u8));
        } else {
            // Encode as UTF-8 bytes, each percent-encoded
            let mut buf = [0u8; 4];
            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
            let utf8 = ch.encode_utf8(&mut buf);
            for &b in utf8.as_bytes() {
                out.push('%');
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0xF));
            }
        }
    }
    Ok(out)
}

/// Decode a percent-encoded WTF-16 string per ES2020 §18.2.6.2.
fn decode_uri_wtf16(
    units: &[u16],
    keep_encoded: impl Fn(u32) -> bool,
) -> Result<Vec<u16>, VmError> {
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        if units[i] == u16::from(b'%') {
            // Decode %XX sequence(s) → UTF-8 bytes → code point → UTF-16
            if i + 2 >= units.len() {
                return Err(VmError::uri_error("URI malformed"));
            }
            let hi =
                decode_hex_u16(units[i + 1]).ok_or_else(|| VmError::uri_error("URI malformed"))?;
            let lo =
                decode_hex_u16(units[i + 2]).ok_or_else(|| VmError::uri_error("URI malformed"))?;
            let mut utf8_bytes = vec![(hi << 4) | lo];
            i += 3;
            // Multi-byte UTF-8: collect continuation bytes
            if utf8_bytes[0] >= 0x80 {
                let expected_len = match utf8_bytes[0] {
                    0xC0..=0xDF => 2,
                    0xE0..=0xEF => 3,
                    0xF0..=0xF7 => 4,
                    _ => 1,
                };
                while utf8_bytes.len() < expected_len
                    && i < units.len()
                    && units[i] == u16::from(b'%')
                {
                    if i + 2 >= units.len() {
                        return Err(VmError::uri_error("URI malformed"));
                    }
                    let h = decode_hex_u16(units[i + 1])
                        .ok_or_else(|| VmError::uri_error("URI malformed"))?;
                    let l = decode_hex_u16(units[i + 2])
                        .ok_or_else(|| VmError::uri_error("URI malformed"))?;
                    utf8_bytes.push((h << 4) | l);
                    i += 3;
                }
            }
            // Decode UTF-8 bytes to a code point
            let s = std::str::from_utf8(&utf8_bytes)
                .map_err(|_| VmError::uri_error("URI malformed"))?;
            for ch in s.chars() {
                let cp = ch as u32;
                if keep_encoded(cp) {
                    // Re-encode: emit %XX for each UTF-8 byte
                    let mut buf = [0u8; 4];
                    let encoded = ch.encode_utf8(&mut buf);
                    for &b in encoded.as_bytes() {
                        out.push(u16::from(b'%'));
                        out.push(u16::from(hex_digit(b >> 4) as u8));
                        out.push(u16::from(hex_digit(b & 0xF) as u8));
                    }
                } else if cp <= 0xFFFF {
                    out.push(cp as u16);
                } else {
                    // Supplementary: encode as surrogate pair
                    let adjusted = cp - 0x10000;
                    out.push((0xD800 + (adjusted >> 10)) as u16);
                    out.push((0xDC00 + (adjusted & 0x3FF)) as u16);
                }
            }
        } else {
            out.push(units[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + n - 10) as char,
    }
}

fn decode_hex_u16(unit: u16) -> Option<u8> {
    let b = u8::try_from(unit).ok()?;
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

// -- Error constructors -----------------------------------------------------

/// §19.5.3.4 Error.prototype.toString.
/// Build `"<name>: <message>"` from the instance's own `.name` / `.message`
/// properties (falling back to "Error" / "").  Each field goes through
/// `ToString` so that non-String values (e.g. `name = 42`) coerce per spec.
pub(super) fn native_error_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Err(VmError::type_error(
            "Error.prototype.toString requires an Object",
        ));
    };
    let name_key = PropertyKey::String(ctx.vm.well_known.name);
    let msg_key = PropertyKey::String(ctx.vm.well_known.message);
    let name_sid = match ctx.try_get_property_value(obj_id, name_key)? {
        None | Some(JsValue::Undefined) => ctx.intern("Error"),
        Some(v) => ctx.to_string_val(v)?,
    };
    let msg_sid = match ctx.try_get_property_value(obj_id, msg_key)? {
        None | Some(JsValue::Undefined) => ctx.vm.well_known.empty,
        Some(v) => ctx.to_string_val(v)?,
    };
    let name_units = ctx.vm.strings.get(name_sid).to_vec();
    let msg_units = ctx.vm.strings.get(msg_sid).to_vec();
    // §19.5.3.4 steps 7-9: empty name → msg; empty msg → name; else name + ": " + msg.
    let result_id = if name_units.is_empty() {
        msg_sid
    } else if msg_units.is_empty() {
        name_sid
    } else {
        let mut units = Vec::with_capacity(name_units.len() + 2 + msg_units.len());
        units.extend_from_slice(&name_units);
        units.push(u16::from(b':'));
        units.push(u16::from(b' '));
        units.extend_from_slice(&msg_units);
        ctx.vm.strings.intern_utf16(&units)
    };
    Ok(JsValue::String(result_id))
}

fn error_ctor_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    error_name: &str,
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        let name_key = PropertyKey::String(ctx.vm.well_known.name);
        let name_val = JsValue::String(ctx.intern(error_name));
        ctx.vm.define_shaped_property(
            id,
            name_key,
            super::value::PropertyValue::Data(name_val),
            super::shape::PropertyAttrs::DATA,
        );
        // §19.5.1.1 step 4: only set `message` when the argument is not
        // undefined.  Otherwise, `.message` falls through to
        // Error.prototype.message (which is the empty string).
        if let Some(&msg) = args.first() {
            if !matches!(msg, JsValue::Undefined) {
                let msg_id = ctx.to_string_val(msg)?;
                let msg_key = PropertyKey::String(ctx.vm.well_known.message);
                ctx.vm.define_shaped_property(
                    id,
                    msg_key,
                    super::value::PropertyValue::Data(JsValue::String(msg_id)),
                    super::shape::PropertyAttrs::DATA,
                );
            }
        }
    }
    Ok(this)
}

pub(super) fn native_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "Error")
}

pub(super) fn native_type_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "TypeError")
}

pub(super) fn native_reference_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "ReferenceError")
}

pub(super) fn native_range_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "RangeError")
}

pub(super) fn native_syntax_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "SyntaxError")
}

pub(super) fn native_uri_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    error_ctor_impl(ctx, this, args, "URIError")
}

// -- Array constructor & static methods --------------------------------------

/// `Array(n)` / `Array(a, b, c)` constructor (ES2020 §22.1.1).
use super::ops::DENSE_ARRAY_LEN_LIMIT;

pub(super) fn native_array_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let elements = if args.len() == 1 {
        if let JsValue::Number(n) = args[0] {
            // Single numeric arg → sparse array of that length.
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let len = n as u32;
            #[allow(clippy::cast_possible_truncation)]
            if n < 0.0
                || !n.is_finite()
                || f64::from(len) != n
                || (len as usize) >= DENSE_ARRAY_LEN_LIMIT
            {
                return Err(VmError::range_error("Invalid array length"));
            }
            #[allow(clippy::cast_possible_truncation)]
            let len_usize = len as usize;
            let mut elems = Vec::new();
            elems
                .try_reserve_exact(len_usize)
                .map_err(|_| VmError::range_error("Array allocation failed"))?;
            elems.resize(len_usize, JsValue::Empty);
            elems
        } else {
            // Single non-numeric arg → array with that element.
            vec![args[0]]
        }
    } else {
        // Zero or 2+ args → array of those elements.
        args.to_vec()
    };
    // `new Array(...)`: reuse the Ordinary instance pre-allocated by do_new
    // (avoids a second allocation).  Plain `Array(...)` call mode allocates
    // fresh via create_array.
    if ctx.is_construct() {
        if let JsValue::Object(instance_id) = this {
            ctx.vm.promote_to_array(instance_id, elements);
            return Ok(JsValue::Object(instance_id));
        }
    }
    Ok(create_array(ctx, elements))
}

pub(super) fn native_array_is_array(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = if let JsValue::Object(id) = val {
        matches!(ctx.get_object(id).kind, ObjectKind::Array { .. })
    } else {
        false
    };
    Ok(JsValue::Boolean(result))
}

// -- Console ----------------------------------------------------------------

fn format_value_for_console(vm: &mut VmInner, val: JsValue) -> String {
    let id = super::coerce::to_display_string(vm, val);
    vm.strings.get_utf8(id)
}

fn console_output(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    prefix: &str,
) -> Result<JsValue, VmError> {
    let parts: Vec<String> = args
        .iter()
        .map(|v| format_value_for_console(ctx.vm, *v))
        .collect();
    eprintln!("{prefix}{}", parts.join(" "));
    Ok(JsValue::Undefined)
}

pub(super) fn native_console_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "")
}

pub(super) fn native_console_error(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "[error] ")
}

pub(super) fn native_console_warn(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    console_output(ctx, args, "[warn] ")
}

// Re-exports from split modules.
pub(super) use super::natives_object::{
    native_object_assign, native_object_create, native_object_define_property,
    native_object_entries, native_object_freeze, native_object_from_entries,
    native_object_get_own_property_descriptor, native_object_get_own_property_names,
    native_object_get_own_property_symbols, native_object_get_prototype_of,
    native_object_has_own_property, native_object_is, native_object_is_extensible,
    native_object_is_frozen, native_object_is_prototype_of, native_object_is_sealed,
    native_object_keys, native_object_prevent_extensions, native_object_property_is_enumerable,
    native_object_seal, native_object_set_prototype_of, native_object_value_of,
    native_object_values,
};
pub(super) use super::natives_string::{
    native_string_char_at, native_string_char_code_at, native_string_ends_with,
    native_string_includes, native_string_index_of, native_string_match, native_string_replace,
    native_string_search, native_string_slice, native_string_split, native_string_starts_with,
    native_string_substring, native_string_to_lower_case, native_string_to_upper_case,
    native_string_trim,
};
pub(super) use super::natives_symbol::{
    native_array_iterator_next, native_array_values, native_iterator_self,
    native_object_prototype_to_locale_string, native_object_prototype_to_string,
    native_string_iterator, native_string_iterator_next, native_symbol_constructor,
    native_symbol_for, native_symbol_key_for, native_symbol_prototype_to_string,
};
