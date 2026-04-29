//! `%TypedArray%.prototype` non-HOF method tests (ES2024 §23.2.3):
//! `fill` / `slice` / `subarray` / `set` / `copyWithin` /
//! `reverse` / `at` / `join` / `indexOf` / `lastIndexOf` /
//! `includes` / `keys` / `values` / `entries` / `@@iterator` /
//! `toString`.
//!
//! Higher-order method tests (forward / reverse / species-allocating
//! HOFs) live in [`super::tests_typed_array_hof`]; accumulator
//! and in-place sort HOFs in
//! [`super::tests_typed_array_reduce_sort`] — the 3-way split
//! keeps each file under the project's 1000-line convention as
//! the HOF surface grows.
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

// ---------------------------------------------------------------------------
// toLocaleString (SP8c-B — no-Intl, mirrors Array.prototype.toLocaleString)
// ---------------------------------------------------------------------------

#[test]
fn to_locale_string_basic_numeric() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Uint8Array([1, 2, 3]).toLocaleString();"),
        "1,2,3"
    );
}

#[test]
fn to_locale_string_empty_array_returns_empty_string() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Uint8Array(0).toLocaleString();"),
        ""
    );
}

#[test]
fn to_locale_string_single_element_no_separator() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Float32Array([3.5]).toLocaleString();"),
        "3.5"
    );
}

#[test]
fn to_locale_string_bigint_array_strips_n_suffix() {
    let mut vm = Vm::new();
    // BigInt.prototype.toLocaleString → toString shape: "1,2,3"
    // (no `n` suffix per BigInt.prototype.toString contract).
    assert_eq!(
        eval_string(&mut vm, "new BigInt64Array([1n, 2n, 3n]).toLocaleString();"),
        "1,2,3"
    );
}

#[test]
fn to_locale_string_locale_args_ignored() {
    let mut vm = Vm::new();
    // No Intl support — locale + options arguments are accepted
    // but ignored, so the result matches the no-arg form.
    assert_eq!(
        eval_string(
            &mut vm,
            "new Uint8Array([1, 2]).toLocaleString('de-DE', { useGrouping: false });"
        ),
        "1,2"
    );
}

#[test]
fn to_locale_string_observes_number_prototype_override() {
    let mut vm = Vm::new();
    // §23.2.3.31 Invoke(elem, "toLocaleString") → finds the
    // user override on Number.prototype because the call's GetV
    // boxes the primitive for property lookup.  The override
    // sees `this === <primitive>` (after non-strict boxing) and
    // can read `.valueOf()`.
    assert_eq!(
        eval_string(
            &mut vm,
            "Number.prototype.toLocaleString = function() { return '<' + this.valueOf() + '>'; }; \
             new Uint8Array([1, 2, 3]).toLocaleString();"
        ),
        "<1>,<2>,<3>"
    );
}

#[test]
fn to_locale_string_throws_on_non_typed_array_receiver() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("Uint8Array.prototype.toLocaleString.call({});")
        .unwrap_err()
        .message
        .contains("non-TypedArray"));
}

#[test]
fn to_locale_string_propagates_throw_from_per_element_method() {
    let mut vm = Vm::new();
    assert!(vm
        .eval(
            "Number.prototype.toLocaleString = function() { throw new Error('boom'); }; \
             new Uint8Array([1]).toLocaleString();"
        )
        .is_err());
}

#[test]
fn to_locale_string_forwards_reserved_args() {
    let mut vm = Vm::new();
    // §23.2.3.31 step 7: per-element `Invoke(elem, "toLocaleString",
    // « locales, options »)` must forward the reserved args to user
    // overrides.  Capture the args via an override that joins them
    // (no Intl in elidex, so the built-in shim ignores them — but
    // overrides MUST observe them).
    assert_eq!(
        eval_string(
            &mut vm,
            "Number.prototype.toLocaleString = function(loc, opt) { \
                 return String(loc) + ':' + (opt && opt.tag); \
             }; \
             new Uint8Array([1, 2]).toLocaleString('de-DE', { tag: 'X' });"
        ),
        "de-DE:X,de-DE:X"
    );
}

