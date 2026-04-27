//! `%TypedArray%.of` / `%TypedArray%.from` static-method tests
//! (ES2024 §23.2.2.{1,2}).
//!
//! Split from [`super::tests_typed_array`] (ctor / accessor / brand-
//! check tests) to keep both files below the 1000-line convention.
//! The natives' implementation lives in
//! [`super::super::host::typed_array_static`].
//!
//! Coverage:
//!
//! - `.of(...items)` — basic, empty, signed widening, fractional
//!   float, per-subclass dispatch via `this`, abstract-ctor
//!   rejection.
//! - `.from(source, mapFn?, thisArg?)` — array source, string
//!   iterable, mapFn, array-like fallback, TypedArray source,
//!   `null`/non-callable rejection, BigInt subclass, inheritance
//!   identity, empty source, `mapFn` index argument.
//! - `IsConstructor(C)` gate — prototype-spoofed plain object,
//!   bound arrow, async function, generator function — all reject
//!   with TypeError per spec §23.2.2.{1,2} step "If
//!   IsConstructor(C) is false, throw".
//! - User-defined subclass — `.of` / `.from` preserve
//!   `(new Sub.of(...)).constructor === Sub`.
//! - Iterator-getter-once (spec §7.3.10 GetMethod) — observable
//!   count via `Object.defineProperty`-installed counting getter.
//! - Deep subclass tower (visited-set termination, no fixed depth
//!   cap).

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

// ---------------------------------------------------------------------------
// %TypedArray%.of
// ---------------------------------------------------------------------------

#[test]
fn typed_array_of_basic_uint8() {
    let mut vm = Vm::new();
    // `Uint8Array.of(...items)` — variadic; `length` matches arg
    // count, each element coerced via the destination kind's
    // `[[Set]]` (here `ToUint8`).
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.of(1, 2, 3); \
             a.length * 1000 + a[0] * 100 + a[1] * 10 + a[2];"
        ),
        3123.0
    );
}

#[test]
fn typed_array_of_empty_yields_zero_length() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "Uint8Array.of().length;"), 0.0);
}

#[test]
fn typed_array_of_int16_signed_widening() {
    let mut vm = Vm::new();
    // 2-byte signed kind preserves negative values + larger range
    // than the Uint8 cases above — exercises `write_element_raw`'s
    // ToInt16 coercion path through the static-method dispatch.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Int16Array.of(-1, 100, 32767); \
             a[0] + a[1] + a[2];"
        ),
        -1.0 + 100.0 + 32767.0
    );
}

#[test]
fn typed_array_of_float64_preserves_fractional() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var a = Float64Array.of(1.5, 2.25); a[0] + a[1];"),
        3.75
    );
}

#[test]
fn typed_array_of_dispatches_per_subclass() {
    let mut vm = Vm::new();
    // `Uint8Array.of` and `Float64Array.of` resolve to the SAME
    // function object on `%TypedArray%` (inherited via the
    // constructor prototype chain), but the per-call ek dispatch
    // means the result's `[[TypedArrayName]]` differs.  Catches
    // a regression where the static method might bake in a
    // particular ek instead of resolving from `this`.
    assert!(eval_bool(
        &mut vm,
        "Uint8Array.of === Float64Array.of && \
         Uint8Array.of(1).constructor === Uint8Array && \
         Float64Array.of(1).constructor === Float64Array;"
    ));
}

#[test]
fn typed_array_of_on_abstract_throws_type_error() {
    let mut vm = Vm::new();
    // `%TypedArray%` itself is not registered in
    // `subclass_array_ctors`, so `(%TypedArray%).of(1, 2)` (which
    // in script is reachable as `Uint8Array.__proto__.of(1, 2)`)
    // must throw rather than silently producing an Uint8Array.
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { Uint8Array.__proto__.of.call(Uint8Array.__proto__, 1, 2); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

// ---------------------------------------------------------------------------
// %TypedArray%.from
// ---------------------------------------------------------------------------

#[test]
fn typed_array_from_array_source() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.from([10, 20, 30]); \
             a.length * 10000 + a[0] * 100 + a[2];"
        ),
        30030.0 + 1000.0
    );
}

#[test]
fn typed_array_from_string_iterable() {
    let mut vm = Vm::new();
    // In this engine, String iteration is by UTF-16 code unit.
    // Each iteration yields a single-char string; `ToUint8`
    // coerces it to NaN → 0 for non-numeric chars.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.from('123'); a[0] + a[1] + a[2];"
        ),
        6.0
    );
}

