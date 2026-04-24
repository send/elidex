//! `%TypedArray%` + 11 subclass + DataView tests (ES2024 §23.2 / §25.3).
//!
//! Covers the C2 shipped surface: constructor dispatch, prototype
//! chain identity, `BYTES_PER_ELEMENT` on ctor + prototype, generic
//! accessors (`buffer` / `byteOffset` / `byteLength` / `length`),
//! abstract `%TypedArray%` throwing invocation, `@@species`,
//! `@@toStringTag`.  Element-value readback lands with indexed
//! element access in C3.

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

/// Run `source` in a fresh VM and read a global String back out —
/// matches the `tests_body_mixin` pattern for Promise-consumer
/// tests (microtask drain happens during `vm.eval`).
fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
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
    // f64 can represent 1.1 exactly (via round-trip).
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

// ---------------------------------------------------------------------------
// %TypedArray%.prototype methods (C4a)
// ---------------------------------------------------------------------------

#[test]
fn fill_basic() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(4); a.fill(7); a[0] + a[1] + a[2] + a[3];"
        ),
        28.0
    );
}

#[test]
fn fill_returns_receiver() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(3); a.fill(9) === a;"
    ));
}

#[test]
fn fill_with_start_and_end() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(4); a.fill(5, 1, 3); \
             a[0] + '-' + a[1] + '-' + a[2] + '-' + a[3]; (a[0] << 0) * 1000 + a[1] * 100 + a[2] * 10 + a[3];"
        ),
        0.0_f64 * 1000.0 + 5.0_f64 * 100.0 + 5.0_f64 * 10.0 + 0.0_f64
    );
}

#[test]
fn fill_negative_indices() {
    let mut vm = Vm::new();
    // a.fill(9, -2) — start counts from end: indices 2, 3 in a 4-length.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(4); a.fill(9, -2); \
             a[0] * 1000 + a[1] * 100 + a[2] * 10 + a[3];"
        ),
        99.0
    );
}

#[test]
fn subarray_shares_buffer() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(4); var sub = a.subarray(1, 3); \
         sub.buffer === a.buffer;"
    ));
}

#[test]
fn subarray_writes_propagate_to_original() {
    let mut vm = Vm::new();
    // Write through the subarray view — original sees the change
    // since both views share the backing buffer.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(4); var sub = a.subarray(1, 3); \
             sub[0] = 42; a[1];"
        ),
        42.0
    );
}

#[test]
fn subarray_length_and_byte_offset() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(10); var sub = a.subarray(2, 7); sub.length;"
        ),
        5.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(10); var sub = a.subarray(2, 7); sub.byteOffset;"
        ),
        2.0
    );
}

#[test]
fn slice_fresh_buffer() {
    let mut vm = Vm::new();
    // slice returns a new TypedArray over a NEW buffer.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(4); a[0] = 1; a[1] = 2; \
         var cp = a.slice(); cp.buffer !== a.buffer;"
    ));
}

#[test]
fn slice_copies_values() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); a[0] = 10; a[1] = 20; a[2] = 30; \
             var cp = a.slice(1); cp[0] + cp[1];"
        ),
        50.0
    );
}

#[test]
fn slice_writes_do_not_propagate() {
    let mut vm = Vm::new();
    // Writing to the slice must NOT affect the original.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); a[0] = 10; var cp = a.slice(); \
             cp[0] = 99; a[0];"
        ),
        10.0
    );
}

#[test]
fn iterator_values_round_trip() {
    let mut vm = Vm::new();
    // Use a for-of loop to verify the iterator protocol.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); a[0] = 1; a[1] = 2; a[2] = 3; \
             var sum = 0; for (var v of a) sum += v; sum;"
        ),
        6.0
    );
}

#[test]
fn iterator_keys() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(3); var keys = []; \
             for (var k of a.keys()) keys.push(k); \
             keys[0] + keys[1] + keys[2];"
        ),
        3.0
    );
}

#[test]
fn iterator_entries() {
    let mut vm = Vm::new();
    // Entries are [idx, val] pairs: for a = Uint8Array([10, 20]),
    // iter yields [0, 10] then [1, 20].  Flat push into `e` gives
    // e = [0, 10, 1, 20].  Encode as 0*1000 + 10*100 + 1*10 + 20
    // = 1030.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array(2); a[0] = 10; a[1] = 20; \
             var e = []; for (var x of a.entries()) { e.push(x[0], x[1]); } \
             e[0] * 1000 + e[1] * 100 + e[2] * 10 + e[3];"
        ),
        1030.0
    );
}

