//! `AbortSignal.abort` / `.timeout` / `.any` static factory tests
//! (WHATWG DOM §3.1.3).
//!
//! Split out of [`super::tests_abort`] to keep that file under the
//! project's 1000-line convention.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn abort_signal_abort_returns_already_aborted_signal() {
    let mut vm = Vm::new();
    assert!(eval_bool(&mut vm, "AbortSignal.abort().aborted;"));
}

#[test]
fn abort_signal_abort_default_reason_is_dom_exception_abort_error() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var s = AbortSignal.abort(); \
         s.reason instanceof DOMException && s.reason.name === 'AbortError';"
    ));
}

#[test]
fn abort_signal_abort_preserves_custom_reason() {
    let mut vm = Vm::new();
    // Non-undefined reason passthrough — matches the
    // `controller.abort('custom')` path.
    assert_eq!(
        eval_string(&mut vm, "AbortSignal.abort('boom').reason;"),
        "boom"
    );
}

#[test]
fn abort_signal_timeout_returns_not_yet_aborted_signal() {
    let mut vm = Vm::new();
    // Immediately after `timeout`, the signal must not be
    // aborted yet — the timer only fires on the next
    // `drain_timers` call.
    assert!(!eval_bool(&mut vm, "AbortSignal.timeout(100).aborted;"));
}

#[test]
fn abort_signal_timeout_fires_on_drain() {
    use std::time::{Duration, Instant};
    let mut vm = Vm::new();
    vm.eval("globalThis.s = AbortSignal.timeout(0);").unwrap();
    // Drain past the deadline — the internal abort path
    // should set `s.aborted = true` with a
    // `DOMException("TimeoutError")` reason.
    let future = Instant::now() + Duration::from_millis(10);
    vm.inner.drain_timers(future);
    assert!(eval_bool(&mut vm, "s.aborted;"));
    assert!(eval_bool(
        &mut vm,
        "s.reason instanceof DOMException && s.reason.name === 'TimeoutError';"
    ));
}

#[test]
fn abort_signal_any_empty_returns_non_aborted() {
    let mut vm = Vm::new();
    assert!(!eval_bool(&mut vm, "AbortSignal.any([]).aborted;"));
}

#[test]
fn abort_signal_any_already_aborted_input_propagates() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = AbortSignal.abort('src'); \
         var composite = AbortSignal.any([a]); \
         composite.aborted && composite.reason === 'src';"
    ));
}

#[test]
fn abort_signal_any_invalid_element_does_not_strand_composite_state() {
    // Regression: `AbortSignal.any([invalid])` used to allocate
    // the composite signal (+ insert into `abort_signal_states`)
    // BEFORE validating the iterable elements.  A throw on a bogus
    // element left the composite rooted in the state map until the
    // next GC cycle — frequent misuse could accumulate entries.
    // The pre-validation reorder ensures no allocation happens on
    // the error path.
    let mut vm = Vm::new();
    let baseline = vm.inner.abort_signal_states.len();
    for _ in 0..5 {
        let r = vm.eval("try { AbortSignal.any([42]); 0; } catch(e) { 1; }");
        assert!(matches!(r.unwrap(), JsValue::Number(n) if (n - 1.0).abs() < f64::EPSILON));
    }
    assert_eq!(
        vm.inner.abort_signal_states.len(),
        baseline,
        "no composite signal should be left in abort_signal_states after validation throws"
    );
}

#[test]
fn abort_signal_any_non_signal_arg_throws_type_error() {
    let mut vm = Vm::new();
    // PR5a scope: coercion failure is a plain TypeError (not
    // DOMException) — the iterable validation runs before the
    // "convert to signal" step.
    let caught = vm
        .eval(
            "var caught = ''; \
             try { AbortSignal.any([42]); } catch (e) { caught = e.name; } \
             caught;",
        )
        .unwrap();
    match caught {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn abort_signal_timeout_astronomical_ms_does_not_panic() {
    // Regression: `AbortSignal.timeout(Number.MAX_VALUE)` previously
    // built a `Duration` that overflowed `Instant::now() + delay` and
    // panicked.  The shared `clamp_delay_to_duration` helper caps
    // at 100 years so this path now returns a pending signal
    // without incident.
    let mut vm = Vm::new();
    let r = vm
        .eval("typeof AbortSignal.timeout(Number.MAX_VALUE) === 'object';")
        .unwrap();
    assert!(matches!(r, JsValue::Boolean(true)));
    // Infinity and NaN also must not panic (clamp to 0ms).
    let r = vm
        .eval("typeof AbortSignal.timeout(Infinity) === 'object';")
        .unwrap();
    assert!(matches!(r, JsValue::Boolean(true)));
    let r = vm
        .eval("typeof AbortSignal.timeout(NaN) === 'object';")
        .unwrap();
    assert!(matches!(r, JsValue::Boolean(true)));
}

#[test]
fn clear_timeout_immediately_drops_pending_timeout_signals_entry() {
    // Regression: `clearTimeout(id)` that happens to cancel an
    // `AbortSignal.timeout(ms)` timer must drop the
    // `pending_timeout_signals` entry synchronously — without this,
    // a host that never drives `drain_timers` again would leak the
    // signal.
    let mut vm = Vm::new();
    vm.eval("globalThis.s = AbortSignal.timeout(1000);")
        .unwrap();
    // Only one timer entry in flight — grab its id from the map.
    assert_eq!(vm.inner.pending_timeout_signals.len(), 1);
    let timer_id = *vm
        .inner
        .pending_timeout_signals
        .keys()
        .next()
        .expect("timer id should be registered");
    // `clearTimeout` accepts the underlying numeric id regardless of
    // the JS API that scheduled the timer.  Verifies the drop path
    // runs even though no `drain_timers` tick followed.
    vm.eval(&format!("clearTimeout({timer_id});")).unwrap();
    assert!(
        vm.inner.pending_timeout_signals.is_empty(),
        "clearTimeout should have dropped the signal back-ref"
    );
}

#[test]
fn abort_signal_timeout_canceled_signal_state_cleaned() {
    // When the signal's pending_timeout_signals entry fires,
    // the map entry is removed.  Drain twice — second drain
    // should be a no-op since the signal already aborted.
    use std::time::{Duration, Instant};
    let mut vm = Vm::new();
    vm.eval("globalThis.s = AbortSignal.timeout(0);").unwrap();
    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(10));
    assert!(vm.inner.pending_timeout_signals.is_empty());
}
