//! `DataView` — endian-aware byte-level view over an `ArrayBuffer`
//! (ES2024 §25.3).
//!
//! Unlike [`super::typed_array`], `DataView` has no fixed element
//! kind: callers select the type + endianness per call via
//! `getInt8` / `getFloat64(offset, littleEndian)` / etc.  The
//! backing `ArrayBuffer` + offset / length live inline in
//! [`ObjectKind::DataView`] (declared in C1).
//!
//! Prototype chain:
//!
//! ```text
//! DataView instance (ObjectKind::DataView { buffer_id, byte_offset, byte_length })
//!   → DataView.prototype (this module)
//!     → Object.prototype
//! ```
//!
//! ## Endianness default
//!
//! DataView's `littleEndian` argument defaults to **`false`** (big-
//! endian) per §25.3.4 — the opposite of [`super::typed_array`]'s
//! unconditional LE choice.  Callers that need LE must pass
//! `true` explicitly.
//!
//! ## Deferred (M4-12 cutover-residual)
//!
//! `DataView.prototype.getFloat16` / `setFloat16` (ES2024 stage 4) —
//! tracked alongside Float16Array.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `DataView.prototype`, install the accessor / method
    /// suite, and expose the `DataView` constructor on `globals`.
    /// Must run during `register_globals()` after
    /// `register_array_buffer_global` (DataView's backing store IS
    /// ArrayBuffer — `array_buffer_byte_length` helper is consumed
    /// here) and after `register_typed_array_prototype_global`
    /// (ordering convention; DataView is independent but is
    /// customarily installed beside TypedArray for locality).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_data_view_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_data_view_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_data_view_members(proto_id);
        self.data_view_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("DataView", native_data_view_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.data_view_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_data_view_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 3] = [
            (
                self.well_known.buffer,
                native_data_view_get_buffer as NativeFn,
            ),
            (
                self.well_known.byte_offset,
                native_data_view_get_byte_offset as NativeFn,
            ),
            (
                self.well_known.byte_length,
                native_data_view_get_byte_length as NativeFn,
            ),
        ];
        for (name_sid, getter_fn) in accessors {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter_fn);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::ES_BUILTIN_ACCESSOR,
            );
        }

        // 10 get* methods + 10 set* methods.  Names are
        // fresh-interned here (not WellKnownStrings) because
        // they're unique to DataView and the intern pool
        // dedup-caches on repeat calls.
        let methods: [(&str, NativeFn); 20] = [
            ("getInt8", native_data_view_get_int8),
            ("getUint8", native_data_view_get_uint8),
            ("getInt16", native_data_view_get_int16),
            ("getUint16", native_data_view_get_uint16),
            ("getInt32", native_data_view_get_int32),
            ("getUint32", native_data_view_get_uint32),
            ("getFloat32", native_data_view_get_float32),
            ("getFloat64", native_data_view_get_float64),
            ("getBigInt64", native_data_view_get_bigint64),
            ("getBigUint64", native_data_view_get_biguint64),
            ("setInt8", native_data_view_set_int8),
            ("setUint8", native_data_view_set_uint8),
            ("setInt16", native_data_view_set_int16),
            ("setUint16", native_data_view_set_uint16),
            ("setInt32", native_data_view_set_int32),
            ("setUint32", native_data_view_set_uint32),
            ("setFloat32", native_data_view_set_float32),
            ("setFloat64", native_data_view_set_float64),
            ("setBigInt64", native_data_view_set_bigint64),
            ("setBigUint64", native_data_view_set_biguint64),
        ];
        for (name, fn_ptr) in methods {
            let name_sid = self.strings.intern(name);
            let fn_id = self.create_native_function(name, fn_ptr);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new DataView(buffer, byteOffset?, byteLength?)` (ES §25.3.2.1).
fn native_data_view_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'DataView': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let buffer_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(buffer_id) = buffer_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'DataView': First parameter is not of type 'ArrayBuffer'",
        ));
    };
    if !matches!(ctx.vm.get_object(buffer_id).kind, ObjectKind::ArrayBuffer) {
        return Err(VmError::type_error(
            "Failed to construct 'DataView': First parameter is not of type 'ArrayBuffer'",
        ));
    }

    let buf_len_usize = super::array_buffer::array_buffer_byte_length(ctx.vm, buffer_id);
    let buf_len: u32 = buf_len_usize.try_into().map_err(|_| {
        VmError::range_error("Failed to construct 'DataView': ArrayBuffer is larger than 4 GiB")
    })?;

    let byte_offset = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0,
        other => coerce::to_index_u32(ctx, other, "DataView", "byteOffset")?,
    };
    if byte_offset > buf_len {
        return Err(VmError::range_error(format!(
            "Failed to construct 'DataView': byteOffset {byte_offset} exceeds ArrayBuffer length {buf_len}"
        )));
    }

    let byte_length = match args.get(2).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => buf_len - byte_offset,
        other => {
            let len = coerce::to_index_u32(ctx, other, "DataView", "byteLength")?;
            if byte_offset
                .checked_add(len)
                .map_or(true, |end| end > buf_len)
            {
                return Err(VmError::range_error(
                    "Failed to construct 'DataView': Invalid data view length",
                ));
            }
            len
        }
    };

    // Promote the pre-allocated Ordinary instance to DataView —
    // `new.target.prototype` is preserved (PR5a2 R7.2/R7.3).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::DataView {
        buffer_id,
        byte_offset,
        byte_length,
    };
    Ok(JsValue::Object(inst_id))
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_data_view_parts(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(ObjectId, u32, u32), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "DataView.prototype.{method} called on non-DataView"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::DataView {
            buffer_id,
            byte_offset,
            byte_length,
        } => Ok((buffer_id, byte_offset, byte_length)),
        _ => Err(VmError::type_error(format!(
            "DataView.prototype.{method} called on non-DataView"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_data_view_get_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (buffer_id, _, _) = require_data_view_parts(ctx, this, "buffer")?;
    Ok(JsValue::Object(buffer_id))
}

fn native_data_view_get_byte_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, byte_offset, _) = require_data_view_parts(ctx, this, "byteOffset")?;
    Ok(JsValue::Number(f64::from(byte_offset)))
}

fn native_data_view_get_byte_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_, _, byte_length) = require_data_view_parts(ctx, this, "byteLength")?;
    Ok(JsValue::Number(f64::from(byte_length)))
}

// ---------------------------------------------------------------------------
// Shared byte read/write helpers
// ---------------------------------------------------------------------------

/// Read `N` bytes at `byte_offset` relative to the DataView's own
/// `[[ByteOffset]]`.  Returns a copy of the bytes into a fixed-
/// size array so the caller can decode without a live borrow on
/// `body_data`.  RangeError if the requested span extends past
/// the view's byte length.
fn read_bytes<const N: usize>(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
    offset_f: f64,
) -> Result<[u8; N], VmError> {
    let (buffer_id, dv_offset, dv_len) = require_data_view_parts(ctx, this, method)?;
    let rel_offset = ensure_in_range(offset_f, dv_len, N as u32, method)?;
    let abs = (dv_offset + rel_offset) as usize;
    let mut out = [0_u8; N];
    if let Some(bytes) = ctx.vm.body_data.get(&buffer_id) {
        if let Some(slice) = bytes.get(abs..abs + N) {
            out.copy_from_slice(slice);
        }
    }
    Ok(out)
}

/// Write `bytes` at `byte_offset` relative to the DataView's own
/// `[[ByteOffset]]`.  Replaces the entire `body_data` entry with a
/// fresh `Arc<[u8]>` so downstream views over the same buffer see
/// the mutation through `body_data.get(&buffer_id)` (same model as
/// [`super::typed_array::write_element_raw`]).
fn write_bytes<const N: usize>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    offset_f: f64,
    bytes: [u8; N],
) -> Result<(), VmError> {
    let (buffer_id, dv_offset, dv_len) = require_data_view_parts(ctx, this, method)?;
    let rel_offset = ensure_in_range(offset_f, dv_len, N as u32, method)?;
    let abs = (dv_offset + rel_offset) as usize;
    let needed = abs + N;
    let current: &[u8] = ctx
        .vm
        .body_data
        .get(&buffer_id)
        .map(AsRef::as_ref)
        .unwrap_or(&[]);
    let mut new_bytes: Vec<u8> = current.to_vec();
    if new_bytes.len() < needed {
        new_bytes.resize(needed, 0);
    }
    new_bytes[abs..abs + N].copy_from_slice(&bytes);
    use std::sync::Arc;
    ctx.vm.body_data.insert(buffer_id, Arc::from(new_bytes));
    Ok(())
}

/// Validate that `offset + size ≤ dv_len` and return the unsigned
/// `byte_offset`.  Throws RangeError on out-of-range.  `n` is the
/// already-coerced `ToNumber(offsetArg)` from the caller.
///
/// NaN / undefined coerce to `0` (per `ToIndex` §7.1.22) but still
/// participate in the bounds check — e.g. `new DataView(buf_len_1).
/// getInt16(NaN)` must throw RangeError because `0 + 2 > 1`.
fn ensure_in_range(n: f64, dv_len: u32, size: u32, method: &str) -> Result<u32, VmError> {
    let n = if n.is_nan() { 0.0 } else { n };
    let truncated = n.trunc();
    if !truncated.is_finite() || truncated < 0.0 {
        return Err(VmError::range_error(format!(
            "Failed to execute '{method}' on 'DataView': byteOffset must be a non-negative safe integer"
        )));
    }
    if truncated > f64::from(u32::MAX) {
        return Err(VmError::range_error(format!(
            "Failed to execute '{method}' on 'DataView': byteOffset exceeds the supported maximum"
        )));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let offset = truncated as u32;
    if offset.checked_add(size).map_or(true, |end| end > dv_len) {
        return Err(VmError::range_error(format!(
            "Failed to execute '{method}' on 'DataView': Offset is outside the bounds of the DataView"
        )));
    }
    Ok(offset)
}

/// Decode `little_endian_arg` (optional 3rd arg for multi-byte
/// methods) as a boolean.  `undefined` → `false` (big-endian per
/// §25.3.4 default).  ToBoolean semantics.
fn decode_little_endian(ctx: &NativeContext<'_>, arg: Option<JsValue>) -> bool {
    match arg {
        Some(v) if !matches!(v, JsValue::Undefined) => ctx.to_boolean(v),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------------

fn native_data_view_get_int8(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let bytes = read_bytes::<1>(ctx, this, "getInt8", n)?;
    Ok(JsValue::Number(f64::from(bytes[0] as i8)))
}

fn native_data_view_get_uint8(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let bytes = read_bytes::<1>(ctx, this, "getUint8", n)?;
    Ok(JsValue::Number(f64::from(bytes[0])))
}

fn native_data_view_get_int16(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<2>(ctx, this, "getInt16", n)?;
    let v = if little_endian {
        i16::from_le_bytes(bytes)
    } else {
        i16::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(f64::from(v)))
}

fn native_data_view_get_uint16(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<2>(ctx, this, "getUint16", n)?;
    let v = if little_endian {
        u16::from_le_bytes(bytes)
    } else {
        u16::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(f64::from(v)))
}

fn native_data_view_get_int32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<4>(ctx, this, "getInt32", n)?;
    let v = if little_endian {
        i32::from_le_bytes(bytes)
    } else {
        i32::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(f64::from(v)))
}

fn native_data_view_get_uint32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<4>(ctx, this, "getUint32", n)?;
    let v = if little_endian {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(f64::from(v)))
}

fn native_data_view_get_float32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<4>(ctx, this, "getFloat32", n)?;
    let v = if little_endian {
        f32::from_le_bytes(bytes)
    } else {
        f32::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(f64::from(v)))
}

fn native_data_view_get_float64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<8>(ctx, this, "getFloat64", n)?;
    let v = if little_endian {
        f64::from_le_bytes(bytes)
    } else {
        f64::from_be_bytes(bytes)
    };
    Ok(JsValue::Number(v))
}