#[test]
fn to_locale_string_passes_exactly_two_args_to_override() {
    let mut vm = Vm::new();
    // §23.2.3.31 step 7: `Invoke(elem, "toLocaleString",
    // « locales, options »)` always materialises a 2-element
    // arg list.  Extra caller args (3rd+) MUST NOT reach the
    // override; missing args (0 / 1) MUST be undefined-padded
    // so `arguments.length === 2` always holds.
    assert_eq!(
        eval_string(
            &mut vm,
            "Number.prototype.toLocaleString = function() { return String(arguments.length); }; \
             new Uint8Array([1]).toLocaleString('a', 'b', 'c', 'd');"
        ),
        "2"
    );
    assert_eq!(
        eval_string(
            &mut vm,
            "Number.prototype.toLocaleString = function() { return String(arguments.length); }; \
             new Uint8Array([1]).toLocaleString();"
        ),
        "2"
    );
}

#[test]
fn to_locale_string_preserves_lone_surrogate_from_override() {
    let mut vm = Vm::new();
    // WTF-16 round-trip — a user override returning `'\uD800'`
    // (a lone high surrogate) must survive accumulation intact.
    // The lossy `StringPool::get_utf8` path would replace it with
    // U+FFFD; the `Vec<u16>` + `intern_utf16` shape preserves it.
    // Length 3 = U+D800 + ',' + U+D800 → 3 UTF-16 code units.
    assert_eq!(
        eval_number(
            &mut vm,
            "Number.prototype.toLocaleString = function() { return '\\uD800'; }; \
             new Uint8Array([1, 2]).toLocaleString().length;"
        ),
        3.0
    );
    // charCodeAt(0) confirms the surrogate is preserved (not the
    // U+FFFD replacement that lossy UTF-8 round-trip would yield).
    assert_eq!(
        eval_number(
            &mut vm,
            "Number.prototype.toLocaleString = function() { return '\\uD800'; }; \
             new Uint8Array([1]).toLocaleString().charCodeAt(0);"
        ),
        f64::from(0xD800_u32)
    );
}

#[test]
fn to_locale_string_accessor_getter_sees_primitive_receiver() {
    let mut vm = Vm::new();
    // §7.3.2 GetV(V, P): when `toLocaleString` resolves through
    // an accessor getter on the prototype chain, the getter
    // receives the *original* primitive value as `this`, not the
    // throw-away wrapper used for the prototype-chain lookup.
    // Pre-R6 the `try_get_property_value` path passed the wrapper
    // as receiver — observable in non-strict mode by capturing
    // `typeof this`, which boxes back to "object" for the wrapper
    // but stays "object" for primitive→box-on-non-strict-call too.
    // Strict-mode getter is the cleanest way to observe the
    // primitive: `'use strict'` keeps the receiver unboxed, so
    // `typeof this === 'number'` confirms primitive identity.
    assert_eq!(
        eval_string(
            &mut vm,
            "Object.defineProperty(Number.prototype, 'toLocaleString', { \
                 configurable: true, \
                 get: function() { 'use strict'; var t = this; return function() { return typeof t; }; } \
             }); \
             new Uint8Array([1]).toLocaleString();"
        ),
        "number"
    );
}

#[test]
fn to_locale_string_throws_on_non_callable_method() {
    let mut vm = Vm::new();
    // Per `Invoke` semantics (§7.3.16) a present-but-non-callable
    // `toLocaleString` is a TypeError, not a silent fallback to
    // `ToString(receiver)` — the prior behaviour masked user
    // mistakes like the assignment below.
    assert!(vm
        .eval(
            "Number.prototype.toLocaleString = 42; \
             new Uint8Array([1]).toLocaleString();"
        )
        .unwrap_err()
        .message
        .contains("not callable"));
}
