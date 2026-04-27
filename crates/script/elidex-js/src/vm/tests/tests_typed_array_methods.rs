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
fn fill_int32_pattern_propagates_to_every_element() {
    // Multi-byte element (Int32: 4 bytes per element) — verify the
    // bulk-fill helper correctly repeats the LE byte pattern across
    // every covered slot and leaves uncovered slots untouched.
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Int32Array(4); a.fill(0x12345678, 1, 3); \
             (a[0] === 0 && a[1] === 0x12345678 && a[2] === 0x12345678 && a[3] === 0) \
                 ? 1 : 0;"
        ),
        1.0
    );
}

#[test]
fn fill_float64_pattern_propagates_to_every_element() {
    // 8-byte element fast-path — exercises the `_ => chunk-write`
    // branch in `byte_io::fill_pattern` (Int8/Uint8 take the
    // single-byte `slice::fill` arm; multi-byte widths land here).
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Float64Array(3); a.fill(1.5); \
             (a[0] === 1.5 && a[1] === 1.5 && a[2] === 1.5) ? 1 : 0;"
        ),
        1.0
    );
}

#[test]
fn fill_empty_range_is_a_noop() {
    // `start >= end` after relative-index normalisation must leave
    // the array unmodified.  Exercises the early-return guard
    // before the coerce step (skipping coercion when no element
    // would be written matches the spec's vacuous loop).
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4]); a.fill(99, 2, 2); \
             a[0] * 1000 + a[1] * 100 + a[2] * 10 + a[3];"
        ),
        1234.0
    );
}

#[test]
fn slice_bulk_copy_preserves_int32_pattern() {
    // Multi-byte element bulk-copy regression — `slice()` now goes
    // through `byte_io::copy_bytes` (not per-element decode/encode),
    // so verify LE byte sequence survives the snapshot+install path
    // for a 4-byte element width.
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Int32Array([1, 2, 3, 4, 5]); var b = a.slice(1, 4); \
             (b.length === 3 && b[0] === 2 && b[1] === 3 && b[2] === 4) ? 1 : 0;"
        ),
        1.0
    );
}

#[test]
fn copy_within_forward_overlap_uses_pre_snapshot() {
    // Forward overlap: dst > src.  The pre-snapshot in
    // `byte_io::copy_bytes` is what makes this correct under
    // overlap — copying `[0, 1, 2, 3, 4][0..3]` to position 2
    // must yield `[0, 1, 0, 1, 2]`, NOT
    // `[0, 1, 0, 1, 0]` (which is what a naive forward in-place
    // copy would produce).
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([0, 1, 2, 3, 4]); a.copyWithin(2, 0, 3); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        1012.0
    );
}

#[test]
fn copy_within_backward_overlap_uses_pre_snapshot() {
    // Backward overlap: dst < src.  Spec §23.2.3.6 requires
    // pre-snapshot semantics; copying `[10, 11, 12, 13, 14][2..5]`
    // to position 0 yields `[12, 13, 14, 13, 14]` —
    // 12*10000 + 13*1000 + 14*100 + 13*10 + 14 = 134544.
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([10, 11, 12, 13, 14]); a.copyWithin(0, 2); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        134544.0
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
fn set_same_ek_self_overlap_is_identity() {
    let mut vm = Vm::new();
    // `a.set(a)` exercises the same-`ElementKind` bulk-copy path
    // with src and dst aliasing the entire backing buffer.  The
    // pre-snapshot in `byte_io::copy_bytes` keeps the result
    // identical to the source even when src and dst point at the
    // same `(buffer_id, byte_offset)`.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); a.set(a); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        12345.0
    );
}

#[test]
fn set_same_ek_forward_overlap_subarray() {
    let mut vm = Vm::new();
    // Src view (`a.subarray(2, 4)`) lives ahead of the destination
    // (`set(..., 0)`); a naive forward in-place copy would overwrite
    // bytes that the later iterations still need to read.  The
    // pre-snapshot path is direction-agnostic.
    // [1,2,3,4,5] with subarray(2,4)=[3,4] copied to offset 0 → [3,4,3,4,5]
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); \
             a.set(a.subarray(2, 4), 0); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        3.0 * 10000.0 + 4.0 * 1000.0 + 3.0 * 100.0 + 4.0 * 10.0 + 5.0
    );
}

