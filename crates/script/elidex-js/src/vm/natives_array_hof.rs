//! Array.prototype higher-order, iterator, and static methods (ES2020 §22.1).
//!
//! Mutator and accessor methods live in `natives_array.rs`.

use super::natives_array::{
    check_len, clone_elements, create_array, index_to_number, this_object_id,
};
use super::shape;
use super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract callback `ObjectId` from first arg. TypeError if not callable.
fn require_callback(args: &[JsValue]) -> Result<ObjectId, VmError> {
    match args.first().copied() {
        Some(JsValue::Object(id)) => Ok(id),
        _ => Err(VmError::type_error("callback is not a function")),
    }
}

// ---------------------------------------------------------------------------
// Group 3: Higher-order (callback) methods
// ---------------------------------------------------------------------------

/// `Array.prototype.forEach(callback, thisArg?)` — call for each non-hole.
pub(super) fn native_array_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
    }
    Ok(JsValue::Undefined)
}

/// `Array.prototype.map(callback, thisArg?)` — map non-holes, preserve holes.
pub(super) fn native_array_map(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    let mut result = Vec::with_capacity(elements.len());
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            result.push(JsValue::Empty);
        } else {
            let mapped = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
            result.push(mapped);
        }
    }
    Ok(create_array(ctx, result))
}

/// `Array.prototype.filter(callback, thisArg?)` — keep elements where callback is truthy.
pub(super) fn native_array_filter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    let mut result = Vec::new();
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let keep = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if ctx.to_boolean(keep) {
            result.push(*v);
        }
    }
    Ok(create_array(ctx, result))
}

/// `Array.prototype.every(callback, thisArg?)` — true if all non-holes pass.
pub(super) fn native_array_every(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let result = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if !ctx.to_boolean(result) {
            return Ok(JsValue::Boolean(false));
        }
    }
    Ok(JsValue::Boolean(true))
}

/// `Array.prototype.some(callback, thisArg?)` — true if any non-hole passes.
pub(super) fn native_array_some(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let result = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if ctx.to_boolean(result) {
            return Ok(JsValue::Boolean(true));
        }
    }
    Ok(JsValue::Boolean(false))
}

/// `Array.prototype.reduce(callback, initialValue?)` — left fold, skip holes.
///
/// The accumulator is pinned to a `vm.stack` slot under
/// `push_stack_scope` so user callbacks returning fresh
/// `JsValue::Object` handles stay GC-rooted across every
/// `ctx.call_function` invocation.  Held only as a Rust local,
/// such handles would be invisible to the GC scanner.  Today GC
/// is disabled for the entire duration of a `NativeFunction`
/// call (`interpreter.rs:81` sets `gc_enabled = false` and that
/// flag stays false through every re-entrant `ctx.call_function`
/// invocation), so the accumulator **cannot currently be
/// collected** mid-loop.  The rooted-slot shape is future-proofing
/// for when GC is permitted during native→JS callbacks, and it
/// matches `%TypedArray%.prototype.reduce`'s contract by
/// construction (SP8c-A R2 same-pattern audit, R8 wording fix).
pub(super) fn native_array_reduce(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let elements = clone_elements(ctx, id);

    let (initial_accumulator, start_idx) = if args.len() > 1 {
        (args[1], 0)
    } else {
        let first = elements.iter().enumerate().find(|(_, v)| !v.is_empty());
        match first {
            Some((i, v)) => (*v, i + 1),
            None => {
                return Err(VmError::type_error(
                    "Reduce of empty array with no initial value",
                ));
            }
        }
    };

    let mut frame = ctx.vm.push_stack_scope();
    let acc_slot = frame.saved_len();
    frame.stack.push(initial_accumulator);
    let mut sub_ctx = NativeContext { vm: &mut frame };

    #[allow(clippy::needless_range_loop)]
    for i in start_idx..elements.len() {
        if elements[i].is_empty() {
            continue;
        }
        // Snapshot the rooted accumulator slot into a `Copy`
        // local; the slot stays populated with the previous
        // iteration's value until we overwrite it after the
        // call returns.
        let accumulator = sub_ctx.vm.stack[acc_slot];
        let result = sub_ctx.call_function(
            cb,
            JsValue::Undefined,
            &[accumulator, elements[i], index_to_number(i), this],
        )?;
        sub_ctx.vm.stack[acc_slot] = result;
    }
    let final_accumulator = sub_ctx.vm.stack[acc_slot];
    drop(frame);
    Ok(final_accumulator)
}

