//! `DataView` ctor + accessor / per-type getter / setter tests
//! (ES2024 §25.3, originally PR5-typed-array C5).
//!
//! Covers: ctor argument validation (non-buffer / out-of-range
//! offset+length → TypeError / RangeError), default-byte-length
//! computation, representative per-type getter / setter
//! round-trips (`Int8` / `Uint8` / `Int32` / `Uint32` / `Float32` /
//! `Float64` / `BigInt64` / `BigUint64`), default big-endian
//! decoding via `setInt16` + `getUint8` cross-call, the
//! `littleEndian` flag, range / offset bounds, BigInt-only payload
//! enforcement on `setBigInt64`, and the shared-buffer invariant
//! between `DataView` and `Uint8Array` views over the same
//! `ArrayBuffer`.  `Int16` / `Uint16` round-trips and the matching
//! BigInt-payload-only check on `setBigUint64` rely on the
//! macro-generated coverage in `vm/host/data_view.rs` and aren't
//! repeated here.
//!
//! Sibling-extracted from [`super::tests_typed_array_methods`] —
//! the pre-split file noted "moving them to
//! `tests_typed_array_extras` would push that module back over the
//! 1000-line convention", which is exactly the constraint this
//! split removes.  Constructor + prototype-chain-identity tests
//! remain in [`super::tests_typed_array`]; cross-interface tests
//! (`ArrayBuffer.isView`, `structuredClone` identity,
//! CanonicalNumericIndexString, BigInt equality, Fetch-body
//! integration) in [`super::tests_typed_array_extras`].

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn data_view_ctor_and_accessors() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(16); var dv = new DataView(b, 4, 8); \
             dv.byteOffset;"
        ),
        4.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(16); var dv = new DataView(b, 4, 8); \
             dv.byteLength;"
        ),
        8.0
    );
    assert!(eval_bool(
        &mut vm,
        "var b = new ArrayBuffer(16); var dv = new DataView(b); dv.buffer === b;"
    ));
}

#[test]
fn data_view_ctor_default_length() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(16); var dv = new DataView(b, 4); \
             dv.byteLength;"
        ),
        12.0
    );
}

#[test]
fn data_view_ctor_non_buffer_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new DataView({}); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn data_view_ctor_out_of_range_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new ArrayBuffer(8); var ok = false; \
         try { new DataView(b, 4, 16); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn data_view_get_set_int8_uint8() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setInt8(0, -1); dv.getInt8(0);"
        ),
        -1.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setUint8(0, 255); dv.getUint8(0);"
        ),
        255.0
    );
}

#[test]
fn data_view_default_endianness_is_big_endian() {
    let mut vm = Vm::new();
    // setInt16(0, 0x1234) with default BE → bytes [0x12, 0x34].
    // Reading byte 0 via getUint8 must see 0x12.
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setInt16(0, 0x1234); dv.getUint8(0);"
        ),
        0x12 as f64
    );
}

#[test]
fn data_view_little_endian_flag() {
    let mut vm = Vm::new();
    // setInt16(0, 0x1234, true) = LE → [0x34, 0x12].
    // getUint8(0) must see 0x34.
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setInt16(0, 0x1234, true); dv.getUint8(0);"
        ),
        0x34 as f64
    );
}

#[test]
fn data_view_int32_round_trip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setInt32(0, -123456789); dv.getInt32(0);"
        ),
        -123_456_789.0
    );
}

#[test]
fn data_view_uint32_round_trip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setUint32(0, 4294967295); dv.getUint32(0);"
        ),
        4_294_967_295.0
    );
}

#[test]
fn data_view_float32_round_trip_le() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(4)); \
             dv.setFloat32(0, 1.0, true); dv.getFloat32(0, true);"
        ),
        1.0
    );
}

#[test]
fn data_view_float64_round_trip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(8)); \
             dv.setFloat64(0, 1.5); dv.getFloat64(0);"
        ),
        1.5
    );
}

#[test]
fn data_view_bigint64_round_trip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(8)); \
             dv.setBigInt64(0, -100n); String(dv.getBigInt64(0));"
        ),
        "-100"
    );
}

#[test]
fn data_view_biguint64_round_trip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var dv = new DataView(new ArrayBuffer(8)); \
             dv.setBigUint64(0, 18446744073709551615n); String(dv.getBigUint64(0));"
        ),
        "18446744073709551615"
    );
}

#[test]
fn data_view_out_of_range_get_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(4)); var ok = false; \
         try { dv.getInt32(4); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn data_view_out_of_range_set_throws() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(4)); var ok = false; \
         try { dv.setInt32(3, 0); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn data_view_shares_backing_with_typed_array() {
    let mut vm = Vm::new();
    // Uint8Array writes the explicit byte sequence into the
    // shared ArrayBuffer (no endianness — byte-level access).
    // Reading those same backing bytes via DataView with
    // `littleEndian=true` must reconstruct the expected u32,
    // proving the view shares memory with the TypedArray.
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(4); var u = new Uint8Array(b); \
             u[0] = 0x78; u[1] = 0x56; u[2] = 0x34; u[3] = 0x12; \
             var dv = new DataView(b); dv.getUint32(0, true);"
        ),
        0x12345678_u32 as f64
    );
}

#[test]
fn data_view_bigint_set_number_throws_type_error() {
    let mut vm = Vm::new();
    // setBigInt64 routes through strict ToBigInt — Number throws.
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(8)); var ok = false; \
         try { dv.setBigInt64(0, 1); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}
