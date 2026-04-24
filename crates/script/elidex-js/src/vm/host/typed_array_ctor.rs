//! TypedArray constructor `init_from_*` dispatch helpers
//! (ES2024 §23.2.5).
//!
//! Split from [`super::typed_array`] to keep both files below the
//! 1000-line convention (cleanup tranche 2 lesson).
//! [`super::typed_array::construct_typed_array`] dispatches on the
//! `args[0]` shape into one of three public entry points here
//! ([`init_from_array_buffer`] / [`init_from_typed_array`] /
//! [`init_from_iterable`]); the iterable path then falls through to
//! the array-like helper internally if `@@iterator` is missing or
//! `null`/`undefined`.  Every entry point returns the
//! `(buffer_id, byte_offset, byte_length)` triple that
//! `construct_typed_array` bakes into the receiver's
//! [`super::super::value::ObjectKind::TypedArray`] kind.
//!
//! ## Dispatch table
//!
//! | `args[0]` shape                       | Spec ref                  | Entry point                  |
//! |---------------------------------------|---------------------------|------------------------------|
//! | `ArrayBuffer` (with `byteOffset?` / `length?`) | §23.2.5.1.3      | [`init_from_array_buffer`]   |
//! | another `%TypedArray%`                | §23.2.5.1.2               | [`init_from_typed_array`]    |
//! | iterable (callable `@@iterator`)      | §23.2.5.1.4               | [`init_from_iterable`]       |
//! | other Object (array-like fallback)    | §23.2.5.1 steps 9-12      | reached via `init_from_iterable` → `init_from_array_like` |
//!
//! The fresh-zero-buffer arms (`()` / `(undefined)` / `(number)`) are
//! handled inline in `construct_typed_array` because they don't
//! consult the source object — they only call
//! [`super::typed_array::allocate_fresh_buffer`] directly.
//!
//! Low-level [`super::typed_array::read_element_raw`] /
//! [`super::typed_array::write_element_raw`] /
//! [`super::typed_array::to_index_u32`] live in the parent module
//! because they're also reused by the indexed read/write path and by
//! [`super::typed_array_methods`].

#![cfg(feature = "engine")]

use super::super::value::{
    ElementKind, JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError,
};
use super::typed_array::{
    allocate_fresh_buffer, read_element_raw, to_index_u32, write_element_raw,
};

/// Variant 3: share an existing `ArrayBuffer`.  Validates
/// `byteOffset % bpe === 0` (RangeError) and range coverage
/// (`byteOffset + byteLength ≤ buffer.byteLength`).  `length`
/// in elements is multiplied by `bpe` to derive the byte count.
pub(super) fn init_from_array_buffer(
    ctx: &mut NativeContext<'_>,
    buffer_id: ObjectId,
    byte_offset_arg: JsValue,
    length_arg: JsValue,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let buf_len_usize = super::array_buffer::array_buffer_byte_length(ctx.vm, buffer_id);
    // Clamp the buffer byte length into `u32` — anything larger
    // can't be addressed by the `[[ByteLength]]` slot anyway
    // (u32 on the TypedArray variant).
    let buf_len: u32 = buf_len_usize.try_into().map_err(|_| {
        VmError::range_error(format!(
            "Failed to construct '{}': ArrayBuffer is larger than 4 GiB",
            ek.name()
        ))
    })?;

    let byte_offset = match byte_offset_arg {
        JsValue::Undefined => 0,
        other => {
            let n = ctx.to_number(other)?;
            to_index_u32(n, ek.name(), "byteOffset")?
        }
    };
    if byte_offset % bpe != 0 {
        return Err(VmError::range_error(format!(
            "Failed to construct '{}': start offset must be a multiple of {bpe}",
            ek.name()
        )));
    }
    if byte_offset > buf_len {
        return Err(VmError::range_error(format!(
            "Failed to construct '{}': byteOffset {byte_offset} exceeds ArrayBuffer length {buf_len}",
            ek.name()
        )));
    }

    let byte_length = match length_arg {
        JsValue::Undefined => {
            // Auto-length: `buffer.byteLength - byteOffset`, and
            // that remainder must itself be aligned.
            let remainder = buf_len - byte_offset;
            if remainder % bpe != 0 {
                return Err(VmError::range_error(format!(
                    "Failed to construct '{}': byte length of buffer should be a multiple of {bpe}",
                    ek.name()
                )));
            }
            remainder
        }
        other => {
            let n = ctx.to_number(other)?;
            let length = to_index_u32(n, ek.name(), "length")?;
            let byte_len = length.checked_mul(bpe).ok_or_else(|| {
                VmError::range_error(format!(
                    "Failed to construct '{}': length too large",
                    ek.name()
                ))
            })?;
            if byte_offset
                .checked_add(byte_len)
                .map_or(true, |end| end > buf_len)
            {
                return Err(VmError::range_error(format!(
                    "Failed to construct '{}': length out of range of buffer",
                    ek.name()
                )));
            }
            byte_len
        }
    };

    Ok((buffer_id, byte_offset, byte_length))
}

