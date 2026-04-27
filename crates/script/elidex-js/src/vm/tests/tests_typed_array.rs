//! `%TypedArray%` + subclass constructor / identity / prototype / brand-check /
//! indexed-element tests (ES2024 §23.2).
//!
//! Covers: constructor dispatch (basic shapes), `BYTES_PER_ELEMENT` on ctor +
//! prototype, prototype chain identity, `@@species` / `@@toStringTag`,
//! generic accessor (`buffer` / `byteOffset` / `byteLength` / `length`) brand
//! checks, and C3 indexed element access.  Prototype method tests live in
//! [`super::tests_typed_array_methods`]; `DataView` ctor / accessor /
//! getter / setter tests in [`super::tests_data_view`];
//! `structuredClone` + integration tests in
//! [`super::tests_typed_array_extras`].

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

// ---------------------------------------------------------------------------
// Constructor — basic shapes
// ---------------------------------------------------------------------------

#[test]
fn uint8_ctor_zero_length_default() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(); a.length;"),
        0.0
    );
}

#[test]
fn uint8_ctor_length_form() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(5); a.length;"),
        5.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(5); a.byteLength;"),
        5.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(5); a.byteOffset;"),
        0.0
    );
}

#[test]
fn uint32_ctor_length_form_computes_bytelen() {
    let mut vm = Vm::new();
    // Uint32Array(3): length=3, byteLength=12, BYTES_PER_ELEMENT=4.
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint32Array(3); a.length;"),
        3.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint32Array(3); a.byteLength;"),
        12.0
    );
}

#[test]
fn ctor_buffer_form_shares_buffer() {
    let mut vm = Vm::new();
    // Two views over the same ArrayBuffer yield the same
    // `.buffer` identity.
    assert!(eval_bool(
        &mut vm,
        "var b = new ArrayBuffer(16); \
         var v1 = new Uint8Array(b); \
         var v2 = new Uint8Array(b); \
         v1.buffer === v2.buffer;"
    ));
}

#[test]
fn ctor_buffer_form_offset_and_length() {
    let mut vm = Vm::new();
    // Uint8Array(buf, 4, 8) → byteOffset=4, byteLength=8, length=8.
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(16); \
             var v = new Uint8Array(b, 4, 8); v.byteOffset;"
        ),
        4.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(16); \
             var v = new Uint8Array(b, 4, 8); v.length;"
        ),
        8.0
    );
}

#[test]
fn ctor_buffer_form_misaligned_offset_range_error() {
    let mut vm = Vm::new();
    // Uint32Array requires 4-byte-aligned byteOffset.
    assert!(eval_bool(
        &mut vm,
        "var b = new ArrayBuffer(16); var ok = false; \
         try { new Uint32Array(b, 1); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn ctor_buffer_form_length_out_of_range() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new ArrayBuffer(8); var ok = false; \
         try { new Uint8Array(b, 4, 16); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn ctor_iterable_form_length() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); a.length;"
        ),
        5.0
    );
}

#[test]
fn ctor_typed_array_copies_across_kinds() {
    let mut vm = Vm::new();
    // Uint16Array(2) → 4 bytes; Uint8Array(uint16arr) → length 2
    // (element count, each element coerced).
    assert_eq!(
        eval_number(
            &mut vm,
            "var src = new Uint16Array(2); \
             var dst = new Uint8Array(src); dst.length;"
        ),
        2.0
    );
    // Fresh buffer — NOT the source's buffer.
    assert!(eval_bool(
        &mut vm,
        "var src = new Uint16Array(2); \
         var dst = new Uint8Array(src); dst.buffer !== src.buffer;"
    ));
}

#[test]
fn ctor_bigint_from_number_array_throws_type_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new BigInt64Array([1, 2, 3]); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn ctor_bigint_from_bigint_array_ok() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new BigInt64Array([1n, 2n, 3n]); a.length;"
        ),
        3.0
    );
}

#[test]
fn ctor_string_primitive_goes_through_tonumber() {
    let mut vm = Vm::new();
    // `new Uint8Array("abc")` — String primitive is NOT an Object,
    // so takes the ToNumber branch.  ToNumber("abc") → NaN →
    // ToIndex(NaN) → 0.  Spec §23.2.5.1 step 6.
    assert_eq!(eval_number(&mut vm, "new Uint8Array(\"abc\").length;"), 0.0);
}

