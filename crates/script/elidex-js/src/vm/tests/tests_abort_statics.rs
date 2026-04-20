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

// ---------------------------------------------------------------------------
// AbortSignal.any multi-input propagation
// ---------------------------------------------------------------------------

#[test]
fn any_two_inputs_first_abort_propagates_with_first_reason() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new AbortController(); \
         var b = new AbortController(); \
         var c = AbortSignal.any([a.signal, b.signal]); \
         a.abort('first'); \
         c.aborted && c.reason === 'first';"
    ));
}

#[test]
fn any_two_inputs_second_abort_is_noop_after_first() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var a = new AbortController(); \
             var b = new AbortController(); \
             var c = AbortSignal.any([a.signal, b.signal]); \
             a.abort('first'); \
             b.abort('second'); \
             c.reason;"
        ),
        "first"
    );
}

#[test]
fn any_three_inputs_first_already_aborted_uses_first_reason() {
    // WHATWG §3.1.3.3 step 2 already-aborted fast path — the
    // composite is sync-aborted at construction time with the
    // *first* already-aborted input's reason, not whichever
    // iterable entry comes first unconditionally.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new AbortController(); \
         var b = AbortSignal.abort('b-reason'); \
         var cc = new AbortController(); \
         var c = AbortSignal.any([a.signal, b, cc.signal]); \
         c.aborted && c.reason === 'b-reason';"
    ));
}

#[test]
fn any_chained_composites_propagate_through_chain() {
    // `c2 = any([c1, c])` where `c1 = any([a, b])`.  Abort `a`
    // should fire `c1`, which fans out to `c2`.  Tests recursive
    // `abort_signal` invocation inside the fan-out hook.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new AbortController(); \
         var b = new AbortController(); \
         var cc = new AbortController(); \
         var c1 = AbortSignal.any([a.signal, b.signal]); \
         var c2 = AbortSignal.any([c1, cc.signal]); \
         a.abort('deep'); \
         c1.aborted && c2.aborted && c2.reason === 'deep';"
    ));
}

#[test]
fn any_direct_composite_abort_does_not_touch_inputs() {
    // Calling `abort()` on a composite (through the `reason`
    // accessor that short-circuits on `aborted`) must NOT
    // propagate to its inputs — only the input → composite
    // direction is wired.  Composites are EventTargets, not
    // controllers — there is no direct public `abort()` on
    // `AbortSignal`, so this test uses `AbortSignal.abort(reason)`
    // to reach a pre-aborted composite replacement — which can't
    // happen for a composite built by `any()` since we don't hand
    // out its controller.  Exercise the reverse-direction contract
    // by asserting the inputs are still live after their composite
    // aborts via chained propagation.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var a = new AbortController(); \
         var b = new AbortController(); \
         var c = AbortSignal.any([a.signal, b.signal]); \
         a.abort('src'); \
         !b.signal.aborted;"
    ));
}

#[test]
fn any_duplicate_input_fires_composite_only_once() {
    // `any([a, a])` records two entries in any_composite_map;
    // abort(a) visits the composite twice but the second call
    // short-circuits via `aborted` latch, so listeners fire once.
    let mut vm = Vm::new();
    assert_eq!(
        vm.eval(
            "var a = new AbortController(); \
             var c = AbortSignal.any([a.signal, a.signal]); \
             var fired = 0; \
             c.addEventListener('abort', function () { fired++; }); \
             a.abort('dup'); \
             fired;"
        )
        .unwrap(),
        JsValue::Number(1.0)
    );
}

#[test]
fn any_composite_map_cleaned_after_input_aborts() {
    // Once an input aborts, its `any_composite_map` entry is
    // drained (the fan-out hook removes the key) so subsequent
    // GC / lookup paths don't revisit dead state.  Empty after
    // single-input abort with no other map entries alive.
    let mut vm = Vm::new();
    vm.eval(
        "var a = new AbortController(); \
         var b = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal, b.signal]); \
         a.abort('x');",
    )
    .unwrap();
    // `a` entry is gone (fanned out); `b` entry remains (input
    // still active).  One entry remaining is the expected state.
    assert_eq!(vm.inner.any_composite_map.len(), 1);
}

#[test]
fn any_composite_map_is_weak_bookkeeping() {
    // `any_composite_map` does NOT root composite signal
    // ObjectIds.  If the user holds a reference to the composite,
    // it survives (kept alive via stack / global / etc.); if the
    // user drops all references, the composite is collectable and
    // the sweep tail prunes its entry from the map so unreachable
    // composites don't accumulate while inputs remain alive.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.b = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal, b.signal]);",
    )
    .unwrap();
    // User holds `c` → both input entries stay alive after GC.
    vm.inner.collect_garbage();
    assert_eq!(vm.inner.any_composite_map.len(), 2);

    // User drops `c` → composite collectable; sweep removes dead
    // ObjectIds from each Vec, draining both input entries when
    // the composite was their only element.
    vm.eval("globalThis.c = undefined;").unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.any_composite_map.len(),
        0,
        "dropped composite must be pruned from any_composite_map"
    );

    // Aborting an input after the composite is gone must be a
    // silent no-op (fan-out path tolerates the empty map).
    let result = vm.eval("a.abort('post-gc'); a.signal.aborted;").unwrap();
    assert!(matches!(result, JsValue::Boolean(true)));
}

#[test]
fn any_composite_with_user_ref_survives_gc_and_propagates_abort() {
    // With a live user reference, the composite is marked via the
    // normal GC path (globalThis) and abort propagation still
    // works — weak bookkeeping doesn't break the common case.
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal]);",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(vm.inner.any_composite_map.len(), 1);
    let result = vm
        .eval("a.abort('x'); c.aborted && c.reason === 'x';")
        .unwrap();
    assert!(matches!(result, JsValue::Boolean(true)));
}
