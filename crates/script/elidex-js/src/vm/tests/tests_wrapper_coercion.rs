//! JS-observable primitive-wrapper coercion-override regression tests.
//!
//! ECMA-262 coerces a primitive wrapper (`new Number(5)`, `new String('a')`, …)
//! to a primitive through the ToPrimitive → `valueOf` / `toString` /
//! `@@toPrimitive` machinery (§7.1.4 ToNumber steps 7-10, §7.1.18 ToString
//! steps 9-12, §7.1.1 ToPrimitive), so a user override of those methods on the
//! wrapper's prototype MUST be honored. `coerce::to_number` / `coerce::to_string`
//! and `JSON.stringify` previously took a wrapper "fast-path" that read the
//! internal slot (`[[NumberData]]` / `[[StringData]]` / …) directly, bypassing
//! the override; the fast-path was removed so every wrapper coercion now flows
//! through the spec AO — the same path `ops.rs::ordinary_to_primitive` already
//! took (`#11-vm-wrapper-coercion-override-bypass`; see
//! `docs/plans/2026-07-vm-wrapper-coercion-override.md`).
//!
//! JSON keeps a deliberate per-site asymmetry (§25.5.4.2 step 4 / §25.5.4
//! steps 5-6): Number/String wrappers coerce via `? ToNumber` / `? ToString`
//! (override honored), while Boolean/BigInt wrappers read the slot directly
//! (`[[BooleanData]]` / `[[BigIntData]]` — no override path).

use super::{eval_bool, eval_number, eval_string, eval_throws};

// ── 1. Override honored on the coerce side — all four wrapper kinds ─────────
// §7.1.4 / §7.1.18 draw no per-kind distinction, so removing the fast-path
// routes Number, String, Boolean AND BigInt wrappers through ToPrimitive.

#[test]
fn to_number_honors_overridden_value_of() {
    // Unary + and arithmetic ToNumber the wrapper (number hint) — was the inner
    // primitive (5), now the overridden valueOf.
    assert_eq!(
        eval_number("Number.prototype.valueOf = () => 42; +new Number(5)"),
        42.0
    );
    assert_eq!(
        eval_number("Number.prototype.valueOf = () => 42; new Number(5) * 2"),
        84.0
    );
    assert_eq!(
        eval_number("Number.prototype.valueOf = () => 42; new Number(5) - 1"),
        41.0
    );
}

#[test]
fn to_number_honors_symbol_to_primitive() {
    // @@toPrimitive takes precedence over valueOf/toString (§7.1.1 step 1.b);
    // the old fast-path bypassed even this.
    assert_eq!(
        eval_number("Number.prototype[Symbol.toPrimitive] = () => 99; +new Number(5)"),
        99.0
    );
}

#[test]
fn to_number_honors_override_via_bitwise_and_relational() {
    // Bitwise ToInt32 → ToNumber and relational `<` → ToNumber are further
    // to_number entry points on a wrapper operand.
    assert_eq!(
        eval_number("Number.prototype.valueOf = () => 6; new Number(0) | 1"),
        7.0
    );
    assert!(eval_bool(
        "Number.prototype.valueOf = () => 3; new Number(0) < 5"
    ));
}

#[test]
fn to_number_honors_override_on_boolean_wrapper() {
    // Boolean wrapper: was 1 (read [[BooleanData]]=true), now the override runs.
    assert_eq!(
        eval_number("Boolean.prototype.valueOf = () => 42; +new Boolean(true)"),
        42.0
    );
}

#[test]
fn to_number_honors_override_on_bigint_wrapper() {
    // BigInt wrapper: was a TypeError (fast-path threw before any method ran);
    // now the overridden valueOf runs first and yields a coercible Number.
    assert_eq!(
        eval_number("BigInt.prototype.valueOf = () => 42; +Object(1n)"),
        42.0
    );
}

#[test]
fn to_string_honors_overridden_to_string() {
    // Template substitution and String() ToString the wrapper (string hint).
    assert_eq!(
        eval_string("String.prototype.toString = () => 'x'; `${new String('a')}`"),
        "x"
    );
    assert_eq!(
        eval_string("String.prototype.toString = () => 'x'; String(new String('a'))"),
        "x"
    );
    // A Number wrapper stringified via template honors Number.prototype.toString.
    assert_eq!(
        eval_string("Number.prototype.toString = () => 'z'; `${new Number(5)}`"),
        "z"
    );
    // to_string removes all four wrapper arms, so Boolean/BigInt wrappers must
    // honor an overridden toString too (was 'true' / '1' from the slot).
    assert_eq!(
        eval_string("Boolean.prototype.toString = () => 'z'; String(new Boolean(true))"),
        "z"
    );
    assert_eq!(
        eval_string("BigInt.prototype.toString = () => 'z'; String(Object(1n))"),
        "z"
    );
}

#[test]
fn string_method_this_honors_overridden_to_string() {
    // §22.1.3: a String.prototype method's `this`-coercion (`coerce_this_string`)
    // does ToString(this), so a String-wrapper receiver with overridden toString
    // must use the override — it was reading `[[StringData]]` directly ('abc').
    assert_eq!(
        eval_string(
            "String.prototype.toString = () => 'xyz'; \
             String.prototype.charAt.call(new String('abc'), 0)"
        ),
        "x"
    );
    assert_eq!(
        eval_number(
            "String.prototype.toString = () => 'xyz'; \
             String.prototype.indexOf.call(new String('abc'), 'y')"
        ),
        1.0
    );
}