#[test]
fn ctor_mixes_bigint_typed_array_to_number_throws() {
    let mut vm = Vm::new();
    // BigInt64Array → Uint8Array copy must TypeError (content-type
    // mismatch, §23.2.5.1.2 step 17).
    assert!(eval_bool(
        &mut vm,
        "var src = new BigInt64Array(2); var ok = false; \
         try { new Uint8Array(src); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

// ---------------------------------------------------------------------------
// BYTES_PER_ELEMENT
// ---------------------------------------------------------------------------

#[test]
fn bytes_per_element_on_ctor() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "Uint8Array.BYTES_PER_ELEMENT;"), 1.0);
    assert_eq!(eval_number(&mut vm, "Int8Array.BYTES_PER_ELEMENT;"), 1.0);
    assert_eq!(
        eval_number(&mut vm, "Uint8ClampedArray.BYTES_PER_ELEMENT;"),
        1.0
    );
    assert_eq!(eval_number(&mut vm, "Int16Array.BYTES_PER_ELEMENT;"), 2.0);
    assert_eq!(eval_number(&mut vm, "Uint16Array.BYTES_PER_ELEMENT;"), 2.0);
    assert_eq!(eval_number(&mut vm, "Int32Array.BYTES_PER_ELEMENT;"), 4.0);
    assert_eq!(eval_number(&mut vm, "Uint32Array.BYTES_PER_ELEMENT;"), 4.0);
    assert_eq!(eval_number(&mut vm, "Float32Array.BYTES_PER_ELEMENT;"), 4.0);
    assert_eq!(eval_number(&mut vm, "Float64Array.BYTES_PER_ELEMENT;"), 8.0);
    assert_eq!(
        eval_number(&mut vm, "BigInt64Array.BYTES_PER_ELEMENT;"),
        8.0
    );
    assert_eq!(
        eval_number(&mut vm, "BigUint64Array.BYTES_PER_ELEMENT;"),
        8.0
    );
}

#[test]
fn bytes_per_element_on_prototype() {
    let mut vm = Vm::new();
    // Instance reads BYTES_PER_ELEMENT from the prototype (own
    // property on Xxx.prototype, spec §23.2.7.1).
    assert_eq!(
        eval_number(&mut vm, "new Uint32Array(0).BYTES_PER_ELEMENT;"),
        4.0
    );
}

#[test]
fn bytes_per_element_is_non_writable() {
    let mut vm = Vm::new();
    // `{writable: false}` — assignment silently fails in sloppy
    // mode; `Object.getOwnPropertyDescriptor(Uint8Array,
    // "BYTES_PER_ELEMENT").writable` is observable.
    assert!(eval_bool(
        &mut vm,
        "var d = Object.getOwnPropertyDescriptor(Uint8Array, \"BYTES_PER_ELEMENT\"); \
         d.writable === false && d.configurable === false && d.enumerable === false;"
    ));
}

// ---------------------------------------------------------------------------
// Prototype chain + identity
// ---------------------------------------------------------------------------

#[test]
fn subclass_prototype_chains_to_typed_array_prototype() {
    let mut vm = Vm::new();
    // `%TypedArray%.prototype` is the parent of every subclass
    // prototype — prove via identity between two subclasses.
    assert!(eval_bool(
        &mut vm,
        "Object.getPrototypeOf(Uint8Array.prototype) === \
         Object.getPrototypeOf(Int8Array.prototype);"
    ));
}

#[test]
fn subclass_ctor_chains_to_abstract_typed_array() {
    let mut vm = Vm::new();
    // `Object.getPrototypeOf(Uint8Array)` IS the abstract
    // `%TypedArray%` function (not `Function.prototype`).  Prove
    // via identity between two subclass ctor prototypes.
    assert!(eval_bool(
        &mut vm,
        "Object.getPrototypeOf(Uint8Array) === Object.getPrototypeOf(Int8Array);"
    ));
}

