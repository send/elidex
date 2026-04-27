//! `%TypedArray%.prototype` higher-order method bodies (ES2024
//! §23.2.3).
//!
//! Split from [`super::typed_array_methods`] (PR-spec-polish SP8b)
//! so the latter stays under the 1000-line convention as the
//! HOF surface grows.  Hosts the callback-driven prototype methods
//! that share the [`iterate_with_callback`] / [`require_callback`]
//! plumbing:
//!
//! - `forEach` / `every` / `some` / `find` / `findIndex` (forward)
//! - `findLast` / `findLastIndex` (reverse)
//! - `reduce` / `reduceRight` (linear with accumulator)
//! - `map` / `filter` (species-sensitive — allocate a fresh
//!   destination view via
//!   [`super::typed_array_static::species_constructor_for_typed_array`]
//!   + [`super::typed_array_static::create_typed_array_for_length`])
//! - `sort` (in-place; default numeric / `BigInt` ordering or
//!   user-supplied `compareFn` insertion sort)
//!
//! Install-time wiring lives in
//! [`super::typed_array::install_typed_array_prototype_members`].
//!
//! Section numbers in the per-method docstrings track the **ES2024
//! (15th edition)** numbering for `%TypedArray%.prototype` —
//! deliberately divergent from the legacy numbering still used in
//! sibling [`super::typed_array_methods`].  That file pre-dates
//! the §23.2.3 reshuffle that introduced `findLast` /
//! `findLastIndex` (ES2023) and re-numbered surrounding methods;
//! re-numbering it is a separate doc-debt cleanup outside this
//! PR's scope.  Within this file every spec ref points at the
//! current §23.2.3.* slot.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};
use super::typed_array::{read_element_raw, write_element_raw};
use super::typed_array_parts::{require_typed_array_parts, TypedArrayParts};
use super::typed_array_static::{
    create_typed_array_for_length, species_constructor_for_typed_array,
};

/// Per-HOF short-circuit verdict.  `Short` returns the given value
/// immediately; `Continue` lets the loop advance to the next index.
enum HofDecision {
    Continue,
    Short(JsValue),
}

/// Validate the first argument is a callable function and return
/// its `ObjectId`, otherwise raise the spec-mandated TypeError
/// (`If IsCallable(callbackfn) is false, throw a TypeError`).
/// Shared by every HOF — each spec algorithm runs this check
/// before touching the receiver's elements.  Distinct from
/// [`require_typed_array_parts`], which is the actual receiver
/// brand-check; this helper only inspects the callback slot.
fn require_callback(
    ctx: &NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<ObjectId, VmError> {
    match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(id) if ctx.get_object(id).kind.is_callable() => Ok(id),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': callback is not a function"
        ))),
    }
}

/// Forward / reverse `u32` index range used by
/// [`iterate_with_callback`].  A two-arm enum that implements
/// [`Iterator`] inline so the two iteration directions share the
/// loop body without per-call heap allocation (the `Box<dyn
/// Iterator>` form Copilot R1 flagged).  `Range<u32>` and
/// `Rev<Range<u32>>` are both bounded `u32`-stride iterators; the
/// `next` match dispatches to one inline `Range::next` per
/// element, well within the per-element callback dispatch cost.
enum IndicesRange {
    Forward(std::ops::Range<u32>),
    Reverse(std::iter::Rev<std::ops::Range<u32>>),
}

impl Iterator for IndicesRange {
    type Item = u32;
    #[inline]
    fn next(&mut self) -> Option<u32> {
        match self {
            Self::Forward(r) => r.next(),
            Self::Reverse(r) => r.next(),
        }
    }
}