#[test]
fn set_same_ek_backward_overlap_subarray() {
    let mut vm = Vm::new();
    // Mirror of the forward case — src view (`a.subarray(0, 2)`)
    // lives before the destination (`set(..., 2)`).
    // [1,2,3,4,5] with subarray(0,2)=[1,2] copied to offset 2 → [1,2,1,2,5]
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5]); \
             a.set(a.subarray(0, 2), 2); \
             a[0] * 10000 + a[1] * 1000 + a[2] * 100 + a[3] * 10 + a[4];"
        ),
        1.0 * 10000.0 + 2.0 * 1000.0 + 1.0 * 100.0 + 2.0 * 10.0 + 5.0
    );
}

#[test]
fn set_same_ek_int32_cross_buffer() {
    let mut vm = Vm::new();
    // 4-byte `ElementKind` validates that the bulk-copy `byte_offset
    // + target_offset * bpe` arithmetic walks elements, not bytes.
    // src and dst live in distinct backing buffers (no overlap).
    assert_eq!(
        eval_number(
            &mut vm,
            "var src = new Int32Array([100, 200, 300]); \
             var dst = new Int32Array(4); dst.set(src, 1); \
             dst[0] + dst[1] + dst[2] + dst[3];"
        ),
        600.0
    );
}

#[test]
fn set_same_ek_into_subview_of_outer_buffer() {
    let mut vm = Vm::new();
    // Destination view has a non-zero `byte_offset` within its
    // backing buffer; a sibling outer view lets the test observe
    // bytes outside the destination's window to confirm the bulk
    // copy didn't spill past `byte_offset .. byte_offset + length`.
    assert_eq!(
        eval_number(
            &mut vm,
            "var ab = new ArrayBuffer(8); \
             var dst = new Uint8Array(ab, 2, 4); \
             var src = new Uint8Array([10, 20, 30]); \
             dst.set(src, 1); \
             var v = new Uint8Array(ab); \
             v[0] * 10000000 + v[1] * 1000000 + v[2] * 100000 + v[3] * 10000 + \
             v[4] * 1000 + v[5] * 100 + v[6] * 10 + v[7];"
        ),
        // expected layout: [0, 0, 0, 10, 20, 30, 0, 0]
        10.0 * 10000.0 + 20.0 * 1000.0 + 30.0 * 100.0
    );
}

#[test]
fn set_different_ek_falls_back_to_per_element_coerce() {
    let mut vm = Vm::new();
    // Src `Uint8Array` and dst `Int16Array` differ in `ElementKind`
    // — the bulk-copy fast path must NOT trigger; per-element
    // `ToNumber` widens each byte into a 16-bit slot.
    assert_eq!(
        eval_number(
            &mut vm,
            "var src = new Uint8Array([1, 2, 3]); \
             var dst = new Int16Array(3); dst.set(src); \
             dst[0] * 100 + dst[1] * 10 + dst[2];"
        ),
        123.0
    );
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
// findLast / findLastIndex (SP8b — reverse iteration, no allocation)
// ---------------------------------------------------------------------------

#[test]
fn find_last_returns_last_match() {
    let mut vm = Vm::new();
    // Two elements satisfy `v > 5`; reverse-iteration returns the
    // tail one (16), distinguishing this from `find` which would
    // return 9.
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 4, 9, 16]).findLast(function(v) { return v > 5; });"
        ),
        16.0
    );
}

#[test]
fn find_last_returns_undefined_on_miss() {
    let mut vm = Vm::new();
    assert!(matches!(
        vm.eval("new Uint8Array([1, 2, 3]).findLast(function(v) { return v > 99; });"),
        Ok(JsValue::Undefined)
    ));
}

