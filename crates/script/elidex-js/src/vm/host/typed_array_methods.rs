//! `%TypedArray%.prototype` method bodies (ES2024 §23.2.3).
//!
//! Split from [`super::typed_array`] to keep both files below the
//! 1000-line convention (PR5a-fetch lesson).  The install-time
//! table in `super::typed_array::install_typed_array_prototype_members`
//! wires each native into `%TypedArray%.prototype`, shared across
//! all 11 subclasses via the prototype chain.
//!
//! ## Scope (C4a + C4b)
//!
//! - `fill(value, start?, end?)` (§23.2.3.11)
//! - `subarray(begin?, end?)` (§23.2.3.27) — shares backing buffer
//! - `slice(begin?, end?)` (§23.2.3.25) — fresh buffer copy
//! - `values()` / `keys()` / `entries()` (§23.2.3.34 / .20 / .8) —
//!   reuse `ObjectKind::ArrayIterator` infra
//! - `set(source, offset?)` (§23.2.3.24)
//! - `copyWithin(target, start, end?)` (§23.2.3.6)
//! - `reverse()` (§23.2.3.23)
//! - `indexOf` / `lastIndexOf` / `includes` / `at`
//! - `join(separator?)` (§23.2.3.19)
//! - `toLocaleString(reserved1?, reserved2?)` (§23.2.3.31) —
//!   no-Intl per-element `Invoke("toLocaleString")`, joined `","`
//!
//! Higher-order callback methods (`forEach` / `every` / `some` /
//! `find` / `findIndex` / `findLast` / `findLastIndex` / `map` /
//! `filter` / `reduce` / `reduceRight` / `sort` / `flatMap`) live in
//! [`super::typed_array_hof`] (PR-spec-polish SP8b/c split — keeps
//! both files under the 1000-line convention as the HOF surface
//! grew with SpeciesConstructor + the `findLast` family).

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape;
use super::super::value::{
    ArrayIterState, ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, VmError,
};
use super::super::VmInner;
use super::typed_array::{read_element_raw, write_element_raw};
use super::typed_array_parts::{require_typed_array_parts, TypedArrayParts};

/// Clamp `n` to `[0, len]`, applying `ToIntegerOrInfinity`
/// truncation first (ES §7.1.5).  Negative indices count from the
/// end.  Shared by `fill` / `subarray` / `slice`.  Thin u32-typed
/// wrapper around [`super::super::coerce::relative_index_f64`]; the
/// clamp at the canonical helper guarantees `0.0 <= clamped <=
/// f64::from(len)`, so the final `as u32` cast is exact (no Rust
/// 1.45+ saturating fallback exercised).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn relative_index_u32(n: f64, len: u32) -> u32 {
    coerce::relative_index_f64(n, f64::from(len)) as u32
}

