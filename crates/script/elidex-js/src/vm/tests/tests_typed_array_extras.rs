//! Cross-interface TypedArray tests
//! (ES2024 §23.2.3 / §23.2.5 + WHATWG HTML §2.9 + Fetch §5).
//!
//! Covers the cross-interface surface that doesn't belong with
//! the per-instance constructor or method tests: C6
//! `ArrayBuffer.isView` + Fetch body init via TypedArray + Blob
//! init via TypedArray + `structuredClone` identity preservation,
//! CanonicalNumericIndexString exotic dispatch on string-keyed
//! reads / writes, `set(source, offset?)` negative-offset
//! RangeError (§23.2.3.24), BigInt element equality (pool-based
//! `strict_eq`), and C7 integration (TypedArray ↔ ArrayBuffer ↔
//! Blob ↔ Request/Response).
//!
//! Constructor + prototype tests live in
//! [`super::tests_typed_array`]; prototype method tests + DataView
//! (C5) accessors / getters / setters in
//! [`super::tests_typed_array_methods`].

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
// `eval_global_string` / `eval_global_number` are identical to the
// helpers in the parent test module — reuse them here so behaviour
// (microtask drain order, fresh-Vm allocation) stays in lockstep
// as the harness evolves.
use super::{eval_global_number, eval_global_string};

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
fn canonical_numeric_number_keys_set_is_silent_no_op() {
    let mut vm = Vm::new();
    // Number-key direct path (bytecode may bypass ToPropertyKey):
    // NaN / ±Infinity / negative integer / fractional / out-of-
    // u32-range Number keys must be treated as canonical numeric
    // non-integer — silent no-op on Set, NOT ordinary property
    // creation.  Ordinary property would survive as
    // `u['-1']`/`u['NaN']`/etc. and pollute the TypedArray with
    // user-visible named own keys.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array([5, 6]); \
         u[-1] = 99; u[NaN] = 99; u[Infinity] = 99; u[-Infinity] = 99; \
         u[1.5] = 99; u[4294967295] = 99; \
         u[0] === 5 && u[1] === 6 && \
         u[-1] === undefined && u[NaN] === undefined && \
         u[Infinity] === undefined && u[4294967295] === undefined;"
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

#[test]
fn at_nan_maps_to_zero_per_to_integer_or_infinity() {
    let mut vm = Vm::new();
    // ES §23.2.3.3 step 3: `ToIntegerOrInfinity(NaN) = 0`, so
    // `ta.at(NaN)` resolves to index 0 (unless the receiver is
    // empty).  Used to return `undefined` via a premature NaN
    // short-circuit.
    assert_eq!(
        eval_number(&mut vm, "new Uint8Array([7, 8, 9]).at(NaN);"),
        7.0
    );
    // ±Infinity should still return undefined (out-of-range after
    // the final bounds check, not because of the early-return).
    assert!(eval_bool(
        &mut vm,
        "new Uint8Array([7, 8, 9]).at(Infinity) === undefined && \
         new Uint8Array([7, 8, 9]).at(-Infinity) === undefined;"
    ));
    // NaN on an empty TypedArray → undefined (post-bounds-check).
    assert!(eval_bool(
        &mut vm,
        "new Uint8Array(0).at(NaN) === undefined;"
    ));
}

// ---------------------------------------------------------------------------
// fromIndex / offset coercion edge cases
// (lastIndexOf §23.2.3.17 + set §23.2.3.24)
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
fn set_non_finite_offset_throws_range_error() {
    let mut vm = Vm::new();
    // ES §23.2.3.24 step 6: `ToIntegerOrInfinity(Infinity) = +Infinity`,
    // which step 8 always rejects via the `targetOffset + len >
    // ArrayLength` guard.  Clamping to u32::MAX silently accepted
    // the unrepresentable value for empty sources on u32::MAX-sized
    // destinations; reject non-finite offsets up-front instead.
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new Uint8Array(4).set([], Infinity); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { new Uint8Array(4).set([1], 4294967296); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
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
fn structured_clone_preserves_wrapper_and_regexp_identity() {
    // ES §2.9 StructuredSerialize memory-map: every Object in the
    // input graph must share a single clone even if referenced
    // multiple times.  Wrapper kinds (Number / String / Boolean /
    // BigInt) and RegExp were missing their `memo.insert(src, new)`
    // step — repeated references cloned to distinct Objects.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var n = new String('hi'); \
         var cloned = structuredClone({ a: n, b: n }); \
         cloned.a === cloned.b;"
    ));
    // RegExp literal exercises the `clone_regexp` arm.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var n = /abc/g; \
         var cloned = structuredClone({ a: n, b: n }); \
         cloned.a === cloned.b;"
    ));
}

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
// TypedArray ctor / set ToObject + ToLength coercion (§23.2.5.1) +
// DataView NaN/undefined offset bounds (§25.3.1) +
// BigInt element equality (pool-based compare, SP-coerce strict_eq)
// ---------------------------------------------------------------------------