#[test]
fn find_last_index_returns_last_match_index() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 4, 9, 16]).findLastIndex(function(v) { return v > 5; });"
        ),
        3.0
    );
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 2, 3]).findLastIndex(function(v) { return v > 99; });"
        ),
        -1.0
    );
}

#[test]
fn find_last_on_empty_returns_undefined() {
    let mut vm = Vm::new();
    assert!(matches!(
        vm.eval("new Uint8Array(0).findLast(function() { return true; });"),
        Ok(JsValue::Undefined)
    ));
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array(0).findLastIndex(function() { return true; });"
        ),
        -1.0
    );
}

#[test]
fn find_last_callback_receives_index_and_array() {
    let mut vm = Vm::new();
    // Callback receives (element, index, typedArray) — same shape
    // as `find`.  Asserts reverse iteration ordering by recording
    // the visit sequence.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Uint8Array([10, 20, 30]); var visits = []; \
             a.findLast(function(v, i, arr) { \
                 if (arr !== a) throw new Error('this'); \
                 visits.push(String(i) + ':' + String(v)); \
                 return false; \
             }); \
             visits.join(',');"
        ),
        "2:30,1:20,0:10"
    );
}

// ---------------------------------------------------------------------------
// map (SP8b — TypedArraySpeciesCreate + per-element write)
// ---------------------------------------------------------------------------

#[test]
fn map_basic_doubles_and_preserves_subclass() {
    let mut vm = Vm::new();
    // Default species: receiver.constructor[@@species] resolves to
    // the receiver's own subclass ctor (`%TypedArray%[@@species]`
    // returns `this`).  Result is a fresh Uint8Array of same length.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3, 4]); \
         var b = a.map(function(v) { return v * 2; }); \
         b instanceof Uint8Array && b !== a && b.length === 4 && \
         b[0] === 2 && b[1] === 4 && b[2] === 6 && b[3] === 8;"
    ));
}

#[test]
fn map_callback_receives_index_and_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); var ok = true; \
         var b = a.map(function(v, i, arr) { \
             if (arr !== a) ok = false; if (arr[i] !== v) ok = false; return v; \
         }); \
         ok && b.length === 3;"
    ));
}

#[test]
fn map_truncates_through_destination_coercion() {
    let mut vm = Vm::new();
    // `write_element_raw` for Uint8 applies `ToUint8` coercion —
    // values >= 256 wrap modulo 256.  `1+255=256 → 0`,
    // `2+255=257 → 1`, `3+255=258 → 2` exercises the wrap exactly
    // at the boundary.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var b = a.map(function(v) { return v + 255; }); \
         b[0] === 0 && b[1] === 1 && b[2] === 2;"
    ));
    assert!(eval_bool(
        &mut vm,
        "var a = new Float32Array([1, 2]); \
         var b = a.map(function(v) { return v + 0.5; }); \
         b[0] === 1.5 && b[1] === 2.5;"
    ));
}

#[test]
fn map_throws_when_callback_not_callable() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1]).map(42);")
        .unwrap_err()
        .message
        .contains("callback is not a function"));
}

#[test]
fn map_propagates_callback_error() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1, 2, 3]).map(function() { throw new Error('boom'); });")
        .is_err());
}

#[test]
fn map_on_empty_returns_empty_view() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new Uint8Array(0).map(function() { return 1; }); \
         b instanceof Uint8Array && b.length === 0;"
    ));
}

#[test]
fn map_user_subclass_preserves_species() {
    let mut vm = Vm::new();
    // `class extends` for built-in TypedArray parents isn't fully
    // wired up; build the subclass manually.  Same pattern as
    // `tests_typed_array_static.rs`'s subclass coverage.  The
    // species lookup walks `Sub.prototype.constructor === Sub`,
    // then `Sub[@@species]` (inherited from `%TypedArray%`'s
    // identity getter) returns `Sub`, then the prototype-chain
    // walk finds `Uint8Array` and uses `Sub.prototype` as
    // `proto_override` so `result instanceof Sub`.
    assert!(eval_bool(
        &mut vm,
        "function Sub() {} \
         Object.setPrototypeOf(Sub, Uint8Array); \
         Sub.prototype = Object.create(Uint8Array.prototype); \
         Sub.prototype.constructor = Sub; \
         var s = Sub.from([1, 2, 3]); \
         var m = s.map(function(v) { return v * 10; }); \
         m instanceof Sub && m.length === 3 && m[0] === 10 && m[2] === 30;"
    ));
}