/// Allocate a new TypedArray wrapper of the given `ElementKind` over
/// `buffer_id` at `byte_offset` / `byte_length`.  Picks the built-in
/// subclass prototype associated with `ek` (`Uint8Array.prototype`,
/// etc.) and does NOT consult the receiver's constructor,
/// `new.target`, or `@@species` — current callers
/// (`subarray` / `slice`) pass their own `element_kind`, so the
/// result matches a same-ElementKind built-in subclass.  Full
/// `SpeciesConstructor` dispatch lands with PR-spec-polish SP8.
fn alloc_typed_array_view(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    buffer_id: ObjectId,
    byte_offset: u32,
    byte_length: u32,
) -> ObjectId {
    let proto = subclass_prototype_for(ctx.vm, ek);
    ctx.vm.alloc_object(Object {
        kind: ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind: ek,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Resolve the per-subclass prototype stored on `VmInner`, falling
/// back to the abstract `%TypedArray%.prototype` if the subclass
/// slot is `None`.  Every slot is populated by
/// `register_typed_array_prototype_global`, so the fallback only
/// triggers at very early construction (before `register_globals`
/// finishes); the `or` chain keeps that startup window sound without
/// a callsite-level guard.
pub(super) fn subclass_prototype_for(vm: &VmInner, ek: ElementKind) -> Option<ObjectId> {
    vm.subclass_array_prototypes[ek.index()].or(vm.typed_array_prototype)
}

// ---------------------------------------------------------------------------
// fill(value, start?, end?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.fill(value, start?, end?)` (ES §23.2.3.11).
/// Writes `value` to each element across `[start, end)` and returns
/// the receiver for chaining.  Coerces `value` *once* up front via
/// [`super::typed_array::coerce_element_to_le_bytes`] and bulk-fills
/// the byte range through [`super::byte_io::fill_pattern`] — a
/// single clone-grow-install replaces what was previously N of them.
/// O(N²) bytes-touched → O(N).
pub(crate) fn native_typed_array_fill(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "fill")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        id,
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let value = args.first().copied().unwrap_or(JsValue::Undefined);

    let start_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let start_idx = match start_arg {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let end_idx = match end_arg {
        JsValue::Undefined => len_elem,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };

    if end_idx > start_idx {
        // Coerce the user-supplied `value` exactly once — `valueOf`
        // / `Symbol.toPrimitive` may have observable side effects,
        // and the spec runs the conversion before any element
        // writes (so a thrown coercion leaves the array unmodified).
        let mut scratch = [0_u8; 8];
        let bpe = super::typed_array::coerce_element_to_le_bytes(ctx, ek, value, &mut scratch)?;
        // Compute `abs` in `usize` with checked arithmetic so a
        // malformed receiver (which the upstream view-relative
        // bounds check would normally catch) cannot wrap u32 in
        // `byte_offset + start_idx * bpe`, defeat
        // `fill_pattern`'s overflow guard, and write to the wrong
        // slot.  On overflow, surface the same silent no-op as
        // the byte_io guards.
        let byte_offset = byte_offset as usize;
        let start_us = start_idx as usize;
        let count = (end_idx - start_idx) as usize;
        if let Some(abs) = start_us
            .checked_mul(bpe)
            .and_then(|elem_off| byte_offset.checked_add(elem_off))
        {
            super::byte_io::fill_pattern(
                &mut ctx.vm.body_data,
                buffer_id,
                abs,
                &scratch[..bpe],
                count,
            );
        }
    }
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// subarray(begin?, end?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.subarray(begin?, end?)` (ES §23.2.3.27).
/// Returns a new TypedArray of the same subclass **sharing the
/// backing `ArrayBuffer`** — mutations through either view are
/// visible through the other.  Spec invariant:
/// `ta.subarray(0,1).buffer === ta.buffer`.
pub(crate) fn native_typed_array_subarray(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "subarray")?;
    let len_elem = parts.len_elem();
    let bpe = parts.bpe();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let begin_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let begin = match begin_arg {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let end = match end_arg {
        JsValue::Undefined => len_elem,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let new_len = end.saturating_sub(begin);
    let new_byte_offset = byte_offset + begin * bpe;
    let new_byte_length = new_len * bpe;
    let new_id = alloc_typed_array_view(ctx, ek, buffer_id, new_byte_offset, new_byte_length);
    Ok(JsValue::Object(new_id))
}

// ---------------------------------------------------------------------------
// slice(begin?, end?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.slice(begin?, end?)` (ES §23.2.3.25).
/// Returns a new TypedArray of the same built-in subclass **over a
/// fresh buffer** — mutations do not propagate between receiver and
/// slice.  Uses the receiver's `element_kind` as the allocation
/// ElementKind; `SpeciesConstructor` / user-subclass dispatch is
/// deferred to PR-spec-polish SP8.
///
/// Source and destination share the same `ElementKind`, so the
/// per-element decode/encode round-trip is unnecessary —
/// [`super::byte_io::copy_bytes`] performs the entire range copy
/// in one snapshot + one clone-grow-install.  O(N²) bytes-touched
/// → O(N).
pub(crate) fn native_typed_array_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "slice")?;
    let len_elem = parts.len_elem();
    let bpe = parts.bpe();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let begin_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let begin = match begin_arg {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let end = match end_arg {
        JsValue::Undefined => len_elem,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let new_len = end.saturating_sub(begin);
    let new_byte_length = new_len * bpe;

    let (new_buffer_id, _, _) = super::typed_array::allocate_fresh_buffer(ctx, new_byte_length)?;
    let new_view_id = alloc_typed_array_view(ctx, ek, new_buffer_id, 0, new_byte_length);
    // Bulk byte copy — same `ElementKind` source and destination,
    // so element-wise decode/encode would be a wasted round-trip.
    // `copy_bytes` snapshots the source range up front, so the new
    // buffer (`new_buffer_id`, fresh from `allocate_fresh_buffer`)
    // is written exactly once.  Offset math goes through `usize`
    // with `checked_*` to mirror the helper's overflow contract;
    // `len_elem` is bounded by `[[ByteLength]] / bpe`, but a
    // malformed receiver could still wrap u32 in
    // `byte_offset + begin * bpe`.
    let byte_offset_us = byte_offset as usize;
    let begin_us = begin as usize;
    let bpe_us = bpe as usize;
    if let Some(src_abs) = begin_us
        .checked_mul(bpe_us)
        .and_then(|elem_off| byte_offset_us.checked_add(elem_off))
    {
        super::byte_io::copy_bytes(
            &mut ctx.vm.body_data,
            buffer_id,
            src_abs,
            new_buffer_id,
            0,
            new_byte_length as usize,
        );
    }
    Ok(JsValue::Object(new_view_id))
}

// ---------------------------------------------------------------------------
// values / keys / entries / @@iterator
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.values()` (ES §23.2.3.34) — installed as
/// both `.values` and `[Symbol.iterator]` per §23.2.3.33.
pub(crate) fn native_typed_array_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    create_typed_array_iterator(ctx, this, 0, "values")
}

/// `%TypedArray%.prototype.keys()` (ES §23.2.3.20).
pub(crate) fn native_typed_array_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    create_typed_array_iterator(ctx, this, 1, "keys")
}

/// `%TypedArray%.prototype.entries()` (ES §23.2.3.8).
pub(crate) fn native_typed_array_entries(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    create_typed_array_iterator(ctx, this, 2, "entries")
}

/// Allocate a lazy `ObjectKind::ArrayIterator` over a TypedArray
/// receiver.  Reuses the Array iterator prototype + `.next`
/// implementation (`natives_symbol::native_array_iterator_next`
/// was extended in this commit to recognise TypedArray as an
/// iterable source and route element reads through
/// `get_element` — which in turn dispatches to the TypedArray
/// integer-indexed branch installed in C3).
fn create_typed_array_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    kind: u8,
    method: &str,
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, method)?;
    let iter_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: parts.id,
            index: 0,
            kind,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: ctx.vm.array_iterator_prototype,
        extensible: true,
    });
    Ok(JsValue::Object(iter_id))
}

// ---------------------------------------------------------------------------
// set(source, offset?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.set(source, offset?)` (ES §23.2.3.24).
///
/// Two branches:
/// - If `source` is a TypedArray, copy its elements into `this`
///   starting at `offset`.  Same-`ElementKind` source and destination
///   route through [`super::byte_io::copy_bytes`] for a single
///   pre-snapshot bulk copy (overlap-correct under any direction);
///   different `ElementKind` falls through to a per-element scratch
///   loop that performs the type conversion.
/// - Otherwise treat `source` as array-like: iterate `[0, src.length)`
///   and copy via `ToNumber` / `ToBigInt`.
///
/// RangeError when `offset + sourceLength > this.length`.
pub(crate) fn native_typed_array_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "set")?;
    let dst_len = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: dst_ek,
        ..
    } = parts;
    // ES §23.2.3.24 step 6: `ToIntegerOrInfinity(offset)`; step 8
    // uses the result in a `targetOffset + len > ArrayLength`
    // comparison which always fails for `±Infinity` / values beyond
    // `u32::MAX`.  Reject those up-front rather than saturating —
    // otherwise an empty `src` combined with a `u32::MAX`-sized
    // destination would silently accept an unrepresentable offset.
    let target_offset: u32 = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => {
            let n = ctx.to_number(other)?;
            let i = coerce::to_integer_or_infinity(n);
            if i < 0.0 || !i.is_finite() || i > f64::from(u32::MAX) {
                return Err(VmError::range_error(
                    "Failed to execute 'set' on 'TypedArray': offset is out of bounds",
                ));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                i as u32
            }
        }
    };

    let source = args.first().copied().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(src_id) = source {
        if let ObjectKind::TypedArray {
            buffer_id: src_buf,
            byte_offset: src_off,
            byte_length: src_bytelen,
            element_kind: src_ek,
        } = ctx.vm.get_object(src_id).kind
        {
            if src_ek.is_bigint() != dst_ek.is_bigint() {
                return Err(VmError::type_error(
                    "Failed to execute 'set' on 'TypedArray': Cannot mix BigInt and other types",
                ));
            }
            let src_bpe = u32::from(src_ek.bytes_per_element());
            let src_len = src_bytelen / src_bpe;
            if target_offset
                .checked_add(src_len)
                .is_none_or(|end| end > dst_len)
            {
                return Err(VmError::range_error(
                    "Failed to execute 'set' on 'TypedArray': offset + length out of range",
                ));
            }
            if src_ek == dst_ek {
                // Same `ElementKind` — per-element decode/encode
                // would round-trip through `JsValue` for nothing.
                // `copy_bytes` snapshots the source range up front,
                // so an in-place overlap (`src_buf == buffer_id`,
                // typical of `ta.set(ta.subarray(...))`) is correct
                // under any direction without a forward/backward
                // branch.  Offset math goes through `usize` with
                // `checked_*` to mirror `slice` / `copyWithin` /
                // `fill`'s overflow contract; the upfront
                // `target_offset + src_len > dst_len` RangeError
                // already rules out the realistic overflow paths,
                // but a malformed receiver could still wrap u32 in
                // `byte_offset + target_offset * bpe`.
                let bpe_us = src_bpe as usize;
                let dst_off_us = byte_offset as usize;
                if let Some(dst_abs) = (target_offset as usize)
                    .checked_mul(bpe_us)
                    .and_then(|elem_off| dst_off_us.checked_add(elem_off))
                {
                    super::byte_io::copy_bytes(
                        &mut ctx.vm.body_data,
                        src_buf,
                        src_off as usize,
                        buffer_id,
                        dst_abs,
                        src_bytelen as usize,
                    );
                }
                return Ok(JsValue::Undefined);
            }
            // Different `ElementKind` — per-element coerce loop
            // performs the type conversion.  Source and destination
            // MAY share the backing buffer; read every source
            // element upfront so the write pass doesn't observe
            // its own output (§23.2.3.24 step 26 same-buffer scratch).
            let mut scratch: Vec<JsValue> = Vec::with_capacity(src_len as usize);
            for i in 0..src_len {
                scratch.push(read_element_raw(ctx.vm, src_buf, src_off, i, src_ek));
            }
            for (i, val) in scratch.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let dst_i = target_offset + i as u32;
                write_element_raw(ctx, buffer_id, byte_offset, dst_i, dst_ek, val)?;
            }
            return Ok(JsValue::Undefined);
        }
    }

    // Array-like branch: §23.2.3.24 runs `ToObject(source)` so
    // primitives (strings, numbers, booleans) are wrapped and their
    // indexed/length access proceeds uniformly; only null/undefined
    // TypeError.  The fresh wrapper (primitive source) is only
    // reachable via the returned `src_id` integer — push it onto
    // `vm.stack` as a GC root for the duration of the read loop
    // and truncate unconditionally on every exit (early RangeError
    // or fallible `?` throw inside `get_element` / `write_element_raw`).
    let src_id = super::super::coerce::to_object(ctx.vm, source)?;
    let src_obj = JsValue::Object(src_id);
    let src_root = ctx.vm.stack.len();
    ctx.vm.stack.push(src_obj);
    let outcome = set_array_like_body(
        ctx,
        src_id,
        src_obj,
        buffer_id,
        byte_offset,
        target_offset,
        dst_len,
        dst_ek,
    );
    ctx.vm.stack.truncate(src_root);
    outcome
}