/// Iterate `this`'s elements with `callback`.  `decide(i, elem,
/// truthy)` is invoked once per element with `ToBoolean` already
/// applied to the callback result; it decides whether to short-
/// circuit (`Short(v)` returns `v`) or continue.  `reverse = true`
/// drives the loop from `len - 1` down to `0` for the `findLast`
/// family; otherwise `0..len` ascending.  Returns `fallback` on
/// full drain without a `Short`.
fn iterate_with_callback(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    reverse: bool,
    fallback: JsValue,
    mut decide: impl FnMut(u32, JsValue, bool) -> HofDecision,
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, method)?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let cb = require_callback(ctx, args, method)?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let indices = if reverse {
        IndicesRange::Reverse((0..len_elem).rev())
    } else {
        IndicesRange::Forward(0..len_elem)
    };
    for i in indices {
        let elem = read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek);
        #[allow(clippy::cast_precision_loss)]
        let idx_val = JsValue::Number(f64::from(i));
        let cb_args = [elem, idx_val, this];
        let result = ctx.call_function(cb, this_arg, &cb_args)?;
        let truthy = ctx.to_boolean(result);
        if let HofDecision::Short(v) = decide(i, elem, truthy) {
            return Ok(v);
        }
    }
    Ok(fallback)
}

// ---------------------------------------------------------------------------
// Forward HOFs: forEach / every / some / find / findIndex
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.forEach(cb, thisArg?)` (ES §23.2.3.15).
pub(crate) fn native_typed_array_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "forEach",
        false,
        JsValue::Undefined,
        |_, _, _| HofDecision::Continue,
    )
}

/// `%TypedArray%.prototype.every(cb, thisArg?)` (ES §23.2.3.8).
pub(crate) fn native_typed_array_every(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "every",
        false,
        JsValue::Boolean(true),
        |_, _, truthy| {
            if truthy {
                HofDecision::Continue
            } else {
                HofDecision::Short(JsValue::Boolean(false))
            }
        },
    )
}

/// `%TypedArray%.prototype.some(cb, thisArg?)` (ES §23.2.3.28).
pub(crate) fn native_typed_array_some(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "some",
        false,
        JsValue::Boolean(false),
        |_, _, truthy| {
            if truthy {
                HofDecision::Short(JsValue::Boolean(true))
            } else {
                HofDecision::Continue
            }
        },
    )
}

/// `%TypedArray%.prototype.find(cb, thisArg?)` (ES §23.2.3.11).
pub(crate) fn native_typed_array_find(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "find",
        false,
        JsValue::Undefined,
        |_, elem, truthy| {
            if truthy {
                HofDecision::Short(elem)
            } else {
                HofDecision::Continue
            }
        },
    )
}

/// `%TypedArray%.prototype.findIndex(cb, thisArg?)` (ES §23.2.3.12).
pub(crate) fn native_typed_array_find_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "findIndex",
        false,
        JsValue::Number(-1.0),
        |i, _, truthy| {
            if truthy {
                #[allow(clippy::cast_precision_loss)]
                HofDecision::Short(JsValue::Number(f64::from(i)))
            } else {
                HofDecision::Continue
            }
        },
    )
}

// ---------------------------------------------------------------------------
// Reverse HOFs: findLast / findLastIndex
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.findLast(cb, thisArg?)` (ES §23.2.3.13).
/// Reverse-iterates `[len-1, 0]`; returns the first matching element
/// or `undefined`.  No allocation.
pub(crate) fn native_typed_array_find_last(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "findLast",
        true,
        JsValue::Undefined,
        |_, elem, truthy| {
            if truthy {
                HofDecision::Short(elem)
            } else {
                HofDecision::Continue
            }
        },
    )
}

/// `%TypedArray%.prototype.findLastIndex(cb, thisArg?)`
/// (ES §23.2.3.14).  Reverse-iterates `[len-1, 0]`; returns the
/// first matching index or `-1`.  No allocation.
pub(crate) fn native_typed_array_find_last_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(
        ctx,
        this,
        args,
        "findLastIndex",
        true,
        JsValue::Number(-1.0),
        |i, _, truthy| {
            if truthy {
                #[allow(clippy::cast_precision_loss)]
                HofDecision::Short(JsValue::Number(f64::from(i)))
            } else {
                HofDecision::Continue
            }
        },
    )
}

// ---------------------------------------------------------------------------
// Species-sensitive: map / filter
// ---------------------------------------------------------------------------

