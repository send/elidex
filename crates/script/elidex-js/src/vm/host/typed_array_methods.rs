//! `%TypedArray%.prototype` method bodies (ES2024 §23.2.3).
//!
//! Split from [`super::typed_array`] to keep both files below the
//! 1000-line convention (PR5a-fetch lesson).  The install-time
//! table in `super::typed_array::install_typed_array_prototype_members`
//! wires each native into `%TypedArray%.prototype`, shared across
//! all 11 subclasses via the prototype chain.
//!
//! ## Scope (C4a)
//!
//! - `fill(value, start?, end?)` (§23.2.3.11)
//! - `subarray(begin?, end?)` (§23.2.3.27) — shares backing buffer
//! - `slice(begin?, end?)` (§23.2.3.25) — fresh buffer copy
//! - `values()` / `keys()` / `entries()` (§23.2.3.34 / .20 / .8) —
//!   reuse `ObjectKind::ArrayIterator` infra (natives_symbol.rs
//!   `native_array_iterator_next` extended to read TypedArray
//!   byte-level elements).
//!
//! ## Deferred (C4b / PR-spec-polish)
//!
//! `set` / `copyWithin` / `reverse` / `indexOf` / `lastIndexOf` /
//! `includes` / `at` / `forEach` / `every` / `some` / `find` /
//! `findIndex` / `sort` / `map` / `filter` / `reduce` / `toLocaleString`
//! land in follow-up commits.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    ArrayIterState, ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind,
    PropertyStorage, VmError,
};
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

/// Allocate a new TypedArray wrapper of the given `ElementKind` over
/// `buffer_id` at `byte_offset` / `byte_length`.  Uses the abstract
/// `%TypedArray%.prototype` chain via the ctor's own prototype
/// (identity constructor — same subclass as receiver; SpeciesConstructor
/// support deferred to PR-spec-polish SP8).
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

/// Resolve the per-subclass prototype stored on `VmInner`.  Falls
/// back to the abstract `%TypedArray%.prototype` if the subclass
/// field is unexpectedly `None` (shouldn't happen — all 11 fields
/// populate during `register_typed_array_prototype_global`).
fn subclass_prototype_for(vm: &super::super::VmInner, ek: ElementKind) -> Option<ObjectId> {
    let sub = match ek {
        ElementKind::Int8 => vm.int8_array_prototype,
        ElementKind::Uint8 => vm.uint8_array_prototype,
        ElementKind::Uint8Clamped => vm.uint8_clamped_array_prototype,
        ElementKind::Int16 => vm.int16_array_prototype,
        ElementKind::Uint16 => vm.uint16_array_prototype,
        ElementKind::Int32 => vm.int32_array_prototype,
        ElementKind::Uint32 => vm.uint32_array_prototype,
        ElementKind::Float32 => vm.float32_array_prototype,
        ElementKind::Float64 => vm.float64_array_prototype,
        ElementKind::BigInt64 => vm.bigint64_array_prototype,
        ElementKind::BigUint64 => vm.biguint64_array_prototype,
    };
    sub.or(vm.typed_array_prototype)
}

// ---------------------------------------------------------------------------
// fill(value, start?, end?)
// ---------------------------------------------------------------------------

/// `%TypedArray%.prototype.fill(value, start?, end?)` (ES §23.2.3.11).
/// Coerces `value` to the receiver's element kind once, then writes
/// the encoded bytes repeatedly across `[start, end)`.  Returns the
/// receiver for chaining.
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

    // Coerce + encode once, then replicate the scratch bytes.
    // Delegating to `write_element_raw` on every iteration would
    // re-coerce the value N times and re-clone the whole buffer
    // N times; instead we coerce once and replicate.
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
/// Returns a new TypedArray of the same subclass **over a fresh
/// buffer** — mutations do not propagate between receiver and
/// slice.  Identity constructor (SpeciesConstructor deferred).
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

    // Allocate a fresh ArrayBuffer of the right size, then
    // element-copy through the shared read_element / write_element
    // path (same element_kind on both sides → no coercion).
    use std::sync::Arc;
    let bytes: Arc<[u8]> = if new_byte_length == 0 {
        Arc::from(&[][..])
    } else {
        vec![0_u8; new_byte_length as usize].into()
    };
    let new_buffer_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    let new_view_id = alloc_typed_array_view(ctx, ek, new_buffer_id, 0, new_byte_length);
    for i in 0..new_len {
        let elem = read_element_raw(ctx.vm, buffer_id, byte_offset, begin + i, ek);
        write_element_raw(ctx, new_buffer_id, 0, i, ek, elem)?;
    }
    // Drop unused id (caller sees `new_view_id`).  Prevents the
    // temporary variable from being seen as unused.
    let _ = new_view_id;
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