/// Inner body of `TypedArray.prototype.set` array-like branch,
/// extracted so the caller can truncate the GC-rooting stack entry
/// on every exit (success + every fallible `?` propagation).
#[allow(clippy::too_many_arguments)]
fn set_array_like_body(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    src_obj: JsValue,
    buffer_id: ObjectId,
    byte_offset: u32,
    target_offset: u32,
    dst_len: u32,
    dst_ek: ElementKind,
) -> Result<JsValue, VmError> {
    let length_sid = ctx.vm.well_known.length;
    let len_val =
        ctx.get_property_value(src_id, super::super::value::PropertyKey::String(length_sid))?;
    let len_f = ctx.to_number(len_val)?;
    // §23.2.3.24 step 11 runs `LengthOfArrayLike`/`ToLength`, which
    // clamps NaN and negative lengths to `0` (so `set({length: -1})`
    // is a no-op rather than a RangeError).  Only values larger than
    // `u32::MAX` exceed the engine's byte_length cap.
    let src_len = {
        let clamped = if len_f.is_nan() || len_f <= 0.0 {
            0.0
        } else {
            len_f.trunc()
        };
        if clamped > f64::from(u32::MAX) {
            return Err(VmError::range_error(
                "Failed to execute 'set' on 'TypedArray': source length exceeds the supported maximum",
            ));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let l = clamped as u32;
        l
    };
    if target_offset
        .checked_add(src_len)
        .is_none_or(|end| end > dst_len)
    {
        return Err(VmError::range_error(
            "Failed to execute 'set' on 'TypedArray': offset + length out of range",
        ));
    }
    // §23.2.3.24 step 13 reads `src[ToString(k)]` against the
    // wrapper object returned by `ToObject(source)`, not the raw
    // primitive — receiver identity affects any prototype-installed
    // getter that observes `this`.
    for i in 0..src_len {
        #[allow(clippy::cast_precision_loss)]
        let key = JsValue::Number(f64::from(i));
        let val = ctx.vm.get_element(src_obj, key)?;
        write_element_raw(ctx, buffer_id, byte_offset, target_offset + i, dst_ek, val)?;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// copyWithin(target, start, end?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.copyWithin(target, start, end?)`
/// (ES §23.2.3.6).  In-place byte copy with correct overlap
/// handling — [`super::byte_io::copy_bytes`] snapshots the source
/// range into an owned `Vec<u8>` before mutating the destination,
/// so any direction (forward / backward overlap) is sound.
/// Same `ElementKind` source and destination, so the per-element
/// decode/encode round-trip the previous impl performed is
/// redundant.  O(N²) bytes-touched → O(N).
pub(crate) fn native_typed_array_copy_within(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "copyWithin")?;
    let len_elem = parts.len_elem();
    let bpe = parts.bpe();
    let TypedArrayParts {
        id,
        buffer_id,
        byte_offset,
        ..
    } = parts;
    let target = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let start = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let end = match args.get(2).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => len_elem,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    let count = end
        .saturating_sub(start)
        .min(len_elem.saturating_sub(target));
    if count > 0 {
        // All offset math in `usize` with `checked_*` so a malformed
        // receiver can't wrap u32 and write to the wrong slot —
        // mirrors `fill`'s overflow contract.
        let byte_offset_us = byte_offset as usize;
        let bpe_us = bpe as usize;
        let count_us = count as usize;
        let src_abs = (start as usize)
            .checked_mul(bpe_us)
            .and_then(|elem_off| byte_offset_us.checked_add(elem_off));
        let dst_abs = (target as usize)
            .checked_mul(bpe_us)
            .and_then(|elem_off| byte_offset_us.checked_add(elem_off));
        let total_bytes = count_us.checked_mul(bpe_us);
        if let (Some(src_abs), Some(dst_abs), Some(total_bytes)) = (src_abs, dst_abs, total_bytes) {
            super::byte_io::copy_bytes(
                &mut ctx.vm.body_data,
                buffer_id,
                src_abs,
                buffer_id,
                dst_abs,
                total_bytes,
            );
        }
    }
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// reverse()
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.reverse()` (ES §23.2.3.23).  In-place
/// element swap, returns receiver.
pub(crate) fn native_typed_array_reverse(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "reverse")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        id,
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let mut lo = 0_u32;
    let mut hi = len_elem.saturating_sub(1);
    while lo < hi {
        let a = read_element_raw(ctx.vm, buffer_id, byte_offset, lo, ek);
        let b = read_element_raw(ctx.vm, buffer_id, byte_offset, hi, ek);
        write_element_raw(ctx, buffer_id, byte_offset, lo, ek, b)?;
        write_element_raw(ctx, buffer_id, byte_offset, hi, ek, a)?;
        lo += 1;
        hi -= 1;
    }
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Search: indexOf / lastIndexOf / includes / at
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.indexOf(searchElement, fromIndex?)`
/// (ES §23.2.3.15).  Strict equality (NaN is never equal to
/// NaN — unlike `includes`).  Returns `-1` on miss.
pub(crate) fn native_typed_array_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "indexOf")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let from = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    for i in from..len_elem {
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek);
        if coerce::strict_eq(ctx.vm, v, search) {
            #[allow(clippy::cast_precision_loss)]
            return Ok(JsValue::Number(f64::from(i)));
        }
    }
    Ok(JsValue::Number(-1.0))
}

