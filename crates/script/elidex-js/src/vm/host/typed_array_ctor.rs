//! TypedArray constructor `init_from_*` dispatch helpers
//! (ES2024 §23.2.5).
//!
//! Split from [`super::typed_array`] to keep both files below the
//! 1000-line convention (cleanup tranche 2).
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
//! [`super::typed_array::write_element_raw`] live in the parent
//! module because they're also reused by the indexed read/write path
//! and by [`super::typed_array_methods`].

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::value::{
    ElementKind, JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError,
};
use super::typed_array::{allocate_fresh_buffer, read_element_raw, write_element_raw};

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
        other => coerce::to_index_u32(ctx, other, ek.name(), "byteOffset")?,
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
            if !remainder.is_multiple_of(bpe) {
                return Err(VmError::range_error(format!(
                    "Failed to construct '{}': byte length of buffer should be a multiple of {bpe}",
                    ek.name()
                )));
            }
            remainder
        }
        other => {
            let length = coerce::to_index_u32(ctx, other, ek.name(), "length")?;
            let byte_len = length.checked_mul(bpe).ok_or_else(|| {
                VmError::range_error(format!(
                    "Failed to construct '{}': length too large",
                    ek.name()
                ))
            })?;
            if byte_offset
                .checked_add(byte_len)
                .is_none_or(|end| end > buf_len)
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
    let ObjectKind::TypedArray {
        buffer_id: src_buf_id,
        byte_offset: src_offset,
        byte_length: src_byte_len,
        element_kind: src_ek,
    } = ctx.vm.get_object(src_id).kind
    else {
        unreachable!("caller confirmed TypedArray kind")
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

    // Root `dst_buf_id` while we populate it.  In the current VM,
    // GC is disabled for the duration of native calls (and any
    // nested JS entered from them), so `write_element_raw`'s
    // `ToBigInt` / `valueOf` / `[Symbol.toPrimitive]` user-code
    // paths can't actually trigger a collection today.  We still
    // temp-root the freshly allocated buffer to preserve the
    // standard rooting invariant until the caller installs it on
    // the receiver TypedArray, and to future-proof the helper if
    // native-call GC behavior ever changes.  `read_element_raw`
    // itself allocates only a BigInt id, but the same stack root
    // covers it for free.  Use the RAII `push_temp_root` guard
    // (rather than bare `stack.push` / `stack.truncate`) so the
    // root is restored on every exit — including panic unwinding
    // past a `catch_unwind` boundary upstream.
    {
        let mut g = ctx.vm.push_temp_root(JsValue::Object(dst_buf_id));
        for i in 0..src_len_elem {
            let elem = read_element_raw(&mut g, src_buf_id, src_offset, i, src_ek);
            let mut sub_ctx = NativeContext { vm: &mut g };
            write_element_raw(&mut sub_ctx, dst_buf_id, dst_offset, i, dst_ek, elem)?;
        }
        // Guard `g` drops here, restoring the stack to the
        // pre-`push_temp_root` length.
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
    let iter @ JsValue::Object(_) = ctx.vm.call_value(using_iter, source, &[])? else {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': @@iterator must return an object",
            ek.name()
        )));
    };

    // Collect elements using the stack as GC-safe scratch: each
    // `iter_next` may execute user code, and Rust locals holding
    // `JsValue`s are invisible to the GC scanner.  The iterator
    // itself lives on the stack at `iter_slot`, with drained
    // elements pushed above it; both are released by the
    // [`super::super::VmInner::push_stack_scope`] guard's
    // `truncate(saved_len)` on every exit path (success, `?`
    // propagation, or panic unwinding).
    //
    // [`super::super::VmInner::push_temp_root`] doesn't fit here:
    // it asserts `stack.len() == saved_len + 1` on clean drop
    // (single rooted value), but `init_from_iterable_body` pushes
    // an arbitrary (data-dependent) number of drained elements on
    // top of the iter slot.  The looser stack-scope guard exists
    // for exactly this shape and gives the same panic-safe
    // restore semantics without the value-identity check.
    let mut frame = ctx.vm.push_stack_scope();
    let iter_slot = frame.saved_len();
    frame.stack.push(iter);
    let elem_start = iter_slot + 1;
    let mut sub_ctx = NativeContext { vm: &mut frame };
    init_from_iterable_body(&mut sub_ctx, iter, elem_start, ek)
    // `frame` drops here, restoring `stack.len()` to `iter_slot`
    // (releases the rooted iter + any drained elements remaining
    // above it).
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

    // Root `buf_id` on the stack TOP (above the already-rooted
    // iterator + drained elements at `elem_start..`) via the RAII
    // `push_temp_root` guard.  GC is currently disabled inside
    // native calls (and any nested JS entered from them), so the
    // write loop's `ToBigInt` / `valueOf` user-code path can't
    // actually trigger a sweep today; the rooting preserves the
    // standard invariant and future-proofs the helper.  Index
    // reads `stack[elem_start + i]` are unaffected because `i <
    // count_u32` and the buffer root sits at `elem_start +
    // count_u32`.  The guard restores the stack on every exit
    // (success, `?` propagation, panic unwinding); the parent
    // `init_from_iterable`'s stack-scope drop then releases the
    // iterator + any remaining drained elements.
    //
    // A throw during element write (e.g. `ToBigInt` on a Number
    // for a BigInt64Array) is a body-level abrupt completion —
    // the iterator has already been drained to exhaustion above,
    // so there is nothing to `IteratorClose`.  (IteratorClose is
    // only relevant when we exit MID-iteration — `iter_next`
    // throw is spec-exempt per §7.4.7, and the full-drain
    // pattern here never leaves the iterator open.)
    {
        let mut g = ctx.vm.push_temp_root(JsValue::Object(buf_id));
        for i in 0..count_u32 {
            let elem = g.stack[elem_start + i as usize];
            let mut sub_ctx = NativeContext { vm: &mut g };
            write_element_raw(&mut sub_ctx, buf_id, offset, i, ek, elem)?;
        }
        // Guard `g` drops here, restoring stack to the
        // pre-`push_temp_root` length.
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

    // Root `buf_id` via the RAII `push_temp_root` guard for the
    // duration of the get/write loop.  `get_element` runs
    // user-defined getters / proxies, and `write_element_raw`'s
    // `ToBigInt` / `valueOf` path runs user code too.  GC is
    // currently disabled inside native calls (and any nested JS
    // entered from them), so neither of those paths can actually
    // sweep the freshly allocated buffer today; the rooting
    // preserves the standard invariant and future-proofs the
    // helper if native-call GC behavior ever changes.  The guard
    // restores the stack on every exit including panic unwinding.
    {
        let mut g = ctx.vm.push_temp_root(JsValue::Object(buf_id));
        let source = JsValue::Object(src_id);
        for i in 0..length {
            // `get_element` dispatches through the full element-get
            // pipeline (Array dense, TypedArray integer-indexed,
            // prototype chain), matching what a plain `source[i]` would
            // see from user code.
            let elem = g.get_element(source, JsValue::Number(f64::from(i)))?;
            let mut sub_ctx = NativeContext { vm: &mut g };
            write_element_raw(&mut sub_ctx, buf_id, offset, i, ek, elem)?;
        }
        // Guard `g` drops here, restoring stack.
    }

    Ok((buf_id, offset, byte_len))
}
