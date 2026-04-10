//! Array.prototype mutator and accessor methods (ES2020 §22.1).
//!
//! Higher-order (callback), iterator, and static methods live in
//! `natives_array_hof.rs`.
//!
//! All methods operate on `ObjectKind::Array` only (generic array-like
//! objects are not supported in P0 — see plan).

use super::ops::MAX_DENSE_ARRAY_LEN;
use super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};

/// Wrap a `usize` index as `JsValue::Number`, suppressing the precision-loss
/// lint in a single place instead of scattering `#[allow]` on every call site.
#[inline]
#[allow(clippy::cast_precision_loss)]
pub(super) fn index_to_number(i: usize) -> JsValue {
    JsValue::Number(i as f64)
}

// ---------------------------------------------------------------------------
// Helpers (pub(super) so natives_array_hof.rs can reuse)
// ---------------------------------------------------------------------------

/// Extract `ObjectId` from `this`. TypeError if not an object.
pub(super) fn this_object_id(this: JsValue) -> Result<ObjectId, VmError> {
    match this {
        JsValue::Object(id) => Ok(id),
        _ => Err(VmError::type_error(
            "Array.prototype method called on non-object",
        )),
    }
}

/// Get array elements length. TypeError for non-Array objects.
pub(super) fn array_len(ctx: &NativeContext<'_>, id: ObjectId) -> Result<usize, VmError> {
    match &ctx.get_object(id).kind {
        ObjectKind::Array { elements } => Ok(elements.len()),
        _ => Err(VmError::type_error(
            "Array.prototype method called on non-array",
        )),
    }
}

/// Resolve a relative start index (ES2020 §7.1.22 + clamp).
/// Negative → max(len + val, 0), positive → min(val, len).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn resolve_index(n: f64, len: usize) -> usize {
    if n.is_nan() {
        return 0;
    }
    // ES2020 §7.1.22 ToIntegerOrInfinity: truncate toward zero first.
    let int = n.trunc();
    if int.is_infinite() && int < 0.0 {
        0
    } else if int < 0.0 {
        let adjusted = len as f64 + int;
        if adjusted <= 0.0 {
            0
        } else {
            adjusted as usize
        }
    } else if int.is_infinite() {
        len
    } else {
        let i = int as usize;
        if i > len {
            len
        } else {
            i
        }
    }
}

/// Create a new Array object with given elements.
pub(super) fn create_array(ctx: &mut NativeContext<'_>, elements: Vec<JsValue>) -> JsValue {
    JsValue::Object(ctx.vm.create_array_object(elements))
}

/// Guard against exceeding the dense array limit.
pub(super) fn check_len(new_len: usize) -> Result<(), VmError> {
    if new_len >= MAX_DENSE_ARRAY_LEN {
        return Err(VmError::range_error("Invalid array length"));
    }
    Ok(())
}

