//! `%TypedArray%` + concrete subclasses (ES2024 §23.2): receiver
//! brand-check, generic IDL accessors, the shared constructor
//! dispatch, and the byte-level element read / write primitives.
//!
//! `%TypedArray%` is an abstract base class — not exposed as a global,
//! reachable only via `Object.getPrototypeOf(Uint8Array)` etc. — that
//! carries shared IDL attrs (`buffer` / `byteOffset` / `byteLength` /
//! `length`) plus the prototype method suite (`fill` / `set` /
//! `subarray` / …).  11 concrete subclasses (`Int8Array` /
//! `Uint8Array` / `Uint8ClampedArray` / `Int16Array` / `Uint16Array` /
//! `Int32Array` / `Uint32Array` / `Float32Array` / `Float64Array` /
//! `BigInt64Array` / `BigUint64Array`) chain their prototype to
//! `%TypedArray%.prototype` and differ only by the
//! [`super::super::value::ElementKind`] tag baked into each
//! instance's [`ObjectKind::TypedArray`] variant.
//!
//! ```text
//! new Uint8Array(n)           ObjectKind::TypedArray { element_kind: Uint8, … }
//!   → Uint8Array.prototype
//!     → %TypedArray%.prototype
//!       → Object.prototype
//! ```
//!
//! ## File split
//!
//! Boot-only `Vm::new` registration and the prototype-method
//! install table live in [`super::typed_array_install`] (sibling
//! split — keeps both files under the project's 1000-line
//! convention as the install surface grew across PR-spec-polish
//! SP8b/SP8c).  The module here is reached on **every** indexed
//! access (`read_element_raw` / `write_element_raw`) and on every
//! IDL accessor invocation, so the brand-check + getter +
//! constructor-dispatch surface stays here.
//!
//! ## Byte-order convention
//!
//! TypedArray indexed reads / writes use **little-endian byte order
//! unconditionally** — an elidex implementation choice for
//! cross-platform determinism.  `IsLittleEndian()` (ES §25.1.3.1) is
//! implementation-defined, so a constant choice is spec-compliant.
//! [`super::data_view::DataView`] (PR5-typed-array §C5) exposes both
//! endiannesses explicitly via its `littleEndian` argument (ES
//! §25.3.4, default `false`).
//!
//! ## Backing storage
//!
//! A TypedArray is a **view**: the bytes live in the underlying
//! [`ObjectKind::ArrayBuffer`] (shared [`super::super::VmInner::body_data`]
//! entry), and every view over the same buffer mutates the same
//! bytes.  The view's `[[ByteOffset]]` / `[[ByteLength]]` slots
//! stored inline on `ObjectKind::TypedArray` translate JS indices to
//! buffer offsets.  No side-table is needed because all four spec
//! slots are immutable after construction in this PR —
//! `transfer()` / `resize()` / `detached` tracking (ES2024) are
//! deferred to the M4-12 cutover-residual tranche.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::value::{ElementKind, JsValue, NativeContext, ObjectId, ObjectKind, VmError};
use super::super::VmInner;
use super::typed_array_ctor::{init_from_array_buffer, init_from_iterable, init_from_typed_array};

// ---------------------------------------------------------------------------
// Generic prototype accessors
// ---------------------------------------------------------------------------

fn require_typed_array_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::TypedArray { .. }) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )))
    }
}

pub(super) fn native_typed_array_get_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "buffer")?;
    let ObjectKind::TypedArray { buffer_id, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Object(buffer_id))
}

pub(super) fn native_typed_array_get_byte_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteOffset")?;
    let ObjectKind::TypedArray { byte_offset, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_offset)))
}

pub(super) fn native_typed_array_get_byte_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteLength")?;
    let ObjectKind::TypedArray { byte_length, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_length)))
}

pub(super) fn native_typed_array_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "length")?;
    let ObjectKind::TypedArray {
        byte_length,
        element_kind,
        ..
    } = ctx.vm.get_object(id).kind
    else {
        unreachable!("brand-check passed");
    };
    let bpe = u32::from(element_kind.bytes_per_element());
    let len = byte_length / bpe;
    Ok(JsValue::Number(f64::from(len)))
}

// ---------------------------------------------------------------------------
// Shared constructor dispatch (ES §23.2.5)
// ---------------------------------------------------------------------------

