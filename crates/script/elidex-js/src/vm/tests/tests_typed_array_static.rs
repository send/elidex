//! `%TypedArray%.of` / `%TypedArray%.from` static-method tests
//! (ECMA-262 §23.2.2.{1,2}).
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
//! - The tests call Sub.of(...) and Sub.from(...) with a manually linked constructor chain and
//!   verify the returned instance’s constructor.
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
    // Of step 6.b writes via TypedArraySetElement. This test observes Uint8 values and the
    // result length. (%TypedArray%.of §23.2.2.2.)
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
    // The VM string iterator combines a valid surrogate pair into one element
    // (natives_symbol.rs:403–411). This test uses ASCII digits, which convert to numbers
    // totaling 6.
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
    // obtain the length and read each index property from the object returned by ToObject.
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
    // `BigInt64Array.from([1n, 2n, 3n])` copies BigInt values into a BigInt64Array. Other
    // ToBigInt-convertible inputs are possible.
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
        100_000.0 + 2100.0 + 32.0
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
    // The async function expression has no `[[Construct]]`. Spoofing its prototype leaves the
    // IsConstructor check in of step 3 false. §15.8.4 describes evaluation of an async
    // function body.
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
    // The generator function expression created here lacks `[[Construct]]`. Cite
    // InstantiateGeneratorFunctionExpression §15.5.4 and the IsConstructor check in of step 3.
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
    // BoundFunctionCreate step 6 installs `[[Construct]]` only when the target has it. Binding
    // an arrow and spoofing its prototype leaves it nonconstructable. (IsConstructor §7.2.4.)
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
    // The VM accepts this manually linked constructor chain and installs its object-valued
    // prototype. The test checks the resulting identity. Spec of also requires the constructed
    // result to pass TypedArrayCreateFromConstructor validation.
    //
    // The test explicitly links Sub to Uint8Array and installs Sub.prototype.constructor. It
    // verifies this linked chain. Class-extends construction linkage is a separate issue.
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

// The non-configurable prototype on this function cannot be converted to an accessor or
// removed with Reflect.deleteProperty. The helper uses Get and propagates errors.

#[test]
fn typed_array_of_handles_deep_subclass_tower() {
    let mut vm = Vm::new();
    // The walk tracks visited constructors and has no depth cap; this six-level chain
    // exercises multi-level resolution.
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
    // GetMethod(source, %Symbol.iterator%) uses GetV. GetV boxes the primitive for lookup and
    // preserves the source as receiver.
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
fn typed_array_of_accepts_bound_typed_array_constructor() {
    let mut vm = Vm::new();
    // TypedArrayCreateFromConstructor step 1 calls Construct. Bound `[[Construct]]` delegates
    // to the target. The VM unwraps bound targets to find the element kind.
    // (TypedArrayCreateFromConstructor §23.2.4.1.)
    //
    // Use .of.call(Bound, ...). BoundFunctionCreate steps 1 and 4 obtain and install the
    // target’s prototype. The wrapper in this VM does not inherit that prototype.
    // (BoundFunctionCreate §10.4.1.3.)
    assert!(eval_bool(
        &mut vm,
        "var Bound = Uint8Array.bind(null); \
         var a = Uint8Array.of.call(Bound, 10, 20, 30); \
         a.length === 3 && a[0] === 10 && a[2] === 30 && \
         a instanceof Uint8Array;"
    ));
}

#[test]
fn typed_array_from_accepts_bound_typed_array_constructor() {
    let mut vm = Vm::new();
    // Companion to the `of` bound-ctor test — `from` must also
    // accept a bound TypedArray constructor receiver via the
    // BoundFunction unwrap.  Same `.from.call(...)` workaround
    // for the engine's bind-prototype gap.
    assert!(eval_bool(
        &mut vm,
        "var Bound = Float32Array.bind(null); \
         var a = Float32Array.from.call(Bound, [1.5, 2.5, 3.5]); \
         a.length === 3 && a[0] === 1.5 && a[2] === 3.5 && \
         a instanceof Float32Array;"
    ));
}

#[test]
fn typed_array_from_array_like_allocates_before_indexed_get() {
    let mut vm = Vm::new();
    // From step 10 calls TypedArrayCreateFromConstructor before step 12 indexed access and
    // mapping. The VM rejects the u32 byte product for this Float64Array, and the test checks
    // RangeError with zero indexed getter calls. (%TypedArray%.from §23.2.2.1.)
    assert!(eval_bool(
        &mut vm,
        "var hits = 0; \
         var src = { length: 0x40000000 }; \
         Object.defineProperty(src, 0, { \
             get: function () { hits++; throw new Error('indexed getter ran'); } \
         }); \
         var ok = false; \
         try { Float64Array.from(src); } \
         catch (e) { ok = e instanceof RangeError; } \
         ok && hits === 0;"
    ));
}

#[test]
fn typed_array_from_iterator_receives_source_as_this() {
    let mut vm = Vm::new();
    // Spec §7.4.3 GetIteratorFromMethod calls the iterator
    // method with `source` as the receiver — `Call(method,
    // source)`.  In this VM, `function () {}` literals default
    // to strict-mode `this` binding (no auto-boxing via
    // `OrdinaryCallBindThis`), so a primitive `source = 7`
    // surfaces as `typeof this === "number"` inside the
    // installed iterator.  Catches a regression where the
    // iterator method is called with `Undefined` / `Null` / a
    // boxed wrapper instead of the original primitive `source`.
    assert!(eval_bool(
        &mut vm,
        "var seen_this = null; \
         Number.prototype[Symbol.iterator] = function () { \
             seen_this = this; \
             return [10, 20, 30][Symbol.iterator](); \
         }; \
         var a = Uint8Array.from(7); \
         var ok = seen_this === 7 && a.length === 3 && a[0] === 10 \
                  && a[1] === 20 && a[2] === 30; \
         delete Number.prototype[Symbol.iterator]; \
         ok;"
    ));
}

#[test]
fn typed_array_from_invokes_iterator_getter_exactly_once() {
    let mut vm = Vm::new();
    // The getter runs once when resolve_property resolves the result of the property search.
    // The test observes count === 1.
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