/// Clone elements Vec from an Array object (needed before re-borrowing ctx).
pub(super) fn clone_elements(ctx: &NativeContext<'_>, id: ObjectId) -> Vec<JsValue> {
    match &ctx.get_object(id).kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Group 1: Mutators
// ---------------------------------------------------------------------------

/// `Array.prototype.push(...items)` — append items, return new length.
pub(super) fn native_array_push(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let obj = ctx.get_object_mut(id);
    let ObjectKind::Array { elements } = &mut obj.kind else {
        return Err(VmError::type_error("push called on non-array"));
    };
    check_len(elements.len() + args.len())?;
    elements.extend_from_slice(args);
    Ok(index_to_number(elements.len()))
}

/// `Array.prototype.pop()` — remove and return last element.
pub(super) fn native_array_pop(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let obj = ctx.get_object_mut(id);
    let ObjectKind::Array { elements } = &mut obj.kind else {
        return Err(VmError::type_error("pop called on non-array"));
    };
    Ok(elements.pop().unwrap_or(JsValue::Undefined).or_undefined())
}

/// `Array.prototype.shift()` — remove and return first element.
pub(super) fn native_array_shift(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let obj = ctx.get_object_mut(id);
    let ObjectKind::Array { elements } = &mut obj.kind else {
        return Err(VmError::type_error("shift called on non-array"));
    };
    if elements.is_empty() {
        return Ok(JsValue::Undefined);
    }
    Ok(elements.remove(0).or_undefined())
}

/// `Array.prototype.unshift(...items)` — prepend items, return new length.
pub(super) fn native_array_unshift(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let obj = ctx.get_object_mut(id);
    let ObjectKind::Array { elements } = &mut obj.kind else {
        return Err(VmError::type_error("unshift called on non-array"));
    };
    check_len(elements.len() + args.len())?;
    elements.splice(0..0, args.iter().copied());
    Ok(index_to_number(elements.len()))
}

/// `Array.prototype.reverse()` — in-place reverse, return this.
pub(super) fn native_array_reverse(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let obj = ctx.get_object_mut(id);
    if let ObjectKind::Array { elements } = &mut obj.kind {
        elements.reverse();
    }
    Ok(this)
}

/// `Array.prototype.sort(compareFn?)` — stable in-place sort, holes to end.
pub(super) fn native_array_sort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cmp_fn = match args.first().copied() {
        Some(JsValue::Object(fn_id)) => Some(fn_id),
        Some(JsValue::Undefined) | None => None,
        Some(_) => return Err(VmError::type_error("compareFn is not a function")),
    };

    // §22.1.3.27: partition into (defined, undefined, holes). Sort only defined values.
    let elements = clone_elements(ctx, id);
    let hole_count = elements.iter().filter(|v| v.is_empty()).count();
    let undef_count = elements
        .iter()
        .filter(|v| matches!(v, JsValue::Undefined))
        .count();
    let mut defined: Vec<JsValue> = elements
        .into_iter()
        .filter(|v| !v.is_empty() && !matches!(v, JsValue::Undefined))
        .collect();

    if let Some(fn_id) = cmp_fn {
        // Use simple insertion sort to handle errors from compareFn calls.
        let len = defined.len();
        for i in 1..len {
            let mut j = i;
            while j > 0 {
                let a = defined[j - 1];
                let b = defined[j];
                let result = ctx.call_function(fn_id, JsValue::Undefined, &[a, b])?;
                let cmp_val = ctx.to_number(result)?;
                let cmp = if cmp_val.is_nan() { 0.0 } else { cmp_val };
                if cmp > 0.0 {
                    defined.swap(j - 1, j);
                    j -= 1;
                } else {
                    break;
                }
            }
        }
    } else {
        let mut keyed: Vec<(super::value::StringId, JsValue)> = Vec::with_capacity(defined.len());
        for v in &defined {
            keyed.push((ctx.to_string_val(*v)?, *v));
        }
        keyed.sort_by(|a, b| ctx.get_u16(a.0).cmp(ctx.get_u16(b.0)));
        defined = keyed.into_iter().map(|(_, v)| v).collect();
    }

    // Append: sorted defined values, then undefineds, then holes.
    defined.resize(defined.len() + undef_count, JsValue::Undefined);
    defined.resize(defined.len() + hole_count, JsValue::Empty);
    let obj = ctx.get_object_mut(id);
    if let ObjectKind::Array { elements } = &mut obj.kind {
        *elements = defined;
    }
    Ok(this)
}

/// `Array.prototype.splice(start, deleteCount, ...items)` — remove/insert.
pub(super) fn native_array_splice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let len = array_len(ctx, id)?;

    let raw_start = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let start = resolve_index(raw_start, len);

    let delete_count = if args.len() < 2 {
        len - start
    } else {
        let dc = ctx.to_number(args[1])?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let dc = if dc < 0.0 { 0 } else { dc as usize };
        dc.min(len - start)
    };

    let items = if args.len() > 2 { &args[2..] } else { &[] };
    let new_len = len - delete_count + items.len();
    check_len(new_len)?;

    let obj = ctx.get_object_mut(id);
    let ObjectKind::Array { elements } = &mut obj.kind else {
        return Ok(create_array(ctx, Vec::new()));
    };
    let removed: Vec<JsValue> = elements
        .splice(start..start + delete_count, items.iter().copied())
        .collect();

    Ok(create_array(ctx, removed))
}