#[test]
fn abstract_typed_array_ctor_throws_on_call() {
    let mut vm = Vm::new();
    // `%TypedArray%()` — call-mode invocation throws TypeError
    // per ES §23.2.1.1 ("Abstract class TypedArray not directly
    // constructable").
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { Object.getPrototypeOf(Uint8Array)(); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn abstract_typed_array_ctor_throws_on_new() {
    let mut vm = Vm::new();
    // `new %TypedArray%()` — new-mode also throws.
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new (Object.getPrototypeOf(Uint8Array))(); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

// ---------------------------------------------------------------------------
// @@species + @@toStringTag
// ---------------------------------------------------------------------------

#[test]
fn abstract_typed_array_species_is_identity() {
    let mut vm = Vm::new();
    // `%TypedArray%[@@species]` returns `this`.  Subclasses inherit,
    // so `Uint8Array[@@species] === Uint8Array`.
    assert!(eval_bool(
        &mut vm,
        "Uint8Array[Symbol.species] === Uint8Array;"
    ));
    assert!(eval_bool(
        &mut vm,
        "Int32Array[Symbol.species] === Int32Array;"
    ));
}

#[test]
fn to_string_tag_readable_via_direct_getter() {
    let mut vm = Vm::new();
    // `%TypedArray%.prototype.toString` is identity-equal to
    // `Array.prototype.toString`, which now routes through
    // `.join` (installed in C4b) — so `.toString()` produces
    // comma-separated element output (tested separately in
    // `to_string_invokes_join`).  To observe @@toStringTag
    // directly, fetch the getter off `%TypedArray%.prototype`
    // and invoke it on the instance.
    assert_eq!(
        eval_string(
            &mut vm,
            "var p = Object.getPrototypeOf(Uint8Array.prototype); \
             var g = Object.getOwnPropertyDescriptor(p, Symbol.toStringTag).get; \
             g.call(new Uint8Array());"
        ),
        "Uint8Array"
    );
    assert_eq!(
        eval_string(
            &mut vm,
            "var p = Object.getPrototypeOf(BigInt64Array.prototype); \
             var g = Object.getOwnPropertyDescriptor(p, Symbol.toStringTag).get; \
             g.call(new BigInt64Array());"
        ),
        "BigInt64Array"
    );
}

#[test]
fn to_string_tag_undefined_on_foreign_receiver() {
    let mut vm = Vm::new();
    // Calling the getter with a non-TypedArray `this` yields
    // `undefined`, NOT throw.  `Object.prototype.toString` then
    // falls back to the standard tag path
    // (`[object Object]` here).
    assert!(eval_bool(
        &mut vm,
        "var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(Uint8Array.prototype), \
             Symbol.toStringTag).get; \
         getter.call({}) === undefined;"
    ));
}

// ---------------------------------------------------------------------------
// Getter brand-check
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Indexed element access (C3)
// ---------------------------------------------------------------------------

#[test]
fn uint8_write_and_read_roundtrip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); a[0] = 10; a[1] = 20; a[2] = 30; \
             a[0] + a[1] + a[2];"
        ),
        60.0
    );
}

#[test]
fn int8_wraps_overflow() {
    let mut vm = Vm::new();
    // Int8Array wraps via ToInt8 modular: 128 → -128.
    assert_eq!(
        eval_number(&mut vm, "var a = new Int8Array(1); a[0] = 128; a[0];"),
        -128.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Int8Array(1); a[0] = 255; a[0];"),
        -1.0
    );
}

#[test]
fn uint8_wraps_overflow() {
    let mut vm = Vm::new();
    // Uint8Array wraps via ToUint8 modular: 256 → 0.
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(1); a[0] = 256; a[0];"),
        0.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(1); a[0] = -1; a[0];"),
        255.0
    );
}

#[test]
fn uint8_clamped_rounds_ties_to_even() {
    let mut vm = Vm::new();
    // IEEE 754 roundTiesToEven per §7.1.11 — NOT round-half-up.
    //   0.5 → 0, 1.5 → 2, 2.5 → 2, 3.5 → 4, 4.5 → 4.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 0.5; a[0];"
        ),
        0.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 1.5; a[0];"
        ),
        2.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 2.5; a[0];"
        ),
        2.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 3.5; a[0];"
        ),
        4.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 4.5; a[0];"
        ),
        4.0
    );
}

#[test]
fn uint8_clamped_clamps_extremes() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = -10; a[0];"
        ),
        0.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = 1000; a[0];"
        ),
        255.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8ClampedArray(1); a[0] = NaN; a[0];"
        ),
        0.0
    );
}

#[test]
fn int16_uint16_roundtrip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var a = new Int16Array(1); a[0] = 32768; a[0];"),
        -32768.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint16Array(1); a[0] = 65536; a[0];"),
        0.0
    );
}

