//! PR3 C10: `PromiseRejectionEvent` direct dispatch tests.
//!
//! Verifies that an unhandled Promise rejection drained via the
//! microtask checkpoint surfaces as an `unhandledrejection` event on
//! the `document` global, with `.promise` and `.reason` populated, and
//! that calling `preventDefault()` suppresses the stderr fallback.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

#[allow(unsafe_code)]
fn bound_vm(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom) {
    let doc = dom.create_document_root();
    vm.install_host_data(HostData::new());
    unsafe {
        vm.bind(session as *mut _, dom as *mut _, doc);
    }
}

#[test]
fn unhandled_rejection_fires_event_with_promise_and_reason() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    // Register an unhandledrejection listener that snapshots the
    // event's `.promise` / `.reason` into globals.  Then trigger an
    // unhandled rejection.  The microtask checkpoint runs at the
    // end of `eval`, drains pending_rejections, and dispatches.
    vm.eval(
        "globalThis.captured = null;
         globalThis.captured_reason = null;
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.captured = e;
             globalThis.captured_reason = e.reason;
         });
         Promise.reject(new Error('boom'));",
    )
    .unwrap();

    // The listener must have fired during eval's terminal microtask
    // drain.  `captured` should now be the event object, and
    // `captured_reason.message === 'boom'`.
    let evt = vm
        .get_global("captured")
        .expect("captured must be assigned by listener");
    assert!(
        matches!(evt, JsValue::Object(_)),
        "captured should be the event object, got {evt:?}"
    );

    // Read .reason.message via JS.
    let msg = vm
        .eval("captured_reason.message")
        .expect("reason.message must read");
    let JsValue::String(sid) = msg else {
        panic!("expected String, got {msg:?}");
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "boom");

    vm.unbind();
}

#[test]
fn prevent_default_suppresses_stderr_fallback() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    // Registering a listener that calls preventDefault flips the
    // event's default_prevented flag — the warn fallback path is
    // then bypassed.  We can't capture stderr easily in a unit
    // test; instead verify the listener was invoked AND saw a
    // cancelable event.
    vm.eval(
        "globalThis.was_cancelable = false;
         globalThis.was_called = false;
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.was_called = true;
             globalThis.was_cancelable = e.cancelable;
             e.preventDefault();
         });
         Promise.reject('rejected');",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("was_called").unwrap(),
        JsValue::Boolean(true),
        "listener must fire on unhandled rejection"
    );
    assert_eq!(
        vm.get_global("was_cancelable").unwrap(),
        JsValue::Boolean(true),
        "PromiseRejectionEvent must be cancelable"
    );

    vm.unbind();
}

#[test]
fn rejection_event_target_is_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.target_is_document = false;
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.target_is_document = (e.target === document);
         });
         Promise.reject(0);",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("target_is_document").unwrap(),
        JsValue::Boolean(true),
        "PromiseRejectionEvent.target must equal document"
    );

    vm.unbind();
}

#[test]
fn each_listener_gets_a_fresh_event_object() {
    // Regression: dispatch_unhandled_rejection_event used to create
    // ONE event object and reuse it for every listener.  Listener A
    // mutating `e.foo = 1` would then leak into listener B's view,
    // diverging from the per-listener-rebuild semantics of regular
    // dispatch (engine.rs::call_listener).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.b_saw_foo = null;
         document.addEventListener('unhandledrejection', function (e) {
             // First listener mutates the event obj.
             e.foo = 'leaked';
         });
         document.addEventListener('unhandledrejection', function (e) {
             // Second listener observes — must see undefined since
             // each listener gets a fresh event object.
             globalThis.b_saw_foo = e.foo;
         });
         Promise.reject(new Error('rebuild-test'));",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("b_saw_foo").unwrap(),
        JsValue::Undefined,
        "second listener must see fresh event obj — `e.foo` set by \
         listener A must NOT leak into listener B"
    );

    vm.unbind();
}

#[test]
fn prevent_default_propagates_across_listeners() {
    // After the per-listener-rebuild fix, prior listener's
    // preventDefault still has to be visible (via DispatchFlags)
    // to subsequent listeners through the freshly-built event.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.b_saw_default_prevented = null;
         document.addEventListener('unhandledrejection', function (e) {
             e.preventDefault();
         });
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.b_saw_default_prevented = e.defaultPrevented;
         });
         Promise.reject('cross-listener');",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("b_saw_default_prevented").unwrap(),
        JsValue::Boolean(true),
        "second listener must observe prior listener's preventDefault \
         via DispatchFlags carry-forward"
    );

    vm.unbind();
}

#[test]
fn stop_immediate_propagation_breaks_listener_loop() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.b_fired = false;
         document.addEventListener('unhandledrejection', function (e) {
             e.stopImmediatePropagation();
         });
         document.addEventListener('unhandledrejection', function () {
             globalThis.b_fired = true;
         });
         Promise.reject('stop-immediate');",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("b_fired").unwrap(),
        JsValue::Boolean(false),
        "stopImmediatePropagation in first listener must skip subsequent ones"
    );

    vm.unbind();
}