/// `%TypedArray%.prototype.lastIndexOf(searchElement, fromIndex?)`
/// (ES §23.2.3.17).  Strict equality, reverse scan.
pub(crate) fn native_typed_array_last_index_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "lastIndexOf")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    if len_elem == 0 {
        return Ok(JsValue::Number(-1.0));
    }
    // ES §23.2.3.17 step 5: if the adjusted fromIndex (`len +
    // relativeIndex` when negative) is still < 0, return -1 — the
    // reverse scan has nothing to inspect.  Mirrors
    // `Array.prototype.lastIndexOf`.
    #[allow(clippy::cast_possible_truncation)]
    let from: i64 = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => i64::from(len_elem) - 1,
        other => {
            let k = coerce::to_integer_or_infinity(ctx.to_number(other)?);
            if k == f64::NEG_INFINITY {
                return Ok(JsValue::Number(-1.0));
            }
            if k < 0.0 {
                let adjusted = f64::from(len_elem) + k;
                if adjusted < 0.0 {
                    return Ok(JsValue::Number(-1.0));
                }
                adjusted as i64
            } else {
                (k as i64).min(i64::from(len_elem) - 1)
            }
        }
    };
    let mut i: i64 = from;
    while i >= 0 {
        #[allow(clippy::cast_sign_loss)] // i >= 0 guaranteed by loop condition
        let idx = i as u32;
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, idx, ek);
        if coerce::strict_eq(ctx.vm, v, search) {
            #[allow(clippy::cast_precision_loss)]
            return Ok(JsValue::Number(i as f64));
        }
        i -= 1;
    }
    Ok(JsValue::Number(-1.0))
}

