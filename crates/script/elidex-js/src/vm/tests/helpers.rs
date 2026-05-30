//! Shared helpers for the bytecode VM test suite (extracted from
//! the inline body of [`super`] to keep `tests/mod.rs` under the
//! 1000-line convention).
//!
//! Sibling test modules (`tests_*.rs`) reach these helpers via
//! `use super::{eval, eval_bool, ...}` — the re-export
//! `pub(crate) use helpers::*;` in [`super`] keeps that path
//! intact after extraction.

use super::super::value::{JsValue, VmError};
use super::super::Vm;

/// Drive `vm.tick_network()` until `pending_fetches` is empty, with a
/// 16-iteration ceiling to guard against unbounded reaction loops.
/// Always runs one trailing tick so a chain whose final reaction
/// did not allocate a new pending fetch still gets its microtask
/// drain.  Shared helper for the M4-12 PR5-async-fetch test suite
/// (R9.2 dedup) — used by `tests_fetch`, `tests_integration_fetch`,
/// and `tests_async_fetch`.
#[cfg(feature = "engine")]
pub(crate) fn drain_fetch_replies(vm: &mut Vm) {
    for _ in 0..16 {
        if vm.inner.pending_fetches.is_empty() {
            break;
        }
        vm.tick_network();
    }
    vm.tick_network();
}

pub(crate) fn eval(source: &str) -> Result<JsValue, VmError> {
    let mut vm = Vm::new();
    vm.eval(source)
}

pub(crate) fn eval_throws(source: &str) {
    let result = eval(source);
    assert!(result.is_err(), "expected error, got {result:?}");
}

pub(crate) fn eval_number(source: &str) -> f64 {
    match eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

pub(crate) fn eval_string(source: &str) -> String {
    let mut vm = Vm::new();
    let result = vm.eval(source).unwrap();
    match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

pub(crate) fn eval_bool(source: &str) -> bool {
    match eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// `vm.eval(src)` and assert the result is `Number(expected)` within
/// `f64::EPSILON`.  Companion to [`eval_number`] for tests that need
/// to drive a long-lived VM across multiple eval calls (the realtime
/// suites under `tests_websocket` / `tests_event_source` /
/// `tests_realtime` bind once per `with_*_vm` and then evaluate many
/// scripts against the same VM).
#[cfg(feature = "engine")]
pub(crate) fn assert_eval_number(vm: &mut Vm, src: &str, expected: f64) {
    match vm.eval(src).unwrap() {
        JsValue::Number(n) => assert!(
            (n - expected).abs() < f64::EPSILON,
            "expected {expected}, got {n} (src: {src})"
        ),
        other => panic!("expected Number({expected}), got {other:?} (src: {src})"),
    }
}

/// `vm.eval(src)` and assert the result is `String(expected)`.
/// Companion to [`assert_eval_number`].
#[cfg(feature = "engine")]
pub(crate) fn assert_eval_string(vm: &mut Vm, src: &str, expected: &str) {
    match vm.eval(src).unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), expected, "src: {src}"),
        other => panic!("expected String({expected:?}), got {other:?} (src: {src})"),
    }
}

/// `vm.eval(src)` and assert the result is `Boolean(expected)`.
/// Companion to [`assert_eval_number`].
#[cfg(feature = "engine")]
pub(crate) fn assert_eval_bool(vm: &mut Vm, src: &str, expected: bool) {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => assert_eq!(b, expected, "src: {src}"),
        other => panic!("expected Boolean({expected}), got {other:?} (src: {src})"),
    }
}

/// Evaluate `source`, drain microtasks (via the post-script drain inside
/// `eval`), then read the global `var` named `name` and expect a number.
/// Used to observe state set asynchronously from Promise reactions.
pub(crate) fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
    }
}

/// Evaluate `source`, drain microtasks, then read the global `var` named
/// `name` and expect a string.
pub(crate) fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

