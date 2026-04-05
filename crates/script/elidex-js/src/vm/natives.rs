//! Native (Rust-implemented) JS built-in functions.
//!
//! These are free functions with the `NativeFn` signature, referenced by name
//! in `globals.rs` when registering built-in objects.

use super::value::{
    ArrayIterState, JsValue, NativeContext, NativeFunction, Object, ObjectKind, Property,
    PropertyKey, StringId, VmError,
};
use super::VmInner;
use crate::wtf16::{
    ends_with_u16, find_u16, starts_with_u16, to_lower_u16, to_upper_u16, trim_u16,
};

// -- Global functions -------------------------------------------------------

pub(super) fn native_parse_int(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val);
    let s = ctx.get_utf8(s_id).trim().to_string();

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
        let r = ctx.to_number(radix_arg);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let ri = r as i32;
        // ES2020 §18.2.5: radix 0 (or undefined) → default (10, with 0x prefix detection).
        if ri == 0 {
            if let Some(r2) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
                (16u32, r2)
            } else {
                (10u32, rest)
            }
        } else if r.is_nan() || !(2..=36).contains(&ri) {
            return Ok(JsValue::Number(f64::NAN));
        } else {
            let ru = ri.cast_unsigned();
            let rest = if ru == 16 {
                rest.strip_prefix("0x")
                    .or_else(|| rest.strip_prefix("0X"))
                    .unwrap_or(rest)
            } else {
                rest
            };
            (ru, rest)
        }
    };

    if !(2..=36).contains(&radix) {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Parse as many valid digits as possible (prefix parsing).
    let mut result: f64 = 0.0;
    let mut found = false;
    let chars = rest.chars();

    for ch in chars {
        let Some(digit) = ch.to_digit(radix) else {
            break;
        };
        found = true;
        result = result * f64::from(radix) + f64::from(digit);
    }

    if !found {
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
    let s_id = ctx.to_string_val(val);
    let trimmed = ctx.get_utf8(s_id).trim_start().to_string();
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
    if let Some(rest) = s.strip_prefix("Infinity") {
        let _ = rest;
        return f64::INFINITY;
    }
    if let Some(rest) = s.strip_prefix("+Infinity") {
        let _ = rest;
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
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(val);
    Ok(JsValue::Boolean(n.is_nan()))
}

pub(super) fn native_is_finite(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(val);
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
        ctx.get_object_mut(id)
            .properties
            .push((name_key, Property::data(name_val)));
        let msg = args
            .first()
            .copied()
            .unwrap_or(JsValue::String(ctx.vm.well_known.empty));
        let msg_id = ctx.to_string_val(msg);
        let msg_key = PropertyKey::String(ctx.vm.well_known.message);
        ctx.get_object_mut(id)
            .properties
            .push((msg_key, Property::data(JsValue::String(msg_id))));
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

// -- Object static methods --------------------------------------------------

pub(super) fn native_object_keys(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let keys: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(_, p)| p.enumerable)
        .filter_map(|(k, _)| {
            if let PropertyKey::String(sid) = k {
                Some(JsValue::String(*sid))
            } else {
                None
            }
        })
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: keys },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

pub(super) fn native_object_values(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let values: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(k, p)| p.enumerable && matches!(k, PropertyKey::String(_)))
        .map(|(_, p)| p.value)
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: values },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

pub(super) fn native_object_assign(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let target = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_id) = target else {
        return Ok(target);
    };
    for &source in args.iter().skip(1) {
        let JsValue::Object(src_id) = source else {
            continue;
        };
        // Collect source properties first to avoid borrow conflict.
        let props: Vec<(PropertyKey, JsValue)> = ctx
            .get_object(src_id)
            .properties
            .iter()
            .filter(|(_, p)| p.enumerable)
            .map(|(k, p)| (*k, p.value))
            .collect();
        for (key, value) in props {
            // Update existing or push new.
            let target_obj = ctx.get_object_mut(target_id);
            if let Some(prop) = target_obj.properties.iter_mut().find(|(k, _)| *k == key) {
                prop.1.value = value;
            } else {
                target_obj.properties.push((key, Property::data(value)));
            }
        }
    }
    Ok(target)
}

pub(super) fn native_object_create(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let proto = args.first().copied().unwrap_or(JsValue::Null);
    let prototype = if let JsValue::Object(id) = proto {
        Some(id)
    } else {
        None
    };
    let obj_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        properties: Vec::new(),
        prototype,
    });
    Ok(JsValue::Object(obj_id))
}

