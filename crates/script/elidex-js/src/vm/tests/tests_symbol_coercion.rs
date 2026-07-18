//! JS-observable Symbol-operand coercion regression tests.
//!
//! ECMA-262 requires every abstract-operation coercion of a Symbol to a Number
//! or String to throw a `TypeError` (§7.1.4 ToNumber, §7.1.18 ToString), while a
//! Symbol used as a *value* (property key, `typeof`, equality, `JSON.stringify`,
//! `ToBoolean`) is spec-legal, and `String(sym)` in call form returns
//! `SymbolDescriptiveString` (§22.1.1.1 step 2.a).
//!
//! The canonical `coerce::to_number` / `coerce::to_string` already throw for a
//! primitive Symbol, and every opcode/builtin routes through them by construction
//! (single chokepoint) — but until #474 made `Object.prototype.toString.call(x)`
//! and generic native-fn invocation work, these JS-level spellings could not be
//! observed, so `#11-vm-symbol-operand-coercion-throws` was pinned at the
//! coerce-unit layer with an (incorrect) "opcode bypass" premise. These tests
//! lock the end-to-end behavior (parser → compiler → opcode dispatch → coerce)
//! and close that slot; see `docs/plans/2026-07-vm-symbol-coercion-rediagnosis.md`.

use super::super::value::{JsValue, VmErrorKind};
use super::{eval, eval_bool, eval_number, eval_string};

/// Assert that evaluating `src` at top level throws a `TypeError` — the
/// spec-mandated error *type* for a forbidden Symbol coercion. The impl-defined
/// message ("Cannot convert a Symbol value to a …") is intentionally not
/// asserted, only the type the spec fixes.
fn throws_type_error(src: &str) {
    match eval(src) {
        Err(e) => assert!(
            matches!(e.kind, VmErrorKind::TypeError),
            "`{src}` should throw TypeError, got {e:?}"
        ),
        Ok(v) => panic!("`{src}` should throw, got {v:?}"),
    }
}

// ── Coercion to String → TypeError (§7.1.18) ───────────────────────────────

#[test]
fn template_and_concat_symbol_throws() {
    // Template literal substitution and `+` string-concatenation both ToString.
    for src in [
        "`${Symbol()}`",
        "`x${Symbol('y')}z`",
        "Symbol() + ''",
        "'' + Symbol()",
        // Array whose element ToString throws, reached via template / concat.
        "`${[Symbol()]}`",
        "[Symbol()] + ''",
    ] {
        throws_type_error(src);
    }
}

#[test]
fn string_producing_builtins_symbol_throws() {
    for src in [
        "new String(Symbol())", // NewTarget defined → §22.1.1.1 step 2.b ToString
        "parseInt(Symbol())",
        "[Symbol()].join()",
        "[Symbol()].toString()",
        "'x'.repeat(Symbol())", // ToIntegerOrInfinity → ToNumber
    ] {
        throws_type_error(src);
    }
}

// ── Coercion to Number → TypeError (§7.1.4, via §13.15.3 / ToNumeric) ───────

#[test]
fn arithmetic_and_relational_symbol_throws() {
    for src in [
        "Symbol() * 1",
        "Symbol() - 1",
        "Symbol() + 1", // neither operand a String → numeric branch
        "Symbol() % 2",
        "Symbol() ** 2",
        "2 ** Symbol()",
        "Symbol() < 1", // relational → ToPrimitive(number) → ToNumeric
    ] {
        throws_type_error(src);
    }
}

#[test]
fn unary_bitwise_symbol_throws() {
    for src in [
        "+Symbol()",
        "-Symbol()",
        "~Symbol()",
        "Symbol() | 0",
        "Symbol() & 1",
        "Symbol() >> 1",
    ] {
        throws_type_error(src);
    }
}

#[test]
fn number_producing_builtins_symbol_throws() {
    for src in ["Number(Symbol())", "Math.abs(Symbol())"] {
        throws_type_error(src);
    }
}