/// Shared body of every TypedArray subclass ctor.  Dispatches on
/// `args[0]` shape per ES §23.2.5:
/// 1. `() / (undefined)` → empty view over fresh zero-byte buffer.
/// 2. `(number)` → `ToIndex(n)`, fresh zero-filled buffer of
///    `n * bpe` bytes (§23.2.5.1.1).
/// 3. `(ArrayBuffer, byteOffset?, length?)` → share buffer bytes,
///    validate alignment (`byteOffset % bpe === 0` — RangeError)
///    and range (§23.2.5.1.3).
/// 4. `(TypedArray)` → fresh buffer of `src.length * dst.bpe` bytes,
///    element-copy with type conversion (§23.2.5.1.2).
/// 5. `(iterable)` where `@@iterator` resolves → consume iterator,
///    allocate buffer, write each element (§23.2.5.1.4).  Any
///    throw during body is closed with `IteratorClose` per
///    §7.4.6 (but a throw from `iter_next` itself is NOT
///    closed — §7.4.7, PR5b R13 lesson).
/// 6. Otherwise → TypeError.
///
/// The pre-allocated receiver carries `new.target.prototype` via
/// `do_new`; we promote its `kind` in-place rather than reassigning
/// `prototype`, so subclasses of our builtins (`class X extends
/// Uint8Array`) inherit correctly (PR5a2 R7.2/R7.3 lesson).
#[allow(clippy::similar_names)] // arg0/arg1/arg2 mirrors WebIDL parameter indexing
pub(super) fn construct_typed_array(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    ek: ElementKind,
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': Please use the 'new' operator",
            ek.name()
        )));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let arg0 = args.first().copied().unwrap_or(JsValue::Undefined);
    let arg1 = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let arg2 = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let (buffer_id, byte_offset, byte_length) = match arg0 {
        JsValue::Undefined => allocate_fresh_buffer(ctx, 0)?,
        JsValue::Object(src_id) => match ctx.vm.get_object(src_id).kind {
            ObjectKind::ArrayBuffer => init_from_array_buffer(ctx, src_id, arg1, arg2, ek)?,
            ObjectKind::TypedArray { .. } => init_from_typed_array(ctx, src_id, ek)?,
            _ => init_from_iterable(ctx, arg0, ek)?,
        },
        // Plain number (or anything number-coercible via ToNumber)
        // → length form.  Strings like `"5"` coerce too; this
        // matches V8 / SpiderMonkey.  NaN → 0-length per ToIndex.
        _ => {
            let length = coerce::to_index_u32(ctx, arg0, ek.name(), "length")?;
            let byte_len = length
                .checked_mul(u32::from(ek.bytes_per_element()))
                .ok_or_else(|| {
                    VmError::range_error(format!(
                        "Failed to construct '{}': length too large",
                        ek.name()
                    ))
                })?;
            allocate_fresh_buffer(ctx, byte_len)?
        }
    };

    // Preserve `prototype` on the pre-allocated instance so
    // `new.target.prototype` chains work for subclasses (PR5a2 R7.2/R7.3).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::TypedArray {
        buffer_id,
        byte_offset,
        byte_length,
        element_kind: ek,
    };
    Ok(JsValue::Object(inst_id))
}

/// Allocate a fresh `ArrayBuffer` of `byte_len` zero bytes and
/// return its `(ObjectId, byte_offset=0, byte_length)` triple.
/// Uses the shared `body_data` store so GC sweep prunes it
/// alongside other ArrayBuffers.
pub(super) fn allocate_fresh_buffer(
    ctx: &mut NativeContext<'_>,
    byte_len: u32,
) -> Result<(ObjectId, u32, u32), VmError> {
    let bytes: Vec<u8> = if byte_len == 0 {
        Vec::new()
    } else {
        vec![0_u8; byte_len as usize]
    };
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    Ok((buf_id, 0, byte_len))
}

// ---------------------------------------------------------------------------
// Low-level element read/write (shared with indexed access in C3)
// ---------------------------------------------------------------------------

/// Read the element at `index` from the buffer backing a
/// TypedArray, decoded per `ek`.  Little-endian byte order
/// (elidex convention, documented at module header).  Missing
/// body_data entry (e.g. freshly allocated zero-byte buffer) is
/// treated as all zeros.
///
/// Takes `&mut VmInner` because BigInt element reads need to
/// `alloc` a BigIntId on the interning pool.  For non-BigInt
/// subclasses no allocation occurs.
pub(crate) fn read_element_raw(
    vm: &mut VmInner,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
) -> JsValue {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;
    // Snapshot exactly `bpe` bytes per element kind so each
    // subscript copies only the bytes the decoder will actually
    // consume — small-element reads (e.g. `Uint8Array`) are hot
    // and must not pay for the wider fixed-size scratch.  The
    // const-generic `read_into` produces a per-arm fixed-size
    // array; the BigInt arms own a `&mut vm` reborrow afterwards
    // so the snapshot must complete before the `bigints.alloc`.
    match ek {
        ElementKind::Int8 => {
            let s = super::byte_io::read_into::<1>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(s[0].cast_signed()))
        }
        ElementKind::Uint8 | ElementKind::Uint8Clamped => {
            let s = super::byte_io::read_into::<1>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(s[0]))
        }
        ElementKind::Int16 => {
            let s = super::byte_io::read_into::<2>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(i16::from_le_bytes(s)))
        }
        ElementKind::Uint16 => {
            let s = super::byte_io::read_into::<2>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(u16::from_le_bytes(s)))
        }
        ElementKind::Int32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(i32::from_le_bytes(s)))
        }
        ElementKind::Uint32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(u32::from_le_bytes(s)))
        }
        ElementKind::Float32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(f32::from_le_bytes(s)))
        }
        ElementKind::Float64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from_le_bytes(s))
        }
        ElementKind::BigInt64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            let v = i64::from_le_bytes(s);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
        ElementKind::BigUint64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            let v = u64::from_le_bytes(s);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
    }
}