#[test]
fn concat_honors_wrapper_value_of() {
    // Binary + ToPrimitive(default) → valueOf-first (this path was already
    // correct via ops.rs::ordinary_to_primitive); pins that the wrapper's
    // overridden valueOf feeds the string concatenation.
    assert_eq!(
        eval_string("Number.prototype.valueOf = () => 42; '' + new Number(5)"),
        "42"
    );
}

// ── 2. JSON.stringify — Number/String wrapper coercion honors the override ──
// §25.5.4.2 step 4.b `? ToNumber` / 4.c `? ToString`, plus the replacer-array
// (§25.5.4 step 5.b.ii.4.f.i, BOTH → `? ToString`) and space (step 6.a/6.b).

#[test]
fn json_number_wrapper_value_honors_override() {
    assert_eq!(
        eval_string("Number.prototype.valueOf = () => 42; JSON.stringify(new Number(5))"),
        "42"
    );
}

#[test]
fn json_string_wrapper_value_honors_override() {
    assert_eq!(
        eval_string("String.prototype.toString = () => 'x'; JSON.stringify(new String('a'))"),
        "\"x\""
    );
}

#[test]
fn json_replacer_array_wrapper_key_honors_override() {
    // A Number wrapper in the replacer array becomes an included key via
    // `? ToString(propertyValue)`. Overriding Number.prototype.toString to '42'
    // makes `[new Number(0)]` select the '42' property, not '0'.
    assert_eq!(
        eval_string(
            "Number.prototype.toString = () => '42'; \
             JSON.stringify({ '42': 'kept', '0': 'dropped' }, [new Number(0)])"
        ),
        "{\"42\":\"kept\"}"
    );
}

#[test]
fn json_space_wrapper_honors_override() {
    // A boxed `space` with overridden valueOf sets the indent width via
    // `? ToNumber(space)` (§25.5.4 step 6.a): valueOf → 2 ⇒ two-space indent.
    assert_eq!(
        eval_string(
            "Number.prototype.valueOf = () => 2; JSON.stringify({ a: 1 }, null, new Number(0))"
        ),
        "{\n  \"a\": 1\n}"
    );
}

// ── 3. A throwing override propagates as an abrupt completion ───────────────

#[test]
fn throwing_value_of_propagates_through_coercion() {
    eval_throws("Number.prototype.valueOf = () => { throw new Error('boom'); }; +new Number(5)");
}

#[test]
fn throwing_space_value_of_propagates_through_stringify() {
    // Guards `compute_gap` fallibility: a boxed `space` whose valueOf throws
    // must propagate the abrupt completion, not be swallowed. A naive
    // `unwrap_or` swallow would pass the happy-path items above but fail here.
    eval_throws(
        "Number.prototype.valueOf = () => { throw new Error('boom'); }; \
         JSON.stringify({ a: 1 }, null, new Number(0))",
    );
}

// ── 4. JSON Boolean/BigInt wrappers stay direct (asymmetry preserved) ───────
// §25.5.4.2 step 4.d/4.e read `[[BooleanData]]` / `[[BigIntData]]` directly, so
// an override is NOT consulted for those — only Number/String unwrap coerces.

#[test]
fn json_boolean_wrapper_ignores_override() {
    assert_eq!(
        eval_string(
            "Boolean.prototype.valueOf = () => 42; \
             Boolean.prototype.toString = () => 'x'; \
             JSON.stringify(new Boolean(true))"
        ),
        "true"
    );
}

#[test]
fn json_bigint_wrapper_still_throws() {
    // BigInt wrapper reads [[BigIntData]] directly → JsValue::BigInt → the
    // BigInt-not-serializable TypeError; the override is never reached.
    eval_throws("BigInt.prototype.valueOf = () => 42; JSON.stringify(Object(1n))");
}

// ── 5. Legitimate direct slot reads are unaffected (§4(b) over-reach guard) ─
// builtinTag, thisXValue, and String-exotic index are NOT coercions and must
// stay direct — the fix must not route them through ToPrimitive.

#[test]
fn builtin_tag_unaffected_by_override() {
    // §20.1.3.6 Object.prototype.toString branches on slot PRESENCE (builtinTag),
    // not a coercion, so it stays "[object Number]" / "[object String]" even with
    // valueOf/toString overridden.
    assert_eq!(
        eval_string(
            "Number.prototype.valueOf = () => 42; \
             Object.prototype.toString.call(new Number(5))"
        ),
        "[object Number]"
    );
    assert_eq!(
        eval_string("Object.prototype.toString.call(new String('a'))"),
        "[object String]"
    );
}

#[test]
fn this_number_value_unaffected() {
    // new Number(5).valueOf() reads [[NumberData]] directly (thisNumberValue,
    // §21.1.3.7.1) — the default valueOf, not the coercion path the fix changed.
    assert_eq!(eval_number("new Number(5).valueOf()"), 5.0);
}

#[test]
fn string_exotic_index_unaffected() {
    // §10.4.3 String-exotic own index / length read the slot directly.
    assert_eq!(eval_string("new String('abc')[1]"), "b");
    assert_eq!(eval_number("new String('abc').length"), 3.0);
}

#[test]
fn no_override_coercions_unchanged() {
    // Control: with no override, every wrapper coercion is unchanged.
    assert_eq!(eval_number("+new Number(5)"), 5.0);
    assert_eq!(eval_string("`${new String('a')}`"), "a");
    assert_eq!(eval_string("JSON.stringify(new Number(5))"), "5");
    assert_eq!(eval_string("JSON.stringify(new String('a'))"), "\"a\"");
    assert_eq!(eval_string("JSON.stringify(new Boolean(true))"), "true");
}