#[test]
fn once_listener_fires_only_for_first_rejection() {
    // Regression: dispatch_unhandled_rejection_event used to drop
    // the per-listener `once` flag (collected only ListenerIds),
    // so `{once: true}` listeners would re-fire on every subsequent
    // rejection.  Mirrors WHATWG DOM §2.10 step 15: remove BEFORE
    // invoking.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.fire_count = 0;
         document.addEventListener(
             'unhandledrejection',
             function () { globalThis.fire_count += 1; },
             { once: true });
         Promise.reject('first');",
    )
    .unwrap();
    assert_eq!(
        vm.get_global("fire_count").unwrap(),
        JsValue::Number(1.0),
        "once listener fires on first rejection"
    );

    // Second rejection — the listener has been removed, so the
    // counter must NOT advance.
    vm.eval("Promise.reject('second');").unwrap();
    assert_eq!(
        vm.get_global("fire_count").unwrap(),
        JsValue::Number(1.0),
        "once listener must NOT re-fire after first invocation"
    );

    vm.unbind();
}

#[test]
fn passive_listener_cannot_prevent_default() {
    // Regression: dispatch_unhandled_rejection_event used to call
    // `create_event_object(..., passive: false)` unconditionally,
    // so `{passive: true}` listeners could still successfully
    // invoke `e.preventDefault()`.  Now the per-listener `passive`
    // flag threads through to the event obj's internal slot, where
    // `preventDefault` no-ops it.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    // Two listeners: A is passive and tries to preventDefault, B
    // observes whether the canceled flag actually flipped.
    vm.eval(
        "globalThis.b_saw_default_prevented = null;
         document.addEventListener(
             'unhandledrejection',
             function (e) { e.preventDefault(); },
             { passive: true });
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.b_saw_default_prevented = e.defaultPrevented;
         });
         Promise.reject('passive-test');",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("b_saw_default_prevented").unwrap(),
        JsValue::Boolean(false),
        "passive listener's preventDefault must be a silent no-op"
    );

    vm.unbind();
}

#[test]
fn pending_rejections_drained_after_dispatch() {
    // Smoke test for the structural fix in `process_pending_rejections`:
    // it used to `mem::take(pending_rejections)` before dispatching,
    // moving the Promise ObjectIds out of the GC root set.  An
    // alloc-triggered GC inside `dispatch_unhandled_rejection_event`
    // could then reclaim a Promise whose only reachability was
    // pending_rejections (e.g. `Promise.reject('x')` with no JS
    // reference), leaving `e.promise` pointing at a freed slot.
    //
    // The fix iterates `pending_rejections` by index without `take`,
    // then `drain(..initial_count)` at the end.  This test verifies
    // the visible behaviour: after dispatch, `pending_rejections` is
    // empty (the loop completed and drained).  We can't trivially
    // force the GC-race window here without test-only infrastructure
    // (no JS `gc()` global), so this test guards the structural
    // invariant rather than the precise UAF symptom.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.fired = false;
         document.addEventListener('unhandledrejection', function () {
             globalThis.fired = true;
         });
         (function () { Promise.reject('drain-test'); })();",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("fired").unwrap(),
        JsValue::Boolean(true),
        "listener must fire on unhandled rejection"
    );
    assert!(
        vm.inner.pending_rejections.is_empty(),
        "pending_rejections must be drained after process_pending_rejections completes"
    );

    vm.unbind();
}

#[test]
fn rejection_event_phase_is_at_target_and_current_target_is_document() {
    // Regression: `dispatch_unhandled_rejection_event` used to leave
    // the synthetic DispatchEvent's `phase` at `EventPhase::None` and
    // `current_target` at `None`.  JS observers saw `e.eventPhase ===
    // 0` even though the event was being dispatched at the document
    // target — diverging from regular dispatch (which threads
    // AT_TARGET via `script_dispatch_event_core`).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.observed_phase = -1;
         globalThis.current_is_document = false;
         globalThis.path_length = -1;
         globalThis.path_zero_is_document = false;
         document.addEventListener('unhandledrejection', function (e) {
             globalThis.observed_phase = e.eventPhase;
             globalThis.current_is_document = (e.currentTarget === document);
             var p = e.composedPath();
             globalThis.path_length = p.length;
             globalThis.path_zero_is_document = (p[0] === document);
         });
         (function () { Promise.reject('phase-test'); })();",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("observed_phase").unwrap(),
        JsValue::Number(2.0),
        "PromiseRejectionEvent.eventPhase must be AT_TARGET (2), \
         not NONE (0) — synthetic event needs spec-consistent phase"
    );
    assert_eq!(
        vm.get_global("current_is_document").unwrap(),
        JsValue::Boolean(true),
        "PromiseRejectionEvent.currentTarget must equal document"
    );
    assert_eq!(
        vm.get_global("path_length").unwrap(),
        JsValue::Number(1.0),
        "composedPath() must contain [document] (length 1)"
    );
    assert_eq!(
        vm.get_global("path_zero_is_document").unwrap(),
        JsValue::Boolean(true),
        "composedPath()[0] must be the document wrapper"
    );

    vm.unbind();
}

#[test]
fn no_listener_silently_falls_back_no_panic() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    bound_vm(&mut vm, &mut session, &mut dom);

    // No listener registered — eprintln fallback fires (we don't
    // assert on stderr here, just that the path doesn't panic).
    let res = vm.eval("Promise.reject('silent');");
    assert!(res.is_ok(), "rejection without listener must not panic");

    vm.unbind();
}