// ---------------------------------------------------------------------------
// filter (SP8b — collect-then-species-allocate)
// ---------------------------------------------------------------------------

#[test]
fn filter_keeps_matching_elements() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3, 4, 5]); \
         var b = a.filter(function(v) { return v % 2 === 1; }); \
         b instanceof Uint8Array && b.length === 3 && \
         b[0] === 1 && b[1] === 3 && b[2] === 5;"
    ));
}

#[test]
fn filter_callback_receives_index_and_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); var ok = true; \
         a.filter(function(v, i, arr) { \
             if (arr !== a) ok = false; if (arr[i] !== v) ok = false; return true; \
         }); \
         ok;"
    ));
}

#[test]
fn filter_empty_result_returns_empty_view() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new Uint8Array([1, 2, 3]).filter(function() { return false; }); \
         b instanceof Uint8Array && b.length === 0;"
    ));
}

#[test]
fn filter_all_kept_returns_full_copy() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([4, 5, 6]); \
         var b = a.filter(function() { return true; }); \
         b !== a && b.length === 3 && b[0] === 4 && b[1] === 5 && b[2] === 6;"
    ));
}

#[test]
fn filter_throws_when_callback_not_callable() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1]).filter('nope');")
        .unwrap_err()
        .message
        .contains("callback is not a function"));
}

#[test]
fn filter_bigint_array_keeps_through_alloc_point() {
    let mut vm = Vm::new();
    // BigInt elements allocate fresh `BigIntId`s on every read;
    // collected values must remain GC-rooted across the
    // `create_typed_array_for_length` allocation point inside
    // `filter`.  Same hazard as SP8a `from`'s iterator drain.
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array([1n, 2n, 3n, 4n]); \
         var b = a.filter(function(v) { return v > 1n; }); \
         b instanceof BigInt64Array && b.length === 3 && \
         b[0] === 2n && b[1] === 3n && b[2] === 4n;"
    ));
}

#[test]
fn filter_user_subclass_preserves_species() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "function Sub() {} \
         Object.setPrototypeOf(Sub, Uint8Array); \
         Sub.prototype = Object.create(Uint8Array.prototype); \
         Sub.prototype.constructor = Sub; \
         var s = Sub.from([1, 2, 3, 4]); \
         var f = s.filter(function(v) { return v >= 3; }); \
         f instanceof Sub && f.length === 2 && f[0] === 3 && f[1] === 4;"
    ));
}

// ---------------------------------------------------------------------------
// SpeciesConstructor error paths (map / filter)
// ---------------------------------------------------------------------------

#[test]
fn map_throws_when_constructor_is_non_object() {
    let mut vm = Vm::new();
    // `O.constructor` resolves to a non-Object, non-undefined
    // value — spec §10.1.13 step 3 throws TypeError before
    // `@@species` lookup.
    assert!(vm
        .eval(
            "var a = new Uint8Array([1]); \
             Object.defineProperty(a, 'constructor', { value: 42 }); \
             a.map(function(v) { return v; });"
        )
        .unwrap_err()
        .message
        .contains("constructor"));
}

#[test]
fn map_throws_when_species_not_constructor() {
    let mut vm = Vm::new();
    // `Ctor[@@species]` resolves to a callable but non-constructor
    // (arrow functions lack `[[Construct]]`) — spec §10.1.13 step
    // 6/7 throws TypeError.
    assert!(vm
        .eval(
            "function Ctor() {} \
             Object.defineProperty(Ctor, Symbol.species, { value: () => {} }); \
             var a = new Uint8Array([1, 2]); \
             Object.defineProperty(a, 'constructor', { value: Ctor }); \
             a.map(function(v) { return v; });"
        )
        .unwrap_err()
        .message
        .contains("species"));
}