/// Extract the `(buffer_id, byte_offset)` pair from a freshly
/// `species`-allocated `ObjectKind::TypedArray` view — the only
/// two slots `map` / `filter`'s per-element write loop needs
/// (`byte_length` / `element_kind` are already in scope at the
/// call site as `len_elem` / `dst_ek`).
/// [`create_typed_array_for_length`] always produces a TypedArray
/// view; the `unreachable!` arm guards that contract.
fn destructure_view(vm: &super::super::VmInner, view_id: ObjectId) -> (ObjectId, u32) {
    match vm.get_object(view_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            ..
        } => (buffer_id, byte_offset),
        _ => unreachable!("create_typed_array_for_length always produces ObjectKind::TypedArray"),
    }
}

/// `%TypedArray%.prototype.map(callbackfn, thisArg?)`
/// (ES §23.2.3.22).
///
/// Resolves `TypedArraySpeciesCreate(O, ⟨len⟩)` (§22.2.4.7) BEFORE
/// the read/callback/write loop so a hostile species lookup that
/// throws fires before any element is observed (matching the spec
/// step ordering 4 → 6).  Reads each source element via
/// [`read_element_raw`] (no `[[Get]]` round-trip — the receiver is
/// brand-checked), invokes the callback with `(elem, index,
/// receiver)`, and writes the result through the destination's
/// per-element coercion via [`write_element_raw`].
///
/// The destination view is rooted on `vm.stack` for the duration of
/// the loop so it stays live across the `to_number` / `to_bigint`
/// GC points inside `write_element_raw`.
pub(crate) fn native_typed_array_map(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "map")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        id: receiver_id,
        buffer_id: src_buf,
        byte_offset: src_off,
        element_kind: src_ek,
        ..
    } = parts;
    let cb = require_callback(ctx, args, "map")?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let (dst_ek, proto_override) =
        species_constructor_for_typed_array(ctx, receiver_id, src_ek, "map")?;
    let dst_view_id = create_typed_array_for_length(ctx, dst_ek, proto_override, len_elem)?;
    let mut g = ctx.vm.push_temp_root(JsValue::Object(dst_view_id));
    let mut sub_ctx = NativeContext { vm: &mut g };
    let (dst_buf, dst_off) = destructure_view(sub_ctx.vm, dst_view_id);
    for i in 0..len_elem {
        let elem = read_element_raw(sub_ctx.vm, src_buf, src_off, i, src_ek);
        #[allow(clippy::cast_precision_loss)]
        let idx_val = JsValue::Number(f64::from(i));
        let cb_args = [elem, idx_val, this];
        let mapped = sub_ctx.call_function(cb, this_arg, &cb_args)?;
        write_element_raw(&mut sub_ctx, dst_buf, dst_off, i, dst_ek, mapped)?;
    }
    drop(g);
    Ok(JsValue::Object(dst_view_id))
}

