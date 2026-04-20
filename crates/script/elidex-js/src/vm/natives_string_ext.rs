//! String.prototype complement methods (P2 additions).

use super::natives_string::coerce_this_string;
use super::ops::STRING_LEN_LIMIT;
use super::value::{JsValue, NativeContext, VmError};
use crate::wtf16::{is_js_whitespace, rfind_u16};

/// §21.1.3.13 String.prototype.repeat(count)
pub(super) fn native_string_repeat(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let count_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(count_arg)?;
    let n = if n.is_nan() { 0.0 } else { n.trunc() };
    if n < 0.0 || n.is_infinite() {
        return Err(VmError::range_error("Invalid count value"));
    }
    #[allow(clippy::cast_sign_loss)]
    let count = n as usize;
    let s = ctx.get_u16(sid);
    let result_len = s.len().saturating_mul(count);
    if result_len > STRING_LEN_LIMIT {
        return Err(VmError::range_error("Invalid count value"));
    }
    let repeated: Vec<u16> = s.iter().copied().cycle().take(result_len).collect();
    let id = ctx.vm.strings.intern_utf16(&repeated);
    Ok(JsValue::String(id))
}

/// §21.1.3.14 String.prototype.padStart(maxLength, fillString)
pub(super) fn native_string_pad_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    pad_string(ctx, this, args, true)
}

/// §21.1.3.15 String.prototype.padEnd(maxLength, fillString)
pub(super) fn native_string_pad_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    pad_string(ctx, this, args, false)
}

fn pad_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    at_start: bool,
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let max_len_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let max_len_f = ctx.to_number(max_len_arg)?;
    // ToLength: NaN/negative → 0, clamp to STRING_LEN_LIMIT
    let max_len_f = if max_len_f.is_nan() || max_len_f < 0.0 {
        0.0
    } else {
        max_len_f.trunc()
    };
    #[allow(clippy::cast_sign_loss)]
    let max_len = (max_len_f as usize).min(STRING_LEN_LIMIT);
    let s_len = ctx.get_u16(sid).len();
    if max_len <= s_len {
        return Ok(JsValue::String(sid));
    }
    let fill_id = if let Some(&arg) = args.get(1) {
        if arg == JsValue::Undefined {
            None
        } else {
            Some(ctx.to_string_val(arg)?)
        }
    } else {
        None
    };
    let fill: Vec<u16> = fill_id
        .map_or(&[0x0020u16][..], |id| ctx.get_u16(id))
        .to_vec();
    if fill.is_empty() {
        return Ok(JsValue::String(sid));
    }
    let pad_len = max_len - s_len;
    let s = ctx.get_u16(sid);
    let mut result = Vec::with_capacity(max_len);
    let padding: Vec<u16> = fill.iter().copied().cycle().take(pad_len).collect();
    if at_start {
        result.extend_from_slice(&padding);
        result.extend_from_slice(s);
    } else {
        result.extend_from_slice(s);
        result.extend_from_slice(&padding);
    }
    let id = ctx.vm.strings.intern_utf16(&result);
    Ok(JsValue::String(id))
}

/// §21.1.3.22 String.prototype.trimStart()
pub(super) fn native_string_trim_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let trimmed = {
        let s = ctx.get_u16(sid);
        let start = s
            .iter()
            .position(|&u| !is_js_whitespace(u))
            .unwrap_or(s.len());
        s[start..].to_vec()
    };
    let id = ctx.vm.strings.intern_utf16(&trimmed);
    Ok(JsValue::String(id))
}

/// §21.1.3.23 String.prototype.trimEnd()
pub(super) fn native_string_trim_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let trimmed = {
        let s = ctx.get_u16(sid);
        let end = s
            .iter()
            .rposition(|&u| !is_js_whitespace(u))
            .map_or(0, |i| i + 1);
        s[..end].to_vec()
    };
    let id = ctx.vm.strings.intern_utf16(&trimmed);
    Ok(JsValue::String(id))
}

/// §21.1.3.9 String.prototype.lastIndexOf(searchString, position)
pub(super) fn native_string_last_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = if let Some(&a) = args.get(1) {
        let n = ctx.to_number(a)?;
        if n.is_nan() {
            s.len()
        } else {
            let t = n.trunc();
            if t < 0.0 {
                0usize
            } else {
                (t as usize).min(s.len())
            }
        }
    } else {
        s.len()
    };
    // Search backwards: find last occurrence at or before pos
    if search.is_empty() {
        #[allow(clippy::cast_precision_loss)]
        return Ok(JsValue::Number(pos as f64));
    }
    if search.len() > s.len() {
        return Ok(JsValue::Number(-1.0));
    }
    let end = pos.min(s.len() - search.len()) + search.len();
    #[allow(clippy::cast_precision_loss)]
    let result = rfind_u16(&s[..end], search).map_or(-1.0, |i| i as f64);
    Ok(JsValue::Number(result))
}