#[test]
fn filter_throws_when_species_not_typed_array_constructor() {
    let mut vm = Vm::new();
    // `Ctor[@@species]` resolves to a constructor that is NOT in
    // the TypedArray subclass chain — our chain-walk bypass can't
    // honour it (would need full `Construct` support), surface
    // TypeError.  Routing through a fresh `Ctor` whose `@@species`
    // is `Object` because `Object` itself doesn't define
    // `@@species` (only TypedArray-family ctors and Array / Map /
    // Set / RegExp / Promise / ArrayBuffer do), so the lookup
    // would otherwise fall through to the default subclass.
    assert!(vm
        .eval(
            "function Ctor() {} \
             Object.defineProperty(Ctor, Symbol.species, { value: Object }); \
             var a = new Uint8Array([1, 2]); \
             Object.defineProperty(a, 'constructor', { value: Ctor }); \
             a.filter(function() { return true; });"
        )
        .unwrap_err()
        .message
        .contains("species"));
}

#[test]
fn hofs_bind_this_arg_in_callback() {
    let mut vm = Vm::new();
    // Spec for each HOF (`%TypedArray%.prototype.{map, filter,
    // findLast, findLastIndex, forEach, every, some, find,
    // findIndex}`) invokes the callback via `Call(callback,
    // thisArg, ⟨...⟩)`, so `this` inside the callback must be
    // the user-supplied `thisArg`.  Object thisArg avoids the
    // non-strict primitive-boxing wrap, so identity comparison
    // is unambiguous.  All four new SP8b HOFs are covered here;
    // existing HOFs (forEach/every/some/find/findIndex) share
    // the same `iterate_with_callback` plumbing so the bind is
    // covered by construction, but exercise them too as a
    // sanity check.
    assert!(eval_bool(
        &mut vm,
        "var marker = { id: 42 }; var ok = true; var n = 0; \
         var check = function() { if (this !== marker) ok = false; n++; return false; }; \
         var a = new Uint8Array([1, 2, 3]); \
         a.map(check, marker); \
         a.filter(check, marker); \
         a.findLast(check, marker); \
         a.findLastIndex(check, marker); \
         a.forEach(check, marker); \
         a.every(function() { if (this !== marker) ok = false; n++; return true; }, marker); \
         a.some(check, marker); \
         a.find(check, marker); \
         a.findIndex(check, marker); \
         ok && n === 27;"
    ));
}

#[test]
fn map_throws_when_constructor_is_null() {
    let mut vm = Vm::new();
    // `null` is non-Object but distinct from `undefined`: spec
    // §10.1.13 SpeciesConstructor step 2 only short-circuits on
    // `undefined` (returning the default constructor), step 3
    // rejects everything else that isn't `Object` — including
    // `null` — as a TypeError.  Pairs with
    // `map_default_species_falls_back_when_constructor_undefined`
    // to cover both arms of the asymmetric early-return.
    assert!(vm
        .eval(
            "var a = new Uint8Array([1, 2]); \
             Object.defineProperty(a, 'constructor', { value: null }); \
             a.map(function(v) { return v; });"
        )
        .unwrap_err()
        .message
        .contains("constructor"));
}

#[test]
fn map_default_species_falls_back_when_constructor_undefined() {
    let mut vm = Vm::new();
    // Shadowing the inherited `Uint8Array.prototype.constructor`
    // with an own data property whose value is `undefined` makes
    // `[[Get]]("constructor")` resolve to `undefined` (the own
    // property wins over the inherited one) — spec §10.1.13
    // SpeciesConstructor step 2 then returns the default
    // constructor (here, `Uint8Array`'s ek).  Note this is
    // distinct from `null`, which spec step 3 rejects as
    // "Type(C) is not Object" → TypeError.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         Object.defineProperty(a, 'constructor', { value: undefined }); \
         var b = a.map(function(v) { return v + 1; }); \
         b instanceof Uint8Array && b.length === 3 && b[0] === 2;"
    ));
}

