//! Native (Rust-implemented) JS built-in functions.
//!
//! These are free functions with the `NativeFn` signature, referenced by name
//! in `globals.rs` when registering built-in objects.

use super::value::{JsValue, NativeContext, Object, ObjectKind, Property, VmError};
use super::VmInner;

// -- Global functions -------------------------------------------------------

pub(super) fn native_parse_int(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let s_id = ctx.to_string_val(val);
    let s = ctx.get_string(s_id).trim().to_string();

    let radix = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let radix = if matches!(radix, JsValue::Undefined) {
        if s.starts_with("0x") || s.starts_with("0X") {
            16
        } else {
            10
        }
    } else {
        let r = ctx.to_number(radix);
        if r.is_nan() || !(2.0..=36.0).contains(&r) {
            return Ok(JsValue::Number(f64::NAN));
        }
        // Safe: range checked above.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let ru = r as u32;
        ru
    };

    if !(2..=36).contains(&radix) {
        return Ok(JsValue::Number(f64::NAN));
    }

    let parse_str = if radix == 16 && (s.starts_with("0x") || s.starts_with("0X")) {
        &s[2..]
    } else {
        &s
    };

    // Parse as many valid digits as possible (prefix parsing).
    let mut result: f64 = 0.0;
    let mut found = false;
    let mut negative = false;
    let mut chars = parse_str.chars().peekable();

    if chars.peek() == Some(&'-') {
        negative = true;
        chars.next();
    } else if chars.peek() == Some(&'+') {
        chars.next();
    }

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
    let s = ctx.get_string(s_id).trim().to_string();
    // Try to parse as much of the string as a float. Simple approach: use
    // Rust's parse for the full trimmed string, then progressively shorter.
    // For M4-10, full-string parse is sufficient.
    let n = s.parse::<f64>().unwrap_or(f64::NAN);
    Ok(JsValue::Number(n))
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

pub(super) fn native_error_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        let msg = args.first().copied().unwrap_or(JsValue::Undefined);
        let msg_id = ctx.to_string_val(msg);
        let msg_key = ctx.vm.well_known.message;
        ctx.get_object_mut(id)
            .properties
            .push((msg_key, Property::data(JsValue::String(msg_id))));
    }
    Ok(this)
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
            prototype: None,
        })));
    };
    let keys: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(_, p)| p.enumerable)
        .map(|(k, _)| JsValue::String(*k))
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: keys },
        properties: Vec::new(),
        prototype: None,
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
            prototype: None,
        })));
    };
    let values: Vec<JsValue> = ctx
        .get_object(obj_id)
        .properties
        .iter()
        .filter(|(_, p)| p.enumerable)
        .map(|(_, p)| p.value)
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: values },
        properties: Vec::new(),
        prototype: None,
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
        let props: Vec<(super::value::StringId, JsValue)> = ctx
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
    let key = ctx.to_string_val(prop_val);

    // Extract value from descriptor if it's an object.
    let value = if let JsValue::Object(desc_id) = desc_val {
        let value_key = ctx.intern("value");
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
    // JS Math.round: round half toward +Infinity.
    Ok(JsValue::Number((n + 0.5).floor()))
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
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Simple PRNG: use a basic approach. For M4-10, system time based seed.
    // This is not cryptographically secure but sufficient for Math.random().
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut hasher = s.build_hasher();
    hasher.write_u64(0);
    let bits = hasher.finish();
    // The shift produces a 53-bit value that fits in f64's mantissa exactly.
    #[allow(clippy::cast_precision_loss)]
    let n = (bits >> 11) as f64 / (1u64 << 53) as f64;
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

/// Helper: extract the string from `this` for String.prototype methods.
fn this_to_string(ctx: &NativeContext<'_>, this: JsValue) -> Option<String> {
    match this {
        JsValue::String(id) => Some(ctx.get_string(id).to_string()),
        _ => None,
    }
}

pub(super) fn native_string_char_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let index = args.first().map_or(0, |a| ctx.to_number(*a) as usize);
    let ch = s
        .chars()
        .nth(index)
        .map_or(String::new(), |c| c.to_string());
    let id = ctx.intern(&ch);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_char_code_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Number(f64::NAN));
    };
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let index = args.first().map_or(0, |a| ctx.to_number(*a) as usize);
    let code = s
        .chars()
        .nth(index)
        .map_or(f64::NAN, |c| f64::from(c as u32));
    Ok(JsValue::Number(code))
}