#[test]
fn symbol_iterator_identity_to_values() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var p = Object.getPrototypeOf(Uint8Array.prototype); \
         p[Symbol.iterator] === p.values;"
    ));
}

#[test]
fn to_string_identity_to_array_prototype() {
    let mut vm = Vm::new();
    // Spec §23.2.3.31: the initial value of
    // `%TypedArray%.prototype.toString` is the same built-in
    // function object as `Array.prototype.toString`.
    assert!(eval_bool(
        &mut vm,
        "var p = Object.getPrototypeOf(Uint8Array.prototype); \
         p.toString === Array.prototype.toString;"
    ));
}

#[test]
fn to_string_invokes_join() {
    let mut vm = Vm::new();
    // With `.join` installed (C4b), `.toString()` produces comma-
    // separated element output by delegating through
    // `Array.prototype.toString` → `this.join(",")`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Uint8Array(3); a[0] = 1; a[1] = 2; a[2] = 3; \
             a.toString();"
        ),
        "1,2,3"
    );
}

// ---------------------------------------------------------------------------
// C4b methods: set / copyWithin / reverse / search / at / join / HOFs
// ---------------------------------------------------------------------------

#[test]
fn set_typed_array_source() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dst = new Uint8Array(5); \
             var src = new Uint8Array([10, 20, 30]); \
             dst.set(src, 1); \
             dst[0] * 10000 + dst[1] * 1000 + dst[2] * 100 + dst[3] * 10 + dst[4];"
        ),
        // [0, 10, 20, 30, 0]
        0.0 * 10000.0 + 10.0 * 1000.0 + 20.0 * 100.0 + 30.0 * 10.0 + 0.0
    );
}

#[test]
fn set_array_source() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var dst = new Uint8Array(3); dst.set([7, 8, 9]); \
             dst[0] * 100 + dst[1] * 10 + dst[2];"
        ),
        789.0
    );
}

#[test]
fn set_out_of_range_throws_range_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var dst = new Uint8Array(2); var ok = false; \
         try { dst.set([1, 2, 3]); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

#[test]
fn set_mixed_bigint_throws_type_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var dst = new BigInt64Array(2); \
         var src = new Uint8Array([1, 2]); \
         var ok = false; \
         try { dst.set(src); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn copy_within_basic() {
    let mut vm = Vm::new();
    // [1,2,3,4,5].copyWithin(0, 3) → [4,5,3,4,5]
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(0, 3); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        4.0 * 10000.0 + 5.0 * 1000.0 + 3.0 * 100.0 + 4.0 * 10.0 + 5.0
    );
}

#[test]
fn copy_within_overlap_forward() {
    let mut vm = Vm::new();
    // [1,2,3,4,5].copyWithin(1, 0, 4) → [1,1,2,3,4]
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(1, 0, 4); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        1.0 * 10000.0 + 1.0 * 1000.0 + 2.0 * 100.0 + 3.0 * 10.0 + 4.0
    );
}

#[test]
fn reverse_in_place() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4]); a.reverse(); \
             a[0] * 1000 + a[1] * 100 + a[2] * 10 + a[3];"
        ),
        4.0 * 1000.0 + 3.0 * 100.0 + 2.0 * 10.0 + 1.0
    );
}

#[test]
fn reverse_returns_receiver() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array(3); a.reverse() === a;"
    ));
}

#[test]
fn index_of_hit_and_miss() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([10, 20, 30]); a.indexOf(20);"
        ),
        1.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([10, 20, 30]); a.indexOf(99);"
        ),
        -1.0
    );
}

#[test]
fn index_of_nan_never_matches() {
    let mut vm = Vm::new();
    // indexOf uses strict equality (NaN !== NaN), unlike includes.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Float64Array([1, NaN, 3]); a.indexOf(NaN);"
        ),
        -1.0
    );
}

#[test]
fn last_index_of_scans_in_reverse() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 2, 1]); a.lastIndexOf(2);"
        ),
        3.0
    );
}

#[test]
fn includes_finds_nan_in_float_arrays() {
    let mut vm = Vm::new();
    // includes uses SameValueZero — NaN matches NaN.
    assert!(eval_bool(
        &mut vm,
        "new Float64Array([1, NaN, 3]).includes(NaN);"
    ));
}