/// `Array.prototype.reduceRight(callback, initialValue?)` — right fold, skip holes.
/// Accumulator GC-rooting matches `native_array_reduce`'s contract.
pub(super) fn native_array_reduce_right(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let elements = clone_elements(ctx, id);

    let (initial_accumulator, start_idx) = if args.len() > 1 {
        (args[1], elements.len())
    } else {
        let last = elements.iter().enumerate().rfind(|(_, v)| !v.is_empty());
        match last {
            Some((i, v)) => (*v, i),
            None => {
                return Err(VmError::type_error(
                    "Reduce of empty array with no initial value",
                ));
            }
        }
    };

    let mut frame = ctx.vm.push_stack_scope();
    let acc_slot = frame.saved_len();
    frame.stack.push(initial_accumulator);
    let mut sub_ctx = NativeContext { vm: &mut frame };

    #[allow(clippy::needless_range_loop)]
    for i in (0..start_idx).rev() {
        if elements[i].is_empty() {
            continue;
        }
        let accumulator = sub_ctx.vm.stack[acc_slot];
        let result = sub_ctx.call_function(
            cb,
            JsValue::Undefined,
            &[accumulator, elements[i], index_to_number(i), this],
        )?;
        sub_ctx.vm.stack[acc_slot] = result;
    }
    let final_accumulator = sub_ctx.vm.stack[acc_slot];
    drop(frame);
    Ok(final_accumulator)
}

/// `Array.prototype.find(callback, thisArg?)` — first element passing test.
pub(super) fn native_array_find(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let result = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if ctx.to_boolean(result) {
            return Ok(*v);
        }
    }
    Ok(JsValue::Undefined)
}

/// `Array.prototype.findIndex(callback, thisArg?)` — index of first match.
pub(super) fn native_array_find_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let result = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if ctx.to_boolean(result) {
            return Ok(index_to_number(i));
        }
    }
    Ok(JsValue::Number(-1.0))
}

/// Practical recursion depth limit for `flat()` to prevent stack overflow.
const MAX_FLAT_DEPTH: usize = 100;

/// `Array.prototype.flat(depth?)` — flatten nested arrays.
pub(super) fn native_array_flat(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let depth = match args.first().copied() {
        Some(JsValue::Undefined) | None => 1usize,
        Some(v) => {
            let n = ctx.to_number(v)?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            if n.is_infinite() && n > 0.0 {
                MAX_FLAT_DEPTH
            } else if n < 0.0 || n.is_nan() {
                0
            } else {
                (n as usize).min(MAX_FLAT_DEPTH)
            }
        }
    };

    let elements = clone_elements(ctx, id);
    let mut result = Vec::new();
    flat_into(ctx, &elements, depth, &mut result)?;
    check_len(result.len())?;
    Ok(create_array(ctx, result))
}

/// Recursive helper for flat. Holes are always skipped (HasProperty semantics).
fn flat_into(
    ctx: &NativeContext<'_>,
    source: &[JsValue],
    depth: usize,
    result: &mut Vec<JsValue>,
) -> Result<(), VmError> {
    for &elem in source {
        if elem.is_empty() {
            continue;
        }
        if depth > 0 {
            if let JsValue::Object(sub_id) = elem {
                if let ObjectKind::Array { elements } = &ctx.get_object(sub_id).kind {
                    flat_into(ctx, elements, depth - 1, result)?;
                    continue;
                }
            }
        }
        result.push(elem);
        check_len(result.len())?;
    }
    Ok(())
}

/// `Array.prototype.flatMap(callback, thisArg?)` — map then flat(1).
pub(super) fn native_array_flat_map(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = this_object_id(this)?;
    let cb = require_callback(args)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let elements = clone_elements(ctx, id);
    let mut result = Vec::new();

    for (i, v) in elements.iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        let mapped = ctx.call_function(cb, this_arg, &[*v, index_to_number(i), this])?;
        if let JsValue::Object(mapped_id) = mapped {
            if let ObjectKind::Array { elements: sub } = &ctx.get_object(mapped_id).kind {
                for &sub_elem in sub {
                    if !sub_elem.is_empty() {
                        result.push(sub_elem);
                    }
                }
                check_len(result.len())?;
                continue;
            }
        }
        result.push(mapped);
        check_len(result.len())?;
    }
    Ok(create_array(ctx, result))
}

// ---------------------------------------------------------------------------
// Group 4: Iterator methods (entries, keys)
// ---------------------------------------------------------------------------

/// `Array.prototype.entries()` — returns [index, value] iterator.
pub(super) fn native_array_entries(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    create_array_iterator(ctx, this, 2) // Entries
}

/// `Array.prototype.keys()` — returns index iterator.
pub(super) fn native_array_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    create_array_iterator(ctx, this, 1) // Keys
}

