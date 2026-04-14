//! Extended tests for Array.prototype — Array.from/of, sparse interactions,
//! edge cases, callback propagation, thisArg binding, and return types.

use super::{eval_bool, eval_global_string, eval_number, eval_string, eval_throws};

// ---------------------------------------------------------------------------
// Array.from / Array.of
// ---------------------------------------------------------------------------

#[test]
fn array_from_array() {
    assert_eq!(eval_string("Array.from([1,2,3]).join(',');"), "1,2,3");
}

#[test]
fn array_from_string() {
    assert_eq!(eval_string("Array.from('abc').join(',');"), "a,b,c");
}

#[test]
fn array_from_with_map() {
    assert_eq!(
        eval_string("Array.from([1,2,3], function(v) { return v * 2; }).join(',');"),
        "2,4,6"
    );
}

#[test]
fn array_from_map_throw_closes_inner_iterator() {
    // §7.4.6: `Array.from(iter, mapFn)` where `mapFn` throws must call
    // `IteratorClose` on the inner iterator before propagating.
    // Observable via a hand-rolled iterator whose `.return()` records
    // it was called.  Regression for the `drain_iterator` abrupt path
    // that previously abandoned the iterator on callback throw.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             var iter = { \
               next() { return { value: 1, done: false }; }, \
               return() { globalThis.log += 'closed,'; return { done: true }; }, \
               [Symbol.iterator]() { return this; } \
             }; \
             try { Array.from(iter, function() { throw 'boom'; }); } \
             catch(e) { globalThis.log += 'caught:' + e; }",
            "log"
        ),
        "closed,caught:boom"
    );
}

#[test]
fn array_of_basic() {
    assert_eq!(eval_string("Array.of(1, 2, 3).join(',');"), "1,2,3");
}

#[test]
fn array_of_single() {
    // Array.of(3) creates [3], not Array(3) with 3 empty slots.
    assert_eq!(eval_number("Array.of(3).length;"), 1.0);
    assert_eq!(eval_number("Array.of(3)[0];"), 3.0);
}

// ---------------------------------------------------------------------------
// constructor property
// ---------------------------------------------------------------------------

#[test]
fn array_constructor_property() {
    assert!(eval_bool("[].constructor === Array;"));
}

// ---------------------------------------------------------------------------
// Sparse array interactions
// ---------------------------------------------------------------------------

#[test]
fn array_push_on_sparse() {
    assert_eq!(eval_number("var a = Array(3); a.push(1); a.length;"), 4.0);
}

#[test]
fn array_pop_sparse_hole() {
    // Popping a hole returns undefined.
    assert_eq!(eval_string("typeof Array(3).pop();"), "undefined");
}

#[test]
fn array_reduce_skips_holes() {
    assert_eq!(
        eval_number(
            "var a = Array(5); a[1] = 10; a[3] = 20; a.reduce(function(s,v) { return s + v; });"
        ),
        30.0
    );
}

#[test]
fn array_index_of_skips_holes() {
    assert_eq!(
        eval_number("var a = Array(3); a[2] = undefined; a.indexOf(undefined);"),
        2.0
    );
}

