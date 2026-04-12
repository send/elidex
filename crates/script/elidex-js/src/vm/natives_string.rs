//! Native implementations of String.prototype methods.

use super::natives_array::create_array;
use super::ops::DENSE_ARRAY_LEN_LIMIT;
use super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, StringId, VmError,
};
use crate::wtf16::{
    ends_with_u16, find_u16, starts_with_u16, to_lower_u16, to_upper_u16, trim_u16,
};

// -- Helpers ----------------------------------------------------------------

/// Build an exec-style match result array with `.index` and `.input`.
fn build_match_result(
    ctx: &mut NativeContext<'_>,
    subject: &[u16],
    m: &regress::Match,
    input_sid: StringId,
) -> Result<JsValue, VmError> {
    let mut elements = vec![JsValue::String(
        ctx.vm.strings.intern_utf16(&subject[m.start()..m.end()]),
    )];
    for group in &m.captures {
        match group {
            Some(range) => elements.push(JsValue::String(
                ctx.vm
                    .strings
                    .intern_utf16(&subject[range.start..range.end]),
            )),
            None => elements.push(JsValue::Undefined),
        }
    }
    let arr_id = ctx.alloc_object(Object {
        kind: ObjectKind::Array { elements },
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.array_prototype,
        extensible: true,
    });
    let index_key = PropertyKey::String(ctx.vm.well_known.index);
    #[allow(clippy::cast_precision_loss)]
    ctx.vm.define_shaped_property(
        arr_id,
        index_key,
        super::value::PropertyValue::Data(JsValue::Number(m.start() as f64)),
        super::shape::PropertyAttrs::DATA,
    );
    let input_key = PropertyKey::String(ctx.vm.well_known.input);
    ctx.vm.define_shaped_property(
        arr_id,
        input_key,
        super::value::PropertyValue::Data(JsValue::String(input_sid)),
        super::shape::PropertyAttrs::DATA,
    );
    Ok(JsValue::Object(arr_id))
}

/// Read the current `lastIndex` from a RegExp object (as raw f64).
/// §21.2.5.2.1 step 4: `Get(R, "lastIndex")` — walks the prototype chain
/// and invokes accessors, so user-defined `Object.defineProperty(re,
/// "lastIndex", {get})` overrides are honored.  Non-number results coerce
/// to NaN via `JsValue::as_number`; NaN → 0 to preserve the previous
/// "missing slot → 0" behavior.
pub(super) fn get_regexp_last_index(
    ctx: &mut NativeContext<'_>,
    obj_id: super::value::ObjectId,
) -> f64 {
    let last_index_key = PropertyKey::String(ctx.vm.well_known.last_index);
    match ctx.try_get_property_value(obj_id, last_index_key) {
        Ok(Some(JsValue::Number(n))) if n.is_finite() => n,
        _ => 0.0,
    }
}

/// Set `lastIndex` on a RegExp object (UTF-16 code unit index).
pub(super) fn set_regexp_last_index(
    ctx: &mut NativeContext<'_>,
    obj_id: super::value::ObjectId,
    idx: usize,
) {
    let last_index_key = PropertyKey::String(ctx.vm.well_known.last_index);
    #[allow(clippy::cast_precision_loss)]
    let val = JsValue::Number(idx as f64);
    // Split borrow: access object storage + shapes simultaneously.
    let obj = ctx.vm.objects[obj_id.0 as usize].as_mut().unwrap();
    if let Some((slot, _)) = obj.storage.get_mut(last_index_key, &ctx.vm.shapes) {
        *slot = super::value::PropertyValue::Data(val);
        return;
    }
    // lastIndex: writable, non-enumerable, non-configurable (§21.2.5.3).
    ctx.vm.define_shaped_property(
        obj_id,
        last_index_key,
        super::value::PropertyValue::Data(val),
        super::shape::PropertyAttrs::WRITABLE_HIDDEN,
    );
}

// -- String.prototype methods -----------------------------------------------

