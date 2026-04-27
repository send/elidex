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
//! - `map` / `filter` (species-sensitive — allocate a fresh
//!   destination view via
//!   [`super::typed_array_static::species_constructor_for_typed_array`]
//!   + [`super::typed_array_static::create_typed_array_for_length`])
//!
//! Install-time wiring lives in
//! [`super::typed_array::install_typed_array_prototype_members`].

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

/// Brand-check the first argument as a callable function and return
/// its `ObjectId`.  Shared by every HOF — each spec algorithm runs
/// `If IsCallable(callbackfn) is false, throw a TypeError exception`
/// before touching the receiver's elements.
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

/// `%TypedArray%.prototype.forEach(cb, thisArg?)` (ES §23.2.3.13).
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

/// `%TypedArray%.prototype.every(cb, thisArg?)` (ES §23.2.3.7).
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

/// `%TypedArray%.prototype.some(cb, thisArg?)` (ES §23.2.3.26).
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

/// `%TypedArray%.prototype.find(cb, thisArg?)` (ES §23.2.3.10).
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

/// `%TypedArray%.prototype.findLast(cb, thisArg?)` (ES §23.2.3.11).
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
/// (ES §23.2.3.12).  Reverse-iterates `[len-1, 0]`; returns the
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

/// Snapshot the four `ObjectKind::TypedArray` slots a freshly
/// `species`-allocated view exposes.  `create_typed_array_for_length`
/// always produces a TypedArray view; the `unreachable!` arm guards
/// the contract.
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
/// (ES §23.2.3.21).
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