/// Variant 4: `new Xxx(otherTypedArray)`.  Allocates a fresh
/// buffer sized to the source's `length`, then copies each
/// element through the destination's type coercion.  Source and
/// destination may have different ElementKinds (e.g. `new
/// Uint8Array(new Float32Array([1.7, 2.9]))` coerces to `[1, 2]`).
pub(super) fn init_from_typed_array(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    dst_ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let (src_buf_id, src_offset, src_byte_len, src_ek) = match ctx.vm.get_object(src_id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        } => (buffer_id, byte_offset, byte_length, element_kind),
        _ => unreachable!("caller confirmed TypedArray kind"),
    };
    // Content-type compatibility: BigInt ↔ Number mixing throws
    // TypeError per ES §23.2.5.1.2 step 17 (subclass check).
    if src_ek.is_bigint() != dst_ek.is_bigint() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': Cannot mix BigInt and other types",
            dst_ek.name()
        )));
    }
    let src_len_elem = src_byte_len / u32::from(src_ek.bytes_per_element());
    let dst_byte_len = src_len_elem
        .checked_mul(u32::from(dst_ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                dst_ek.name()
            ))
        })?;
    let (dst_buf_id, dst_offset, _) = allocate_fresh_buffer(ctx, dst_byte_len)?;

    // Element-by-element copy with per-kind coercion.  Same
    // underlying buffer case (`src_buf_id == dst_buf_id`) isn't
    // possible here because `dst_buf_id` is freshly allocated.
    // BigInt read path needs &mut VmInner for the alloc — the
    // two borrows never overlap because `read_element_raw` returns
    // a JsValue before `write_element_raw` touches the VM again.
    for i in 0..src_len_elem {
        let elem = read_element_raw(ctx.vm, src_buf_id, src_offset, i, src_ek);
        write_element_raw(ctx, dst_buf_id, dst_offset, i, dst_ek, elem)?;
    }

    Ok((dst_buf_id, dst_offset, dst_byte_len))
}

/// Variant 5: `new Xxx(object)`.  ES §23.2.5.1 steps 7-12: if the
/// source has a callable `@@iterator`, iterate; otherwise fall back
/// to the array-like path (`length` + integer-indexed `[[Get]]`s).
pub(super) fn init_from_iterable(
    ctx: &mut NativeContext<'_>,
    source: JsValue,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let JsValue::Object(src_id) = source else {
        unreachable!("init_from_iterable called on non-Object");
    };
    // §23.2.5.1 step 7 / GetMethod(object, @@iterator): an absent
    // or `undefined` / `null` value for `@@iterator` falls through
    // to the array-like branch rather than throwing.
    let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
    let using_iter = match super::super::coerce::get_property(ctx.vm, src_id, iter_key) {
        Some(pr) => ctx.vm.resolve_property(pr, source)?,
        None => JsValue::Undefined,
    };
    if matches!(using_iter, JsValue::Null | JsValue::Undefined) {
        return init_from_array_like(ctx, src_id, ek);
    }
    let iter = match ctx.vm.call_value(using_iter, source, &[])? {
        it @ JsValue::Object(_) => it,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to construct '{}': @@iterator must return an object",
                ek.name()
            )));
        }
    };

    // Collect elements using the stack as GC-safe scratch: each
    // `iter_next` may execute user code that triggers GC, and Rust
    // locals holding `JsValue`s are invisible to the scanner.  The
    // iterator itself lives on the stack below the elements so it
    // survives any intervening GC even when the outer `args` slice
    // no longer reaches it (the `@@iterator` call's return value is
    // freshly allocated and not transitively rooted via `source`).
    // Every exit path — success, RangeError, or a `?` propagation
    // from inside `iter_next` / `write_element_raw` — truncates
    // back to `iter_slot` via the outer-scope helper call.
    let iter_slot = ctx.vm.stack.len();
    ctx.vm.stack.push(iter);
    let elem_start = iter_slot + 1;
    let outcome = init_from_iterable_body(ctx, iter, elem_start, ek);
    ctx.vm.stack.truncate(iter_slot);
    outcome
}

