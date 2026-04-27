//! `%TypedArray%.prototype.{reduce, reduceRight, sort}` tests
//! (ES2024 §23.2.3.23/.24/.29).  Accumulator HOFs and the
//! in-place sort kept together because they share the
//! "atomic-on-throw" + GC-rooted-accumulator contracts that
//! Copilot review iteratively hardened during SP8c-A
//! (PR #123).
//!
//! Split from sibling [`super::tests_typed_array_methods`]
//! (basic prototype methods) and
//! [`super::tests_typed_array_hof`] (forward / reverse
//! short-circuit HOFs + species-allocating HOFs) so each file
//! stays well below the project's 1000-line convention.
//!
//! Implementation: see
//! [`crate::vm::host::typed_array_hof::reduce_impl`] +
//! [`crate::vm::host::typed_array_hof::native_typed_array_sort`]
//! plus [`crate::vm::pools::BigIntPool::alloc`]'s value-dedup
//! contract (the latter ensures repeated `.sort()` on BigInt
//! arrays doesn't grow the pool unboundedly).

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
fn sort_validates_compare_fn_before_brand_check() {
    let mut vm = Vm::new();
    // Spec §23.2.3.29 step 1 validates `comparefn` BEFORE the
    // receiver brand-check, so `Uint8Array.prototype.sort.call({},
    // 42)` must throw the "comparefn must be a function" TypeError
    // — NOT the "called on non-TypedArray" brand error.  Easy to
    // regress during refactors that move the brand-check earlier;
    // pin the ordering with this regression test.
    let err = vm
        .eval("Uint8Array.prototype.sort.call({}, 42);")
        .unwrap_err();
    assert!(
        err.message.contains("comparefn"),
        "expected comparefn error, got: {}",
        err.message
    );
    assert!(
        !err.message.contains("non-TypedArray"),
        "comparefn validation should fire BEFORE brand-check, got: {}",
        err.message
    );
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