/// Assert `throwing` throws, AND that after recovery `observation` yields
/// `expected`.  Used when a now-strict operation used to fail silently —
/// verifies both the throw and that state is unchanged.  `setup` runs before
/// both the throwing check and the observation.
///
/// Segments are joined with `;\n` so callers need not worry about trailing
/// semicolons or ASI: a redundant `;` between two well-formed statements is
/// a valid empty statement.
pub(crate) fn assert_throws_preserves_number(
    setup: &str,
    throwing: &str,
    observation: &str,
    expected: f64,
) {
    eval_throws(&format!("{setup};\n{throwing}"));
    assert_eq!(
        eval_number(&format!(
            "{setup};\ntry {{ {throwing} }} catch(_) {{}}\n{observation}"
        )),
        expected,
    );
}

/// Boolean-returning variant of [`assert_throws_preserves_number`].
pub(crate) fn assert_throws_preserves_bool(
    setup: &str,
    throwing: &str,
    observation: &str,
    expected: bool,
) {
    eval_throws(&format!("{setup};\n{throwing}"));
    assert_eq!(
        eval_bool(&format!(
            "{setup};\ntry {{ {throwing} }} catch(_) {{}}\n{observation}"
        )),
        expected,
    );
}

/// Assert that the bare-call expression `expr` throws the canonical
/// `CallShape::ConstructorOnly` TypeError emitted at
/// `vm/interpreter.rs::call_dispatch` (WebIDL §3.7.1 step 1.2 +
/// ECMA-262 §27.2.3.1 step 1).
///
/// `interface` is the WebIDL Interface name (or core-VM ctor name —
/// e.g. `"Promise"`) that appears between the single quotes in the
/// canonical message form
/// `"Failed to construct '{interface}': Please use the 'new' operator"`.
///
/// Spawns a fresh `Vm` per invocation (mirrors the [`eval_string`]
/// shape) and wraps `expr` in a `try {{ <expr>; 'no throw' }} catch (e)
/// {{ e.message }}` so the assertion can equality-compare the message
/// string directly rather than walking a `Result<_, VmError>`.
///
/// Used by per-ctor regression tests in
/// `tests_<feature>.rs::<feature>_ctor_requires_new` — one per the 66
/// ctor sites covered by plan-memo
/// `m4-12-pr-vm-native-constructor-only-flag-plan.md` §5.
pub(crate) fn assert_ctor_requires_new(expr: &str, interface: &str) {
    let mut vm = Vm::new();
    let src = format!("try {{ {expr}; 'no throw' }} catch (e) {{ e.message }}");
    let msg = match vm.eval(&src).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string error message, got {other:?} (expr: {expr})"),
    };
    assert_eq!(
        msg,
        format!("Failed to construct '{interface}': Please use the 'new' operator"),
        "ctor `{expr}` should throw canonical TypeError",
    );
}

/// Assert that a [`CallShape::IllegalConstructor`] WebIDL interface
/// object throws the canonical `"Failed to construct '{interface}':
/// Illegal constructor"` TypeError in BOTH call modes — `new
/// Interface()` (gated at `vm/ops.rs::do_new`) AND bare `Interface()`
/// (gated at `vm/interpreter.rs::call_dispatch`).  Both messages come
/// from the single SoT `VmError::illegal_constructor`, so this verifies
/// the two chokepoints stay in sync (WebIDL §3.7.1 (Interface object)
/// creation algorithm step 1.1 — interface declares no constructor
/// operation).
///
/// `interface` is the WebIDL Interface name appearing between the single
/// quotes (e.g. `"Crypto"`, `"FileList"`).  One call per the 16
/// IllegalConstructor sites covered by plan-memo
/// `m4-12-pr-vm-native-illegal-constructor-shape-plan.md` §3.2 (+ AbortSignal).
pub(crate) fn assert_illegal_constructor(interface: &str) {
    let expected = format!("Failed to construct '{interface}': Illegal constructor");
    for expr in [format!("new {interface}()"), format!("{interface}()")] {
        let mut vm = Vm::new();
        let src = format!("try {{ {expr}; 'no throw' }} catch (e) {{ e.message }}");
        let msg = match vm.eval(&src).unwrap() {
            JsValue::String(id) => vm.get_string(id),
            other => panic!("expected string error message, got {other:?} (expr: {expr})"),
        };
        assert_eq!(
            msg, expected,
            "illegal-ctor `{expr}` should throw canonical TypeError in both call modes",
        );
    }
}
