//! Tests for Array.prototype methods and Array static methods (ES2020 §22.1).

use super::{eval_bool, eval_number, eval_string, eval_throws};

// ---------------------------------------------------------------------------
// push / pop
// ---------------------------------------------------------------------------

#[test]
fn array_push_basic() {
    assert_eq!(eval_number("var a = [1,2]; a.push(3); a.length;"), 3.0);
}

#[test]
fn array_push_returns_new_length() {
    assert_eq!(eval_number("var a = []; a.push(1, 2, 3);"), 3.0);
}

#[test]
fn array_push_multiple() {
    assert_eq!(
        eval_number("var a = [10]; a.push(20, 30); a[1] + a[2];"),
        50.0
    );
}

#[test]
fn array_pop_basic() {
    assert_eq!(eval_number("var a = [1, 2, 3]; a.pop();"), 3.0);
}

#[test]
fn array_pop_empty() {
    assert_eq!(eval_string("var a = []; typeof a.pop();"), "undefined");
}

#[test]
fn array_pop_reduces_length() {
    assert_eq!(eval_number("var a = [1,2,3]; a.pop(); a.length;"), 2.0);
}

// ---------------------------------------------------------------------------
// shift / unshift
// ---------------------------------------------------------------------------

#[test]
fn array_shift_basic() {
    assert_eq!(eval_number("var a = [10, 20, 30]; a.shift();"), 10.0);
}

#[test]
fn array_shift_reduces_length() {
    assert_eq!(eval_number("var a = [1,2,3]; a.shift(); a.length;"), 2.0);
}

#[test]
fn array_shift_empty() {
    assert_eq!(eval_string("typeof [].shift();"), "undefined");
}

#[test]
fn array_unshift_basic() {
    assert_eq!(eval_number("var a = [2,3]; a.unshift(1);"), 3.0);
}

#[test]
fn array_unshift_multiple() {
    assert_eq!(
        eval_number("var a = [3]; a.unshift(1, 2); a[0] + a[1] + a[2];"),
        6.0
    );
}

// ---------------------------------------------------------------------------
// reverse
// ---------------------------------------------------------------------------

#[test]
fn array_reverse_basic() {
    assert_eq!(
        eval_string("var a = [1,2,3]; a.reverse(); a.join(',');"),
        "3,2,1"
    );
}

#[test]
fn array_reverse_returns_this() {
    assert_eq!(eval_string("[3,1,2].reverse().join(',');"), "2,1,3");
}

// ---------------------------------------------------------------------------
// sort
// ---------------------------------------------------------------------------

#[test]
fn array_sort_default_string() {
    assert_eq!(eval_string("[3, 1, 2].sort().join(',');"), "1,2,3");
}

#[test]
fn array_sort_string_order() {
    // Default sort is lexicographic.
    assert_eq!(eval_string("[10, 9, 1].sort().join(',');"), "1,10,9");
}

#[test]
fn array_sort_with_compare_fn() {
    assert_eq!(
        eval_string("[10, 9, 1].sort(function(a, b) { return a - b; }).join(',');"),
        "1,9,10"
    );
}

#[test]
fn array_sort_holes_to_end() {
    assert_eq!(
        eval_number("var a = Array(5); a[0] = 3; a[2] = 1; a[4] = 2; a.sort(); a[0];"),
        1.0
    );
}

#[test]
fn array_sort_stable() {
    // Stability: equal elements maintain relative order.
    assert_eq!(
        eval_string(
            "var a = [{k:1,v:'a'},{k:1,v:'b'},{k:1,v:'c'}]; \
             a.sort(function(x,y) { return x.k - y.k; }); \
             a[0].v + a[1].v + a[2].v;"
        ),
        "abc"
    );
}

// ---------------------------------------------------------------------------
// splice
// ---------------------------------------------------------------------------

#[test]
fn array_splice_delete() {
    assert_eq!(
        eval_string("var a = [1,2,3,4]; a.splice(1, 2); a.join(',');"),
        "1,4"
    );
}

#[test]
fn array_splice_returns_removed() {
    assert_eq!(
        eval_string("var a = [1,2,3,4]; a.splice(1, 2).join(',');"),
        "2,3"
    );
}

#[test]
fn array_splice_insert() {
    assert_eq!(
        eval_string("var a = [1,4]; a.splice(1, 0, 2, 3); a.join(',');"),
        "1,2,3,4"
    );
}

#[test]
fn array_splice_replace() {
    assert_eq!(
        eval_string("var a = [1,2,3]; a.splice(1, 1, 'x'); a.join(',');"),
        "1,x,3"
    );
}