/// Create a lazy `ArrayIterator` with the given kind (0=Values, 1=Keys, 2=Entries).
fn create_array_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    kind: super::value::ArrayIterKind,
) -> Result<JsValue, VmError> {
    let JsValue::Object(arr_id) = this else {
        return Err(VmError::type_error(
            "Array.prototype iterator called on non-object",
        ));
    };
    if !matches!(ctx.get_object(arr_id).kind, ObjectKind::Array { .. }) {
        return Err(VmError::type_error(
            "Array.prototype iterator called on non-array",
        ));
    }
    let iter_obj = ctx.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(super::value::ArrayIterState {
            array_id: arr_id,
            index: 0,
            kind,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: ctx.vm.array_iterator_prototype,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

// ---------------------------------------------------------------------------
// Group 5: Array static methods
// ---------------------------------------------------------------------------

/// Drain an iterator into a `Vec`, optionally applying a map function.
/// Delegates to `VmInner::iter_next` for spec-compliant protocol handling.
///
/// §7.4.6: any abrupt completion *after* `.next()` succeeded (e.g.
/// `mapFn` throws, or the result exceeds `DENSE_ARRAY_LEN_LIMIT`) must
/// call `IteratorClose` on the iterator.  If that `.return()` itself
/// throws, its error takes precedence over the original abrupt.  An
/// abrupt from `.next()` itself does NOT close — per spec, an iterator
/// that threw is already considered closed.
fn drain_iterator(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    map_fn: Option<ObjectId>,
    this_arg: JsValue,
) -> Result<Vec<JsValue>, VmError> {
    let mut result = Vec::new();
    // Err here = iterator's own `.next()` threw → no IteratorClose.
    while let Some(value) = ctx.vm.iter_next(iter_val)? {
        // Any error below this point is an abrupt completion of the
        // for-of-like body; close the iterator before propagating.
        let mapped_result: Result<JsValue, VmError> = if let Some(fn_id) = map_fn {
            ctx.call_function(fn_id, this_arg, &[value, index_to_number(result.len())])
        } else {
            Ok(value)
        };
        let mapped = match mapped_result {
            Ok(v) => v,
            Err(e) => return Err(close_with_precedence(ctx, iter_val, e)),
        };
        result.push(mapped);
        if let Err(e) = check_len(result.len()) {
            return Err(close_with_precedence(ctx, iter_val, e));
        }
    }
    Ok(result)
}

/// Helper: close `iter_val` via `.return()` and return the higher-
/// precedence error — a throw from `.return()` wins over the triggering
/// abrupt completion (§7.4.6 IteratorClose step 6-7).
fn close_with_precedence(
    ctx: &mut NativeContext<'_>,
    iter_val: JsValue,
    fallback: VmError,
) -> VmError {
    ctx.vm.iter_close(iter_val).err().unwrap_or(fallback)
}

/// `Array.from(arrayLike, mapFn?, thisArg?)` — create array from iterable/array-like.
pub(super) fn native_array_from(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let items = args.first().copied().unwrap_or(JsValue::Undefined);
    let map_fn = match args.get(1).copied() {
        Some(JsValue::Object(id)) => Some(id),
        Some(JsValue::Undefined) | None => None,
        Some(_) => return Err(VmError::type_error("mapFn is not a function")),
    };
    let this_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);

    if matches!(items, JsValue::Object(_) | JsValue::String(_)) {
        // ES2020 §7.3.9 GetMethod: treat null/undefined @@iterator as absent.
        let has_iterator = match items {
            JsValue::Object(obj_id) => {
                let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
                match super::coerce::get_property(ctx.vm, obj_id, iter_key) {
                    Some(prop) => {
                        let val = ctx.vm.resolve_property(prop, items)?;
                        !val.is_nullish()
                    }
                    None => false,
                }
            }
            JsValue::String(_) => true,
            _ => false,
        };
        if has_iterator {
            let iter = ctx.vm.resolve_iterator(items)?;
            let Some(iter_val) = iter else {
                return Ok(create_array(ctx, Vec::new()));
            };
            let result = drain_iterator(ctx, iter_val, map_fn, this_arg)?;
            return Ok(create_array(ctx, result));
        }
    }

    if let JsValue::Object(obj_id) = items {
        let len_key = PropertyKey::String(ctx.vm.well_known.length);
        let len_val = ctx.get_property_value(obj_id, len_key)?;
        let len = ctx.to_number(len_val)?.trunc();
        if len.is_nan() || len < 0.0 {
            return Ok(create_array(ctx, Vec::new()));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let len = len as usize;
        check_len(len)?;
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let idx = index_to_number(i);
            let val = ctx.vm.get_element(JsValue::Object(obj_id), idx)?;
            let mapped = if let Some(fn_id) = map_fn {
                ctx.call_function(fn_id, this_arg, &[val, idx])?
            } else {
                val
            };
            result.push(mapped);
        }
        return Ok(create_array(ctx, result));
    }

    Ok(create_array(ctx, Vec::new()))
}

/// `Array.of(...items)` — create array from arguments.
pub(super) fn native_array_of(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    check_len(args.len())?;
    Ok(create_array(ctx, args.to_vec()))
}
