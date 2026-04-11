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

    // Decimal point + fraction (`.5` is valid — digits may appear only after the dot)
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

// -- Error constructors -----------------------------------------------------

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
        let msg = args
            .first()
            .copied()
            .unwrap_or(JsValue::String(ctx.vm.well_known.empty));
        let msg_id = ctx.to_string_val(msg)?;
        let msg_key = PropertyKey::String(ctx.vm.well_known.message);
        ctx.vm.define_shaped_property(
            id,
            msg_key,
            super::value::PropertyValue::Data(JsValue::String(msg_id)),
            super::shape::PropertyAttrs::DATA,
        );
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

// -- Array constructor & static methods --------------------------------------

/// `Array(n)` / `Array(a, b, c)` constructor (ES2020 §22.1.1).
use super::ops::DENSE_ARRAY_LEN_LIMIT;

pub(super) fn native_array_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
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

// -- Math methods -----------------------------------------------------------

pub(super) fn native_math_abs(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.abs()))
}

pub(super) fn native_math_ceil(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ceil()))
}

pub(super) fn native_math_floor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.floor()))
}

pub(super) fn native_math_round(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    // ES2020 §20.2.2.28: if n is in [-0.5, 0), result is -0.
    let result = if (-0.5..0.0).contains(&n) {
        -0.0_f64
    } else {
        (n + 0.5).floor()
    };
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_max(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::NEG_INFINITY));
    }
    let mut result = f64::NEG_INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n > result {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_min(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::INFINITY));
    }
    let mut result = f64::INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n < result {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_random(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // xorshift64 PRNG — not cryptographically secure but sufficient for
    // Math.random(). State is stored in VmInner so successive calls produce
    // distinct values.
    let mut s = ctx.vm.rng_state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    ctx.vm.rng_state = s;
    // The shift produces a 53-bit value that fits in f64's mantissa exactly.
    #[allow(clippy::cast_precision_loss)]
    let n = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(JsValue::Number(n))
}

pub(super) fn native_math_sqrt(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.sqrt()))
}

pub(super) fn native_math_pow(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let exp = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(base.powf(exp)))
}

pub(super) fn native_math_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ln()))
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
    native_object_prototype_to_string, native_string_iterator, native_string_iterator_next,
    native_symbol_constructor, native_symbol_for, native_symbol_key_for,
    native_symbol_prototype_to_string,
};
