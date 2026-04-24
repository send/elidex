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
fn to_string_tag_returns_subclass_name() {
    let mut vm = Vm::new();
    // Use method-call form (not `.call()`) — `ta.toString()`
    // resolves to `Object.prototype.toString` via the prototype
    // chain (C2 has not installed a TypedArray-level toString
    // yet — that lands with C4 as the identity-equal
    // `Array.prototype.toString`).
    assert_eq!(
        eval_string(&mut vm, "var u = new Uint8Array(); u.toString();"),
        "[object Uint8Array]"
    );
    assert_eq!(
        eval_string(&mut vm, "var b = new BigInt64Array(); b.toString();"),
        "[object BigInt64Array]"
    );
    assert_eq!(
        eval_string(&mut vm, "var c = new Uint8ClampedArray(); c.toString();"),
        "[object Uint8ClampedArray]"
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
