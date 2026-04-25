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
//! - `forEach` / `every` / `some` / `find` / `findIndex`
//!
//! ## Deferred (PR-spec-polish SP8)
//!
//! `sort` / `map` / `filter` / `reduce` / `reduceRight` / `flatMap` /
//! `findLast` / `findLastIndex` / `toLocaleString` — all rely on
//! SpeciesConstructor or ICU.  Per-subclass `.of` / `.from` also
//! deferred (use identity ctor, don't need species, but require
//! ctor-ObjectId → ElementKind registry — adds state to VmInner).

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape;
use super::super::value::{
    ArrayIterState, ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind,
    PropertyStorage, VmError,
};
use super::super::VmInner;
use super::typed_array::{read_element_raw, write_element_raw};

// ---------------------------------------------------------------------------
// Shared brand-check helper
// ---------------------------------------------------------------------------

/// WebIDL brand-check for `%TypedArray%.prototype` methods.  Extracts
/// the four immutable spec slots inline from
/// [`ObjectKind::TypedArray`] in one pattern-match, so callers don't
/// repeat the destructuring five ways.
fn require_typed_array_parts(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(ObjectId, ObjectId, u32, u32, ElementKind), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        } => Ok((id, buffer_id, byte_offset, byte_length, element_kind)),
        _ => Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        ))),
    }
}