#[test]
fn at_negative_index_wraps() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var a = new Uint8Array([10, 20, 30]); a.at(-1);"),
        30.0
    );
    assert!(eval_bool(
        &mut vm,
        "new Uint8Array([1, 2, 3]).at(99) === undefined;"
    ));
}

#[test]
fn join_default_separator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Uint8Array([1, 2, 3]).join();"),
        "1,2,3"
    );
}

#[test]
fn join_custom_separator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Uint8Array([1, 2, 3]).join(\"-\");"),
        "1-2-3"
    );
}

#[test]
fn for_each_invokes_callback_per_element() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([10, 20, 30]); var sum = 0; \
             a.forEach(function(v) { sum += v; }); sum;"
        ),
        60.0
    );
}

#[test]
fn for_each_receives_index_and_this() {
    let mut vm = Vm::new();
    // Callback receives (element, index, typedArray).
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20]); \
         var ok = true; \
         a.forEach(function(v, i, arr) { if (arr !== a) ok = false; if (arr[i] !== v) ok = false; }); \
         ok;"
    ));
}

#[test]
fn every_short_circuits_on_false() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new Uint8Array([2, 4, 6]).every(function(v) { return v % 2 === 0; });"
    ));
    assert!(!eval_bool(
        &mut vm,
        "new Uint8Array([2, 3, 6]).every(function(v) { return v % 2 === 0; });"
    ));
}

#[test]
fn some_short_circuits_on_true() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new Uint8Array([1, 2, 3]).some(function(v) { return v === 2; });"
    ));
    assert!(!eval_bool(
        &mut vm,
        "new Uint8Array([1, 3, 5]).some(function(v) { return v === 2; });"
    ));
}

#[test]
fn find_returns_element() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 4, 9, 16]).find(function(v) { return v > 5; });"
        ),
        9.0
    );
}

#[test]
fn find_index_returns_index() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 4, 9, 16]).findIndex(function(v) { return v > 5; });"
        ),
        2.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 2, 3]).findIndex(function(v) { return v > 99; });"
        ),
        -1.0
    );
}

// ---------------------------------------------------------------------------
// DataView (C5)
// ---------------------------------------------------------------------------

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
    // TypedArray always writes little-endian (elidex choice).
    // Reading the same bytes via DataView with littleEndian=true
    // must round-trip the value.
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

// ---------------------------------------------------------------------------
// C6: ArrayBuffer.isView + body init + structured_clone
// ---------------------------------------------------------------------------

#[test]
fn array_buffer_is_view_on_typed_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "ArrayBuffer.isView(new Uint8Array(4));"));
    assert!(eval_bool(
        &mut vm,
        "ArrayBuffer.isView(new Float64Array(2));"
    ));
}

#[test]
fn array_buffer_is_view_on_data_view() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "ArrayBuffer.isView(new DataView(new ArrayBuffer(8)));"
    ));
}

#[test]
fn array_buffer_is_view_on_non_views() {
    let mut vm = Vm::new();
    // Plain ArrayBuffer is NOT a view (spec §25.1.4.3 step 2).
    assert!(!eval_bool(
        &mut vm,
        "ArrayBuffer.isView(new ArrayBuffer(4));"
    ));
    assert!(!eval_bool(&mut vm, "ArrayBuffer.isView({});"));
    assert!(!eval_bool(&mut vm, "ArrayBuffer.isView(null);"));
    assert!(!eval_bool(&mut vm, "ArrayBuffer.isView(42);"));
    assert!(!eval_bool(&mut vm, "ArrayBuffer.isView();"));
}

#[test]
fn request_body_accepts_typed_array() {
    // `new Request(url, { body: typedArray })` + `.text()` returns
    // the UTF-8 decoded byte sequence of the view's bytes.  Promise
    // settle observed via `globalThis.r` after microtask drain.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             var u = new Uint8Array([72, 105]); \
             new Request('http://example.com/', { method: 'POST', body: u }).text() \
                 .then(function(s) { globalThis.r = s; });",
            "r",
        ),
        "Hi"
    );
}

#[test]
fn request_body_respects_typed_array_view_range() {
    // Only the view's byte range is consumed, not the whole buffer.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             var b = new ArrayBuffer(10); var u = new Uint8Array(b); \
             for (var i = 0; i < 10; i++) u[i] = 65 + i; \
             var sub = new Uint8Array(b, 2, 3); \
             new Request('http://example.com/', { method: 'POST', body: sub }).text() \
                 .then(function(s) { globalThis.r = s; });",
            "r",
        ),
        "CDE"
    );
}