/// §21.1.3.3 String.prototype.codePointAt(pos)
pub(super) fn native_string_code_point_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let s = ctx.get_u16(sid);
    let pos_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let pos = ctx.to_number(pos_arg)?;
    let pos = if pos.is_nan() { 0.0 } else { pos.trunc() };
    #[allow(clippy::cast_sign_loss)]
    let idx = pos as usize;
    if pos < 0.0 || idx >= s.len() {
        return Ok(JsValue::Undefined);
    }
    let first = s[idx];
    // Check for surrogate pair
    if (0xD800..=0xDBFF).contains(&first) && idx + 1 < s.len() {
        let second = s[idx + 1];
        if (0xDC00..=0xDFFF).contains(&second) {
            let cp = 0x10000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00);
            return Ok(JsValue::Number(f64::from(cp)));
        }
    }
    Ok(JsValue::Number(f64::from(first)))
}

/// ES2021 String.prototype.replaceAll(searchValue, replaceValue)
pub(super) fn native_string_replace_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let search_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let replace_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let search_id = ctx.to_string_val(search_arg)?;
    let replace_id = ctx.to_string_val(replace_arg)?;
    let s = ctx.get_u16(sid);
    let search = ctx.get_u16(search_id);
    let replace = ctx.get_u16(replace_id);
    if search.is_empty() {
        // Empty search: insert replacement between every character and at boundaries
        let mut result = Vec::with_capacity(s.len() + replace.len() * (s.len() + 1));
        for (i, &unit) in s.iter().enumerate() {
            if i == 0 {
                result.extend_from_slice(replace);
            }
            result.push(unit);
            result.extend_from_slice(replace);
        }
        if s.is_empty() {
            result.extend_from_slice(replace);
        }
        let id = ctx.vm.strings.intern_utf16(&result);
        return Ok(JsValue::String(id));
    }
    let mut result = Vec::with_capacity(s.len());
    let mut pos = 0;
    while pos <= s.len().saturating_sub(search.len()) {
        if s[pos..].starts_with(search) {
            result.extend_from_slice(replace);
            pos += search.len();
        } else {
            result.push(s[pos]);
            pos += 1;
        }
    }
    // Append remaining characters
    if pos < s.len() {
        result.extend_from_slice(&s[pos..]);
    }
    let id = ctx.vm.strings.intern_utf16(&result);
    Ok(JsValue::String(id))
}

/// §21.1.3.4 String.prototype.concat(...args)
pub(super) fn native_string_concat(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let s = ctx.get_u16(sid);
    let mut result = s.to_vec();
    for &arg in args {
        let str_id = ctx.to_string_val(arg)?;
        let units = ctx.get_u16(str_id);
        result.extend_from_slice(units);
    }
    let id = ctx.vm.strings.intern_utf16(&result);
    Ok(JsValue::String(id))
}

/// §21.1.2.1 String.fromCharCode(...codes)
pub(super) fn native_string_from_char_code(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let mut result = Vec::with_capacity(args.len());
    for &arg in args {
        let n = ctx.to_number(arg)?;
        // ToUint16: truncate then modulo 2^16 (handles negative correctly)
        let code = super::coerce::f64_to_uint16(n);
        result.push(code);
    }
    let id = ctx.vm.strings.intern_utf16(&result);
    Ok(JsValue::String(id))
}

/// §21.1.2.2 String.fromCodePoint(...codePoints)
pub(super) fn native_string_from_code_point(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let mut result = Vec::with_capacity(args.len());
    for &arg in args {
        let n = ctx.to_number(arg)?;
        let cp = n.trunc();
        if cp < 0.0 || cp > f64::from(0x0010_FFFFi32) || cp != n {
            return Err(VmError::range_error("Invalid code point"));
        }
        #[allow(clippy::cast_sign_loss)]
        let cp = cp as u32;
        if cp <= 0xFFFF {
            result.push(cp as u16);
        } else {
            // Encode as surrogate pair
            let adjusted = cp - 0x10000;
            result.push((0xD800 + (adjusted >> 10)) as u16);
            result.push((0xDC00 + (adjusted & 0x3FF)) as u16);
        }
    }
    let id = ctx.vm.strings.intern_utf16(&result);
    Ok(JsValue::String(id))
}