pub(super) fn native_object_define_property(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let prop_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let desc_val = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_id) = obj_val else {
        return Err(VmError::type_error(
            "Object.defineProperty called on non-object",
        ));
    };
    let key = PropertyKey::String(ctx.to_string_val(prop_val));

    // Extract value from descriptor if it's an object.
    let value = if let JsValue::Object(desc_id) = desc_val {
        let value_key = PropertyKey::String(ctx.intern("value"));
        ctx.get_object(desc_id)
            .properties
            .iter()
            .find(|(k, _)| *k == value_key)
            .map_or(JsValue::Undefined, |(_, p)| p.value)
    } else {
        JsValue::Undefined
    };

    let obj = ctx.get_object_mut(obj_id);
    if let Some(prop) = obj.properties.iter_mut().find(|(k, _)| *k == key) {
        prop.1.value = value;
    } else {
        obj.properties.push((key, Property::data(value)));
    }
    Ok(obj_val)
}

// -- Array static methods ---------------------------------------------------

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
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(n.abs()))
}

pub(super) fn native_math_ceil(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(n.ceil()))
}

pub(super) fn native_math_floor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(n.floor()))
}

pub(super) fn native_math_round(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
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
        let n = ctx.to_number(arg);
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
        let n = ctx.to_number(arg);
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
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(n.sqrt()))
}

pub(super) fn native_math_pow(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    let exp = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(base.powf(exp)))
}

pub(super) fn native_math_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Number(n.ln()))
}

// -- String.prototype methods -----------------------------------------------

/// Helper: extract the `StringId` from `this` for String.prototype methods.
fn this_string_id(this: JsValue) -> Option<StringId> {
    if let JsValue::String(id) = this {
        Some(id)
    } else {
        None
    }
}

/// Convert an f64 to a non-negative integer index per ES2020 `ToInteger`.
/// Returns `None` for negative values (meaning "out of range").
/// `NaN` maps to `Some(0)` per spec (`ToInteger(NaN) = +0`).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn to_integer_index(n: f64) -> Option<usize> {
    if n.is_nan() {
        return Some(0);
    }
    let i = n.floor();
    if i < 0.0 || i >= (usize::MAX as f64) {
        None
    } else {
        Some(i as usize)
    }
}

pub(super) fn native_string_char_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let index = to_integer_index(args.first().map_or(0.0, |a| ctx.to_number(*a)));
    let s = ctx.get_u16(sid);
    let unit = index.and_then(|i| s.get(i).copied());
    let id = match unit {
        Some(u) => ctx.intern_utf16(&[u]),
        None => ctx.intern(""),
    };
    Ok(JsValue::String(id))
}

pub(super) fn native_string_char_code_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Number(f64::NAN));
    };
    let index = to_integer_index(args.first().map_or(0.0, |a| ctx.to_number(*a)));
    let s = ctx.get_u16(sid);
    let code = index
        .and_then(|i| s.get(i))
        .map_or(f64::NAN, |&u| f64::from(u));
    Ok(JsValue::Number(code))
}

pub(super) fn native_string_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Number(-1.0));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_u16(search_id).to_vec();
    let s = ctx.get_u16(sid);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let from = args.get(1).map_or(0usize, |a| {
        let n = ctx.to_number(*a);
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    });
    #[allow(clippy::cast_precision_loss)]
    let result = find_u16(&s[from..], &search).map_or(-1.0, |pos| (from + pos) as f64);
    Ok(JsValue::Number(result))
}

pub(super) fn native_string_includes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_u16(search_id).to_vec();
    let s = ctx.get_u16(sid);
    // §21.1.3.7 step 4-5: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = args.get(1).map_or(0usize, |a| {
        let n = ctx.to_number(*a);
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    });
    Ok(JsValue::Boolean(find_u16(&s[pos..], &search).is_some()))
}

pub(super) fn native_string_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let s = ctx.get_u16(sid).to_vec();
    let u16_len = s.len();
    #[allow(clippy::cast_possible_wrap)]
    let len_i = u16_len as isize;
    let raw_start = args.first().map_or(0.0, |a| ctx.to_number(*a));
    #[allow(clippy::cast_possible_truncation)]
    let resolve_index = |n_raw: f64, total: usize, total_i: isize| -> usize {
        let n = n_raw as isize;
        if n < 0 {
            (total_i + n).max(0).cast_unsigned()
        } else {
            n.cast_unsigned().min(total)
        }
    };
    let start = resolve_index(raw_start, u16_len, len_i);
    let end = if args.len() > 1 {
        let raw_end = ctx.to_number(args[1]);
        resolve_index(raw_end, u16_len, len_i)
    } else {
        u16_len
    };
    let id = if start <= end {
        ctx.intern_utf16(&s[start..end])
    } else {
        ctx.intern("")
    };
    Ok(JsValue::String(id))
}