// ---------------------------------------------------------------------------
// reduce / reduceRight (SP8c-A — accumulator HOFs)
// ---------------------------------------------------------------------------

#[test]
fn reduce_with_initial_value_threads_accumulator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array([1, 2, 3, 4]).reduce(function(acc, v) { return acc + v; }, 100);"
        ),
        110.0
    );
}

#[test]
fn reduce_without_initial_uses_first_element() {
    let mut vm = Vm::new();
    // Without initialValue, acc starts as A[0] and loop visits A[1..].
    // For [10, 20, 30] this yields 10 + 20 + 30 = 60 with one fewer
    // callback invocation than the initial-value case.
    assert_eq!(
        eval_number(
            &mut vm,
            "var n = 0; \
             var s = new Uint8Array([10, 20, 30]).reduce(function(acc, v) { n++; return acc + v; }); \
             s * 100 + n;"
        ),
        // s = 60, n = 2 → 6002
        6002.0
    );
}

#[test]
fn reduce_callback_args_are_acc_value_index_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); var ok = true; \
         a.reduce(function(acc, v, i, arr) { \
             if (arr !== a) ok = false; \
             if (arr[i] !== v) ok = false; \
             return acc; \
         }, 0); \
         ok;"
    ));
}

#[test]
fn reduce_empty_no_initial_throws() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array(0).reduce(function() {});")
        .unwrap_err()
        .message
        .contains("empty TypedArray"));
}

#[test]
fn reduce_empty_with_initial_returns_initial() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "new Uint8Array(0).reduce(function() { throw new Error('cb'); }, 42);"
        ),
        42.0
    );
}

#[test]
fn reduce_throws_when_callback_not_callable() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1]).reduce(undefined, 0);")
        .unwrap_err()
        .message
        .contains("callback is not a function"));
}

#[test]
fn reduce_right_visits_indices_in_reverse() {
    let mut vm = Vm::new();
    // Visiting order: cb(initial, A[2], 2), cb(_, A[1], 1), cb(_, A[0], 0)
    // — record the index sequence to confirm.
    assert_eq!(
        eval_string(
            &mut vm,
            "var visits = []; \
             new Uint8Array([10, 20, 30]).reduceRight(function(acc, v, i) { \
                 visits.push(String(i)); return acc; \
             }, ''); \
             visits.join(',');"
        ),
        "2,1,0"
    );
}

#[test]
fn reduce_right_without_initial_uses_last_element() {
    let mut vm = Vm::new();
    // acc starts as A[len-1] = 30, callback visits A[len-2..0]
    // = [20, 10] → 30 - 20 - 10 = 0; n = 2.
    assert_eq!(
        eval_number(
            &mut vm,
            "var n = 0; \
             var s = new Uint8Array([10, 20, 30]).reduceRight(function(acc, v) { n++; return acc - v; }); \
             s * 100 + n;"
        ),
        2.0  // s = 0 → 0 * 100 + 2 = 2
    );
}

#[test]
fn reduce_right_empty_no_initial_throws() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array(0).reduceRight(function() {});")
        .unwrap_err()
        .message
        .contains("empty TypedArray"));
}

#[test]
fn reduce_object_accumulator_threads_through_iterations() {
    let mut vm = Vm::new();
    // User callbacks can return arbitrary `JsValue::Object`
    // handles for the accumulator; the rooted-stack-slot pattern
    // in `reduce_impl` keeps each intermediate object reachable
    // by the GC scanner across the next `ctx.call_function`
    // boundary.  Even if no GC is currently triggered in the
    // cross-iteration window, this test pins the contract: an
    // object accumulator survives every iteration with all
    // properties intact (last-element id + accumulated sum).
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); \
         var r = a.reduce(function(acc, v) { \
             return { last: v, sum: (acc.sum || 0) + v }; \
         }, { sum: 0 }); \
         r.last === 30 && r.sum === 60;"
    ));
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); \
         var r = a.reduceRight(function(acc, v) { \
             return { last: v, sum: (acc.sum || 0) + v }; \
         }, { sum: 0 }); \
         r.last === 10 && r.sum === 60;"
    ));
}

