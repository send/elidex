//! `%TypedArray%.prototype` higher-order method tests
//! (ES2024 §23.2.3): `forEach` / `every` / `some` / `find` /
//! `findIndex` / `findLast` / `findLastIndex` / `map` / `filter` /
//! `flatMap` plus the species-resolution and `thisArg`-binding
//! contracts that the species-allocating HOFs (`map` / `filter` /
//! `flatMap`) share.
//!
//! Split from sibling [`super::tests_typed_array_methods`]
//! (basic prototype methods) and
//! [`super::tests_typed_array_reduce_sort`] (accumulator +
//! in-place HOFs) so each file stays well below the project's
//! 1000-line convention.
//!
//! HOF *implementation* lives in
//! [`crate::vm::host::typed_array_hof`]; this module exercises
//! the spec-observable behaviour through the JS surface.

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
// flatMap (SP8c-B — collect-then-species-allocate, splices inner TypedArrays)
// ---------------------------------------------------------------------------

#[test]
fn flat_map_singleton_callbacks_keep_each_value() {
    let mut vm = Vm::new();
    // Callback returning a non-TypedArray is treated as a singleton
    // (no flatten), so `flatMap` collapses into the same shape as
    // `map` for this case.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var b = a.flatMap(function(v) { return v * 2; }); \
         b instanceof Uint8Array && b.length === 3 && \
         b[0] === 2 && b[1] === 4 && b[2] === 6;"
    ));
}

#[test]
fn flat_map_typed_array_callback_splices_inner_elements() {
    let mut vm = Vm::new();
    // Callback returning a TypedArray gets every element spliced
    // into the destination — same as `Array.prototype.flatMap`'s
    // Array-flatten case but TypedArray-only.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var b = a.flatMap(function(v) { return new Uint8Array([v, v * 10]); }); \
         b.length === 6 && b[0] === 1 && b[1] === 10 && \
         b[2] === 2 && b[3] === 20 && b[4] === 3 && b[5] === 30;"
    ));
}

#[test]
fn flat_map_mixed_singleton_and_splice() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var b = a.flatMap(function(v, i) { \
             return i === 1 ? new Uint8Array([20, 21, 22]) : v; \
         }); \
         b.length === 5 && b[0] === 1 && \
         b[1] === 20 && b[2] === 21 && b[3] === 22 && b[4] === 3;"
    ));
}

#[test]
fn flat_map_empty_inner_array_skips_index() {
    let mut vm = Vm::new();
    // An empty inner TypedArray contributes zero elements;
    // result is the same as filtering those indices out.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2, 3]); \
         var b = a.flatMap(function(v) { \
             return v === 2 ? new Uint8Array(0) : new Uint8Array([v]); \
         }); \
         b.length === 2 && b[0] === 1 && b[1] === 3;"
    ));
}

#[test]
fn flat_map_callback_receives_index_and_array() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([10, 20, 30]); var ok = true; \
         var b = a.flatMap(function(v, i, arr) { \
             if (arr !== a) ok = false; if (arr[i] !== v) ok = false; return v; \
         }); \
         ok && b.length === 3;"
    ));
}

#[test]
fn flat_map_on_empty_returns_empty_view() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var b = new Uint8Array(0).flatMap(function() { return [1, 2]; }); \
         b instanceof Uint8Array && b.length === 0;"
    ));
}

#[test]
fn flat_map_throws_when_callback_not_callable() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1]).flatMap('nope');")
        .unwrap_err()
        .message
        .contains("callback is not a function"));
}

#[test]
fn flat_map_propagates_callback_error() {
    let mut vm = Vm::new();
    assert!(vm
        .eval("new Uint8Array([1, 2, 3]).flatMap(function() { throw new Error('boom'); });")
        .is_err());
}

#[test]
fn flat_map_bigint_array_keeps_through_alloc_point() {
    let mut vm = Vm::new();
    // BigInt elements allocate fresh BigIntId on every read; the
    // collect frame must keep them rooted across the destination
    // view's `create_typed_array_for_length` allocation.  Same
    // GC hazard as `filter`'s collect path.
    assert!(eval_bool(
        &mut vm,
        "var a = new BigInt64Array([1n, 2n, 3n]); \
         var b = a.flatMap(function(v) { return new BigInt64Array([v, v * 2n]); }); \
         b instanceof BigInt64Array && b.length === 6 && \
         b[0] === 1n && b[1] === 2n && b[2] === 2n && b[3] === 4n && \
         b[4] === 3n && b[5] === 6n;"
    ));
}

#[test]
fn flat_map_user_subclass_preserves_species() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "function Sub() {} \
         Object.setPrototypeOf(Sub, Uint8Array); \
         Sub.prototype = Object.create(Uint8Array.prototype); \
         Sub.prototype.constructor = Sub; \
         var s = Sub.from([1, 2]); \
         var f = s.flatMap(function(v) { return new Uint8Array([v, v + 1]); }); \
         f instanceof Sub && f.length === 4 && \
         f[0] === 1 && f[1] === 2 && f[2] === 2 && f[3] === 3;"
    ));
}

#[test]
fn flat_map_non_typed_array_object_treated_as_singleton() {
    let mut vm = Vm::new();
    // Callback returning any non-TypedArray (here a plain Object,
    // but the same applies to Array literals) does NOT splice —
    // it's pushed as a singleton, then per-element coercion
    // through `write_element_raw` runs ToNumber on the object
    // which falls back to NaN → 0 for `Uint8Array`.  This matches
    // the documented "splice only for TypedArray" rule.
    assert!(eval_bool(
        &mut vm,
        "var a = new Uint8Array([1, 2]); \
         var b = a.flatMap(function() { return {}; }); \
         b instanceof Uint8Array && b.length === 2 && \
         b[0] === 0 && b[1] === 0;"
    ));
}

#[test]
fn flat_map_binds_this_arg() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var marker = { id: 7 }; var ok = true; \
         new Uint8Array([1, 2]).flatMap(function() { \
             if (this !== marker) ok = false; return 0; \
         }, marker); \
         ok;"
    ));
}
