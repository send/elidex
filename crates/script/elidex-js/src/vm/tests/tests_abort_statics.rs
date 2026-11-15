//! `AbortSignal.abort` / `.timeout` / `.any` static factory tests
//! (DOM §3.2 Interface AbortSignal — the static method steps).
//!
//! Split out of [`super::tests_abort`] to keep that file under the
//! project's 1000-line convention.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

/// A `Vm` with an (unbound) `HostData` installed so an `AbortSignal`'s
/// unified `vm_event_listeners` callbacks (in `HostData::listener_store`)
/// resolve without a DOM bind (see `super::tests_abort::new_vm`).
fn new_vm() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
}

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
    let mut vm = new_vm();
    assert!(eval_bool(&mut vm, "AbortSignal.abort().aborted;"));
}

#[test]
fn abort_signal_abort_default_reason_is_dom_exception_abort_error() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var s = AbortSignal.abort(); \
         s.reason instanceof DOMException && s.reason.name === 'AbortError';"
    ));
}

#[test]
fn abort_signal_abort_preserves_custom_reason() {
    let mut vm = new_vm();
    // Non-undefined reason passthrough — matches the
    // `controller.abort('custom')` path.
    assert_eq!(
        eval_string(&mut vm, "AbortSignal.abort('boom').reason;"),
        "boom"
    );
}

#[test]
fn abort_signal_timeout_returns_not_yet_aborted_signal() {
    let mut vm = new_vm();
    // Immediately after `timeout`, the signal must not be
    // aborted yet — the timer only fires on the next
    // `drain_timers` call.
    assert!(!eval_bool(&mut vm, "AbortSignal.timeout(100).aborted;"));
}

#[test]
fn abort_signal_timeout_fires_on_drain() {
    use std::time::{Duration, Instant};
    let mut vm = new_vm();
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
    let mut vm = new_vm();
    assert!(!eval_bool(&mut vm, "AbortSignal.any([]).aborted;"));
}

#[test]
fn abort_signal_any_already_aborted_input_propagates() {
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    let mut vm = new_vm();
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
    // DOM §3.2 create a dependent abort signal step 2: "For each signal of
    // signals: if signal is aborted, then set resultSignal's abort reason to
    // signal's abort reason and return resultSignal." — the
    // composite is sync-aborted at construction time with the
    // *first* already-aborted input's reason, not whichever
    // iterable entry comes first unconditionally.
    let mut vm = new_vm();
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
    // `c2 = any([c1, c])` where `c1 = any([a, b])`.  `c2` is sourced
    // directly on `a`, `b` and `c` (create a dependent abort signal step
    // 4.2), so aborting `a` reaches `c1` and `c2` in one plan.
    let mut vm = new_vm();
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
    // A composite aborting must NOT propagate to its inputs — the
    // graph has one direction: step 4 of "create a dependent abort
    // signal" appends the result to the source's dependent signals,
    // never the source to the result's.  Composites are EventTargets,
    // not controllers (there is no public `abort()` on `AbortSignal`),
    // so the reverse direction is exercised by asserting a sibling
    // input is still live after the composite aborts through the
    // other input.
    let mut vm = new_vm();
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
    // `any([a, a])`: the source / dependent signals are Infra sets
    // ("append" of a present member is a no-op), so `a` records one
    // dependent and the result one source; the listener fires once.
    let mut vm = new_vm();
    assert_eq!(
        vm.eval(
            "globalThis.a = new AbortController(); \
             globalThis.c = AbortSignal.any([a.signal, a.signal]); \
             var fired = 0; \
             c.addEventListener('abort', function () { fired++; }); \
             a.abort('dup'); \
             fired;"
        )
        .unwrap(),
        JsValue::Number(1.0)
    );
    let a_sig = match vm.eval("a.signal;").unwrap() {
        JsValue::Object(id) => id,
        other => panic!("{other:?}"),
    };
    let c_sig = match vm.eval("c;").unwrap() {
        JsValue::Object(id) => id,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        vm.inner.abort_signal_states[&c_sig].links.source_signals(),
        &[a_sig],
        "one source, not two"
    );
}

#[test]
fn any_dependents_emptied_after_source_aborts() {
    // "signal abort" step 4 plans every dependent and the source's
    // dependent signals set is emptied afterwards (it is never read again),
    // while an untouched source keeps its link.
    let mut vm = new_vm();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.b = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal, b.signal]); \
         a.abort('x');",
    )
    .unwrap();
    let id = |vm: &mut Vm, e: &str| match vm.eval(e).unwrap() {
        JsValue::Object(id) => id,
        other => panic!("{other:?}"),
    };
    let a_sig = id(&mut vm, "a.signal;");
    let b_sig = id(&mut vm, "b.signal;");
    let c_sig = id(&mut vm, "c;");
    let states = &vm.inner.abort_signal_states;
    assert!(states[&a_sig].links.dependent_signals().is_empty());
    assert_eq!(states[&b_sig].links.dependent_signals(), &[c_sig]);
}

#[test]
fn any_composite_with_user_ref_survives_gc_and_propagates_abort() {
    // With a live user reference, the composite is marked via the
    // normal GC path (globalThis) and abort propagation still
    // works — the weak sets don't break the common case.
    let mut vm = new_vm();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal]);",
    )
    .unwrap();
    vm.inner.collect_garbage();
    let a_sig = match vm.eval("a.signal;").unwrap() {
        JsValue::Object(id) => id,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        vm.inner.abort_signal_states[&a_sig]
            .links
            .dependent_signals()
            .len(),
        1,
        "the held dependent stays linked across a collection"
    );
    let result = vm
        .eval("a.abort('x'); c.aborted && c.reason === 'x';")
        .unwrap();
    assert!(matches!(result, JsValue::Boolean(true)));
}

#[test]
fn timeout_signal_survives_gc_via_keepalive_seam() {
    // S5-3a: the `AbortSignal.timeout` root moved from `mark_roots` pass (j)
    // onto the keepalive seam (`gc::keepalive::keepalive_survivors`) —
    // behavior-neutral. A pending-timeout signal whose only anchor is
    // `pending_timeout_signals` (no retained JS reference) must still survive
    // GC so the timer can fire it.
    let mut vm = new_vm();
    vm.eval("AbortSignal.timeout(100000);").unwrap(); // result not stored
    assert_eq!(
        vm.inner.pending_timeout_signals.len(),
        1,
        "the timeout registration must exist before GC",
    );
    let signal_id = *vm
        .inner
        .pending_timeout_signals
        .values()
        .next()
        .expect("one pending timeout signal");
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[signal_id.0 as usize].is_some(),
        "a pending-timeout signal with no JS reference must survive GC (keepalive seam)",
    );
    assert_eq!(
        vm.inner.pending_timeout_signals.len(),
        1,
        "the surviving signal must keep its timeout registration",
    );
}