#[test]
fn array_join_null_undefined() {
    assert_eq!(eval_string("[1, null, undefined, 2].join(',');"), "1,,,2");
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn array_slice_start_gt_end() {
    assert_eq!(eval_number("[1,2,3].slice(2, 1).length;"), 0.0);
}

#[test]
fn array_for_each_returns_undefined() {
    assert_eq!(
        eval_string("typeof [1].forEach(function() {});"),
        "undefined"
    );
}

#[test]
fn array_map_callback_receives_array() {
    // Third argument to callback should be the array.
    assert!(eval_bool(
        "var arr = [1]; arr.map(function(v, i, a) { return a === arr; })[0];"
    ));
}

#[test]
fn array_filter_this_arg() {
    assert_eq!(
        eval_string(
            "var ctx = {min: 2}; [1,2,3].filter(function(v) { return v >= this.min; }, ctx).join(',');"
        ),
        "2,3"
    );
}

#[test]
fn array_sort_returns_this() {
    assert!(eval_bool("var a = [3,1,2]; a.sort() === a;"));
}

#[test]
fn array_flat_holes_skipped() {
    // Holes in nested arrays should be skipped during flattening.
    assert_eq!(
        eval_number("var a = Array(3); a[0] = [1]; a[2] = [2]; a.flat().length;"),
        2.0
    );
}

#[test]
fn array_from_empty() {
    assert_eq!(eval_number("Array.from([]).length;"), 0.0);
}

#[test]
fn array_concat_preserves_holes() {
    // Holes in concat sources should be preserved.
    assert_eq!(
        eval_number("var a = Array(2); a[0] = 1; var b = a.concat([2]); b.length;"),
        3.0
    );
}

#[test]
fn array_some_short_circuits() {
    assert_eq!(
        eval_number("var c = 0; [1,2,3].some(function(v) { c++; return v === 2; }); c;"),
        2.0
    );
}

#[test]
fn array_every_short_circuits() {
    assert_eq!(
        eval_number("var c = 0; [1,2,3].every(function(v) { c++; return v < 2; }); c;"),
        2.0
    );
}

#[test]
fn array_reduce_all_holes_no_initial_throws() {
    eval_throws("Array(5).reduce(function(a, b) { return a + b; });");
}

#[test]
fn array_fill_returns_this() {
    assert!(eval_bool("var a = [1,2]; a.fill(0) === a;"));
}

#[test]
fn array_copy_within_returns_this() {
    assert!(eval_bool("var a = [1,2,3]; a.copyWithin(0, 1) === a;"));
}

#[test]
fn array_reverse_returns_same_ref() {
    assert!(eval_bool("var a = [1,2]; a.reverse() === a;"));
}

// ===========================================================================
// Additional coverage: empty arrays
// ===========================================================================

#[test]
fn array_reverse_empty() {
    assert_eq!(eval_number("[].reverse().length;"), 0.0);
}

#[test]
fn array_sort_empty() {
    assert_eq!(eval_number("[].sort().length;"), 0.0);
}

#[test]
fn array_splice_empty() {
    assert_eq!(eval_number("[].splice(0).length;"), 0.0);
}

#[test]
fn array_map_empty() {
    assert_eq!(
        eval_number("[].map(function(v) { return v; }).length;"),
        0.0
    );
}

#[test]
fn array_filter_empty() {
    assert_eq!(
        eval_number("[].filter(function() { return true; }).length;"),
        0.0
    );
}

#[test]
fn array_find_empty() {
    assert_eq!(
        eval_string("typeof [].find(function() { return true; });"),
        "undefined"
    );
}

#[test]
fn array_find_index_empty() {
    assert_eq!(
        eval_number("[].findIndex(function() { return true; });"),
        -1.0
    );
}

#[test]
fn array_flat_empty() {
    assert_eq!(eval_number("[].flat().length;"), 0.0);
}

#[test]
fn array_flat_map_empty() {
    assert_eq!(
        eval_number("[].flatMap(function(v) { return [v]; }).length;"),
        0.0
    );
}

// ===========================================================================
// Single-element edge cases
// ===========================================================================

#[test]
fn array_reduce_single_no_initial() {
    assert_eq!(
        eval_number("[42].reduce(function(a, b) { return a + b; });"),
        42.0
    );
}

#[test]
fn array_reduce_right_single_no_initial() {
    assert_eq!(
        eval_number("[42].reduceRight(function(a, b) { return a + b; });"),
        42.0
    );
}

#[test]
fn array_sort_single() {
    assert_eq!(eval_number("[7].sort()[0];"), 7.0);
}

#[test]
fn array_every_single_pass() {
    assert!(eval_bool("[5].every(function(v) { return v > 0; });"));
}

#[test]
fn array_some_single_fail() {
    assert!(!eval_bool("[5].some(function(v) { return v > 10; });"));
}

// ===========================================================================
// Callback exception propagation
// ===========================================================================

#[test]
fn array_for_each_callback_throws() {
    eval_throws("[1,2,3].forEach(function() { throw new Error('boom'); });");
}

#[test]
fn array_map_callback_throws() {
    eval_throws("[1,2,3].map(function() { throw new Error('boom'); });");
}

#[test]
fn array_filter_callback_throws() {
    eval_throws("[1,2,3].filter(function() { throw new Error('boom'); });");
}

#[test]
fn array_reduce_callback_throws() {
    eval_throws("[1,2,3].reduce(function() { throw new Error('boom'); });");
}

#[test]
fn array_sort_comparefn_throws() {
    eval_throws("[3,1,2].sort(function() { throw new Error('boom'); });");
}

// ===========================================================================
// thisArg binding
// ===========================================================================

#[test]
fn array_for_each_this_arg() {
    assert_eq!(
        eval_number("var ctx = {s: 0}; [1,2,3].forEach(function(v) { this.s += v; }, ctx); ctx.s;"),
        6.0
    );
}

#[test]
fn array_map_this_arg() {
    assert_eq!(
        eval_string(
            "var ctx = {m: 10}; [1,2,3].map(function(v) { return v * this.m; }, ctx).join(',');"
        ),
        "10,20,30"
    );
}

#[test]
fn array_every_this_arg() {
    assert!(eval_bool(
        "var ctx = {min: 0}; [1,2,3].every(function(v) { return v > this.min; }, ctx);"
    ));
}

#[test]
fn array_some_this_arg() {
    assert!(eval_bool(
        "var ctx = {target: 2}; [1,2,3].some(function(v) { return v === this.target; }, ctx);"
    ));
}

#[test]
fn array_find_this_arg() {
    assert_eq!(
        eval_number(
            "var ctx = {min: 2}; [1,2,3].find(function(v) { return v >= this.min; }, ctx);"
        ),
        2.0
    );
}

#[test]
fn array_find_index_this_arg() {
    assert_eq!(
        eval_number(
            "var ctx = {min: 2}; [1,2,3].findIndex(function(v) { return v >= this.min; }, ctx);"
        ),
        1.0
    );
}

// ===========================================================================
// Sparse array interactions (deeper)
// ===========================================================================

#[test]
fn array_slice_sparse() {
    assert_eq!(
        eval_number("var a = Array(5); a[1] = 10; a[3] = 30; a.slice(1, 4).length;"),
        3.0
    );
}

#[test]
fn array_reverse_sparse() {
    assert_eq!(
        eval_number("var a = Array(3); a[0] = 1; a.reverse(); a[2];"),
        1.0
    );
}

#[test]
fn array_fill_overwrites_holes() {
    assert_eq!(
        eval_string("var a = Array(3); a.fill(0).join(',');"),
        "0,0,0"
    );
}

#[test]
fn array_splice_sparse() {
    assert_eq!(
        eval_number("var a = Array(5); a[2] = 99; a.splice(1, 3).length;"),
        3.0
    );
}

// ===========================================================================
// Sort edge cases
// ===========================================================================

#[test]
fn array_sort_comparefn_returns_zero() {
    // All equal → should preserve original order (stable).
    assert_eq!(
        eval_string("[3,1,2].sort(function() { return 0; }).join(',');"),
        "3,1,2"
    );
}

#[test]
fn array_sort_comparefn_returns_nan() {
    // NaN treated as 0 (equal) → should preserve original order.
    assert_eq!(
        eval_string("[3,1,2].sort(function() { return NaN; }).join(',');"),
        "3,1,2"
    );
}

// ===========================================================================
// indexOf / lastIndexOf / includes edge cases
// ===========================================================================

#[test]
fn array_index_of_nan_not_found() {
    // indexOf uses strict equality: NaN !== NaN.
    assert_eq!(eval_number("[NaN].indexOf(NaN);"), -1.0);
}

#[test]
fn array_index_of_negative_from_index() {
    assert_eq!(eval_number("[1,2,3,2,1].indexOf(2, -3);"), 3.0);
}

#[test]
fn array_last_index_of_negative_from_index() {
    assert_eq!(eval_number("[1,2,3,2,1].lastIndexOf(2, -2);"), 3.0);
}

#[test]
fn array_includes_negative_from_index() {
    assert!(eval_bool("[1,2,3].includes(2, -2);"));
    assert!(!eval_bool("[1,2,3].includes(1, -1);"));
}

// ===========================================================================
// copyWithin edge cases
// ===========================================================================

#[test]
fn array_copy_within_overlap_forward() {
    assert_eq!(
        eval_string("[1,2,3,4,5].copyWithin(1, 0, 3).join(',');"),
        "1,1,2,3,5"
    );
}

#[test]
fn array_copy_within_negative() {
    assert_eq!(
        eval_string("[1,2,3,4,5].copyWithin(-2, 0, 2).join(',');"),
        "1,2,3,1,2"
    );
}

// ===========================================================================
// Array.from edge cases
// ===========================================================================

#[test]
fn array_from_array_like() {
    assert_eq!(
        eval_string("Array.from({length: 3, 0: 'a', 1: 'b', 2: 'c'}).join(',');"),
        "a,b,c"
    );
}

#[test]
fn array_from_no_iterable_no_length() {
    assert_eq!(eval_number("Array.from({}).length;"), 0.0);
}

// ===========================================================================
// flat depth edge cases
// ===========================================================================

#[test]
fn array_flat_negative_depth() {
    // Negative depth → flat(0) → structure copy.
    assert_eq!(eval_number("[1, [2]].flat(-1).length;"), 2.0);
}

#[test]
fn array_flat_nan_depth() {
    assert_eq!(eval_number("[1, [2]].flat(NaN).length;"), 2.0);
}

// ===========================================================================
// Return type verification
// ===========================================================================

#[test]
fn array_reduce_returns_accumulator() {
    assert_eq!(
        eval_string("[1,2,3].reduce(function(a, b) { return a + ',' + b; });"),
        "1,2,3"
    );
}

#[test]
fn array_find_returns_element_not_index() {
    assert_eq!(
        eval_number("[{v:1},{v:2}].find(function(o) { return o.v === 2; }).v;"),
        2.0
    );
}

#[test]
fn array_flat_returns_new_array() {
    assert!(eval_bool("var a = [[1]]; a.flat() !== a;"));
}

#[test]
fn array_map_returns_new_array() {
    assert!(eval_bool(
        "var a = [1]; a.map(function(v) { return v; }) !== a;"
    ));
}

#[test]
fn array_filter_returns_new_array() {
    assert!(eval_bool(
        "var a = [1]; a.filter(function() { return true; }) !== a;"
    ));
}

#[test]
fn array_concat_returns_new_array() {
    assert!(eval_bool("var a = [1]; a.concat([2]) !== a;"));
}

#[test]
fn array_slice_returns_new_array() {
    assert!(eval_bool("var a = [1,2]; a.slice() !== a;"));
}
