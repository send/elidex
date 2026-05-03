//! Spec coverage for ES §7.1.1 ToPrimitive / §7.1.1.1 OrdinaryToPrimitive
//! on plain Objects.  Slot #10.7 replaces the prior `"[object Object]"` /
//! `NaN` shortcuts in `coerce::to_string` / `coerce::to_number` with a
//! full §7.1.1.1 walk through `valueOf` / `toString`, so the same tests
//! that previously documented the limitation now assert spec-correct
//! behaviour.

use super::{eval_bool, eval_number, eval_string, eval_throws};

// -- Hint=string method order: toString first, then valueOf -----------------

#[test]
fn hint_string_prefers_to_string() {
    // §7.1.1.1 step 1: hint "string" tries ["toString", "valueOf"].
    assert_eq!(
        eval_string(
            "String({ toString() { return 'T'; },
                      valueOf()  { return 'V'; } });"
        ),
        "T"
    );
}

#[test]
fn hint_string_falls_back_to_value_of_when_to_string_returns_object() {
    // First method returns Object → continue to second method.
    assert_eq!(
        eval_string(
            "String({ toString() { return {}; },
                      valueOf()  { return 'V'; } });"
        ),
        "V"
    );
}

// -- Hint=number method order: valueOf first, then toString -----------------

#[test]
fn hint_number_prefers_value_of() {
    // §7.1.1.1 step 1: hint "number" tries ["valueOf", "toString"].
    // Unary `+` runs ToNumber which routes through ToPrimitive(hint="number").
    assert_eq!(
        eval_number(
            "+{ toString() { return '1'; },
                valueOf()  { return 2; } };"
        ),
        2.0
    );
}

#[test]
fn hint_number_falls_back_to_to_string_when_value_of_returns_object() {
    // First method returns Object → continue to second method (returns "x"),
    // then ToNumber("x") = NaN.
    assert!(eval_bool(
        "var n = +{ valueOf()  { return {}; },
                    toString() { return 'x'; } };
         Number.isNaN(n);"
    ));
}

// -- Hint=default method order matches "number" -----------------------------

#[test]
fn hint_default_uses_value_of_first() {
    // `+` operator uses ToPrimitive with hint="default", which §7.1.1.1
    // routes through the same ["valueOf", "toString"] order as "number".
    // Without OrdinaryToPrimitive, this would produce "1[object Object]".
    assert_eq!(
        eval_number(
            "1 + { valueOf()  { return 2; },
                   toString() { return 'never'; } };"
        ),
        3.0
    );
}

// -- Type errors when neither method yields a primitive ---------------------

#[test]
fn both_methods_missing_throws_type_error() {
    // §7.1.1.1 step 3: neither "valueOf" nor "toString" exists →
    // TypeError.  `Object.create(null)` produces such an object.
    eval_throws("Object.create(null) + 1;");
}

#[test]
fn both_methods_returning_object_throws_type_error() {
    // §7.1.1.1 step 3: both methods return Object → no primitive →
    // TypeError.
    eval_throws(
        "'' + { toString() { return {}; },
                valueOf()  { return {}; } };",
    );
}

// -- Method-thrown error propagates -----------------------------------------

#[test]
fn method_thrown_error_propagates() {
    // The `?` mark on §7.1.1.1 step 2.b.ii (Call) requires abrupt-
    // completion propagation.  A user-defined `toString` that throws
    // must surface its error rather than silently fall through.
    assert_eq!(
        eval_string(
            "var caught = 'no-throw';
             try {
               String({ toString() { throw 42; } });
             } catch (e) { caught = String(e); }
             caught;"
        ),
        "42"
    );
}

// -- @@toPrimitive precedence -----------------------------------------------

#[test]
fn at_to_primitive_overrides_ordinary_to_primitive() {
    // §7.1.1 step 2.a: a user-installed `Symbol.toPrimitive` short-
    // circuits the §7.1.1.1 OrdinaryToPrimitive walk entirely.  This
    // regression test ensures slot #10.7 didn't accidentally drop the
    // pre-existing @@toPrimitive precedence.
    assert_eq!(
        eval_number(
            "var obj = {
               [Symbol.toPrimitive](hint) { return 7; },
               valueOf()  { return 'never-1'; },
               toString() { return 'never-2'; },
             };
             obj + 0;"
        ),
        7.0
    );
}

// -- WebIDL enum coercion via custom toString -------------------------------
//
// `ReadableStream` and `CountQueuingStrategy` are installed only under
// `feature = "engine"`, so the two surface-level integration tests below
// are gated.  The other coverage above runs in both build modes.

#[cfg(feature = "engine")]
#[test]
fn web_idl_enum_extracts_via_user_to_string() {
    // §7.1.12 step 9 → §7.1.1.1: `getReader({ mode })` runs ToString
    // on `mode` to extract the enum value.  A plain Object whose
    // `toString` returns "byob" should select the BYOB branch (which
    // currently throws "BYOB-unsupported"); pre-slot-#10.7 the
    // placeholder shortcut returned `"[object Object]"` and the enum
    // match silently fell through to `"default"`.
    assert_eq!(
        eval_string(
            "var caught = 'no-throw';
             try {
               new ReadableStream().getReader(
                 { mode: { toString() { return 'byob'; } } }
               );
             } catch (e) { caught = String(e); }
             caught.indexOf('BYOB') >= 0 ? 'byob-branch' : caught;"
        ),
        "byob-branch"
    );
}

// -- WebIDL `unrestricted double` coercion via custom valueOf ---------------

#[cfg(feature = "engine")]
#[test]
fn web_idl_unrestricted_double_extracts_via_user_value_of() {
    // §7.1.4 step 4 → §7.1.1.1: `new CountQueuingStrategy({highWaterMark})`
    // runs ToNumber on `highWaterMark`.  A plain Object whose `valueOf`
    // returns a number should be accepted and surface as that number on
    // the resulting strategy; pre-slot-#10.7 the shortcut returned NaN
    // and rejected the init.
    assert_eq!(
        eval_number(
            "new CountQueuingStrategy(
               { highWaterMark: { valueOf() { return 5; } } }
             ).highWaterMark;"
        ),
        5.0
    );
}