#[test]
fn array_splice_negative_start() {
    assert_eq!(
        eval_string("var a = [1,2,3,4,5]; a.splice(-2, 1); a.join(',');"),
        "1,2,3,5"
    );
}

// ---------------------------------------------------------------------------
// fill
// ---------------------------------------------------------------------------

#[test]
fn array_fill_basic() {
    assert_eq!(eval_string("[1,2,3].fill(0).join(',');"), "0,0,0");
}

#[test]
fn array_fill_range() {
    assert_eq!(eval_string("[1,2,3,4].fill(0, 1, 3).join(',');"), "1,0,0,4");
}

#[test]
fn array_fill_negative() {
    assert_eq!(eval_string("[1,2,3,4].fill(0, -2).join(',');"), "1,2,0,0");
}

// ---------------------------------------------------------------------------
// copyWithin
// ---------------------------------------------------------------------------

#[test]
fn array_copy_within_basic() {
    assert_eq!(
        eval_string("[1,2,3,4,5].copyWithin(0, 3).join(',');"),
        "4,5,3,4,5"
    );
}

#[test]
fn array_copy_within_with_end() {
    assert_eq!(
        eval_string("[1,2,3,4,5].copyWithin(1, 3, 4).join(',');"),
        "1,4,3,4,5"
    );
}

// ---------------------------------------------------------------------------
// slice
// ---------------------------------------------------------------------------

#[test]
fn array_slice_basic() {
    assert_eq!(eval_string("[1,2,3,4,5].slice(1, 3).join(',');"), "2,3");
}

#[test]
fn array_slice_no_args() {
    assert_eq!(eval_number("[1,2,3].slice().length;"), 3.0);
}

#[test]
fn array_slice_negative() {
    assert_eq!(eval_string("[1,2,3,4,5].slice(-2).join(',');"), "4,5");
}

// ---------------------------------------------------------------------------
// concat
// ---------------------------------------------------------------------------

#[test]
fn array_concat_basic() {
    assert_eq!(eval_string("[1,2].concat([3,4]).join(',');"), "1,2,3,4");
}

#[test]
fn array_concat_non_array() {
    assert_eq!(eval_string("[1].concat(2, 3).join(',');"), "1,2,3");
}

#[test]
fn array_concat_multiple_arrays() {
    assert_eq!(eval_string("[1].concat([2], [3,4]).join(',');"), "1,2,3,4");
}

// ---------------------------------------------------------------------------
// join
// ---------------------------------------------------------------------------

#[test]
fn array_join_default() {
    assert_eq!(eval_string("[1,2,3].join();"), "1,2,3");
}

#[test]
fn array_join_custom_separator() {
    assert_eq!(eval_string("[1,2,3].join(' - ');"), "1 - 2 - 3");
}

#[test]
fn array_join_empty_array() {
    assert_eq!(eval_string("[].join(',');"), "");
}

#[test]
fn array_join_holes_empty_string() {
    // Holes should produce empty strings in join.
    assert_eq!(
        eval_string("var a = Array(3); a[1] = 'x'; a.join(',');"),
        ",x,"
    );
}

// ---------------------------------------------------------------------------
// indexOf / lastIndexOf / includes
// ---------------------------------------------------------------------------

#[test]
fn array_index_of_basic() {
    assert_eq!(eval_number("[1,2,3,2].indexOf(2);"), 1.0);
}

#[test]
fn array_index_of_not_found() {
    assert_eq!(eval_number("[1,2,3].indexOf(4);"), -1.0);
}

#[test]
fn array_index_of_from_index() {
    assert_eq!(eval_number("[1,2,3,2].indexOf(2, 2);"), 3.0);
}

#[test]
fn array_last_index_of_basic() {
    assert_eq!(eval_number("[1,2,3,2].lastIndexOf(2);"), 3.0);
}

#[test]
fn array_last_index_of_from_index() {
    assert_eq!(eval_number("[1,2,3,2].lastIndexOf(2, 2);"), 1.0);
}

#[test]
fn array_includes_basic() {
    assert!(eval_bool("[1,2,3].includes(2);"));
    assert!(!eval_bool("[1,2,3].includes(4);"));
}

#[test]
fn array_includes_nan() {
    // SameValueZero: NaN == NaN
    assert!(eval_bool("[1, NaN, 3].includes(NaN);"));
}

#[test]
fn array_includes_from_index() {
    assert!(!eval_bool("[1,2,3].includes(1, 1);"));
}

// ---------------------------------------------------------------------------
// toString / toLocaleString
// ---------------------------------------------------------------------------

#[test]
fn array_to_string() {
    assert_eq!(eval_string("[1,2,3].toString();"), "1,2,3");
}

