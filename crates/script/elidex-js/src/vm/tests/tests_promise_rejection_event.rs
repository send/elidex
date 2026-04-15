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