/// `%TypedArray%.prototype.includes(searchElement, fromIndex?)`
/// (ES §23.2.3.16).  SameValueZero (NaN equals NaN).
pub(crate) fn native_typed_array_includes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "includes")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let from = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => relative_index_u32(ctx.to_number(other)?, len_elem),
    };
    for i in from..len_elem {
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek);
        if same_value_zero(ctx.vm, v, search) {
            return Ok(JsValue::Boolean(true));
        }
    }
    Ok(JsValue::Boolean(false))
}

/// `%TypedArray%.prototype.at(index)` (ES §23.2.3.3).  Negative
/// index wraps.  Out-of-range → `undefined`.
pub(crate) fn native_typed_array_at(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "at")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    // §23.2.3.3 step 3: `ToIntegerOrInfinity(index)` — NaN → 0,
    // ±Infinity preserved.  Bounds are applied uniformly below so
    // `at(NaN)` returns the first element (unless empty) and
    // `at(±Infinity)` returns `undefined`.
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let relative_index = coerce::to_integer_or_infinity(n);
    #[allow(clippy::cast_precision_loss)]
    let len_f = f64::from(len_elem);
    let idx_f = if relative_index < 0.0 {
        len_f + relative_index
    } else {
        relative_index
    };
    if idx_f < 0.0 || idx_f >= len_f {
        return Ok(JsValue::Undefined);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = idx_f as u32;
    Ok(read_element_raw(ctx.vm, buffer_id, byte_offset, idx, ek))
}