/// §21.1.3 String.prototype method this coercion:
/// RequireObjectCoercible(this) then ToString(this).
/// Handles String primitive, StringWrapper, and other values via ToString.
pub(super) fn coerce_this_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
) -> Result<StringId, VmError> {
    match this {
        JsValue::Null | JsValue::Undefined => Err(VmError::type_error(
            "String.prototype method called on null or undefined",
        )),
        JsValue::String(id) => Ok(id),
        JsValue::Object(obj_id) => {
            if let ObjectKind::StringWrapper(sid) = ctx.get_object(obj_id).kind {
                Ok(sid)
            } else {
                ctx.to_string_val(this)
            }
        }
        other => ctx.to_string_val(other),
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
    let sid = coerce_this_string(ctx, this)?;
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
    let sid = coerce_this_string(ctx, this)?;
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
    let sid = coerce_this_string(ctx, this)?;
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let from = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?.trunc();
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
    let sid = coerce_this_string(ctx, this)?;
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.7 step 4-5: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?.trunc();
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
    let sid = coerce_this_string(ctx, this)?;
    let u16_len = ctx.get_u16(sid).len();
    #[allow(clippy::cast_possible_wrap)]
    let len_i = u16_len as isize;
    let raw_start = match args.first() {
        Some(a) => ctx.to_number(*a)?,
        None => 0.0,
    };
    #[allow(clippy::cast_possible_truncation)]
    let resolve_index = |n_raw: f64, total: usize, total_i: isize| -> usize {
        let n = n_raw.trunc() as isize;
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
        // Copy only the result slice (not the whole string) to release the
        // immutable borrow before calling intern_utf16.
        let result = ctx.get_u16(sid)[start..end].to_vec();
        ctx.intern_utf16(&result)
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
    let sid = coerce_this_string(ctx, this)?;
    let u16len = ctx.get_u16(sid).len();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let clamp = |n: f64| -> usize {
        let t = n.trunc();
        if t.is_nan() || t < 0.0 {
            0
        } else {
            (t as usize).min(u16len)
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
    // Copy only the result slice to release the immutable borrow before intern.
    let result = ctx.get_u16(sid)[a..b].to_vec();
    let id = ctx.intern_utf16(&result);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_to_lower_case(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
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
    let sid = coerce_this_string(ctx, this)?;
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
    let sid = coerce_this_string(ctx, this)?;
    // Copy only the trimmed portion to release the immutable borrow before intern.
    let trimmed = trim_u16(ctx.get_u16(sid)).to_vec();
    let id = ctx.intern_utf16(&trimmed);
    Ok(JsValue::String(id))
}

pub(super) fn native_string_split(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let sep_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    // sep must be owned: we need it across intern_utf16 calls that borrow ctx mutably.
    let sep = ctx.get_u16(sep_id).to_vec();
    let mut parts: Vec<JsValue> = Vec::new();
    if sep.is_empty() {
        // Split into individual code units — no full-string clone needed.
        let len = ctx.get_u16(sid).len();
        if len >= DENSE_ARRAY_LEN_LIMIT {
            return Err(VmError::range_error("Array allocation failed"));
        }
        for i in 0..len {
            let unit = ctx.get_u16(sid)[i];
            let id = ctx.intern_utf16(&[unit]);
            parts.push(JsValue::String(id));
        }
    } else {
        // Compute split ranges under immutable borrows, then intern each part.
        let s_len = ctx.get_u16(sid).len();
        let sep_len = sep.len();
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut start = 0;
        while start <= s_len {
            if let Some(pos) = find_u16(&ctx.get_u16(sid)[start..], &sep) {
                ranges.push((start, start + pos));
                start += pos + sep_len;
            } else {
                ranges.push((start, s_len));
                break;
            }
        }
        if ranges.len() >= DENSE_ARRAY_LEN_LIMIT {
            return Err(VmError::range_error("Array allocation failed"));
        }
        for (a, b) in ranges {
            // Copy only each part slice to release the borrow before intern.
            let part = ctx.get_u16(sid)[a..b].to_vec();
            let id = ctx.intern_utf16(&part);
            parts.push(JsValue::String(id));
        }
    }
    Ok(create_array(ctx, parts))
}

pub(super) fn native_string_starts_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.20 step 5-8: position argument (UTF-16 index, default 0).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?.trunc();
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
    let sid = coerce_this_string(ctx, this)?;
    let search_id = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id);
    let s = ctx.get_u16(sid);
    // §21.1.3.6 step 5-8: endPosition (UTF-16 index, default len).
    let u16len = s.len();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let end_pos = if let Some(a) = args.get(1) {
        let n = ctx.to_number(*a)?.trunc();
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
    let sid = coerce_this_string(ctx, this)?;
    let search_val = args.first().copied().unwrap_or(JsValue::Undefined);

    // RegExp first argument: operate on WTF-16 via find_from_utf16.
    if let JsValue::Object(re_id) = search_val {
        let regexp_flags = {
            let obj = ctx.get_object(re_id);
            if let ObjectKind::RegExp { flags, .. } = &obj.kind {
                let f = ctx.vm.strings.get_utf8(*flags);
                Some((f.contains('g'), f.contains('y')))
            } else {
                None
            }
        };
        if let Some((is_global, _is_sticky)) = regexp_flags {
            let replacement_id =
                ctx.to_string_val(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
            let replacement = ctx.vm.strings.get(replacement_id).to_vec();
            let subject = ctx.vm.strings.get(sid).to_vec();

            // Reset lastIndex before dispatching; loop only when global.
            // Sticky (y) is enforced internally by run_regexp via lastIndex.
            set_regexp_last_index(ctx, re_id, 0);
            let result: Vec<u16> = if is_global {
                let mut out = Vec::new();
                let mut last_end = 0;
                while let Some(m) = super::natives_regexp::run_regexp(ctx, re_id, &subject)? {
                    out.extend_from_slice(&subject[last_end..m.start()]);
                    out.extend_from_slice(&replacement);
                    last_end = m.end();
                    // Prevent infinite loop on zero-length match.
                    if m.start() == m.end() {
                        set_regexp_last_index(ctx, re_id, m.end() + 1);
                    }
                }
                out.extend_from_slice(&subject[last_end..]);
                out
            } else {
                let m = super::natives_regexp::run_regexp(ctx, re_id, &subject)?;
                if let Some(m) = m {
                    let mut out = Vec::with_capacity(subject.len());
                    out.extend_from_slice(&subject[..m.start()]);
                    out.extend_from_slice(&replacement);
                    out.extend_from_slice(&subject[m.end()..]);
                    out
                } else {
                    subject
                }
            };
            set_regexp_last_index(ctx, re_id, 0);
            let id = ctx.vm.strings.intern_utf16(&result);
            return Ok(JsValue::String(id));
        }
    }

    // String pattern: replace first occurrence.
    let search_id = ctx.to_string_val(search_val)?;
    let replacement_id = ctx.to_string_val(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let search = ctx.get_u16(search_id).to_vec();
    let replacement = ctx.get_u16(replacement_id).to_vec();
    let s = ctx.get_u16(sid).to_vec();
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

#[allow(clippy::too_many_lines)]
pub(super) fn native_string_match(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let re_val = args.first().copied().unwrap_or(JsValue::Undefined);

    // Resolve the compiled regex + subject as WTF-16.
    let subject = ctx.vm.strings.get(sid).to_vec();

    // Non-RegExp: coerce to string and compile a regex on the fly.
    #[allow(clippy::manual_let_else)]
    let re_id = if let JsValue::Object(id) = re_val {
        id
    } else {
        let pattern_str = {
            let pat_id = super::coerce::to_string(ctx.vm, re_val)?;
            ctx.vm.strings.get_utf8(pat_id)
        };
        let compiled = regress::Regex::new(&pattern_str)
            .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
        let Some(m) = compiled.find_from_utf16(&subject, 0).next() else {
            return Ok(JsValue::Null);
        };
        return build_match_result(ctx, &subject, &m, sid);
    };

    // RegExp path: extract match data on WTF-16.
    let (_is_global, match_data) = {
        let obj = ctx.get_object(re_id);
        let ObjectKind::RegExp { ref flags, .. } = obj.kind else {
            // Non-RegExp object: coerce to string and compile (like non-object path).
            let pat_id = super::coerce::to_string(ctx.vm, re_val)?;
            let pattern_str = ctx.vm.strings.get_utf8(pat_id);
            let compiled = regress::Regex::new(&pattern_str)
                .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
            let Some(m) = compiled.find_from_utf16(&subject, 0).next() else {
                return Ok(JsValue::Null);
            };
            return build_match_result(ctx, &subject, &m, sid);
        };
        let flags_str = ctx.vm.strings.get_utf8(*flags);
        let is_global = flags_str.contains('g');

        if is_global {
            // Use run_regexp loop for correct sticky (gy) semantics.
            set_regexp_last_index(ctx, re_id, 0);
            let mut matches = Vec::new();
            while let Some(m) = super::natives_regexp::run_regexp(ctx, re_id, &subject)? {
                if matches.len() >= DENSE_ARRAY_LEN_LIMIT {
                    return Err(VmError::range_error("Array allocation failed"));
                }
                matches.push(ctx.vm.strings.intern_utf16(&subject[m.start()..m.end()]));
                if m.start() == m.end() {
                    set_regexp_last_index(ctx, re_id, m.end() + 1);
                }
            }
            (
                is_global,
                if matches.is_empty() {
                    None
                } else {
                    Some(matches)
                },
            )
        } else {
            // Non-global: delegate to run_regexp for correct lastIndex/sticky.
            if let Some(m) = super::natives_regexp::run_regexp(ctx, re_id, &subject)? {
                return build_match_result(ctx, &subject, &m, sid);
            }
            return Ok(JsValue::Null);
        }
    };

    let Some(matches) = match_data else {
        return Ok(JsValue::Null);
    };

    // Global: array of match strings.
    let elements: Vec<JsValue> = matches.into_iter().map(JsValue::String).collect();
    Ok(create_array(ctx, elements))
}

pub(super) fn native_string_search(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_this_string(ctx, this)?;
    let re_val = args.first().copied().unwrap_or(JsValue::Undefined);

    let subject = ctx.vm.strings.get(sid).to_vec();

    // Non-RegExp: coerce to string and compile a regex.
    if !matches!(re_val, JsValue::Object(_)) {
        // ToString coercion: undefined → "undefined", not "".
        let pat_id = super::coerce::to_string(ctx.vm, re_val)?;
        let pattern_str = ctx.vm.strings.get_utf8(pat_id);
        let compiled = regress::Regex::new(&pattern_str)
            .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
        #[allow(clippy::cast_precision_loss)]
        return Ok(JsValue::Number(
            compiled
                .find_from_utf16(&subject, 0)
                .next()
                .map_or(-1.0, |m| m.start() as f64),
        ));
    }
    let JsValue::Object(re_id) = re_val else {
        unreachable!();
    };
    // §21.1.3.15: save lastIndex, set to 0, run, restore.
    // For non-RegExp objects, fall through to ToString coercion.
    {
        let obj = ctx.get_object(re_id);
        if !matches!(obj.kind, ObjectKind::RegExp { .. }) {
            let pat_id = super::coerce::to_string(ctx.vm, re_val)?;
            let pattern_str = ctx.vm.strings.get_utf8(pat_id);
            let compiled = regress::Regex::new(&pattern_str)
                .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
            #[allow(clippy::cast_precision_loss)]
            return Ok(JsValue::Number(
                compiled
                    .find_from_utf16(&subject, 0)
                    .next()
                    .map_or(-1.0, |m| m.start() as f64),
            ));
        }
    }
    // §21.1.3.15: save lastIndex, set to 0, run, restore.
    let saved = get_regexp_last_index(ctx, re_id);
    set_regexp_last_index(ctx, re_id, 0);
    let result = super::natives_regexp::run_regexp(ctx, re_id, &subject)?.map(|m| m.start());
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    set_regexp_last_index(ctx, re_id, saved as usize);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(result.map_or(-1.0, |i| i as f64)))
}

// -- String.prototype.valueOf / toString (§21.1.3.32 / §21.1.3.25) ----------

/// `String.prototype.valueOf()` — return the primitive string value.
/// Works on both string primitives and StringWrapper objects.
pub(crate) fn native_string_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    match this {
        JsValue::String(sid) => Ok(JsValue::String(sid)),
        JsValue::Object(obj_id) => {
            if let ObjectKind::StringWrapper(sid) = ctx.get_object(obj_id).kind {
                Ok(JsValue::String(sid))
            } else {
                Err(VmError::type_error(
                    "String.prototype.valueOf requires a String",
                ))
            }
        }
        _ => Err(VmError::type_error(
            "String.prototype.valueOf requires a String",
        )),
    }
}

// §21.1.3.25 String.prototype.toString shares the valueOf implementation —
// registered twice in globals.rs with different `name` attributes.

// -- String constructor (§21.1.1) -------------------------------------------

/// `String(value)` as a function call returns a primitive string (§21.1.1.1
/// step 1 when NewTarget is undefined).  `new String(value)` promotes the
/// pre-allocated Ordinary instance (passed as `this` by `do_new`) to a
/// StringWrapper in-place, avoiding a second allocation.  For symbol input
/// on the call path, `SymbolDescriptiveString` semantics apply.
pub(crate) fn native_string_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let str_val = if args.is_empty() {
        ctx.vm.well_known.empty
    } else if ctx.is_construct() {
        ctx.to_string_val(args[0])?
    } else {
        // §21.1.1.1 step 2: `String(Symbol(...))` returns descriptive string,
        // not a TypeError.  Handle specially; all other values via ToString.
        if let JsValue::Symbol(sid) = args[0] {
            return Ok(JsValue::String(symbol_to_descriptive_string(ctx, sid)));
        }
        ctx.to_string_val(args[0])?
    };

    if ctx.is_construct() {
        let JsValue::Object(instance_id) = this else {
            // Defensive: do_new always passes an Object receiver.
            let wrapper = ctx.vm.create_string_wrapper(str_val);
            return Ok(JsValue::Object(wrapper));
        };
        ctx.vm.promote_to_string_wrapper(instance_id, str_val);
        Ok(JsValue::Object(instance_id))
    } else {
        Ok(JsValue::String(str_val))
    }
}

/// Format a symbol as `"Symbol(<description>)"` per §19.4.3.2.1.
fn symbol_to_descriptive_string(
    ctx: &mut NativeContext<'_>,
    sid: super::value::SymbolId,
) -> StringId {
    let desc = ctx.vm.symbols[sid.0 as usize]
        .description
        .map_or_else(String::new, |d| ctx.vm.strings.get_utf8(d));
    ctx.vm.strings.intern(&format!("Symbol({desc})"))
}