fn native_data_view_get_bigint64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<8>(ctx, this, "getBigInt64", n)?;
    let v = if little_endian {
        i64::from_le_bytes(bytes)
    } else {
        i64::from_be_bytes(bytes)
    };
    let bi = num_bigint::BigInt::from(v);
    Ok(JsValue::BigInt(ctx.vm.bigints.alloc(bi)))
}

fn native_data_view_get_biguint64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(1).copied());
    let bytes = read_bytes::<8>(ctx, this, "getBigUint64", n)?;
    let v = if little_endian {
        u64::from_le_bytes(bytes)
    } else {
        u64::from_be_bytes(bytes)
    };
    let bi = num_bigint::BigInt::from(v);
    Ok(JsValue::BigInt(ctx.vm.bigints.alloc(bi)))
}

// ---------------------------------------------------------------------------
// Setters
// ---------------------------------------------------------------------------

fn native_data_view_set_int8(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v =
        super::super::coerce::to_int8(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    write_bytes::<1>(ctx, this, "setInt8", n, [v as u8])?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_uint8(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v =
        super::super::coerce::to_uint8(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    write_bytes::<1>(ctx, this, "setUint8", n, [v])?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_int16(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v =
        super::super::coerce::to_int16(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<2>(ctx, this, "setInt16", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_uint16(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v = super::super::coerce::to_uint16(
        ctx.vm,
        args.get(1).copied().unwrap_or(JsValue::Undefined),
    )?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<2>(ctx, this, "setUint16", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_int32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v =
        super::super::coerce::to_int32(ctx.vm, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<4>(ctx, this, "setInt32", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_uint32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let v = super::super::coerce::to_uint32(
        ctx.vm,
        args.get(1).copied().unwrap_or(JsValue::Undefined),
    )?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<4>(ctx, this, "setUint32", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_float32(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let val_f64 = ctx.to_number(val_arg)?;
    #[allow(clippy::cast_possible_truncation)]
    let v = val_f64 as f32;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<4>(ctx, this, "setFloat32", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_float64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let v = ctx.to_number(val_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<8>(ctx, this, "setFloat64", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_bigint64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let v = super::super::natives_bigint::to_bigint64(ctx, val_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<8>(ctx, this, "setBigInt64", n, bytes)?;
    Ok(JsValue::Undefined)
}

fn native_data_view_set_biguint64(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = ctx.to_number(offset_arg)?;
    let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let v = super::super::natives_bigint::to_biguint64(ctx, val_arg)?;
    let little_endian = decode_little_endian(ctx, args.get(2).copied());
    let bytes = if little_endian {
        v.to_le_bytes()
    } else {
        v.to_be_bytes()
    };
    write_bytes::<8>(ctx, this, "setBigUint64", n, bytes)?;
    Ok(JsValue::Undefined)
}
