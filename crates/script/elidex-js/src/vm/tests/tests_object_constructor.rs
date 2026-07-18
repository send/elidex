//! ECMA-262 §20.1.1 Object constructor + §20.1.3.6 builtinTag — the JS-observable
//! surface the historic namespace-object `Object` shape hid.
//!
//! Re-diagnosis of `#11-vm-native-fn-generic-invocation`: global `Object` was a
//! non-callable `ObjectKind::Ordinary` namespace object (via
//! `create_object_with_methods`) with no `.prototype` forward-link and no
//! `Object.prototype.constructor` back-link, so `Object.prototype` evaluated to
//! `undefined` and `Object.prototype.toString.call(x)` threw "Cannot convert
//! undefined or null to object" (ToObject(undefined), §7.1.19). The fix routes
//! `Object` through `wire_constructor_global` like every sibling constructor.

#![cfg(feature = "engine")]

use super::helpers::{eval_bool, eval_string};

#[test]
fn object_is_a_callable_constructor() {
    assert!(eval_bool("typeof Object === 'function'"));
    assert!(eval_bool("new Object() instanceof Object"));
    // §20.1.1.1 step 2: call/construct with a nullish value → a fresh object.
    assert!(eval_bool("typeof Object() === 'object'"));
    assert!(eval_bool("typeof Object(null) === 'object'"));
    assert!(eval_bool("typeof Object(undefined) === 'object'"));
    // §20.1.1.1 step 3: Object(primitive) / new Object(primitive) → wrapper.
    assert!(eval_bool("typeof Object(5) === 'object'"));
    assert!(eval_bool("(new Object(5)) instanceof Number"));
    assert!(eval_bool("Object('s') instanceof String"));
    // §20.2.4.2: `name` is installed by create_native_function_keyed; `length`
    // is intentionally NOT installed (uniform with every sibling ctor).
    assert_eq!(eval_string("Object.name"), "Object");
}

#[test]
fn object_prototype_and_constructor_links() {
    // The global Object.prototype is reachable and IS {}'s [[Prototype]].
    assert!(eval_bool("Object.prototype === Object.getPrototypeOf({})"));
    assert!(eval_bool(
        "Object.getPrototypeOf(Object.prototype) === null"
    ));
    // §20.1.2.21 / §20.1.3.1 back-link.
    assert!(eval_bool("Object.prototype.constructor === Object"));
    assert!(eval_bool(
        "Object.getPrototypeOf({}).constructor === Object"
    ));
    // Generic access to Object.prototype members now works (was throwing because
    // `Object.prototype` evaluated to undefined).
    assert!(eval_bool("typeof Object.prototype.toString === 'function'"));
    assert!(eval_bool(
        "typeof Object.prototype.hasOwnProperty === 'function'"
    ));
    assert!(eval_bool(
        "Object.prototype.hasOwnProperty.call({a: 1}, 'a')"
    ));
}

#[test]
fn object_statics_still_work() {
    // Regression guard: the static methods that were the whole of the old
    // namespace-object shape still resolve on the constructor.
    assert!(eval_bool("Object.keys({a: 1}).length === 1"));
    assert!(eval_bool("typeof Object.assign === 'function'"));
    assert!(eval_bool("typeof Object.getPrototypeOf === 'function'"));
    assert!(eval_bool("typeof Object.defineProperty === 'function'"));
}

#[test]
fn generic_object_prototype_tostring_builtin_tags() {
    // §20.1.3.6: Object.prototype.toString.call(x) reachable through the
    // global-Object spelling, and the full builtinTag match is covered.
    assert_eq!(
        eval_string("Object.prototype.toString.call(new Date())"),
        "[object Date]" // step 12
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call([])"),
        "[object Array]" // step 5
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call(new Number(5))"),
        "[object Number]" // step 10
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call(new String('x'))"),
        "[object String]" // step 11
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call(new Boolean(true))"),
        "[object Boolean]" // step 9
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call({})"),
        "[object Object]" // step 14 default
    );
    // step 6: an arguments object carries a [[ParameterMap]] slot even in a
    // strict-only VM (§10.4.4.6 steps 2-3), so it tags "Arguments".
    assert_eq!(
        eval_string("Object.prototype.toString.call((function () { return arguments; })())"),
        "[object Arguments]"
    );
    // Existing Function arm still correct (regression).
    assert_eq!(
        eval_string("Object.prototype.toString.call(function () {})"),
        "[object Function]" // step 7
    );
    // NB: `new Error('e')` tags "[object Object]", NOT "[object Error]", because
    // a user-constructed Error is an ordinary object on `Error.prototype`, not
    // `ObjectKind::Error` (only the throw path at `ops.rs` produces that kind).
    // That Error-object dual-representation gap is pre-existing and out of this
    // builtin-tag slice's scope (step 8, not the wrapper/Arguments residual);
    // tracked as a candidate slot in this PR's plan-memo §7.
    assert_eq!(
        eval_string("Object.prototype.toString.call(new Error('e'))"),
        "[object Object]"
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call(/re/)"),
        "[object RegExp]" // step 13
    );
}

#[test]
fn object_prototype_tostring_generic_invocation_spellings() {
    // The generic-invocation spellings `#11-vm-native-fn-generic-invocation`
    // deferred: assigned own-property, `.call`, `.apply` all reach the builtin.
    assert_eq!(
        eval_string("var f = Object.prototype.toString; f.call(new Date())"),
        "[object Date]"
    );
    assert_eq!(
        eval_string("Object.prototype.toString.apply(new Date())"),
        "[object Date]"
    );
    assert_eq!(
        eval_string("var o = {}; o.ts = Object.prototype.toString; o.ts()"),
        "[object Object]"
    );
}

#[test]
fn subclass_new_returns_subclass_instance() {
    // §20.1.1.1 step 1: a subclass `new` (NewTarget ≠ %Object%, distinguished via
    // the `object_constructor` intrinsic id compare — Option A) returns the
    // subclass instance and ignores the value argument.
    assert!(eval_bool("class X extends Object {}; new X() instanceof X"));
    assert!(eval_bool(
        "class X extends Object {}; new X(5) instanceof Object"
    ));
    assert!(eval_bool(
        "class X extends Object {}; (new X(5)).constructor === X"
    ));
    // Contrast: `new Object(5)` (NewTarget === %Object%) falls to step 3 → wrapper,
    // NOT a plain Object instance — the id compare is what separates the two.
    assert!(eval_bool("(new Object(5)) instanceof Number"));
}
