//! `%TypedArray%.prototype` method tests (ES2024 §23.2.4, C4a + C4b).
//!
//! C4a — `fill` / `slice` / `subarray` / `indexOf` / `lastIndexOf` /
//! `includes` / `find` / `findIndex`.
//!
//! C4b — `set(source, offset?)` / `copyWithin` / `reverse` /
//! `at` / `join` plus the higher-order method suite (`map` /
//! `filter` / `forEach` / `every` / `some` / `reduce` /
//! `reduceRight` / `entries` / `keys` / `values` / `@@iterator`).
//!
//! Constructor + prototype-chain-identity tests live in
//! [`super::tests_typed_array`]; `DataView` ctor / accessor /
//! getter / setter tests in [`super::tests_data_view`];
//! cross-interface tests (`ArrayBuffer.isView` + body init +
//! `structuredClone` identity, CanonicalNumericIndexString,
//! `set` negative-offset, BigInt equality, C7 Fetch-body
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
             a[0] * 1000 + a[1] * 100 + a[2] * 10 + a[3];"
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
