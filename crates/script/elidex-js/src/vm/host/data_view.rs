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
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter_fn,
                None,
                PropertyAttrs::ES_BUILTIN_ACCESSOR,
            );
        }

        // 10 get* methods + 10 set* methods.  Names are
        // pre-interned in `WellKnownStrings` so this table skips
        // the per-call `strings.intern(...)` round-trip during
        // `Vm::new`.  The displayed function name (used by
        // `Function.prototype.name`) is carried by the same
        // interned `StringId` passed to
        // `create_native_function_with_sid`.
        let methods: [(StringId, NativeFn); 20] = [
            (self.well_known.get_int8, native_data_view_get_int8),
            (self.well_known.get_uint8, native_data_view_get_uint8),
            (self.well_known.get_int16, native_data_view_get_int16),
            (self.well_known.get_uint16, native_data_view_get_uint16),
            (self.well_known.get_int32, native_data_view_get_int32),
            (self.well_known.get_uint32, native_data_view_get_uint32),
            (self.well_known.get_float32, native_data_view_get_float32),
            (self.well_known.get_float64, native_data_view_get_float64),
            (self.well_known.get_bigint64, native_data_view_get_bigint64),
            (
                self.well_known.get_biguint64,
                native_data_view_get_biguint64,
            ),
            (self.well_known.set_int8, native_data_view_set_int8),
            (self.well_known.set_uint8, native_data_view_set_uint8),
            (self.well_known.set_int16, native_data_view_set_int16),
            (self.well_known.set_uint16, native_data_view_set_uint16),
            (self.well_known.set_int32, native_data_view_set_int32),
            (self.well_known.set_uint32, native_data_view_set_uint32),
            (self.well_known.set_float32, native_data_view_set_float32),
            (self.well_known.set_float64, native_data_view_set_float64),
            (self.well_known.set_bigint64, native_data_view_set_bigint64),
            (
                self.well_known.set_biguint64,
                native_data_view_set_biguint64,
            ),
        ];
        for (name_sid, fn_ptr) in methods {
            self.install_native_method(proto_id, name_sid, fn_ptr, PropertyAttrs::METHOD);
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
            if byte_offset.checked_add(len).is_none_or(|end| end > buf_len) {
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
/// the view's byte length.  The underlying byte snapshot is
/// performed by [`super::byte_io::read_into`].
fn read_bytes<const N: usize>(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
    offset_f: f64,
) -> Result<[u8; N], VmError> {
    let (buffer_id, dv_offset, dv_len) = require_data_view_parts(ctx, this, method)?;
    let rel_offset = ensure_in_range(offset_f, dv_len, N as u32, method)?;
    let abs = (dv_offset + rel_offset) as usize;
    Ok(super::byte_io::read_into::<N>(
        &ctx.vm.body_data,
        buffer_id,
        abs,
    ))
}

/// Write `bytes` at `byte_offset` relative to the DataView's own
/// `[[ByteOffset]]`.  The underlying clone-grow-install step is
/// performed by [`super::byte_io::write_at`], so downstream views
/// over the same buffer see the mutation through their next
/// `body_data.get(&buffer_id)` (same model as
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
    super::byte_io::write_at(&mut ctx.vm.body_data, buffer_id, abs, &bytes);
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
    if offset.checked_add(size).is_none_or(|end| end > dv_len) {
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
// Getter / setter macro pair
// ---------------------------------------------------------------------------
//
// Each `DataView.prototype.{get,set}<Type>` shares the same shape:
// extract `byteOffset` from `args[0]`, optionally read the
// `littleEndian` boolean from `args[N-1]`, dispatch through
// `read_bytes`/`write_bytes` with a compile-time `N` size, and
// (de)serialise the active byte window via `<T>::from_le_bytes` /
// `<T>::from_be_bytes` / `<T>::to_le_bytes` / `<T>::to_be_bytes`.
// `setInt8` and `setUint8` (and the matching getters) carry no
// `littleEndian` arg because a single byte is endian-agnostic; they
// take the one-byte arms below.
//
// The two macros expand into the per-method `fn native_data_view_*`
// plumbing.  Each invocation provides a closure-shaped `|$ctx, $v|
// $expr` block that picks the per-type wrap (getters: pack into
// `JsValue::Number`/`JsValue::BigInt`) or coerce (setters: ToNumber
// / `coerce::to_int*` / `natives_bigint::to_*`).  Macro-bound
// identifiers (`$ctx` / `$v`) follow the caller's hygiene context so
// the closure body can reach `ctx`, `val_arg`, `?`, etc. just like a
// hand-written body.

macro_rules! dv_get {
    // Single-byte arm: no `littleEndian` argument.  The wrap closure
    // sees the raw byte (`$b: u8`) and decides whether to sign-extend
    // (`getInt8`) or lift directly (`getUint8`).
    ($fn_name:ident, $method:literal, byte, |$ctx:ident, $b:ident| $wrap:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
            let n = ctx.to_number(offset_arg)?;
            let bytes = read_bytes::<1>(ctx, this, $method, n)?;
            let $ctx = &mut *ctx;
            let $b: u8 = bytes[0];
            $wrap
        }
    };
    // Multi-byte arm: reads `args[1]` as `littleEndian` and decodes
    // via `<$ty>::from_{le,be}_bytes`.  The wrap closure sees the
    // typed value (`$v: $ty`) and picks the `JsValue` shape
    // (`Number` for ints/floats, `BigInt` for 64-bit big-int
    // variants).
    ($fn_name:ident, $method:literal, $ty:ty, $size:literal, |$ctx:ident, $v:ident| $wrap:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
            let n = ctx.to_number(offset_arg)?;
            let little_endian = decode_little_endian(ctx, args.get(1).copied());
            let bytes = read_bytes::<$size>(ctx, this, $method, n)?;
            let $v: $ty = if little_endian {
                <$ty>::from_le_bytes(bytes)
            } else {
                <$ty>::from_be_bytes(bytes)
            };
            let $ctx = &mut *ctx;
            $wrap
        }
    };
}

macro_rules! dv_set {
    // Single-byte arm: no `littleEndian` argument.  The coerce
    // closure takes (`$ctx`, `$val: JsValue`) and yields the
    // single-byte payload (`u8`), so callers either return the
    // unsigned coercion directly (`setUint8`) or cast through `as
    // u8` (`setInt8`'s i8 → u8).
    ($fn_name:ident, $method:literal, byte, |$ctx:ident, $val:ident| $coerce:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
            let n = ctx.to_number(offset_arg)?;
            let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
            let byte: u8 = {
                let $ctx = &mut *ctx;
                let $val: JsValue = val_arg;
                $coerce
            };
            write_bytes::<1>(ctx, this, $method, n, [byte])?;
            Ok(JsValue::Undefined)
        }
    };
    // Multi-byte arm: reads `args[2]` as `littleEndian`, dispatches
    // through `<T>::to_{le,be}_bytes` after the coerce closure
    // produces the typed value.  `coerce::to_int*` / `to_uint*` cover
    // 16/32-bit ints, `ctx.to_number` handles f32/f64 (with a single
    // `as f32` truncation for `setFloat32`), and
    // `natives_bigint::to_{,u}bigint64` covers the 64-bit BigInt
    // variants.
    ($fn_name:ident, $method:literal, $size:literal, |$ctx:ident, $val:ident| $coerce:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
            let n = ctx.to_number(offset_arg)?;
            let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
            let v = {
                let $ctx = &mut *ctx;
                let $val: JsValue = val_arg;
                $coerce
            };
            let little_endian = decode_little_endian(ctx, args.get(2).copied());
            let bytes = if little_endian {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            };
            write_bytes::<$size>(ctx, this, $method, n, bytes)?;
            Ok(JsValue::Undefined)
        }
    };
}