// ---------------------------------------------------------------------------
// join(separator?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.join(separator?)` (ES §23.2.3.19).
/// `separator` defaults to `","`; `undefined` also `,`.  Elements
/// are coerced via `ToString` (per-subclass number / bigint
/// formatting — BigInts stringify without the `n` suffix).
pub(crate) fn native_typed_array_join(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "join")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    // WTF-16 accumulation — preserves lone surrogates that user
    // overrides on `Number.prototype.toString` /
    // `BigInt.prototype.toString` could return.  The lossy
    // `StringPool::get_utf8` path would clobber them; mirror
    // `native_array_join`'s `Vec<u16>` + `intern_utf16` shape.
    let sep: Vec<u16> = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => vec![u16::from(b',')],
        other => {
            let sid = ctx.to_string_val(other)?;
            ctx.vm.strings.get(sid).to_vec()
        }
    };
    let mut out: Vec<u16> = Vec::new();
    for i in 0..len_elem {
        if i > 0 {
            out.extend_from_slice(&sep);
        }
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek);
        let sid = ctx.to_string_val(v)?;
        out.extend_from_slice(ctx.vm.strings.get(sid));
    }
    let out_sid = ctx.vm.strings.intern_utf16(&out);
    Ok(JsValue::String(out_sid))
}