#[test]
fn request_body_accepts_data_view() {
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             var b = new ArrayBuffer(3); var u = new Uint8Array(b); \
             u[0] = 65; u[1] = 66; u[2] = 67; \
             var dv = new DataView(b); \
             new Request('http://example.com/', { method: 'POST', body: dv }).text() \
                 .then(function(s) { globalThis.r = s; });",
            "r",
        ),
        "ABC"
    );
}

#[test]
fn blob_accepts_typed_array_part() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var u = new Uint8Array([1, 2, 3, 4, 5]); \
             var b = new Blob([u]); b.size;"
        ),
        5.0
    );
}

#[test]
fn blob_accepts_typed_array_view_subrange() {
    let mut vm = Vm::new();
    // Only the view's slice is included, not the full backing buffer.
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(10); \
             var sub = new Uint8Array(buf, 2, 3); \
             var b = new Blob([sub]); b.size;"
        ),
        3.0
    );
}

#[test]
fn structured_clone_typed_array_round_trip() {
    let mut vm = Vm::new();
    // Clone Uint8Array, then read via indexed access on the clone.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([10, 20, 30]); \
             var c = structuredClone(a); \
             c[0] * 10000 + c[1] * 100 + c[2];"
        ),
        10.0 * 10000.0 + 20.0 * 100.0 + 30.0
    );
}

#[test]
fn structured_clone_typed_array_preserves_subclass() {
    let mut vm = Vm::new();
    // Cloning a Uint16Array yields a Uint16Array (not Uint8Array).
    // Check via @@toStringTag.
    assert_eq!(
        eval_string(
            &mut vm,
            "var p = Object.getPrototypeOf(Uint16Array.prototype); \
             var g = Object.getOwnPropertyDescriptor(p, Symbol.toStringTag).get; \
             g.call(structuredClone(new Uint16Array(3)));"
        ),
        "Uint16Array"
    );
}

#[test]
fn structured_clone_typed_array_fresh_buffer() {
    let mut vm = Vm::new();
    // Clone does NOT share the source's buffer — mutations on the
    // clone's buffer must not reach the source.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var c = structuredClone(a); \
         c.buffer !== a.buffer;"
    ));
}

#[test]
fn structured_clone_two_views_share_cloned_buffer() {
    let mut vm = Vm::new();
    // Critical spec invariant (memo threading): two views over the
    // same source ArrayBuffer must clone to two views over the
    // SAME cloned ArrayBuffer.  Mutations through one cloned view
    // must be visible through the other.
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(4); \
             var v1 = new Uint8Array(buf); \
             var v2 = new Uint8Array(buf); \
             v1[0] = 10; v2[1] = 20; \
             var cloned = structuredClone([v1, v2]); \
             cloned[0].buffer === cloned[1].buffer ? 1 : 0;"
        ),
        1.0
    );
}

#[test]
fn structured_clone_data_view() {
    let mut vm = Vm::new();
    // DataView clone preserves offset/length + shares a cloned buffer
    // with any TypedArray sibling view (spec §2.9).
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(8); var dv = new DataView(buf, 2, 4); \
             dv.setInt32(0, 12345, true); \
             var clone = structuredClone(dv); \
             clone.getInt32(0, true);"
        ),
        12345.0
    );
}

#[test]
fn buffer_getter_brand_check_rejects_foreign() {
    let mut vm = Vm::new();
    // `TypedArray.prototype.buffer` is shared via the abstract
    // prototype — calling it on a non-TypedArray receiver throws
    // TypeError per WebIDL brand checks.
    assert!(eval_bool(
        &mut vm,
        "var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(Uint8Array.prototype), \
             \"buffer\").get; \
         var ok = false; \
         try { getter.call({}); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

// ---------------------------------------------------------------------------
// CanonicalNumericIndexString — string-key TypedArray exotic dispatch
// ---------------------------------------------------------------------------

#[test]
fn canonical_numeric_keys_return_undefined_not_ordinary() {
    let mut vm = Vm::new();
    // ES §7.1.16.1: `"-0"`, `"Infinity"`, `"-Infinity"`, `"NaN"` are
    // canonical numeric index strings.  On a TypedArray integer-
    // indexed exotic they must return `undefined` (§10.4.5.15
    // step 3), NOT fall through to ordinary [[Get]] lookup.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array([1, 2]); \
         u['-0'] === undefined && u['Infinity'] === undefined && \
         u['-Infinity'] === undefined && u['NaN'] === undefined && \
         u['1.5'] === undefined && u['-1'] === undefined;"
    ));
}

#[test]
fn canonical_numeric_keys_set_is_silent_no_op() {
    let mut vm = Vm::new();
    // ES §10.4.5.16 step 1: storing on a canonical numeric index
    // that is not a valid integer index is a silent no-op — the
    // write doesn't create an own property, so a subsequent read
    // still returns `undefined`.  The in-range integer elements
    // stay untouched.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array([5, 6]); \
         u['-0'] = 99; u['NaN'] = 99; u['Infinity'] = 99; u['-1'] = 99; \
         u[0] === 5 && u[1] === 6 && \
         u['-0'] === undefined && u['NaN'] === undefined;"
    ));
}