// ---------------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------------

dv_get!(native_data_view_get_int8, "getInt8", byte, |_ctx, b| Ok(
    JsValue::Number(f64::from(b.cast_signed()))
));
dv_get!(native_data_view_get_uint8, "getUint8", byte, |_ctx, b| Ok(
    JsValue::Number(f64::from(b))
));

dv_get!(
    native_data_view_get_int16,
    "getInt16",
    i16,
    2,
    |_ctx, v| Ok(JsValue::Number(f64::from(v)))
);
dv_get!(
    native_data_view_get_uint16,
    "getUint16",
    u16,
    2,
    |_ctx, v| Ok(JsValue::Number(f64::from(v)))
);
dv_get!(
    native_data_view_get_int32,
    "getInt32",
    i32,
    4,
    |_ctx, v| Ok(JsValue::Number(f64::from(v)))
);
dv_get!(
    native_data_view_get_uint32,
    "getUint32",
    u32,
    4,
    |_ctx, v| Ok(JsValue::Number(f64::from(v)))
);
dv_get!(
    native_data_view_get_float32,
    "getFloat32",
    f32,
    4,
    |_ctx, v| Ok(JsValue::Number(f64::from(v)))
);
dv_get!(
    native_data_view_get_float64,
    "getFloat64",
    f64,
    8,
    |_ctx, v| Ok(JsValue::Number(v))
);
dv_get!(
    native_data_view_get_bigint64,
    "getBigInt64",
    i64,
    8,
    |ctx, v| Ok(JsValue::BigInt(
        ctx.vm.bigints.alloc(num_bigint::BigInt::from(v))
    ))
);
dv_get!(
    native_data_view_get_biguint64,
    "getBigUint64",
    u64,
    8,
    |ctx, v| Ok(JsValue::BigInt(
        ctx.vm.bigints.alloc(num_bigint::BigInt::from(v))
    ))
);

// ---------------------------------------------------------------------------
// Setters
// ---------------------------------------------------------------------------

dv_set!(native_data_view_set_int8, "setInt8", byte, |ctx, val| {
    super::super::coerce::to_int8(ctx.vm, val)?.cast_unsigned()
});
dv_set!(native_data_view_set_uint8, "setUint8", byte, |ctx, val| {
    super::super::coerce::to_uint8(ctx.vm, val)?
});

dv_set!(native_data_view_set_int16, "setInt16", 2, |ctx, val| {
    super::super::coerce::to_int16(ctx.vm, val)?
});
dv_set!(native_data_view_set_uint16, "setUint16", 2, |ctx, val| {
    super::super::coerce::to_uint16(ctx.vm, val)?
});
dv_set!(native_data_view_set_int32, "setInt32", 4, |ctx, val| {
    super::super::coerce::to_int32(ctx.vm, val)?
});
dv_set!(native_data_view_set_uint32, "setUint32", 4, |ctx, val| {
    super::super::coerce::to_uint32(ctx.vm, val)?
});
dv_set!(native_data_view_set_float32, "setFloat32", 4, |ctx, val| {
    #[allow(clippy::cast_possible_truncation)]
    {
        ctx.to_number(val)? as f32
    }
});
dv_set!(native_data_view_set_float64, "setFloat64", 8, |ctx, val| {
    ctx.to_number(val)?
});
dv_set!(
    native_data_view_set_bigint64,
    "setBigInt64",
    8,
    |ctx, val| super::super::natives_bigint::to_bigint64(ctx, val)?
);
dv_set!(
    native_data_view_set_biguint64,
    "setBigUint64",
    8,
    |ctx, val| super::super::natives_bigint::to_biguint64(ctx, val)?
);