pub(super) fn native_string_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Number(-1.0));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_string(search_id).to_string();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let from = args.get(1).map_or(0, |a| {
        let n = ctx.to_number(*a);
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            n as usize
        }
    });
    // Work with char indices: find starting from `from`-th char.
    let byte_from = s
        .char_indices()
        .nth(from)
        .map_or(s.len(), |(byte_idx, _)| byte_idx);
    let result = s[byte_from..].find(&search).map_or(-1.0, |byte_pos| {
        // Convert byte position back to char index.
        #[allow(clippy::cast_precision_loss)]
        let char_idx = s[..byte_from + byte_pos].chars().count() as f64;
        char_idx
    });
    Ok(JsValue::Number(result))
}

pub(super) fn native_string_includes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_string(search_id).to_string();
    Ok(JsValue::Boolean(s.contains(&search)))
}

pub(super) fn native_string_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let char_len = s.chars().count();
    #[allow(clippy::cast_possible_wrap)]
    let len = char_len as isize;
    let raw_start = args.first().map_or(0.0, |a| ctx.to_number(*a));
    #[allow(clippy::cast_possible_truncation)]
    let resolve_index = |n_raw: f64, char_len_usize: usize, len_i: isize| -> usize {
        let n = n_raw as isize;
        if n < 0 {
            (len_i + n).max(0).cast_unsigned()
        } else {
            n.cast_unsigned().min(char_len_usize)
        }
    };
    let start = resolve_index(raw_start, char_len, len);
    let end = if args.len() > 1 {
        let raw_end = ctx.to_number(args[1]);
        resolve_index(raw_end, char_len, len)
    } else {
        char_len
    };
    let result: String = if start <= end {
        s.chars().skip(start).take(end - start).collect()
    } else {
        String::new()
    };
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_substring(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let len = s.chars().count();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let clamp = |n: f64| -> usize {
        if n.is_nan() || n < 0.0 {
            0
        } else {
            (n as usize).min(len)
        }
    };
    let mut a = clamp(args.first().map_or(0.0, |v| ctx.to_number(*v)));
    let mut b = if args.len() > 1 {
        clamp(ctx.to_number(args[1]))
    } else {
        len
    };
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    let result: String = s.chars().skip(a).take(b - a).collect();
    let id = ctx.intern(&result);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_to_lower_case(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let lower = s.to_lowercase();
    let id = ctx.intern(&lower);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_to_upper_case(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let upper = s.to_uppercase();
    let id = ctx.intern(&upper);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_trim(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let trimmed = s.trim();
    let id = ctx.intern(trimmed);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_split(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Object(ctx.alloc_object(Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: None,
        })));
    };
    let sep_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let sep = ctx.get_string(sep_id).to_string();
    let parts: Vec<JsValue> = s
        .split(&sep)
        .map(|part| {
            let id = ctx.intern(part);
            JsValue::String(id)
        })
        .collect();
    Ok(JsValue::Object(ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements: parts },
        properties: Vec::new(),
        prototype: None,
    })))
}

pub(super) fn native_string_starts_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_string(search_id).to_string();
    Ok(JsValue::Boolean(s.starts_with(&search)))
}

pub(super) fn native_string_ends_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_string(search_id).to_string();
    Ok(JsValue::Boolean(s.ends_with(&search)))
}

pub(super) fn native_string_replace(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(s) = this_to_string(ctx, this) else {
        let id = ctx.intern("");
        return Ok(JsValue::String(id));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined));
    let search = ctx.get_string(search_id).to_string();
    let replacement_id = ctx.to_string_val(args.get(1).copied().unwrap_or(JsValue::Undefined));
    let replacement = ctx.get_string(replacement_id).to_string();
    // Replace first occurrence only (like JS String.prototype.replace with a string pattern).
    let result = if let Some(pos) = s.find(&search) {
        let mut r = String::with_capacity(s.len() - search.len() + replacement.len());
        r.push_str(&s[..pos]);
        r.push_str(&replacement);
        r.push_str(&s[pos + search.len()..]);
        r
    } else {
        s
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
    vm.strings.get(id).to_string()
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