#[test]
fn typed_array_accepts_u32_max_boundary_index_string() {
    let mut vm = Vm::new();
    // `"4294967295"` = u32::MAX is a valid CanonicalNumericIndexString.
    // Out-of-range integer index → Get returns `undefined`, Set is
    // a silent no-op (§10.4.5.15/16); must not fall through to
    // ordinary property storage.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array(1); \
         u['4294967295'] === undefined && \
         (u['4294967295'] = 42, u['4294967295'] === undefined);"
    ));
}

#[test]
fn non_canonical_string_keys_fall_through_to_ordinary() {
    let mut vm = Vm::new();
    // `"01"` / `"foo"` are NOT canonical (ToString round-trip fails) —
    // they DO create ordinary own properties observable via Get.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array(2); u['01'] = 1; u['foo'] = 2; \
         u['01'] === 1 && u['foo'] === 2;"
    ));
}

// ---------------------------------------------------------------------------
// set(source, offset?) — negative offset → RangeError (§23.2.3.24)
// ---------------------------------------------------------------------------

#[test]
fn last_index_of_too_negative_from_index_returns_minus_one() {
    let mut vm = Vm::new();
    // ES §23.2.3.17 step 5: when `len + fromIndex` < 0, the scan
    // has nothing to inspect → return -1.  Must NOT wrap to
    // `max(len + fromIndex, 0)` (the indexOf semantics) — that
    // would surface a false positive at index 0.
    assert_eq!(
        eval_number(&mut vm, "new Uint8Array([9, 1, 2]).lastIndexOf(9, -10);"),
        -1.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([9, 1, 2]).lastIndexOf(9, -Infinity);"
        ),
        -1.0
    );
    // Sanity: within-range negative still matches.
    assert_eq!(
        eval_number(&mut vm, "new Uint8Array([9, 1, 9]).lastIndexOf(9, -1);"),
        2.0
    );
}

#[test]
fn set_negative_offset_throws_range_error() {
    let mut vm = Vm::new();
    // ES §23.2.3.24 step 6: `ToIntegerOrInfinity(offset)` RangeErrors
    // on any negative result.  Must not silently wrap via `length +
    // offset` (old `relative_index_u32` behavior).
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new Uint8Array(4).set([1], -1); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
}

// ---------------------------------------------------------------------------
// structuredClone — preserves TypedArray / DataView identity in graph
// ---------------------------------------------------------------------------

#[test]
fn structured_clone_preserves_typed_array_identity() {
    let mut vm = Vm::new();
    // Same TypedArray referenced twice in the source graph → single
    // cloned TypedArray observed at both sites (§2.9 memory map +
    // graph identity).
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array([1, 2, 3]); \
         var cloned = structuredClone({ a: u, b: u }); \
         cloned.a === cloned.b;"
    ));
}

#[test]
fn structured_clone_preserves_data_view_identity() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(4)); \
         var cloned = structuredClone({ a: dv, b: dv }); \
         cloned.a === cloned.b;"
    ));
}

// ---------------------------------------------------------------------------
// BigInt element equality — pool-based compare (SP-coerce strict_eq)
// ---------------------------------------------------------------------------

#[test]
fn big_int64_includes_compares_by_value_not_handle() {
    let mut vm = Vm::new();
    // Every read on a BigInt TypedArray allocates a fresh BigIntId,
    // so handle-equality (`a == b` on JsValue::BigInt) would always
    // miss.  `includes` / `indexOf` / `lastIndexOf` must compare the
    // mathematical value through the BigInt pool.
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array([1n, 2n, 3n]); a.includes(2n);"
    ));
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new BigInt64Array([1n, 2n, 3n, 2n]); a.indexOf(2n);"
        ),
        1.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new BigInt64Array([1n, 2n, 3n, 2n]); a.lastIndexOf(2n);"
        ),
        3.0
    );
}

