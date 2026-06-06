//! Tests for destructuring formal parameters
//! (`function f([a, b]) {}`, `({x}) => x`, …).
//!
//! Regression guard: destructuring params were compiled as flat
//! positional params — every identifier inside a pattern consumed its
//! own positional argument slot, so `([a, b]) => …` behaved like
//! `(a, b) => …` (`a` received the whole argument, `b` `undefined`).
//! The fix gives each formal parameter exactly one positional slot
//! (anonymous for patterns) and unpacks it in a parameter prologue,
//! mirroring §10.2.11 FunctionDeclarationInstantiation. The failure was
//! masked by truthiness in callers that only tested `a ? … : …`, so the
//! discriminating tests below assert the *bound values*, not just
//! identity.

use super::{eval_bool, eval_number, eval_string};

// ── Array-pattern params: the bound names, not just identity ──

#[test]
fn array_param_binds_first_element() {
    assert_eq!(eval_number("(([a, b]) => a)([10, 20]);"), 10.0);
}

#[test]
fn array_param_binds_second_element() {
    // Pre-fix this was `undefined` (no second positional arg).
    assert_eq!(eval_number("(([a, b]) => b)([10, 20]);"), 20.0);
}

#[test]
fn array_param_function_declaration() {
    assert_eq!(
        eval_number("function f([a, b]) { return a + b; } f([3, 4]);"),
        7.0
    );
}

// ── Object-pattern params ──

#[test]
fn object_param_binds_property() {
    // Pre-fix this was the whole object (no positional `x`).
    assert_eq!(eval_number("(({x}) => x)({x: 7});"), 7.0);
}

#[test]
fn object_param_binds_multiple_properties() {
    assert_eq!(eval_number("(({x, y}) => x + y)({x: 1, y: 2});"), 3.0);
}

// ── Identity: the original repro (`Promise.all([…]).then(([a, b]) => a === b)`) ──

#[test]
fn array_param_preserves_element_identity() {
    assert!(eval_bool("let o = {}; (([a, b]) => a === b)([o, o]);"));
}

#[test]
fn destructuring_param_via_map_callback() {
    assert!(eval_bool(
        "let o = {}; [[o, o]].map(([a, b]) => a === b)[0];"
    ));
}

// ── Pattern params keep later simple params positionally aligned ──

#[test]
fn pattern_param_does_not_shift_later_simple_params() {
    assert_eq!(
        eval_number("((x, [a, b], y) => x + a + b + y)(1, [2, 3], 4);"),
        10.0
    );
}

#[test]
fn multiple_pattern_params() {
    assert_eq!(eval_number("(([a], [b]) => a + b)([1], [2]);"), 3.0);
}

// ── Nesting ──

#[test]
fn nested_array_patterns() {
    assert_eq!(eval_number("(([[a], [b]]) => a + b)([[1], [2]]);"), 3.0);
}

#[test]
fn object_pattern_nested_in_array_pattern() {
    assert_eq!(eval_string("(([{k}]) => k)([{k: 'hi'}]);"), "hi");
}

// ── Defaults ──

#[test]
fn defaults_inside_pattern() {
    assert_eq!(eval_number("(([a = 5, b = 6]) => a + b)([1]);"), 7.0);
}

#[test]
fn whole_pattern_param_default() {
    assert_eq!(eval_number("(([a, b] = [7, 8]) => a + b)();"), 15.0);
}

#[test]
fn later_default_observes_earlier_destructured_binding() {
    // §10.2.11: parameters initialize left-to-right, so `[a]` is
    // destructured before `b`'s default expression `a` evaluates.
    assert_eq!(eval_number("(([a], b = a) => a + b)([5]);"), 10.0);
}

// ── Rest ──

#[test]
fn object_pattern_rest() {
    assert_eq!(
        eval_number("(({a, ...rest}) => rest.b)({a: 1, b: 2});"),
        2.0
    );
}

#[test]
fn array_pattern_rest() {
    assert_eq!(
        eval_number("(([a, ...rest]) => rest.length)([1, 2, 3]);"),
        2.0
    );
}

// ── Interaction with `arguments` and closures ──

#[test]
fn arguments_reflects_raw_argument_not_destructured_element() {
    assert_eq!(
        eval_number("function f([a, b]) { return arguments[0].length; } f([1, 2]);"),
        2.0
    );
}

#[test]
fn destructured_binding_captured_by_closure() {
    assert_eq!(eval_number("(([a, b]) => () => a + b)([1, 2])();"), 3.0);
}

// ── Other function forms sharing the same compile path ──

#[test]
fn class_method_destructuring_param() {
    assert_eq!(
        eval_number("class C { m([a, b]) { return a + b; } } new C().m([3, 4]);"),
        7.0
    );
}

#[test]
fn class_setter_destructuring_param() {
    assert_eq!(
        eval_number("let r; class C { set p([a, b]) { r = a + b; } } new C().p = [3, 4]; r;"),
        7.0
    );
}

#[test]
fn generator_destructuring_param() {
    assert_eq!(
        eval_string("function* g([a, b]) { yield a; yield b; } [...g([1, 2])].join(',');"),
        "1,2"
    );
}

// ── Simple params remain unaffected by the new prologue ──

#[test]
fn simple_params_unaffected() {
    assert_eq!(eval_number("((a, b) => a + b)(1, 2);"), 3.0);
    assert_eq!(eval_number("((a = 5) => a)();"), 5.0);
    assert_eq!(eval_number("((...r) => r.length)(1, 2, 3);"), 3.0);
}