pub(super) fn native_string_substring(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let s = ctx.get_u16(sid).to_vec();
    let u16len = s.len();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let clamp = |n: f64| -> usize {
        if n.is_nan() || n < 0.0 {
            0
        } else {
            (n as usize).min(u16len)
        }
    };
    let mut a = clamp(args.first().map_or(0.0, |v| ctx.to_number(*v)));
    let mut b = if args.len() > 1 {
        clamp(ctx.to_number(args[1]))
    } else {
        u16len
    };
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    let id = ctx.intern_utf16(&s[a..b]);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_to_lower_case(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let s = ctx.get_u16(sid);
    let lower = to_lower_u16(s);
    let id = ctx.intern_utf16(&lower);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_to_upper_case(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let s = ctx.get_u16(sid);
    let upper = to_upper_u16(s);
    let id = ctx.intern_utf16(&upper);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_trim(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let s = ctx.get_u16(sid).to_vec();
    let trimmed = trim_u16(&s);
    let id = ctx.intern_utf16(trimmed);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_split(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let sep_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let sep = ctx.get_u16(sep_id).to_vec();
    let s = ctx.get_u16(sid).to_vec();
    let mut parts: Vec<JsValue> = Vec::new();
    if sep.is_empty() {
        // Split into individual code units.
        for &unit in &s {
            let id = ctx.intern_utf16(&[unit]);
            parts.push(JsValue::String(id));
        }
    } else {
        let mut start = 0;
        while start <= s.len() {
            if let Some(pos) = find_u16(&s[start..], &sep) {
                let id = ctx.intern_utf16(&s[start..start + pos]);
                parts.push(JsValue::String(id));
                start += pos + sep.len();
            } else {
                let id = ctx.intern_utf16(&s[start..]);
                parts.push(JsValue::String(id));
                break;
            }
        }
    }
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: parts },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

pub(super) fn native_string_starts_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_u16(search_id).to_vec();
    let s = ctx.get_u16(sid);
    // §21.1.3.20 step 5-8: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = args.get(1).map_or(0usize, |a| {
        let n = ctx.to_number(*a);
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    });
    Ok(JsValue::Boolean(starts_with_u16(s, &search, pos)))
}

pub(super) fn native_string_ends_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_u16(search_id).to_vec();
    let s = ctx.get_u16(sid);
    // §21.1.3.6 step 5-8: endPosition (UTF-16 index, default len).
    let u16len = s.len();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let end_pos = args.get(1).map_or(u16len, |a| {
        let n = ctx.to_number(*a);
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(u16len)
        }
    });
    Ok(JsValue::Boolean(ends_with_u16(s, &search, end_pos)))
}

pub(super) fn native_string_replace(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let replacement_id = ctx.to_string_val(args.get(1).copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_u16(search_id).to_vec();
    let replacement = ctx.get_u16(replacement_id).to_vec();
    let s = ctx.get_u16(sid).to_vec();
    // Replace first occurrence only (like JS String.prototype.replace with a string pattern).
    let id = if let Some(pos) = find_u16(&s, &search) {
        let mut r: Vec<u16> = Vec::with_capacity(s.len() - search.len() + replacement.len());
        r.extend_from_slice(&s[..pos]);
        r.extend_from_slice(&replacement);
        r.extend_from_slice(&s[pos + search.len()..]);
        ctx.intern_utf16(&r)
    } else {
        sid
    };
    Ok(JsValue::String(id))
}

// -- Object.getOwnPropertySymbols ---------------------------------------------

pub(super) fn native_object_get_own_property_symbols(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: ctx.vm.array_prototype,
        })));
    };
    let syms: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter_map(|(k, _)| {
            if let PropertyKey::Symbol(sid) = k {
                Some(JsValue::Symbol(*sid))
            } else {
                None
            }
        })
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: syms },
        properties: Vec::new(),
        prototype: ctx.vm.array_prototype,
    })))
}

// -- Symbol constructor & methods -------------------------------------------

pub(super) fn native_symbol_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // NOTE: `new Symbol()` should throw TypeError, but detecting constructor
    // calls requires knowing if invoked via the New opcode. Deferred.
    let desc = match args.first().copied() {
        Some(JsValue::Undefined) | None => None,
        Some(val) => Some(ctx.to_string_val(val)),
    };
    let sid = ctx.vm.alloc_symbol(desc);
    Ok(JsValue::Symbol(sid))
}

pub(super) fn native_symbol_for(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    if let Some(&sid) = ctx.vm.symbol_registry.get(&key_id) {
        return Ok(JsValue::Symbol(sid));
    }
    let sid = ctx.vm.alloc_symbol(Some(key_id));
    ctx.vm.symbol_registry.insert(key_id, sid);
    Ok(JsValue::Symbol(sid))
}