#[test]
fn reduce_callback_this_is_undefined() {
    let mut vm = Vm::new();
    // Spec passes `undefined` as thisArg; non-strict callback sees
    // the global object (boxed undefined).  Strict callback sees
    // undefined directly.  Use strict mode for unambiguous identity.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); var ok = true; \
         a.reduce(function() { 'use strict'; if (this !== undefined) ok = false; return 0; }, 0); \
         a.reduceRight(function() { 'use strict'; if (this !== undefined) ok = false; return 0; }, 0); \
         ok;"
    ));
}

// ---------------------------------------------------------------------------
// sort (SP8c-A — in-place; default numeric / BigInt or compareFn)
// ---------------------------------------------------------------------------

#[test]
fn sort_default_ascending_numeric() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([3, 1, 4, 1, 5, 9, 2, 6]); \
         a.sort(); \
         a[0] === 1 && a[1] === 1 && a[2] === 2 && a[3] === 3 && \
         a[4] === 4 && a[5] === 5 && a[6] === 6 && a[7] === 9;"
    ));
}

#[test]
fn sort_default_handles_negative_numbers() {
    let mut vm = Vm::new();
    // Int8Array preserves negatives; default sort is true numeric
    // ascending (NOT lexicographic, unlike Array.prototype.sort).
    assert!(eval_bool(
        &mut vm,
        "var a = new Int8Array([5, -10, 0, 3, -1, -100]); \
         a.sort(); \
         a[0] === -100 && a[1] === -10 && a[2] === -1 && \
         a[3] === 0 && a[4] === 3 && a[5] === 5;"
    ));
}

#[test]
fn sort_default_places_nan_at_end_in_float_arrays() {
    let mut vm = Vm::new();
    // Spec TypedArrayElementSortCompare sorts NaN to the end.
    assert!(eval_bool(
        &mut vm,
        "var a = new Float64Array([3, NaN, 1, NaN, 2]); \
         a.sort(); \
         a[0] === 1 && a[1] === 2 && a[2] === 3 && \
         isNaN(a[3]) && isNaN(a[4]);"
    ));
}

#[test]
fn sort_default_bigint_ascending() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array([3n, -10n, 1n, 0n, -1n]); \
         a.sort(); \
         a[0] === -10n && a[1] === -1n && a[2] === 0n && \
         a[3] === 1n && a[4] === 3n;"
    ));
}

#[test]
fn sort_default_biguint_ascending() {
    let mut vm = Vm::new();
    // BigUint64 default sort uses `BigInt::cmp` ordering via the
    // pool lookup — distinct from BigInt64 because the underlying
    // `BigInt` representation is signed, but values stored in a
    // `BigUint64Array` are non-negative by element-coercion.
    assert!(eval_bool(
        &mut vm,
        "var a = new BigUint64Array([3n, 100n, 1n, 0n, 50n]); \
         a.sort(); \
         a[0] === 0n && a[1] === 1n && a[2] === 3n && \
         a[3] === 50n && a[4] === 100n;"
    ));
}

#[test]
fn sort_repeated_on_bigint_array_is_idempotent() {
    let mut vm = Vm::new();
    // Functional regression: repeated `.sort()` on a BigInt
    // typed array stays correct across many cycles.  Pool dedup
    // (`BigIntPool::alloc` short-circuits on existing values)
    // means repeated reads of the same BigInts share `BigIntId`s
    // — verifies the dedup doesn't break sort ordering or
    // element values.  10 cycles × 100 elements proves
    // correctness at scale.
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array(100); var ok = true; \
         for (var i = 0; i < 100; i++) a[i] = BigInt(99 - i); \
         for (var c = 0; c < 10; c++) { \
             a.sort(); \
             if (a[0] !== 0n || a[99] !== 99n) ok = false; \
             a.reverse(); \
             if (a[0] !== 99n || a[99] !== 0n) ok = false; \
         } \
         ok;"
    ));
}