/// Inner body of [`init_from_iterable`], extracted so the caller can
/// truncate the GC-rooting stack prefix unconditionally on every
/// exit (spec `?` throws + inlined RangeError short-circuits alike).
fn init_from_iterable_body(
    ctx: &mut NativeContext<'_>,
    iter: JsValue,
    elem_start: usize,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    loop {
        let next = ctx.vm.iter_next(iter)?;
        match next {
            Some(v) => ctx.vm.stack.push(v),
            None => break,
        }
    }
    let count = ctx.vm.stack.len() - elem_start;
    let count_u32 = u32::try_from(count).map_err(|_| {
        VmError::range_error(format!(
            "Failed to construct '{}': too many elements in source iterable",
            ek.name()
        ))
    })?;
    let byte_len = count_u32
        .checked_mul(u32::from(ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                ek.name()
            ))
        })?;
    let (buf_id, offset, _) = allocate_fresh_buffer(ctx, byte_len)?;

    // Drain elements off the stack into the buffer.  A throw
    // during element write (e.g. `ToBigInt` on a Number for a
    // BigInt64Array) is a body-level abrupt completion — the
    // iterator has already been drained to exhaustion above, so
    // there is nothing to `IteratorClose`.  (IteratorClose is
    // only relevant when we exit MID-iteration — `iter_next`
    // throw is spec-exempt per §7.4.7, and the full-drain
    // pattern here never leaves the iterator open.)
    for i in 0..count_u32 {
        let elem = ctx.vm.stack[elem_start + i as usize];
        write_element_raw(ctx, buf_id, offset, i, ek, elem)?;
    }

    Ok((buf_id, offset, byte_len))
}

/// Variant 5b: §23.2.5.1 array-like fallback when `source` has no
/// callable `@@iterator`.  Reads `source.length` → `ToLength` →
/// allocates buffer → drains `source[i]` through the shared property
/// path (§23.2.5.1 steps 9-12).  `ToLength` clamps negative / NaN
/// lengths to `0` (an empty TypedArray) rather than throwing —
/// `{length: -1}` / `{length: NaN}` produce `new Uint8Array(0)`.
fn init_from_array_like(
    ctx: &mut NativeContext<'_>,
    src_id: ObjectId,
    ek: ElementKind,
) -> Result<(ObjectId, u32, u32), VmError> {
    let length_sid = ctx.vm.well_known.length;
    let len_val = ctx.get_property_value(src_id, PropertyKey::String(length_sid))?;
    let len_f = ctx.to_number(len_val)?;
    let length = {
        let clamped = if len_f.is_nan() || len_f <= 0.0 {
            0.0
        } else {
            len_f.trunc()
        };
        if clamped > f64::from(u32::MAX) {
            return Err(VmError::range_error(format!(
                "Failed to construct '{}': source length exceeds the supported maximum",
                ek.name()
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let l = clamped as u32;
        l
    };
    let byte_len = length
        .checked_mul(u32::from(ek.bytes_per_element()))
        .ok_or_else(|| {
            VmError::range_error(format!(
                "Failed to construct '{}': length too large",
                ek.name()
            ))
        })?;
    let (buf_id, offset, _) = allocate_fresh_buffer(ctx, byte_len)?;
    let source = JsValue::Object(src_id);
    for i in 0..length {
        // `get_element` dispatches through the full element-get
        // pipeline (Array dense, TypedArray integer-indexed,
        // prototype chain), matching what a plain `source[i]` would
        // see from user code.
        let elem = ctx.vm.get_element(source, JsValue::Number(f64::from(i)))?;
        write_element_raw(ctx, buf_id, offset, i, ek, elem)?;
    }
    Ok((buf_id, offset, byte_len))
}