#[test]
fn int32_uint32_roundtrip() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Int32Array(1); a[0] = 2147483648; a[0];"
        ),
        -2_147_483_648.0
    );
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint32Array(1); a[0] = -1; a[0];"),
        4_294_967_295.0
    );
}

#[test]
fn float32_roundtrip_lossy() {
    let mut vm = Vm::new();
    // f32 has 23 bits of mantissa; 1.1 doesn't round-trip exactly
    // but 1.0 / 2.0 do.
    assert_eq!(
        eval_number(&mut vm, "var a = new Float32Array(1); a[0] = 1.0; a[0];"),
        1.0
    );
}

#[test]
fn float64_roundtrip_exact() {
    let mut vm = Vm::new();
    // f64 stores an approximation of 0.1-family decimals, but the
    // stored binary64 value round-trips bit-identically, so the
    // readback compares equal to the source literal.
    assert_eq!(
        eval_number(&mut vm, "var a = new Float64Array(1); a[0] = 1.1; a[0];"),
        1.1
    );
}

#[test]
fn bigint64_write_number_throws_type_error() {
    let mut vm = Vm::new();
    // Writing a Number to BigInt64Array throws per strict ToBigInt
    // (§7.1.13 / §10.4.5.16 step 1).
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array(1); var ok = false; \
         try { a[0] = 1; } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn bigint64_write_bigint_ok() {
    let mut vm = Vm::new();
    // `bi[0] = 5n` → readback `5n`.  BigInt bracket-read returns
    // `JsValue::BigInt`, not a number — we observe via string-coerce.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new BigInt64Array(1); a[0] = 5n; \
             String(a[0]);"
        ),
        "5"
    );
}

#[test]
fn bigint64_write_string_coerces() {
    let mut vm = Vm::new();
    // String "123" → ToBigInt → 123n (ES §7.1.13 accepts strings).
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new BigInt64Array(1); a[0] = \"123\"; \
             String(a[0]);"
        ),
        "123"
    );
}

#[test]
fn out_of_range_read_returns_undefined() {
    let mut vm = Vm::new();
    // `u8[5]` on a length-3 array returns undefined, does NOT
    // walk the prototype chain (ES §10.4.5.15 step 3).
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(3); a[5] === undefined;"
    ));
}

#[test]
fn out_of_range_write_is_no_op() {
    let mut vm = Vm::new();
    // `u8[5] = 99` on a length-3 array silently no-ops (ES
    // §10.4.5.16) — does NOT create an own property, does NOT
    // throw.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(3); a[5] = 99; \
         a[5] === undefined;"
    ));
}

#[test]
fn non_canonical_string_key_falls_through_to_prototype() {
    let mut vm = Vm::new();
    // Leading-zero numeric string "01" is NOT a Canonical Numeric
    // Index String per §23.2.2, so falls through to prototype
    // lookup.  Writing stores an ordinary own property; reading
    // sees it.  No indexed-element interception.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); a[\"01\"] = 99; a[\"01\"];"
        ),
        99.0
    );
    // Confirm the write did NOT touch the indexed storage at
    // canonical index 1.
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array(3); a[\"01\"] = 99; a[1];"),
        0.0
    );
}

#[test]
fn views_over_same_buffer_share_bytes() {
    let mut vm = Vm::new();
    // Two Uint8Arrays over the same ArrayBuffer: writing through
    // one is visible on the other.
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(4); \
             var v1 = new Uint8Array(buf); \
             var v2 = new Uint8Array(buf); \
             v1[0] = 42; v2[0];"
        ),
        42.0
    );
}

#[test]
fn uint16_view_over_uint8_buffer_reads_little_endian() {
    let mut vm = Vm::new();
    // Uint8Array [1, 0] = Uint16 value 1 under LE encoding
    // (elidex choice per module header — IsLittleEndian() = true).
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(2); \
             var u8 = new Uint8Array(buf); \
             u8[0] = 1; u8[1] = 0; \
             var u16 = new Uint16Array(buf); u16[0];"
        ),
        1.0
    );
    // Sanity: [0x80, 0x3f] = 1.0 as f32 (IEEE 754 LE) — ties
    // the written spec-divergence disclaimer.
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(4); \
             var u8 = new Uint8Array(buf); \
             u8[0] = 0; u8[1] = 0; u8[2] = 0x80; u8[3] = 0x3f; \
             var f32 = new Float32Array(buf); f32[0];"
        ),
        1.0
    );
}