// ---------------------------------------------------------------------------
// C7 — integration: TypedArray ↔ ArrayBuffer ↔ Blob ↔ Request/Response
// ---------------------------------------------------------------------------

#[test]
fn request_body_typed_array_round_trips_via_array_buffer() {
    // Full body pipeline: Uint8Array → Request({ body }) →
    // `.arrayBuffer()` → re-wrap Uint8Array → indexed read.
    // Exercises C2 ctor (ArrayBuffer form), C3 indexed access, and
    // WHATWG Fetch §5 `arrayBuffer()` chain.
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             var u = new Uint8Array([7, 11, 13, 17, 19]); \
             new Request('http://example.com/', { method: 'POST', body: u }) \
                 .arrayBuffer() \
                 .then(ab => { var v = new Uint8Array(ab); \
                               globalThis.r = v[0] + v[1] + v[2] + v[3] + v[4]; });",
            "r",
        ),
        67.0
    );
}

#[test]
fn typed_array_method_chain_fill_subarray_hof() {
    let mut vm = Vm::new();
    // Combined method coverage: `fill` writes, `subarray` shares
    // buffer, `forEach` iterates, closure captures accumulate —
    // verifies the C4a/C4b surfaces compose correctly.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Int32Array(6); a.fill(3); \
             var sub = a.subarray(1, 5); \
             sub[0] = 10; sub[3] = 40; \
             var sum = 0; sub.forEach(function(v) { sum += v; }); \
             sum;"
        ),
        56.0
    );
}

#[test]
fn blob_array_buffer_read_via_data_view() {
    // Blob init from TypedArray → `.arrayBuffer()` → DataView
    // readback.  Exercises C6 Blob part acceptance + C5 DataView
    // read path in one pipeline.  Big-endian default (spec §25.3.4)
    // verified by writing LE and reading BE (swapped bytes).
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             var src = new Uint8Array([0x01, 0x02, 0x03, 0x04]); \
             new Blob([src]).arrayBuffer().then(ab => { \
                 var dv = new DataView(ab); \
                 globalThis.r = dv.getUint32(0); \
             });",
            "r",
        ),
        0x0102_0304 as f64
    );
}

#[test]
fn structured_clone_nested_views_preserve_sharing_and_isolation() {
    let mut vm = Vm::new();
    // Clone a plain Object containing two views that share one
    // backing buffer, then mutate the original.  Clone must:
    //   (a) preserve shared-buffer identity (spec §2.9 memory map),
    //   (b) be isolated from subsequent mutation of the source.
    assert!(eval_bool(
        &mut vm,
        "var buf = new ArrayBuffer(8); \
         var u = new Uint8Array(buf); \
         var dv = new DataView(buf); \
         u[0] = 0xAA; dv.setUint8(4, 0xBB); \
         var wrap = { a: u, b: dv }; \
         var clone = structuredClone(wrap); \
         u[0] = 0x11; dv.setUint8(4, 0x22); \
         var sharing = clone.a.buffer === clone.b.buffer; \
         var isolated = clone.a[0] === 0xAA && clone.b.getUint8(4) === 0xBB; \
         sharing && isolated;"
    ));
}

#[test]
fn overlapping_views_on_shared_buffer_method_composition() {
    let mut vm = Vm::new();
    // Multiple overlapping views on one ArrayBuffer.  Mutations via
    // `set` / indexed assignment on any view are visible through
    // every other view over the same bytes — validates the
    // whole-buffer replace semantics (D3) without detached tracking.
    assert_eq!(
        eval_number(
            &mut vm,
            "var buf = new ArrayBuffer(8); \
             var u8 = new Uint8Array(buf); \
             var u16 = new Uint16Array(buf); \
             u8.set([0x34, 0x12, 0x78, 0x56, 0, 0, 0, 0]); \
             var lo = u16[0]; \
             u16[1] = 0xCAFE; \
             var hi_byte = u8[2]; \
             var copied = new Uint8Array(buf.slice(0, 4)); \
             copied[0] + lo + hi_byte;"
        ),
        // u8[0] via copy = 0x34 ; lo = 0x1234 (LE) ; hi_byte = u8[2] post-write = 0xFE
        (0x34 + 0x1234 + 0xFE) as f64
    );
}
