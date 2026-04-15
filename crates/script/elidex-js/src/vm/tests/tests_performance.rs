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
    let t1 = match vm.get_global("__t1").unwrap() {
        JsValue::Number(n) => n,
        _ => unreachable!(),
    };
    let t2 = match vm.get_global("__t2").unwrap() {
        JsValue::Number(n) => n,
        _ => unreachable!(),
    };
    assert!(t2 >= t1, "non-monotonic: t1={t1} t2={t2}");
}

#[test]
fn performance_now_advances_across_sleep() {
    // Sleep for a short, reliable-on-CI duration and verify that
    // `performance.now()` moves forward by approximately that amount.
    let mut vm = Vm::new();
    let t1 = eval_number(&mut vm, "performance.now();");
    std::thread::sleep(std::time::Duration::from_millis(20));
    let t2 = eval_number(&mut vm, "performance.now();");
    let delta = t2 - t1;
    assert!(
        delta >= 15.0,
        "expected >=15ms advance after 20ms sleep, got {delta}"
    );
    // CI scheduling jitter: leave a generous ceiling.
    assert!(delta < 5_000.0, "delta={delta} way too large");
}

#[test]
fn performance_time_origin_is_zero_for_now() {
    // Phase 2 reports 0; a real wall-clock mapping arrives with the
    // shell in Phase 3.
    let mut vm = Vm::new();
    let v = eval_number(&mut vm, "performance.timeOrigin;");
    assert_eq!(v, 0.0);
}