/// Clamp `n` to `[0, len]`, applying `ToIntegerOrInfinity`
/// truncation first (ES §7.1.5).  Negative indices count from the
/// end.  Shared by `fill` / `subarray` / `slice`.
fn relative_index_u32(n: f64, len: u32) -> u32 {
    if n.is_nan() {
        return 0;
    }
    let trunc = n.trunc();
    #[allow(clippy::cast_precision_loss)]
    let len_f = f64::from(len);
    let clamped = if trunc < 0.0 {
        (len_f + trunc).max(0.0)
    } else {
        trunc.min(len_f)
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = clamped as u32;
    out
}

/// ES §7.1.5 `ToIntegerOrInfinity`.  NaN → 0; ±Infinity preserved;
/// otherwise truncate toward zero.  Caller applies additional range
/// checks (e.g. "RangeError on negative") as needed.
fn to_integer_or_infinity(n: f64) -> f64 {
    if n.is_nan() {
        0.0
    } else {
        n.trunc()
    }
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
/// the receiver for chaining.  Current impl delegates to
/// `write_element_raw` per element, so coercion runs once per
/// iteration; a pre-coerced byte-level helper lands with SP9.
pub(crate) fn native_typed_array_fill(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "fill")?;
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;

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

    // O(N²) in bytes — coerces per iteration and re-clones the
    // whole backing Arc per write.  Deferred to SP9 (byte-level
    // fill helper) to keep the C4a change minimal.
    for i in start_idx..end_idx {
        write_element_raw(ctx, buffer_id, byte_offset, i, ek, value)?;
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "subarray")?;
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
pub(crate) fn native_typed_array_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "slice")?;
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
    for i in 0..new_len {
        let elem = read_element_raw(ctx.vm, buffer_id, byte_offset, begin + i, ek);
        write_element_raw(ctx, new_buffer_id, 0, i, ek, elem)?;
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
    let (id, _buffer_id, _byte_offset, _byte_length, _ek) =
        require_typed_array_parts(ctx, this, method)?;
    let iter_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: id,
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
///   starting at `offset`, with per-element type conversion and
///   overlap-aware copy (same-buffer case uses a scratch Vec).
/// - Otherwise treat `source` as array-like: iterate `[0, src.length)`
///   and copy via `ToNumber` / `ToBigInt`.
///
/// RangeError when `offset + sourceLength > this.length`.
pub(crate) fn native_typed_array_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_id, buffer_id, byte_offset, byte_length, dst_ek) =
        require_typed_array_parts(ctx, this, "set")?;
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
            let i = to_integer_or_infinity(n);
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
    let dst_bpe = u32::from(dst_ek.bytes_per_element());
    let dst_len = byte_length / dst_bpe;

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
                .map_or(true, |end| end > dst_len)
            {
                return Err(VmError::range_error(
                    "Failed to execute 'set' on 'TypedArray': offset + length out of range",
                ));
            }
            // Source and destination MAY share the backing buffer
            // (e.g. `ta.set(ta.subarray(0))`).  Read every source
            // element upfront so the write pass doesn't observe
            // its own output — spec §23.2.3.24 step 26 handles
            // this via an explicit same-buffer check + scratch
            // copy.  Our simpler full-read-then-write serialises
            // correctly for all cases.
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
        .map_or(true, |end| end > dst_len)
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
/// handling via an intermediate read-all-then-write-all pass.
/// Returns receiver for chaining.
pub(crate) fn native_typed_array_copy_within(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "copyWithin")?;
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
        let mut scratch: Vec<JsValue> = Vec::with_capacity(count as usize);
        for i in 0..count {
            scratch.push(read_element_raw(
                ctx.vm,
                buffer_id,
                byte_offset,
                start + i,
                ek,
            ));
        }
        for (i, v) in scratch.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let dst_i = target + i as u32;
            write_element_raw(ctx, buffer_id, byte_offset, dst_i, ek, v)?;
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
    let (id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "reverse")?;
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "indexOf")?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "lastIndexOf")?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
            let k = to_integer_or_infinity(ctx.to_number(other)?);
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
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, i as u32, ek);
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "includes")?;
    let search = args.first().copied().unwrap_or(JsValue::Undefined);
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "at")?;
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
    // §23.2.3.3 step 3: `ToIntegerOrInfinity(index)` — NaN → 0,
    // ±Infinity preserved.  Bounds are applied uniformly below so
    // `at(NaN)` returns the first element (unless empty) and
    // `at(±Infinity)` returns `undefined`.
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let relative_index = to_integer_or_infinity(n);
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
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, "join")?;
    let sep = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => ",".to_string(),
        other => {
            let sid = ctx.to_string_val(other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
    let mut out = String::new();
    for i in 0..len_elem {
        if i > 0 {
            out.push_str(&sep);
        }
        let v = read_element_raw(ctx.vm, buffer_id, byte_offset, i, ek);
        let sid = ctx.to_string_val(v)?;
        out.push_str(&ctx.vm.strings.get_utf8(sid));
    }
    let out_sid = ctx.vm.strings.intern(&out);
    Ok(JsValue::String(out_sid))
}

// ---------------------------------------------------------------------------
// HOFs: forEach / every / some / find / findIndex
// ---------------------------------------------------------------------------

/// Per-HOF short-circuit verdict.  `Short` returns the given value
/// immediately; `Continue` lets the loop advance to the next index.
enum HofDecision {
    Continue,
    Short(JsValue),
}

/// Iterate with callback.  `decide(i, elem, cb_result_is_truthy)`
/// is called once per element with ToBoolean already applied to
/// the callback result; it decides whether to short-circuit and
/// with what value.  Returns the final fallback on full drain.
fn iterate_with_callback(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    fallback: JsValue,
    mut decide: impl FnMut(u32, JsValue, bool) -> HofDecision,
) -> Result<JsValue, VmError> {
    let (_id, buffer_id, byte_offset, byte_length, ek) =
        require_typed_array_parts(ctx, this, method)?;
    let cb = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(id) if ctx.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'TypedArray': callback is not a function"
            )));
        }
    };
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let bpe = u32::from(ek.bytes_per_element());
    let len_elem = byte_length / bpe;
    for i in 0..len_elem {
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

/// `%TypedArray%.prototype.forEach(cb, thisArg?)` (ES §23.2.3.13).
pub(crate) fn native_typed_array_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    iterate_with_callback(ctx, this, args, "forEach", JsValue::Undefined, |_, _, _| {
        HofDecision::Continue
    })
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
