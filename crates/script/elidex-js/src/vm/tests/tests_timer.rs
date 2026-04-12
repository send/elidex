//! Timer tests (WHATWG HTML §8.7).
//!
//! `setTimeout`/`setInterval` schedule callbacks on the VM's timer heap;
//! `clearTimeout`/`clearInterval` cancel them.  Driving is deterministic
//! in tests via [`VmInner::drain_timers(now)`] — we pass explicit
//! `Instant` values so "advance time" doesn't depend on wall-clock.

use std::time::{Duration, Instant};

use super::Vm;
use crate::vm::value::JsValue;

fn installed_vm() -> Vm {
    Vm::new()
}

// ─── Basic scheduling ────────────────────────────────────────────────────

#[test]
fn set_timeout_returns_numeric_id() {
    let mut vm = installed_vm();
    let r = vm.eval("setTimeout(() => {}, 100);").unwrap();
    assert!(matches!(r, JsValue::Number(_)));
}

#[test]
fn set_timeout_unique_ids_per_call() {
    let mut vm = installed_vm();
    let a = vm.eval("setTimeout(() => {}, 100);").unwrap();
    let b = vm.eval("setTimeout(() => {}, 100);").unwrap();
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => assert_ne!(x, y),
        _ => panic!("non-number ids"),
    }
}

// ─── Drain fires expired callbacks ────────────────────────────────────────

#[test]
fn drain_fires_expired_set_timeout() {
    let mut vm = installed_vm();
    vm.eval("globalThis.x = 0; setTimeout(() => { globalThis.x = 42; }, 10);")
        .unwrap();
    // Before the deadline — nothing fires.
    vm.inner.drain_timers(Instant::now());
    assert_eq!(vm.get_global("x"), Some(JsValue::Number(0.0)));
    // After deadline — the 10 ms timer fires.
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(50));
    assert_eq!(vm.get_global("x"), Some(JsValue::Number(42.0)));
}

#[test]
fn drain_passes_extra_args_to_callback() {
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.out = 0; \
         setTimeout((a, b) => { globalThis.out = a + b; }, 0, 3, 4);",
    )
    .unwrap();
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(1));
    assert_eq!(vm.get_global("out"), Some(JsValue::Number(7.0)));
}

#[test]
fn drain_skips_non_expired() {
    let mut vm = installed_vm();
    vm.eval("globalThis.fired = 0; setTimeout(() => { globalThis.fired = 1; }, 1000);")
        .unwrap();
    // 1000 ms deadline, advance only 100 ms.
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(100));
    assert_eq!(vm.get_global("fired"), Some(JsValue::Number(0.0)));
}

// ─── clearTimeout ────────────────────────────────────────────────────────

#[test]
fn clear_timeout_prevents_fire() {
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.fired = 0; \
         var id = setTimeout(() => { globalThis.fired = 1; }, 10); \
         clearTimeout(id);",
    )
    .unwrap();
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(100));
    assert_eq!(vm.get_global("fired"), Some(JsValue::Number(0.0)));
}

#[test]
fn clear_timeout_unknown_id_is_noop() {
    let mut vm = installed_vm();
    // Should not throw; silently ignores unknown ids (spec).
    vm.eval("clearTimeout(99999);").unwrap();
}

// ─── setInterval ─────────────────────────────────────────────────────────

#[test]
fn set_interval_re_arms_after_each_fire() {
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.count = 0; \
         setInterval(() => { globalThis.count += 1; }, 10);",
    )
    .unwrap();
    // Advance 35 ms — should fire 3 times (at +10, +20, +30).
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(35));
    assert_eq!(vm.get_global("count"), Some(JsValue::Number(3.0)));
    // Advance further to +75 — should fire 4 more times (+40, +50, +60, +70).
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(75));
    assert_eq!(vm.get_global("count"), Some(JsValue::Number(7.0)));
}

#[test]
fn clear_interval_stops_repetition() {
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.count = 0; \
         globalThis.id = setInterval(() => { \
             globalThis.count += 1; \
             if (globalThis.count >= 2) clearInterval(globalThis.id); \
         }, 10);",
    )
    .unwrap();
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(100));
    assert_eq!(vm.get_global("count"), Some(JsValue::Number(2.0)));
}

// ─── Drain flushes microtasks after timers ────────────────────────────────

#[test]
fn drain_timers_flushes_microtasks_after() {
    // HTML §8.1.4.2 step 8: microtasks run after each macrotask.  A
    // timer callback that schedules a microtask should see it drain
    // during the same drain_timers call.
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.log = ''; \
         setTimeout(() => { \
             globalThis.log += 'T'; \
             queueMicrotask(() => { globalThis.log += 'M'; }); \
         }, 0);",
    )
    .unwrap();
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(1));
    match vm.get_global("log") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "TM"),
        other => panic!("expected log string, got {other:?}"),
    }
}

// ─── Non-callable handler throws ──────────────────────────────────────────

#[test]
fn set_timeout_non_callable_throws_typeerror() {
    let mut vm = installed_vm();
    assert!(vm.eval("setTimeout(42, 10);").is_err());
}

// ─── Negative / non-finite delay clamps to 0 ──────────────────────────────

#[test]
fn set_timeout_negative_delay_clamps_to_zero() {
    let mut vm = installed_vm();
    vm.eval(
        "globalThis.fired = 0; \
         setTimeout(() => { globalThis.fired = 1; }, -100);",
    )
    .unwrap();
    // 0 ms deadline: fires on the very next drain.
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(1));
    assert_eq!(vm.get_global("fired"), Some(JsValue::Number(1.0)));
}