#[test]
fn array_to_locale_string() {
    assert_eq!(eval_string("[1,2,3].toLocaleString();"), "1,2,3");
}

#[test]
fn array_to_locale_string_forwards_reserved_args() {
    // §22.1.3.30 step 6: per-element `Invoke(elem, "toLocaleString",
    // « locales, options »)` must forward the reserved args.
    assert_eq!(
        eval_string(
            "Number.prototype.toLocaleString = function(loc, opt) { \
                 return String(loc) + ':' + (opt && opt.tag); \
             }; \
             [1, 2].toLocaleString('de-DE', { tag: 'X' });"
        ),
        "de-DE:X,de-DE:X"
    );
}

#[test]
fn array_to_locale_string_throws_on_non_callable_method() {
    // Spec `Invoke` (§7.3.16) throws TypeError when the resolved
    // method is non-callable; silent ToString fallback would mask
    // the user mistake.
    let result = super::eval(
        "Number.prototype.toLocaleString = 42; \
         [1].toLocaleString();",
    );
    assert!(result.unwrap_err().message.contains("not callable"));
}

// ---------------------------------------------------------------------------
// forEach
// ---------------------------------------------------------------------------

#[test]
fn array_for_each_basic() {
    assert_eq!(
        eval_number("var s = 0; [1,2,3].forEach(function(v) { s += v; }); s;"),
        6.0
    );
}

#[test]
fn array_for_each_index() {
    assert_eq!(
        eval_number("var s = 0; [10,20,30].forEach(function(v, i) { s += i; }); s;"),
        3.0
    );
}

#[test]
fn array_for_each_holes_skipped() {
    assert_eq!(
        eval_number(
            "var c = 0; var a = Array(5); a[1] = 1; a[3] = 3; a.forEach(function() { c++; }); c;"
        ),
        2.0
    );
}

// ---------------------------------------------------------------------------
// map
// ---------------------------------------------------------------------------

#[test]
fn array_map_basic() {
    assert_eq!(
        eval_string("[1,2,3].map(function(v) { return v * 2; }).join(',');"),
        "2,4,6"
    );
}

#[test]
fn array_map_index() {
    assert_eq!(
        eval_string("[10,20,30].map(function(v, i) { return i; }).join(',');"),
        "0,1,2"
    );
}

#[test]
fn array_map_holes_preserved() {
    // Holes in source should be holes in result.
    assert_eq!(
        eval_number("var a = Array(3); a[0] = 1; a[2] = 3; var r = a.map(function(v) { return v * 2; }); r.length;"),
        3.0
    );
}

// ---------------------------------------------------------------------------
// filter
// ---------------------------------------------------------------------------

#[test]
fn array_filter_basic() {
    assert_eq!(
        eval_string("[1,2,3,4,5].filter(function(v) { return v > 2; }).join(',');"),
        "3,4,5"
    );
}

#[test]
fn array_filter_empty_result() {
    assert_eq!(
        eval_number("[1,2,3].filter(function(v) { return v > 10; }).length;"),
        0.0
    );
}

// ---------------------------------------------------------------------------
// every / some
// ---------------------------------------------------------------------------

#[test]
fn array_every_true() {
    assert!(eval_bool(
        "[2,4,6].every(function(v) { return v % 2 === 0; });"
    ));
}

#[test]
fn array_every_false() {
    assert!(!eval_bool(
        "[2,3,6].every(function(v) { return v % 2 === 0; });"
    ));
}

#[test]
fn array_every_empty() {
    // every on empty array returns true (vacuous truth).
    assert!(eval_bool("[].every(function() { return false; });"));
}

#[test]
fn array_some_true() {
    assert!(eval_bool("[1,2,3].some(function(v) { return v === 2; });"));
}

#[test]
fn array_some_false() {
    assert!(!eval_bool("[1,2,3].some(function(v) { return v > 10; });"));
}

#[test]
fn array_some_empty() {
    assert!(!eval_bool("[].some(function() { return true; });"));
}

// ---------------------------------------------------------------------------
// reduce / reduceRight
// ---------------------------------------------------------------------------

#[test]
fn array_reduce_sum() {
    assert_eq!(
        eval_number("[1,2,3,4].reduce(function(acc, v) { return acc + v; });"),
        10.0
    );
}

#[test]
fn array_reduce_with_initial() {
    assert_eq!(
        eval_number("[1,2,3].reduce(function(acc, v) { return acc + v; }, 10);"),
        16.0
    );
}

#[test]
fn array_reduce_empty_with_initial() {
    assert_eq!(
        eval_number("[].reduce(function(acc, v) { return acc + v; }, 42);"),
        42.0
    );
}