/// `Array.prototype.fill(value, start?, end?)` — fill range with value.
pub(super) fn native_array_fill(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let len = array_len(ctx, id)?;
    let value = args.first().copied().unwrap_or(JsValue::Undefined);

    let raw_start = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Number(0.0)))?;
    let start = resolve_index(raw_start, len);

    #[allow(clippy::cast_precision_loss)]
    let raw_end = if args.len() > 2 {
        ctx.to_number(args[2])?
    } else {
        len as f64
    };
    let end = resolve_index(raw_end, len);

    let obj = ctx.get_object_mut(id);
    if let ObjectKind::Array { elements } = &mut obj.kind {
        for elem in &mut elements[start..end] {
            *elem = value;
        }
    }
    Ok(this)
}

/// `Array.prototype.copyWithin(target, start, end?)` — copy range within array.
pub(super) fn native_array_copy_within(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let len = array_len(ctx, id)?;

    let raw_target = ctx.to_number(args.first().copied().unwrap_or(JsValue::Number(0.0)))?;
    let target = resolve_index(raw_target, len);

    let raw_start = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Number(0.0)))?;
    let start = resolve_index(raw_start, len);

    #[allow(clippy::cast_precision_loss)]
    let raw_end = if args.len() > 2 {
        ctx.to_number(args[2])?
    } else {
        len as f64
    };
    let end = resolve_index(raw_end, len);

    if start >= end {
        return Ok(this);
    }

    let obj = ctx.get_object_mut(id);
    if let ObjectKind::Array { elements } = &mut obj.kind {
        let count = (end - start).min(len - target);
        if target <= start {
            for i in 0..count {
                elements[target + i] = elements[start + i];
            }
        } else {
            for i in (0..count).rev() {
                elements[target + i] = elements[start + i];
            }
        }
    }
    Ok(this)
}

// ---------------------------------------------------------------------------
// Group 2: Accessors
// ---------------------------------------------------------------------------

/// `Array.prototype.slice(start?, end?)` — shallow copy of a portion.
pub(super) fn native_array_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let len = array_len(ctx, id)?;

    let raw_start = ctx.to_number(args.first().copied().unwrap_or(JsValue::Number(0.0)))?;
    let start = resolve_index(raw_start, len);

    #[allow(clippy::cast_precision_loss)]
    let raw_end = if args.len() > 1 {
        ctx.to_number(args[1])?
    } else {
        len as f64
    };
    let end = resolve_index(raw_end, len);

    let elements = clone_elements(ctx, id);
    let result = if start < end {
        elements[start..end].to_vec()
    } else {
        Vec::new()
    };
    Ok(create_array(ctx, result))
}

/// `Array.prototype.concat(...items)` — merge arrays and/or values.
pub(super) fn native_array_concat(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let mut result = clone_elements(ctx, id);

    for &arg in args {
        if let JsValue::Object(arg_id) = arg {
            let is_array = matches!(ctx.get_object(arg_id).kind, ObjectKind::Array { .. });
            if is_array {
                let elems = clone_elements(ctx, arg_id);
                check_len(result.len() + elems.len())?;
                result.extend_from_slice(&elems);
                continue;
            }
        }
        check_len(result.len() + 1)?;
        result.push(arg);
    }
    Ok(create_array(ctx, result))
}