// ── Boxed Symbol (SymbolWrapper) — newly JS-reachable via #474's callable
//    `Object()`; locks the #467 R9 `Symbol.prototype` valueOf/@@toPrimitive fix
//    at the JS level for the first time (§20.4.3.4 / §20.4.3.5). ─────────────

#[test]
fn boxed_symbol_coercion_throws() {
    for src in [
        "`${Object(Symbol())}`",
        "Object(Symbol()) + 1",
        "Number(Object(Symbol()))",
        "Object(Symbol()) * 2",
        // `String(wrapper)`: value is an Object, not a primitive Symbol, so the
        // §22.1.1.1 step 2.a descriptive-string special-case does NOT apply →
        // step 2.b ToString → throw. Contrast `string_call_form_of_symbol_*`.
        "String(Object(Symbol()))",
    ] {
        throws_type_error(src);
    }
}

#[test]
fn boxed_symbol_is_an_object_not_a_throw() {
    // `Object(Symbol())` itself is a legal boxing (ToObject), not a coercion.
    assert_eq!(eval_string("typeof Object(Symbol())"), "object");
}

// ── Non-coercion Symbol use is spec-legal (guards against an over-eager
//    "throw on any Symbol" regression). ──────────────────────────────────────

#[test]
fn string_call_form_of_symbol_returns_descriptive_string() {
    // §22.1.1.1 step 2.a: NewTarget undefined + Symbol → SymbolDescriptiveString.
    assert_eq!(eval_string("String(Symbol())"), "Symbol()");
    assert_eq!(eval_string("String(Symbol('foo'))"), "Symbol(foo)");
    // §20.4.3.3 explicit Symbol.prototype.toString is likewise allowed.
    assert_eq!(eval_string("Symbol().toString()"), "Symbol()");
    assert_eq!(eval_string("Symbol('foo').toString()"), "Symbol(foo)");
}

#[test]
fn symbol_typeof_and_boolean_do_not_throw() {
    assert_eq!(eval_string("typeof Symbol()"), "symbol");
    assert!(eval_bool("Boolean(Symbol())")); // §7.1.2 ToBoolean never throws
    assert_eq!(eval_number("Symbol() ? 1 : 0"), 1.0);
}

#[test]
fn symbol_equality_is_false_not_a_throw() {
    // §7.2.13 IsLooselyEqual / §7.2.14 IsStrictlyEqual: no Symbol coercion
    // clause — unequal, no TypeError.
    assert!(!eval_bool("Symbol() == 1"));
    assert!(!eval_bool("Symbol() === 1"));
    assert!(!eval_bool("Symbol() == 'x'"));
}

#[test]
fn symbol_as_property_key_and_json_yield_undefined() {
    // Symbol is a valid property key (no coercion); JSON skips it; `.description`
    // reads the slot. None throws.
    assert!(matches!(eval("({})[Symbol()]"), Ok(JsValue::Undefined)));
    assert!(matches!(
        eval("JSON.stringify(Symbol())"),
        Ok(JsValue::Undefined)
    ));
    assert!(matches!(
        eval("Symbol().description"),
        Ok(JsValue::Undefined)
    ));
}

// ── The thrown value is a JS-observable `TypeError` (prototype chain intact),
//    not merely an internal error kind. ────────────────────────────────────

#[test]
fn symbol_coercion_throws_are_observable_type_errors() {
    for src in [
        "try { `${Symbol()}`; false } catch (e) { e instanceof TypeError }",
        "try { Number(Symbol()); false } catch (e) { e instanceof TypeError }",
        "try { Symbol() * 1; false } catch (e) { e instanceof TypeError }",
        "try { [Symbol()].join(); false } catch (e) { e instanceof TypeError }",
        "try { Object(Symbol()) + 1; false } catch (e) { e instanceof TypeError }",
    ] {
        assert!(eval_bool(src), "src: {src}");
    }
}