#[test]
fn array_reduce_empty_no_initial_throws() {
    eval_throws("[].reduce(function(acc, v) { return acc + v; });");
}

#[test]
fn array_reduce_right_basic() {
    assert_eq!(
        eval_string("[1,2,3].reduceRight(function(acc, v) { return acc + ',' + v; });"),
        "3,2,1"
    );
}

#[test]
fn array_reduce_right_with_initial() {
    assert_eq!(
        eval_number("[1,2,3].reduceRight(function(acc, v) { return acc + v; }, 0);"),
        6.0
    );
}

#[test]
fn array_reduce_object_accumulator_threads_through_iterations() {
    // User callbacks can return arbitrary `JsValue::Object` handles
    // for the accumulator; the rooted-stack-slot pattern in
    // `native_array_reduce` keeps each intermediate object reachable
    // by the GC scanner across the next `ctx.call_function`
    // boundary.  Pinning contract: object accumulator survives every
    // iteration with all properties intact (last + sum).  Pairs with
    // `reduceRight` to exercise both the forward and reverse rooted
    // loops.  Same-shape regression as
    // `tests_typed_array_methods::reduce_object_accumulator_*`.
    assert!(eval_bool(
        "var r = [10, 20, 30].reduce(function(acc, v) { \
             return { last: v, sum: (acc.sum || 0) + v }; \
         }, { sum: 0 }); \
         r.last === 30 && r.sum === 60;"
    ));
    assert!(eval_bool(
        "var r = [10, 20, 30].reduceRight(function(acc, v) { \
             return { last: v, sum: (acc.sum || 0) + v }; \
         }, { sum: 0 }); \
         r.last === 10 && r.sum === 60;"
    ));
}

// ---------------------------------------------------------------------------
// find / findIndex
// ---------------------------------------------------------------------------

#[test]
fn array_find_basic() {
    assert_eq!(
        eval_number("[1,2,3,4].find(function(v) { return v > 2; });"),
        3.0
    );
}

#[test]
fn array_find_not_found() {
    assert_eq!(
        eval_string("typeof [1,2,3].find(function(v) { return v > 10; });"),
        "undefined"
    );
}

#[test]
fn array_find_index_basic() {
    assert_eq!(
        eval_number("[1,2,3,4].findIndex(function(v) { return v > 2; });"),
        2.0
    );
}

#[test]
fn array_find_index_not_found() {
    assert_eq!(
        eval_number("[1,2,3].findIndex(function(v) { return v > 10; });"),
        -1.0
    );
}

// ---------------------------------------------------------------------------
// flat / flatMap
// ---------------------------------------------------------------------------

#[test]
fn array_flat_basic() {
    assert_eq!(eval_string("[1, [2, 3], [4]].flat().join(',');"), "1,2,3,4");
}

#[test]
fn array_flat_depth_0() {
    // flat(0) is a structure copy — no flattening.
    assert_eq!(eval_number("[1, [2, 3]].flat(0).length;"), 2.0);
}

#[test]
fn array_flat_deep() {
    // 3 levels of nesting requires flat(3) to fully flatten.
    assert_eq!(
        eval_string("[1, [2, [3, [4]]]].flat(3).join(',');"),
        "1,2,3,4"
    );
}

#[test]
fn array_flat_infinity() {
    assert_eq!(
        eval_string("[1, [2, [3, [4]]]].flat(Infinity).join(',');"),
        "1,2,3,4"
    );
}

#[test]
fn array_flat_map_basic() {
    assert_eq!(
        eval_string("[1, 2, 3].flatMap(function(v) { return [v, v * 2]; }).join(',');"),
        "1,2,2,4,3,6"
    );
}

#[test]
fn array_flat_map_non_array_return() {
    assert_eq!(
        eval_string("[1, 2, 3].flatMap(function(v) { return v * 2; }).join(',');"),
        "2,4,6"
    );
}

// ---------------------------------------------------------------------------
// entries / keys / values
// ---------------------------------------------------------------------------

#[test]
fn array_keys_basic() {
    assert_eq!(
        eval_number("var s = 0; for (var k of [10, 20, 30].keys()) { s += k; } s;"),
        3.0 // 0+1+2
    );
}

#[test]
fn array_entries_basic() {
    assert_eq!(
        eval_number(
            "var s = 0; for (var e of [10, 20, 30].entries()) { s += e[0] * 100 + e[1]; } s;"
        ),
        360.0 // 0*100+10 + 1*100+20 + 2*100+30 = 10+120+230=360
    );
}

#[test]
fn array_values_is_iterator() {
    assert_eq!(
        eval_number("var s = 0; for (var v of [10, 20, 30].values()) { s += v; } s;"),
        60.0
    );
}