/// `Array.prototype.join(separator?)` — join elements as string. Holes → empty string.
pub(super) fn native_array_join(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let sep = match args.first().copied() {
        Some(JsValue::Undefined) | None => ",".to_string(),
        Some(v) => {
            let s = ctx.to_string_val(v)?;
            ctx.get_utf8(s)
        }
    };
    let elements = clone_elements(ctx, id);
    let mut result = String::new();
    for (i, v) in elements.iter().enumerate() {
        if i > 0 {
            result.push_str(&sep);
        }
        if !v.is_empty() && !v.is_nullish() {
            let s = ctx.to_string_val(*v)?;
            result.push_str(&ctx.get_utf8(s));
        }
    }
    let sid = ctx.intern(&result);
    Ok(JsValue::String(sid))
}

/// `Array.prototype.indexOf(searchElement, fromIndex?)` — strict equality search.
pub(super) fn native_array_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let len = array_len(ctx, id)?;

    #[allow(clippy::cast_precision_loss)]
    let from = if args.len() > 1 {
        let n = ctx.to_number(args[1])?;
        if n >= len as f64 {
            return Ok(JsValue::Number(-1.0));
        }
        resolve_index(n, len)
    } else {
        0
    };

    // No callbacks → borrow elements directly (no clone needed).
    if let ObjectKind::Array { elements } = &ctx.get_object(id).kind {
        for (i, elem) in elements.iter().enumerate().skip(from) {
            if elem.is_empty() {
                continue;
            }
            if *elem == search {
                return Ok(index_to_number(i));
            }
        }
    }
    Ok(JsValue::Number(-1.0))
}

/// `Array.prototype.lastIndexOf(searchElement, fromIndex?)` — reverse strict equality search.
pub(super) fn native_array_last_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let len = array_len(ctx, id)?;
    if len == 0 {
        return Ok(JsValue::Number(-1.0));
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    let from = if args.len() > 1 {
        let n = ctx.to_number(args[1])?.trunc();
        if n < 0.0 {
            let adjusted = len as f64 + n;
            if adjusted < 0.0 {
                return Ok(JsValue::Number(-1.0));
            }
            adjusted as usize
        } else {
            (n as usize).min(len - 1)
        }
    } else {
        len - 1
    };

    if let ObjectKind::Array { elements } = &ctx.get_object(id).kind {
        for i in (0..=from).rev() {
            if elements[i].is_empty() {
                continue;
            }
            if elements[i] == search {
                return Ok(index_to_number(i));
            }
        }
    }
    Ok(JsValue::Number(-1.0))
}

/// SameValueZero (ES2020 §7.2.12): NaN == NaN, +0 == -0.
fn same_value_zero(a: JsValue, b: JsValue) -> bool {
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => {
            if x.is_nan() && y.is_nan() {
                return true;
            }
            x == y // +0 == -0 is true for f64 ==
        }
        _ => a == b,
    }
}

/// `Array.prototype.includes(searchElement, fromIndex?)` — SameValueZero search.
pub(super) fn native_array_includes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let len = array_len(ctx, id)?;

    #[allow(clippy::cast_precision_loss)]
    let from = if args.len() > 1 {
        let n = ctx.to_number(args[1])?;
        if n >= len as f64 {
            return Ok(JsValue::Boolean(false));
        }
        resolve_index(n, len)
    } else {
        0
    };

    // includes treats holes as undefined (unlike indexOf which skips them).
    if let ObjectKind::Array { elements } = &ctx.get_object(id).kind {
        for elem in elements.iter().skip(from) {
            let value = elem.or_undefined();
            if same_value_zero(value, search) {
                return Ok(JsValue::Boolean(true));
            }
        }
    }
    Ok(JsValue::Boolean(false))
}

/// `Array.prototype.toString()` — equivalent to `join()`.
pub(super) fn native_array_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    native_array_join(ctx, this, &[])
}

/// `Array.prototype.toLocaleString()` — same as toString (locale not supported).
pub(super) fn native_array_to_locale_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    native_array_join(ctx, this, &[])
}
