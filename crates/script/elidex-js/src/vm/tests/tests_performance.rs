//! PR4b C5: `performance.now()` / `performance.timeOrigin` tests.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn performance_is_object() {
    let mut vm = Vm::new();
    match vm.eval("typeof performance;").unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "object"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn performance_now_returns_positive_ms() {
    let mut vm = Vm::new();
    let v = eval_number(&mut vm, "performance.now();");
    assert!(v >= 0.0, "expected non-negative, got {v}");
    // Reasonable upper bound: each script should complete well under
    // an hour of wall time.
    assert!(v < 3_600_000.0, "implausibly large: {v}");
}

#[test]
fn performance_now_is_monotonic() {
    // Two back-to-back calls inside a single `eval` must produce a
    // monotonically non-decreasing sequence — the spec forbids clock
    // rewinds.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.__t1 = performance.now();
         globalThis.__t2 = performance.now();",
    )
    .unwrap();
    let JsValue::Number(t1) = vm.get_global("__t1").unwrap() else {
        unreachable!()
    };
    let JsValue::Number(t2) = vm.get_global("__t2").unwrap() else {
        unreachable!()
    };
    assert!(t2 >= t1, "non-monotonic: t1={t1} t2={t2}");
}

#[test]
fn performance_now_is_monotonic_across_evals() {
    // Separate `eval` calls must observe a monotonically
    // non-decreasing clock, without relying on real sleeping or
    // scheduler timing — deterministic even on busy CI runners.
    let mut vm = Vm::new();
    let t1 = eval_number(&mut vm, "performance.now();");
    let t2 = eval_number(&mut vm, "performance.now();");
    let t3 = eval_number(&mut vm, "performance.now();");
    assert!(t2 >= t1, "non-monotonic across evals: t1={t1} t2={t2}");
    assert!(t3 >= t2, "non-monotonic across evals: t2={t2} t3={t3}");
}

#[test]
fn performance_time_origin_is_zero_for_now() {
    // Phase 2 reports 0; a real wall-clock mapping arrives with the
    // shell in Phase 3.
    let mut vm = Vm::new();
    let v = eval_number(&mut vm, "performance.timeOrigin;");
    assert_eq!(v, 0.0);
}