/// `%TypedArray%.prototype.filter(callbackfn, thisArg?)`
/// (ES §23.2.3.10).
///
/// Two-phase per spec: (1) iterate `[0, len)` calling `callback` and
/// collect kept elements onto `vm.stack`; (2) resolve
/// `TypedArraySpeciesCreate(O, ⟨captured⟩)` and write each kept
/// value into the destination view.  Both phases run inside a
/// single `push_stack_scope` so collected `JsValue::BigInt` /
/// `JsValue::Object` handles remain GC-rooted across the
/// allocation point in `create_typed_array_for_length` and the
/// per-element write loop's `write_element_raw` GC points.
///
/// Snapshotting the kept range to an unrooted `Vec<JsValue>`
/// between phases would leave any object-typed elements invisible
/// to GC and produce stale `ObjectId`s (SP8a Copilot R7 lesson —
/// same hazard the iterator drain helper guards against).
pub(crate) fn native_typed_array_filter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "filter")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        id: receiver_id,
        buffer_id: src_buf,
        byte_offset: src_off,
        element_kind: src_ek,
        ..
    } = parts;
    let cb = require_callback(ctx, args, "filter")?;
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let mut frame = ctx.vm.push_stack_scope();
    let elem_start = frame.saved_len();
    let mut sub_ctx = NativeContext { vm: &mut frame };
    for i in 0..len_elem {
        let elem = read_element_raw(sub_ctx.vm, src_buf, src_off, i, src_ek);
        #[allow(clippy::cast_precision_loss)]
        let idx_val = JsValue::Number(f64::from(i));
        let cb_args = [elem, idx_val, this];
        let result = sub_ctx.call_function(cb, this_arg, &cb_args)?;
        if sub_ctx.to_boolean(result) {
            sub_ctx.vm.stack.push(elem);
        }
    }
    let kept_len = sub_ctx.vm.stack.len() - elem_start;
    // `kept_len <= len_elem <= u32::MAX` because the only `push`
    // inside the loop is gated on the source range `0..len_elem`,
    // and `len_elem` is itself a `u32`.  `try_from` is the cheap
    // belt-and-braces against any future caller that might widen
    // the loop bound.
    let kept_u32 = u32::try_from(kept_len).map_err(|_| {
        VmError::range_error(
            "Failed to execute 'filter' on 'TypedArray': result length exceeds the supported maximum",
        )
    })?;

    let (dst_ek, proto_override) =
        species_constructor_for_typed_array(&mut sub_ctx, receiver_id, src_ek, "filter")?;
    let dst_view_id =
        create_typed_array_for_length(&mut sub_ctx, dst_ek, proto_override, kept_u32)?;
    let mut g = sub_ctx.vm.push_temp_root(JsValue::Object(dst_view_id));
    let mut deeper = NativeContext { vm: &mut g };
    let (dst_buf, dst_off) = destructure_view(deeper.vm, dst_view_id);
    for i in 0..kept_len {
        // `JsValue` is `Copy`; this snapshot ends before the
        // mutable `&mut deeper` borrow on the next line.  The
        // stack entry remains rooted under the parent
        // `push_stack_scope` frame.
        let value = deeper.vm.stack[elem_start + i];
        #[allow(clippy::cast_possible_truncation)]
        let dst_i = i as u32;
        write_element_raw(&mut deeper, dst_buf, dst_off, dst_i, dst_ek, value)?;
    }
    drop(g);
    drop(frame);
    Ok(JsValue::Object(dst_view_id))
}

// ---------------------------------------------------------------------------
// reduce / reduceRight (linear with accumulator)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.reduce(callbackfn, initialValue?)`
/// (ES §23.2.3.23).  Forward iterate, accumulator threaded through.
pub(crate) fn native_typed_array_reduce(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reduce_impl(ctx, this, args, "reduce", false)
}

/// `%TypedArray%.prototype.reduceRight(callbackfn, initialValue?)`
/// (ES §23.2.3.24).  Reverse iterate, accumulator threaded through.
pub(crate) fn native_typed_array_reduce_right(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reduce_impl(ctx, this, args, "reduceRight", true)
}