/// Coerce `value` per `ek` and serialise the per-element
/// little-endian byte sequence into `out`, returning the number of
/// bytes written (always equal to `ek.bytes_per_element()`).
/// Shared by [`write_element_raw`] (single-element writes) and
/// [`super::typed_array_methods::native_typed_array_fill`] (one
/// coerce per fill, not per element).
///
/// Coercion may run user code (`valueOf` / `Symbol.toPrimitive`)
/// and throw — callers therefore invoke this before any
/// irreversible mutation, so a thrown coercion leaves the backing
/// buffer untouched.
pub(crate) fn coerce_element_to_le_bytes(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    value: JsValue,
    out: &mut [u8; 8],
) -> Result<usize, VmError> {
    Ok(match ek {
        ElementKind::Int8 => {
            let v = super::super::coerce::to_int8(ctx.vm, value)?;
            out[0] = v.cast_unsigned();
            1
        }
        ElementKind::Uint8 => {
            let v = super::super::coerce::to_uint8(ctx.vm, value)?;
            out[0] = v;
            1
        }
        ElementKind::Uint8Clamped => {
            let v = super::super::coerce::to_uint8_clamp(ctx.vm, value)?;
            out[0] = v;
            1
        }
        ElementKind::Int16 => {
            let v = super::super::coerce::to_int16(ctx.vm, value)?;
            out[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Uint16 => {
            let v = super::super::coerce::to_uint16(ctx.vm, value)?;
            out[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Int32 => {
            let v = super::super::coerce::to_int32(ctx.vm, value)?;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Uint32 => {
            let v = super::super::coerce::to_uint32(ctx.vm, value)?;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float32 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            #[allow(clippy::cast_possible_truncation)]
            let v = n as f32;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float64 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            out[..8].copy_from_slice(&n.to_le_bytes());
            8
        }
        ElementKind::BigInt64 => {
            let v = super::super::natives_bigint::to_bigint64(ctx, value)?;
            out[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
        ElementKind::BigUint64 => {
            let v = super::super::natives_bigint::to_biguint64(ctx, value)?;
            out[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
    })
    .inspect(|&len| {
        debug_assert_eq!(
            len,
            usize::from(ek.bytes_per_element()),
            "coerce_element_to_le_bytes wrote {len} bytes but ek={ek:?} declares bytes_per_element()={}",
            ek.bytes_per_element()
        );
    })
}

/// Write `value` into the buffer at `index`, coerced per `ek`.
/// Returns `Err` when BigInt coercion fails (writing a Number into
/// a `BigInt64Array` — `ToBigInt` rejects) or when user-level
/// coercion (valueOf / Symbol.toPrimitive) throws.
///
/// Coerce first so a thrown `valueOf` / `Symbol.toPrimitive` leaves
/// the backing buffer unmodified.  The actual byte-level write
/// goes through [`super::byte_io::write_at`], which mutates the
/// backing `Vec<u8>` in place via `entry().or_default()` — other
/// views over the same `buffer_id` see the mutation through their
/// next `body_data.get`.
pub(crate) fn write_element_raw(
    ctx: &mut NativeContext<'_>,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
    value: JsValue,
) -> Result<(), VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;

    // Coerce first — user code may run (valueOf / toPrimitive)
    // and throw.  Scratch buffer holds the encoded little-endian
    // bytes; the actual write (mutating the backing `Vec<u8>` in
    // place) only runs if coercion succeeds.
    let mut scratch = [0_u8; 8];
    let written_len = coerce_element_to_le_bytes(ctx, ek, value, &mut scratch)?;

    super::byte_io::write_at(
        &mut ctx.vm.body_data,
        buffer_id,
        abs,
        &scratch[..written_len],
    );
    Ok(())
}