pub(super) fn native_symbol_key_for(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Symbol(sid) = val else {
        return Err(VmError::type_error("Symbol.keyFor requires a symbol"));
    };
    for (&key, &reg_sid) in &ctx.vm.symbol_registry {
        if reg_sid == sid {
            return Ok(JsValue::String(key));
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_symbol_prototype_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Symbol(sid) = this else {
        return Err(VmError::type_error(
            "Symbol.prototype.toString requires a symbol value",
        ));
    };
    let desc = ctx.vm.symbols[sid.0 as usize]
        .description
        .map(|d| ctx.vm.strings.get_utf8(d));
    let result = match desc {
        Some(d) => format!("Symbol({d})"),
        None => "Symbol()".to_string(),
    };
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

// -- JSON stubs (M4-10) -----------------------------------------------------

pub(super) fn native_json_stringify_stub(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

pub(super) fn native_json_parse_stub(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

// -- Console ----------------------------------------------------------------

fn format_value_for_console(vm: &mut VmInner, val: JsValue) -> String {
    let id = super::coerce::to_string(vm, val);
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

// -- Array iterator (Symbol.iterator protocol) --------------------------------

/// `Array.prototype[Symbol.iterator]()` — creates an ArrayIterator.
pub(super) fn native_array_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(arr_id) = this else {
        return Err(VmError::type_error(
            "Array.prototype[Symbol.iterator] called on non-object",
        ));
    };
    // Create a "next" native function object inline.
    let next_name = ctx.vm.well_known.next;
    let next_fn_id = ctx.alloc_object(Object {
        kind: ObjectKind::NativeFunction(NativeFunction {
            name: next_name,
            func: native_array_iterator_next,
        }),
        properties: Vec::new(),
        prototype: None,
    });
    let iter_obj = ctx.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: arr_id,
            index: 0,
        }),
        properties: vec![(
            PropertyKey::String(next_name),
            Property::method(JsValue::Object(next_fn_id)),
        )],
        prototype: None,
    });
    Ok(JsValue::Object(iter_obj))
}

/// `ArrayIterator.prototype.next()` — returns `{ value, done }`.
pub(super) fn native_array_iterator_next(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(iter_id) = this else {
        return create_iter_result(ctx, JsValue::Undefined, true);
    };
    // Read state.
    let (array_id, idx) = {
        let iter_obj = ctx.get_object(iter_id);
        if let ObjectKind::ArrayIterator(state) = &iter_obj.kind {
            (state.array_id, state.index)
        } else {
            return create_iter_result(ctx, JsValue::Undefined, true);
        }
    };
    // Get value from array.
    let (value, done) = {
        let arr_obj = ctx.get_object(array_id);
        if let ObjectKind::Array { elements } = &arr_obj.kind {
            if idx < elements.len() {
                (elements[idx], false)
            } else {
                (JsValue::Undefined, true)
            }
        } else {
            (JsValue::Undefined, true)
        }
    };
    // Advance index.
    if !done {
        let iter_obj = ctx.get_object_mut(iter_id);
        if let ObjectKind::ArrayIterator(state) = &mut iter_obj.kind {
            state.index += 1;
        }
    }
    create_iter_result(ctx, value, done)
}

// -- Object.prototype.toString (ES2020 §19.1.3.6) -------------------------

pub(super) fn native_object_prototype_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let tag = match this {
        JsValue::Undefined => "Undefined",
        JsValue::Null => "Null",
        JsValue::Boolean(_) => "Boolean",
        JsValue::Number(_) => "Number",
        JsValue::String(_) => "String",
        JsValue::Symbol(_) => "Symbol",
        JsValue::Object(obj_id) => {
            // Check @@toStringTag
            let tag_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.to_string_tag);
            if let Some(JsValue::String(tag_id)) =
                super::coerce::get_property(ctx.vm, obj_id, tag_key)
            {
                let tag_str = ctx.get_utf8(tag_id);
                let result = format!("[object {tag_str}]");
                let id = ctx.intern(&result);
                return Ok(JsValue::String(id));
            }
            // Default tags based on object kind
            let obj = ctx.get_object(obj_id);
            match &obj.kind {
                ObjectKind::Array { .. } => "Array",
                ObjectKind::Function(_)
                | ObjectKind::NativeFunction(_)
                | ObjectKind::BoundFunction { .. } => "Function",
                ObjectKind::Error { .. } => "Error",
                ObjectKind::RegExp { .. } => "RegExp",
                _ => "Object",
            }
        }
    };
    let result = format!("[object {tag}]");
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

/// Helper: create a `{ value, done }` iterator result object.
fn create_iter_result(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    done: bool,
) -> Result<JsValue, VmError> {
    let value_key = PropertyKey::String(ctx.vm.well_known.value);
    let done_key = PropertyKey::String(ctx.vm.well_known.done);
    let obj = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        properties: vec![
            (value_key, Property::data(value)),
            (done_key, Property::data(JsValue::Boolean(done))),
        ],
        prototype: None,
    });
    Ok(JsValue::Object(obj))
}