/// Shared body of `reduce` / `reduceRight`.  Per spec
/// §23.2.3.23/.24 the callback is invoked as `Call(callbackfn,
/// undefined, ⟨acc, kValue, F(k), O⟩)` — `this` inside the
/// callback is always `undefined` (no `thisArg` parameter).
///
/// Initial-value handling per §23.2.3.23 step 4-7:
/// - `initialValue` provided → `acc = initialValue`, scan all
///   elements (`[0..len)` forward / `[len-1..-1]` reverse).
/// - `initialValue` absent + `len > 0` → `acc = O[start]`
///   (`start = 0` forward / `len-1` reverse), scan remaining
///   elements (`[1..len)` forward / `[len-2..-1]` reverse).
/// - `initialValue` absent + `len == 0` → spec-mandated
///   TypeError ("Reduce of empty TypedArray with no initial value").
///
/// Indices are tracked in `i64` because the reverse-empty-loop
/// terminating sentinel is `-1`, which would underflow `u32`.
/// `len_elem` is bounded by `[[ByteLength]] / bpe`, so the
/// `len_elem.into()` widening is exact (no clamp).
fn reduce_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    reverse: bool,
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, method)?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let cb = require_callback(ctx, args, method)?;
    let initial_value = args.get(1).copied();

    if len_elem == 0 && initial_value.is_none() {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'TypedArray': Reduce of empty TypedArray with no initial value"
        )));
    }

    // Compute initial accumulator + first-iteration index.
    // Forward k counts up from `start_k` to `len_elem`; reverse k
    // counts down from `start_k` to `-1`.
    let len_signed = i64::from(len_elem);
    let (initial_acc, mut k_signed) = match (initial_value, reverse) {
        (Some(v), false) => (v, 0_i64),
        (Some(v), true) => (v, len_signed - 1),
        (None, false) => (
            read_element_raw(ctx.vm, buffer_id, byte_offset, 0, ek),
            1_i64,
        ),
        (None, true) => {
            let last = len_elem - 1;
            (
                read_element_raw(ctx.vm, buffer_id, byte_offset, last, ek),
                len_signed - 2,
            )
        }
    };
    let (limit, step) = if reverse {
        (-1_i64, -1_i64)
    } else {
        (len_signed, 1_i64)
    };

    // Pin the accumulator to a fixed `vm.stack` slot for the
    // duration of the loop.  User callbacks can return arbitrary
    // `JsValue::Object` handles; held only as a Rust local, those
    // would be invisible to the GC scanner across the next
    // `ctx.call_function` GC point and could be collected mid-
    // iteration (Copilot SP8c-A R2).  No GC trigger sits in the
    // current cross-iteration window for TypedArray reads
    // (`read_element_raw` returns `Number` / `BigInt` only,
    // neither GC-allocated), but the rooted-slot pattern is the
    // future-proof shape and matches `filter`'s rooted collect.
    let mut frame = ctx.vm.push_stack_scope();
    let acc_slot = frame.saved_len();
    frame.stack.push(initial_acc);
    let mut sub_ctx = NativeContext { vm: &mut frame };

    while k_signed != limit {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let k = k_signed as u32;
        let kv = read_element_raw(sub_ctx.vm, buffer_id, byte_offset, k, ek);
        #[allow(clippy::cast_precision_loss)]
        let idx_val = JsValue::Number(k_signed as f64);
        // Snapshot the rooted slot into a `Copy` local so the
        // mutable borrow of `sub_ctx` for `call_function` doesn't
        // overlap the immutable index read.  The slot stays
        // populated with the previous iteration's `acc` until
        // we overwrite it after the call returns.
        let acc = sub_ctx.vm.stack[acc_slot];
        let cb_args = [acc, kv, idx_val, this];
        let result = sub_ctx.call_function(cb, JsValue::Undefined, &cb_args)?;
        sub_ctx.vm.stack[acc_slot] = result;
        k_signed += step;
    }

    // Extract the final accumulator BEFORE dropping the frame —
    // the drop truncates `vm.stack` past `acc_slot`, so the
    // intermediate `Copy` local is the only safe handoff path.
    // No GC point sits between this read and the function
    // return, so the Rust-local-only window is sound.
    let final_acc = sub_ctx.vm.stack[acc_slot];
    drop(frame);
    Ok(final_acc)
}

