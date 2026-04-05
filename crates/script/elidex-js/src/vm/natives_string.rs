//! Native implementations of String.prototype methods.

use super::value::{JsValue, NativeContext, Object, ObjectKind, StringId, VmError};
use crate::wtf16::{
    ends_with_u16, find_u16, starts_with_u16, to_lower_u16, to_upper_u16, trim_u16,
};

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
    let i = n.trunc();
    if i < 0.0 || i.is_infinite() {
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
    let raw = match args.first() {
        Some(a) => ctx.to_number(*a)?,
        None => 0.0,
    };
    let index = to_integer_index(raw);
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
    let raw = match args.first() {
        Some(a) => ctx.to_number(*a)?,
        None => 0.0,
    };
    let index = to_integer_index(raw);
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
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let from = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?;
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    } else {
        0usize
    };
    #[allow(clippy::cast_precision_loss)]
    let result = find_u16(&s[from..], search).map_or(-1.0, |pos| (from + pos) as f64);
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
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.7 step 4-5: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?;
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    } else {
        0usize
    };
    Ok(JsValue::Boolean(find_u16(&s[pos..], search).is_some()))
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
    let raw_start = match args.first() {
        Some(a) => ctx.to_number(*a)?,
        None => 0.0,
    };
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
        let raw_end = ctx.to_number(args[1])?;
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
    let raw_a = match args.first() {
        Some(v) => ctx.to_number(*v)?,
        None => 0.0,
    };
    let mut a = clamp(raw_a);
    let mut b = if args.len() > 1 {
        clamp(ctx.to_number(args[1])?)
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
    let sep_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
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
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.20 step 5-8: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?;
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(s.len())
        }
    } else {
        0usize
    };
    Ok(JsValue::Boolean(starts_with_u16(s, search, pos)))
}

pub(super) fn native_string_ends_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(sid) = this_string_id(this) else {
        return Ok(JsValue::Boolean(false));
    };
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.6 step 5-8: endPosition (UTF-16 index, default len).
    let u16len = s.len();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let end_pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?;
        if n.is_nan() || n < 0.0 {
            0usize
        } else {
            (n as usize).min(u16len)
        }
    } else {
        u16len
    };
    Ok(JsValue::Boolean(ends_with_u16(s, search, end_pos)))
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
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let replacement_id = ctx.to_string_val(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
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