#[test]
fn data_view_nan_offset_still_bounds_checks() {
    let mut vm = Vm::new();
    // ES §25.3.1 GetViewValue step 3-8: `ToIndex(NaN) = 0`, but the
    // `requestIndex + elementSize > viewSize` bounds check still
    // runs — `new DataView(new ArrayBuffer(1)).getInt16(NaN)` must
    // throw RangeError because `0 + 2 > 1`.
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(1)); \
         var ok = false; \
         try { dv.getInt16(NaN); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
    // And for undefined offset (also ToIndex(undefined) = 0):
    assert!(eval_bool(
        &mut vm,
        "var dv = new DataView(new ArrayBuffer(1)); \
         var ok = false; \
         try { dv.setInt32(undefined, 0); } \
         catch (e) { ok = e instanceof RangeError; } ok;"
    ));
    // Sanity: NaN offset with enough room still returns 0 bytes.
    assert_eq!(
        eval_number(&mut vm, "new DataView(new ArrayBuffer(8)).getInt16(NaN);"),
        0.0
    );
}

#[test]
fn typed_array_set_accepts_primitive_source_via_to_object() {
    let mut vm = Vm::new();
    // ES §23.2.3.24 TypedArraySetArrayElements step 3: `ToObject(source)`.
    // Primitive strings become StringWrapper whose length + indexed
    // access drive the write loop (each 1-char string ToNumber's to
    // NaN → 0 for Uint8Array).
    assert_eq!(
        eval_number(
            &mut vm,
            "var u = new Uint8Array(3); u.set('abc'); \
             u[0] + u[1] + u[2];"
        ),
        0.0
    );
    // Null / undefined still TypeError (§7.1.18 ToObject).
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array(1); var ok = false; \
         try { u.set(null); } catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_ctor_and_set_use_to_length_for_array_like_length() {
    let mut vm = Vm::new();
    // ES §23.2.5.1 array-like path uses `LengthOfArrayLike`/`ToLength`,
    // which clamps NaN / negative / -Infinity lengths to `0` — a
    // `{length: -1}` source must yield an empty TypedArray, not a
    // RangeError.
    assert_eq!(
        eval_number(&mut vm, "new Uint8Array({ length: -1 }).length;"),
        0.0
    );
    assert_eq!(
        eval_number(&mut vm, "new Uint8Array({ length: NaN, 0: 1 }).length;"),
        0.0
    );
    // `%TypedArray%.prototype.set` array-like branch follows the
    // same ToLength rule — `u.set({length: -1})` is a silent no-op.
    assert!(eval_bool(
        &mut vm,
        "var u = new Uint8Array([1, 2, 3]); u.set({ length: -1 }); \
         u[0] === 1 && u[1] === 2 && u[2] === 3;"
    ));
}

#[test]
fn typed_array_ctor_accepts_array_like_without_iterator() {
    let mut vm = Vm::new();
    // ES §23.2.5.1 steps 9-12: when `usingIterator` is undefined
    // (no `@@iterator` / explicitly nulled), the ctor walks
    // `source.length` + `source[i]` as an array-like.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array({ length: 3, 0: 1, 1: 2, 2: 3 }); \
             a.length * 100 + a[0] * 10 + a[2];"
        ),
        313.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "var src = [10, 20]; src[Symbol.iterator] = undefined; \
             var a = new Uint16Array(src); a[0] + a[1];"
        ),
        30.0
    );
}

#[test]
fn bigint_ctor_applies_to_primitive_on_object() {
    let mut vm = Vm::new();
    // ES §21.2.1.1 step 2: `BigInt(Object(1n))` unwraps via
    // `@@toPrimitive`/`valueOf` through `ToPrimitive(value, number)`.
    assert!(eval_bool(
        &mut vm,
        "var hook = {}; hook[Symbol.toPrimitive] = function () { return 7n; }; \
         BigInt(hook) === 7n;"
    ));
    // Same audit applies to BigInt.asIntN / asUintN arguments.
    assert!(eval_bool(
        &mut vm,
        "var hook = {}; hook[Symbol.toPrimitive] = function () { return 5n; }; \
         BigInt.asIntN(64, hook) === 5n;"
    ));
}

#[test]
fn to_bigint_strict_honors_at_to_primitive_on_object() {
    let mut vm = Vm::new();
    // ES §7.1.13 ToBigInt step 1 runs ToPrimitive(argument, number).
    // A `@@toPrimitive` method returning a BigInt must be accepted
    // by TypedArray BigInt element writes — readback verifies via
    // the BigInt64Array's own bit-width-preserving round-trip
    // (BigInt `42n` encodes as the 8-byte little-endian integer 42).
    assert!(eval_bool(
        &mut vm,
        "var hook = {}; \
         hook[Symbol.toPrimitive] = function () { return 42n; }; \
         var a = new BigInt64Array(1); a[0] = hook; \
         a[0] === 42n;"
    ));
}

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
    // is verified by reading the fixed byte sequence
    // [0x01, 0x02, 0x03, 0x04] with `getUint32(0)` (no
    // `littleEndian` arg → BE) and asserting against the
    // BE-interpreted u32 `0x0102_0304`.
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