// ---------------------------------------------------------------------------
// toLocaleString(reserved1?, reserved2?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.toLocaleString(reserved1?, reserved2?)`
/// (ES §23.2.3.31).  Per-element `? ToString(? Invoke(elem,
/// "toLocaleString", « locales, options »))`, joined with `","`.
///
/// elidex has no `Intl` support yet, so `(locales, options)` flow
/// through to per-element overrides unobserved by the built-in
/// [`super::super::natives_symbol::native_object_prototype_to_locale_string`]
/// shim (which redirects to `toString`).  Forwarding the reserved
/// args still matters for user overrides on
/// `Number.prototype.toLocaleString` /
/// `BigInt.prototype.toLocaleString`, which can read them.
///
/// TypedArray elements are always non-nullish (Number or BigInt),
/// so the `Array.prototype.toLocaleString` empty-or-nullish skip
/// (spec §22.1.3.30 step 7.a) doesn't apply here — every index
/// contributes a string segment.
///
/// ## Rooting
///
/// Boxed primitive wrappers (`coerce::to_object` outputs) are
/// pinned in a single rooted slot for the duration of each
/// iteration's `try_get_property_value` + `call_function` GC
/// points.  Without it a sufficiently aggressive GC could collect
/// the wrapper between the lookup-time accessor (which `GetV`
/// permits) and the call-time receiver dispatch, leaving
/// `obj_id` dangling.  Today GC is disabled across native calls
/// (`interpreter.rs:81`), so the pin is future-proofing —
/// matching the SP8c-A reduce-accumulator slot pattern.
pub(crate) fn native_typed_array_to_locale_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let parts = require_typed_array_parts(ctx, this, "toLocaleString")?;
    let len_elem = parts.len_elem();
    let TypedArrayParts {
        buffer_id,
        byte_offset,
        element_kind: ek,
        ..
    } = parts;
    let to_locale_key = PropertyKey::String(ctx.vm.well_known.to_locale_string);
    // §23.2.3.31 step 7 forwards exactly `« locales, options »` to
    // each per-element `Invoke` — extra caller-supplied args must
    // not reach the override.  Materialise the pair once outside the
    // loop so every iteration calls with the same fixed-arity slice.
    let invoke_args = [
        args.first().copied().unwrap_or(JsValue::Undefined),
        args.get(1).copied().unwrap_or(JsValue::Undefined),
    ];

    // Single rooted slot for the boxed-primitive wrapper.  Object
    // elements skip the box and overwrite the slot with the
    // already-pinned receiver id; primitive elements box and pin
    // their fresh wrapper.  Drop on early `?` is panic-safe via
    // the scope guard's truncate-on-drop.
    let mut frame = ctx.vm.push_stack_scope();
    let wrapper_slot = frame.saved_len();
    frame.stack.push(JsValue::Undefined);
    let mut sub_ctx = NativeContext { vm: &mut frame };

    // WTF-16 accumulation preserves lone surrogates: a user
    // override returning `'\uD800'` must round-trip exactly,
    // which the lossy `StringPool::get_utf8` path would clobber.
    // Mirrors `native_array_to_locale_string` / `native_array_join`.
    let mut out: Vec<u16> = Vec::new();
    for i in 0..len_elem {
        if i > 0 {
            out.push(u16::from(b','));
        }
        let elem = read_element_raw(sub_ctx.vm, buffer_id, byte_offset, i, ek);
        // Invoke(V, P, args) — GetV boxes the primitive element
        // via ToObject for the property lookup, but the call
        // receiver stays the original primitive so user overrides
        // see the raw element value rather than the wrapper.
        let (obj_id, receiver) = match elem {
            JsValue::Object(id) => (id, elem),
            primitive => (coerce::to_object(sub_ctx.vm, primitive)?, primitive),
        };
        sub_ctx.vm.stack[wrapper_slot] = JsValue::Object(obj_id);
        // GetV(V, P) (§7.3.2): the wrapper is just the prototype-
        // chain anchor for the *lookup*; an accessor getter must
        // see the original primitive `receiver` as `this`, not the
        // wrapper.  `try_get_property_value` resolves getters with
        // `this = Object(obj_id)` and would diverge from spec for
        // strict-mode user getters on `Number.prototype.toLocaleString`.
        // The `get_property` + `resolve_property` pair preserves
        // the spec receiver semantics — same shape as
        // `super::typed_array_static::lookup_iterator_method`.
        let method = match coerce::get_property(sub_ctx.vm, obj_id, to_locale_key) {
            Some(prop) => Some(sub_ctx.vm.resolve_property(prop, receiver)?),
            None => None,
        };
        let str_sid = match method {
            Some(JsValue::Object(fn_id)) if sub_ctx.get_object(fn_id).kind.is_callable() => {
                let ret = sub_ctx.call_function(fn_id, receiver, &invoke_args)?;
                sub_ctx.to_string_val(ret)?
            }
            // Per `Invoke` semantics (§7.3.16) `?Call(?GetV(V, P), …)`
            // throws TypeError when the resolved property is either
            // present-but-non-callable OR absent.  The `None` branch
            // covers the user-reachable case where `toLocaleString`
            // has been removed from the chain (e.g. via
            // `delete Object.prototype.toLocaleString`); silent
            // fallback to `ToString(receiver)` would mask user
            // mistakes like `Number.prototype.toLocaleString = 42`
            // and diverge from observable `Invoke` semantics.
            Some(_) | None => {
                return Err(VmError::type_error(
                    "Failed to execute 'toLocaleString' on 'TypedArray': \
                     element's toLocaleString is not callable",
                ));
            }
        };
        out.extend_from_slice(sub_ctx.vm.strings.get(str_sid));
    }
    drop(frame);
    let out_sid = ctx.vm.strings.intern_utf16(&out);
    Ok(JsValue::String(out_sid))
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// SameValueZero (ES §7.2.12) — used by `includes`.  NaN equals
/// NaN; `+0` equals `-0`.  BigInt comparison goes through the pool so
/// freshly-allocated handles with equal mathematical value match
/// (every TypedArray read for `BigInt64Array`/`BigUint64Array` mints
/// a new `BigIntId`).
fn same_value_zero(vm: &VmInner, a: JsValue, b: JsValue) -> bool {
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => x.is_nan() && y.is_nan() || x == y,
        (JsValue::BigInt(ai), JsValue::BigInt(bi)) => vm.bigints.get(ai) == vm.bigints.get(bi),
        _ => a == b,
    }
}