// ---------------------------------------------------------------------------
// sort (in-place; default numeric / BigInt ordering or compareFn)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.sort(comparefn?)` (ES §23.2.3.29).
///
/// In-place sort, returns receiver.  `comparefn` must be callable
/// or `undefined` — anything else surfaces TypeError per spec
/// §23.2.3.29 step 1.  Default ordering (no `compareFn`) is
/// ascending numeric for non-`BigInt` element kinds with `NaN`
/// sorted to the end (per spec `TypedArrayElementSortCompare`),
/// or ascending `BigInt::cmp` for `BigInt64Array` /
/// `BigUint64Array` (no NaN concern).
///
/// Implementation: snapshot all elements into an unrooted
/// `Vec<JsValue>` (TypedArray stores only `Number` / `BigInt` —
/// neither is GC-traced; `BigInt` allocations are deduplicated
/// by value in `BigIntPool` so repeated reads don't grow the
/// pool unboundedly), sort, write back via
/// [`write_element_raw`].  Snapshot-then-write-back gives the
/// receiver **atomic-on-throw** semantics — a throwing
/// `compareFn` returns the abrupt completion *before* any
/// write-back happens, so the receiver is left unchanged rather
/// than exposing a half-sorted state.  This matches spec
/// §23.2.3.29 step 5 → 7 ordering: `SortIndexedProperties`
/// (step 5) collects sorted values into a list, then step 7
/// writes them back — an abrupt completion in step 5 short-
/// circuits before step 7 runs, so the receiver is never
/// observed mid-sort either way.  `compareFn` branch uses a
/// stable insertion sort (same shape as
/// `Array.prototype.sort`); default branch uses Rust's stable
/// `slice::sort_by`.
pub(crate) fn native_typed_array_sort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Spec §23.2.3.29 step 1 validates `comparefn` BEFORE the
    // receiver brand-check, so even calling `[].sort.call(non_ta,
    // 'not_a_function')` throws the comparefn TypeError first.
    let compare_fn = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => None,
        JsValue::Object(id) if ctx.get_object(id).kind.is_callable() => Some(id),
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'sort' on 'TypedArray': comparefn must be a function",
            ));
        }
    };
    let parts = require_typed_array_parts(ctx, this, "sort")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        id: receiver_id,
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;

    if len_elem < 2 {
        return Ok(JsValue::Object(receiver_id));
    }

    let mut snapshot: Vec<JsValue> = (0..len_elem)
        .map(|i| read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek))
        .collect();

    if let Some(fn_id) = compare_fn {
        // Insertion sort with fallible comparator.  Pair-wise
        // adjacent compares only — Array.prototype.sort uses the
        // same shape, so we keep the per-PR mental model
        // consistent.  Throwing `fn_id` propagates immediately;
        // the swaps live on the local `snapshot`, not the
        // receiver, so the early return leaves the receiver
        // unchanged (atomic-on-throw — see fn-level docstring).
        for i in 1..snapshot.len() {
            let mut j = i;
            while j > 0 {
                let a = snapshot[j - 1];
                let b = snapshot[j];
                let result = ctx.call_function(fn_id, JsValue::Undefined, &[a, b])?;
                let cmp_val = ctx.to_number(result)?;
                let cmp = if cmp_val.is_nan() { 0.0 } else { cmp_val };
                if cmp > 0.0 {
                    snapshot.swap(j - 1, j);
                    j -= 1;
                } else {
                    // Stable: equal or `cmp < 0` means current
                    // pair is in order; insertion sort stops the
                    // inward walk at first non-swap.
                    break;
                }
            }
        }
    } else {
        // Default ordering — `&VmInner` capture is sound because
        // `snapshot` is a local `Vec`, not `vm.stack`, so the
        // borrow doesn't overlap any mutable VM access inside
        // the comparator.
        let vm: &super::super::VmInner = ctx.vm;
        snapshot.sort_by(|a, b| default_typed_array_compare(vm, *a, *b));
    }

    for (i, v) in snapshot.into_iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let idx = i as u32;
        write_element_raw(ctx, buffer_id, byte_offset, idx, ek, v)?;
    }
    Ok(JsValue::Object(receiver_id))
}

/// Default ascending sort comparator for the no-`compareFn`
/// branch of [`native_typed_array_sort`].  Matches spec
/// `TypedArrayElementSortCompare` (§23.2.4.7 / .8):
///
/// - `Number / Number` → `<` ordering with `NaN` sorted to the
///   end (`partial_cmp` is `None` for any NaN comparison; the
///   fallback `Equal` only fires for the unreachable
///   `NaN.cmp(NaN)` case which is already handled by the
///   `is_nan` arms above).
/// - `BigInt / BigInt` → `BigInt::cmp` ordering via the canonical
///   `BigIntPool` lookup.  BigInt values are permanent (per
///   `BigIntPool`'s "not garbage-collected" contract), so the
///   `&BigInt` borrows are valid across the entire sort.
/// - Mixed types → `Equal` (impossible by the TypedArray brand:
///   each subclass stores a single primitive type, never mixed).
fn default_typed_array_compare(
    vm: &super::super::VmInner,
    a: JsValue,
    b: JsValue,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => match (x.is_nan(), y.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        },
        (JsValue::BigInt(ai), JsValue::BigInt(bi)) => vm.bigints.get(ai).cmp(vm.bigints.get(bi)),
        _ => Ordering::Equal,
    }
}