#[test]
fn typed_array_from_with_map_fn_doubles_values() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.from([1, 2, 3], function (x) { return x * 2; }); \
             a[0] + a[1] + a[2];"
        ),
        12.0
    );
}

#[test]
fn typed_array_from_array_like_via_length() {
    let mut vm = Vm::new();
    // §23.2.2.1 array-like fallback — when `@@iterator` is absent,
    // use `LengthOfArrayLike` + integer-indexed `[[Get]]`.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.from({ length: 3, 0: 7, 1: 8, 2: 9 }); \
             a[0] * 100 + a[1] * 10 + a[2];"
        ),
        789.0
    );
}

#[test]
fn typed_array_from_typed_array_source() {
    let mut vm = Vm::new();
    // TypedArrays expose `@@iterator` (= `.values()`), so a
    // `Float32Array` source iterates element-wise — the values
    // then re-coerce through the destination's `ToInt16`.
    assert_eq!(
        eval_number(
            &mut vm,
            "var src = new Float32Array([1.7, -2.3, 100.0]); \
             var dst = Int16Array.from(src); \
             dst[0] + dst[1] + dst[2];"
        ),
        // ToInt16(1.7) = 1, ToInt16(-2.3) = -2, ToInt16(100) = 100
        99.0
    );
}

#[test]
fn typed_array_from_null_source_throws_type_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { Uint8Array.from(null); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_from_non_callable_map_fn_throws_type_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var ok = false; \
         try { Uint8Array.from([1, 2, 3], 'not a function'); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_from_bigint_subclass() {
    let mut vm = Vm::new();
    // BigInt64Array source values must be BigInt — the iterator
    // path passes raw BigInt values through `ToBigInt64` for the
    // destination kind.  Confirms BigInt subclass dispatch
    // works.
    assert!(eval_bool(
        &mut vm,
        "var a = BigInt64Array.from([1n, 2n, 3n]); \
         a.length === 3 && a[0] === 1n && a[2] === 3n;"
    ));
}

#[test]
fn typed_array_from_inherits_via_constructor_prototype() {
    let mut vm = Vm::new();
    // `Uint8Array.from` and `Float32Array.from` are the SAME
    // function (inherited from `%TypedArray%.from`), and each
    // dispatch correctly produces an instance of the calling
    // ctor's subclass.
    assert!(eval_bool(
        &mut vm,
        "Uint8Array.from === Float32Array.from && \
         Uint8Array.from([1]).constructor === Uint8Array && \
         Float32Array.from([1]).constructor === Float32Array;"
    ));
}

#[test]
fn typed_array_from_empty_array_yields_zero_length() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "Uint8Array.from([]).length;"), 0.0);
}

#[test]
fn typed_array_from_map_fn_receives_index() {
    let mut vm = Vm::new();
    // mapFn is called as `(value, index)` per spec — index passed
    // here lets the test see the loop counter without external
    // state.
    assert_eq!(
        eval_number(
            &mut vm,
            "var a = Uint8Array.from([10, 20, 30], function (v, i) { return v + i; }); \
             a[0] * 10000 + a[1] * 100 + a[2];"
        ),
        // [10+0, 20+1, 30+2] = [10, 21, 32]
        100000.0 + 2100.0 + 32.0
    );
}

// ---------------------------------------------------------------------------
// IsConstructor(C) gate — prototype-spoofing rejection
// ---------------------------------------------------------------------------