#[test]
fn sort_with_user_compare_fn() {
    let mut vm = Vm::new();
    // Descending sort via user comparefn returning b - a.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([3, 1, 4, 1, 5, 9, 2, 6]); \
         a.sort(function(x, y) { return y - x; }); \
         a[0] === 9 && a[1] === 6 && a[2] === 5 && a[3] === 4 && \
         a[4] === 3 && a[5] === 2 && a[6] === 1 && a[7] === 1;"
    ));
}

#[test]
fn sort_compare_fn_nan_treated_as_zero() {
    let mut vm = Vm::new();
    // Per spec, comparefn returning NaN is treated as 0 (no swap),
    // matching Array.prototype.sort behaviour.  With the all-NaN
    // comparator the array stays unchanged (insertion sort
    // breaks at first non-positive cmp).
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([3, 1, 2]); \
         a.sort(function() { return NaN; }); \
         a[0] === 3 && a[1] === 1 && a[2] === 2;"
    ));
}

#[test]
fn sort_throws_when_compare_fn_not_callable() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1, 2]).sort(42);")
        .unwrap_err()
        .message
        .contains("comparefn"));
    // String comparefn — most-common user mistake (`a.sort('asc')`).
    assert!(vm
        .eval("new Uint8Array([1, 2]).sort('asc');")
        .unwrap_err()
        .message
        .contains("comparefn"));
}

#[test]
fn sort_compare_fn_propagates_throw_atomically() {
    let mut vm = Vm::new();
    // Throwing comparefn surfaces as abrupt completion AND the
    // receiver is left **unchanged** — `native_typed_array_sort`
    // snapshots into a local Vec, sorts there, then writes back
    // only after the sort completes, so an error during the sort
    // means no write-back ever happens.  Spec §23.2.3.29 step 5
    // (`SortIndexedProperties`) → step 7 ordering matches: an
    // abrupt completion in step 5 short-circuits step 7.
    let result = vm.eval(
        "var a = new Uint8Array([3, 1, 2]); var captured = null; \
         try { a.sort(function() { throw new Error('boom'); }); } \
         catch (e) { captured = e; } \
         (captured ? 1 : 0) * 1000 + a[0] * 100 + a[1] * 10 + a[2];",
    );
    // captured = error → 1; receiver bytes unchanged at [3, 1, 2]
    // → 1 * 1000 + 3 * 100 + 1 * 10 + 2 = 1312.
    match result {
        Ok(JsValue::Number(n)) => assert_eq!(n, 1312.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn sort_returns_receiver() {
    let mut vm = Vm::new();
    // Spec returns the original receiver unchanged (after in-place sort).
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([2, 1]); var r = a.sort(); r === a;"
    ));
}

#[test]
fn sort_short_arrays_no_op() {
    let mut vm = Vm::new();
    // len < 2 short-circuits without invoking the comparefn.
    assert!(eval_bool(
        &mut vm,
        "var n = 0; \
         var a = new Uint8Array(0); a.sort(function() { n++; return 0; }); \
         var b = new Uint8Array([42]); b.sort(function() { n++; return 0; }); \
         n === 0 && b[0] === 42;"
    ));
}

#[test]
fn sort_is_stable_for_equal_elements_under_compare_fn() {
    let mut vm = Vm::new();
    // Insertion sort is stable.  Sort by parity (even before odd)
    // and verify within-parity order is preserved.  `a.join` goes
    // through `%TypedArray%.prototype.join` directly — the
    // `Array.prototype.join.call(typedArray, ...)` cross-class
    // borrow doesn't pass our brand-check.
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new Uint8Array([1, 2, 3, 4, 5, 6]); \
             a.sort(function(x, y) { return (x & 1) - (y & 1); }); \
             a.join(',');"
        ),
        "2,4,6,1,3,5"
    );
}