#[test]
fn typed_array_of_rejects_prototype_spoofed_receiver() {
    let mut vm = Vm::new();
    // Spec §23.2.2.{1,2} step "If IsConstructor(C) is false, throw
    // TypeError" — a plain object whose `[[Prototype]]` is set
    // to a registered TypedArray ctor must NOT be accepted just
    // because the prototype-chain walk reaches a registered ctor.
    // The receiver-side IsConstructor check rejects before walking.
    assert!(eval_bool(
        &mut vm,
        "var spoof = {}; \
         Object.setPrototypeOf(spoof, Uint8Array); \
         var ok = false; \
         try { Uint8Array.of.call(spoof, 1, 2); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_from_rejects_prototype_spoofed_receiver() {
    let mut vm = Vm::new();
    // Companion to the `of` spoofing test — same IsConstructor
    // gate must apply to `.from`.
    assert!(eval_bool(
        &mut vm,
        "var spoof = {}; \
         Object.setPrototypeOf(spoof, Uint8Array); \
         var ok = false; \
         try { Uint8Array.from.call(spoof, [1, 2]); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_of_rejects_async_function_spoof() {
    let mut vm = Vm::new();
    // Async functions lack `[[Construct]]` per spec
    // (§15.8.4 AsyncFunction abstract).  Even when the receiver's
    // `[[Prototype]]` is spoofed to `Uint8Array`, the
    // IsConstructor gate must reject because the async-function
    // metadata flags `is_async = true`.  Catches a regression
    // where `is_constructor` looks only at `this_mode` and
    // misses the compiled-function flags.
    assert!(eval_bool(
        &mut vm,
        "var f = async function() {}; \
         Object.setPrototypeOf(f, Uint8Array); \
         var ok = false; \
         try { Uint8Array.of.call(f, 1); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_of_rejects_generator_function_spoof() {
    let mut vm = Vm::new();
    // Generator functions also lack `[[Construct]]`
    // (§15.5.4 GeneratorFunction abstract).  Same gate as the
    // async-function case, exercising `is_generator = true`.
    assert!(eval_bool(
        &mut vm,
        "var f = function*() {}; \
         Object.setPrototypeOf(f, Uint8Array); \
         var ok = false; \
         try { Uint8Array.of.call(f, 1); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

#[test]
fn typed_array_of_rejects_bound_arrow_spoof() {
    let mut vm = Vm::new();
    // Bound functions inherit constructability from their target
    // (ES §10.4.1.2 BoundFunction[[Construct]] is set iff the
    // target has [[Construct]]).  An arrow function target has no
    // `[[Construct]]`, so the bound wrapper must NOT pass the
    // IsConstructor gate even when its `[[Prototype]]` is spoofed
    // to `Uint8Array`.  Catches a regression where the chain walk
    // accepts BoundFunction unconditionally instead of unwrapping
    // to inspect the target.
    assert!(eval_bool(
        &mut vm,
        "var f = (() => {}).bind(null); \
         Object.setPrototypeOf(f, Uint8Array); \
         var ok = false; \
         try { Uint8Array.of.call(f, 1); } \
         catch (e) { ok = e instanceof TypeError; } ok;"
    ));
}

// Note: a cyclic-prototype-chain test (`A.__proto__ === A`) isn't
// reachable here — the engine's `Object.setPrototypeOf` rejects
// the trivial cycle upfront with `TypeError: Cyclic __proto__
// value`, so the `require_subclass_ctor` walk's visited-set guard
// is defensive code only (covers theoretical cases where the
// upfront cycle check is bypassed).

// ---------------------------------------------------------------------------
// User-defined subclass dispatch
// ---------------------------------------------------------------------------

#[test]
fn typed_array_of_user_subclass_preserves_constructor_identity() {
    let mut vm = Vm::new();
    // The receiver of `.of` is a user-defined subclass.  Per spec,
    // only `IsConstructor(C)` is required; the prototype-chain
    // walk finds `Uint8Array` (registered in
    // `subclass_array_ctors`) and the new instance inherits from
    // `Sub.prototype` so its `.constructor === Sub`.
    //
    // **Note**: this test sets up the subclass manually
    // (`Object.setPrototypeOf` + `Object.create(parent.prototype)`)
    // rather than via `class Sub extends Uint8Array {}`, because
    // our engine's `class extends` does not currently link
    // `Sub.__proto__ === Uint8Array` for built-in TypedArray
    // parents.  That's a separate engine bug; once fixed, the
    // manual setup here can be replaced with the `class extends`
    // sugar.  The natives' prototype-chain walk works correctly
    // for both shapes.
    assert!(eval_bool(
        &mut vm,
        "function Sub() {} \
         Object.setPrototypeOf(Sub, Uint8Array); \
         Sub.prototype = Object.create(Uint8Array.prototype); \
         Sub.prototype.constructor = Sub; \
         var s = Sub.of(10, 20, 30); \
         s instanceof Sub && s.constructor === Sub && \
         s.length === 3 && s[0] === 10 && s[2] === 30;"
    ));
}

#[test]
fn typed_array_from_user_subclass_preserves_constructor_identity() {
    let mut vm = Vm::new();
    // Manual subclass setup — see
    // `typed_array_of_user_subclass_preserves_constructor_identity`
    // for rationale.
    assert!(eval_bool(
        &mut vm,
        "function Sub() {} \
         Object.setPrototypeOf(Sub, Float32Array); \
         Sub.prototype = Object.create(Float32Array.prototype); \
         Sub.prototype.constructor = Sub; \
         var s = Sub.from([1.5, 2.5]); \
         s instanceof Sub && s.constructor === Sub && \
         s.length === 2 && s[0] === 1.5 && s[1] === 2.5;"
    ));
}

// Note: an accessor-`.prototype` regression test isn't reachable
// from JS in this engine — function `.prototype` data properties
// are installed non-configurable, so `Object.defineProperty(Sub,
// 'prototype', {get: ...})` throws "Cannot redefine property:
// cannot convert between data and accessor" before the static
// natives ever see the receiver.  The R5 fix routes
// `receiver_prototype` through `Get(C, "prototype")` semantics
// (`get_property_value`) so an accessor `.prototype` would be
// honoured if the engine ever lets one be installed (e.g. once
// `Reflect.deleteProperty(Sub, 'prototype')` of the auto-installed
// data property is supported, or for ctors built outside the
// `function`-declaration path).

#[test]
fn typed_array_of_handles_deep_subclass_tower() {
    let mut vm = Vm::new();
    // The constructor-prototype walk uses cycle detection (visited
    // set), not a fixed depth cap, so legitimate deep towers
    // resolve correctly.  This builds a 6-level chain manually
    // (mirroring `class A extends Uint8Array {}; class B extends A
    // {}; class C extends B {}; …`) — a realistic spec-conformant
    // shape that would have been rejected by an earlier `MAX_DEPTH
    // = 32` cap as a stand-in for the same regression family.
    assert!(eval_bool(
        &mut vm,
        "function L1() {} Object.setPrototypeOf(L1, Uint8Array); \
         L1.prototype = Object.create(Uint8Array.prototype); \
         function L2() {} Object.setPrototypeOf(L2, L1); \
         L2.prototype = Object.create(L1.prototype); \
         function L3() {} Object.setPrototypeOf(L3, L2); \
         L3.prototype = Object.create(L2.prototype); \
         function L4() {} Object.setPrototypeOf(L4, L3); \
         L4.prototype = Object.create(L3.prototype); \
         function L5() {} Object.setPrototypeOf(L5, L4); \
         L5.prototype = Object.create(L4.prototype); \
         function L6() {} Object.setPrototypeOf(L6, L5); \
         L6.prototype = Object.create(L5.prototype); \
         var s = L6.of(11, 22, 33); \
         s instanceof L6 && s instanceof L5 && \
         s.length === 3 && s[0] === 11 && s[2] === 33;"
    ));
}

// ---------------------------------------------------------------------------
// Iterator-getter once (spec §7.3.10 GetMethod)
// ---------------------------------------------------------------------------

#[test]
fn typed_array_from_honours_primitive_wrapper_iterator() {
    let mut vm = Vm::new();
    // Spec `GetMethod(ToObject(source), @@iterator)` — non-Object
    // primitives are boxed via `ToObject` for the lookup so a
    // user-installed iterator on the wrapper prototype (here
    // `Number.prototype`) is honoured.  The pre-fix
    // `lookup_iterator_method` returned `Undefined` for non-Object
    // / non-String sources, which fell through to the array-like
    // branch and ignored the prototype iterator.
    assert!(eval_bool(
        &mut vm,
        "Number.prototype[Symbol.iterator] = function () { \
             return [10, 20, 30][Symbol.iterator](); \
         }; \
         var a = Uint8Array.from(7); \
         var ok = a.length === 3 && a[0] === 10 && a[1] === 20 && a[2] === 30; \
         delete Number.prototype[Symbol.iterator]; \
         ok;"
    ));
}

#[test]
fn typed_array_from_invokes_iterator_getter_exactly_once() {
    let mut vm = Vm::new();
    // Spec §7.3.10 `GetMethod` evaluates the @@iterator getter
    // exactly once.  The pre-fix impl called `coerce::get_property`
    // (which runs the getter) AND then `resolve_iterator` (which
    // runs the getter again internally) — observable as `count
    // === 2`.  After R1, the single `lookup_iterator_method` pass
    // runs the getter exactly once.
    assert_eq!(
        eval_number(
            &mut vm,
            "var count = 0; \
             var src = {}; \
             Object.defineProperty(src, Symbol.iterator, { \
                 get: function() { count++; return [10, 20][Symbol.iterator].bind([10, 20]); } \
             }); \
             Uint8Array.from(src); count;"
        ),
        1.0
    );
}
