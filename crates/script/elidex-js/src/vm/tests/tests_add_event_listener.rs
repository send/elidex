//! PR3 C7: `EventTarget.prototype.addEventListener` integration tests.
//!
//! Drives the native via real JS (`el.addEventListener('click', fn)`)
//! and verifies the resulting state in the ECS `EventListeners`
//! component + `HostData::listener_store`.
//!
//! Compiled only under the `engine` feature.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::host_data::HostData;
use super::super::test_helpers::{listeners_on, setup_with_element};
use super::super::value::JsValue;
use super::super::Vm;

#[test]
fn add_event_listener_inserts_into_ecs_component() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    vm.eval("el.addEventListener('click', function () {});")
        .unwrap();

    let listeners = listeners_on(&dom, el);
    let entries = listeners.matching_all("click");
    assert_eq!(entries.len(), 1, "click listener must be registered");
    assert!(!entries[0].capture, "default capture is false");
    assert!(!entries[0].once);
    assert!(!entries[0].passive);

    vm.unbind();
}

#[test]
fn options_boolean_form_sets_capture() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    vm.eval("el.addEventListener('click', function () {}, true);")
        .unwrap();

    let entries = listeners_on(&dom, el)
        .matching_all("click")
        .iter()
        .map(|e| (**e).clone())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].capture, "boolean true → capture=true");

    vm.unbind();
}

#[test]
fn options_object_form_reads_capture_once_passive() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    vm.eval(
        "el.addEventListener('click', function () {}, \
         { capture: true, once: true, passive: true });",
    )
    .unwrap();

    let listeners = listeners_on(&dom, el);
    let entries = listeners.matching_all("click");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].capture);
    assert!(entries[0].once);
    assert!(entries[0].passive);

    vm.unbind();
}

#[test]
fn options_object_missing_keys_default_to_false() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    // `passive: true` only — capture/once must default to false.
    vm.eval("el.addEventListener('click', function () {}, { passive: true });")
        .unwrap();

    let listeners = listeners_on(&dom, el);
    let entries = listeners.matching_all("click");
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].capture);
    assert!(!entries[0].once);
    assert!(entries[0].passive);

    vm.unbind();
}

#[test]
fn duplicate_same_callback_same_capture_is_silently_skipped() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    // Same function identity, same capture (default false) → second
    // registration must be discarded per WHATWG DOM §2.6 step 4.
    vm.eval(
        "var f = function () {};
         el.addEventListener('click', f);
         el.addEventListener('click', f);",
    )
    .unwrap();

    let listeners = listeners_on(&dom, el);
    assert_eq!(
        listeners.matching_all("click").len(),
        1,
        "duplicate ignored"
    );

    vm.unbind();
}

#[test]
fn duplicate_check_is_per_capture_phase() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    // Same callback registered for both bubble and capture phase
    // must yield TWO entries — capture differs, so they are not
    // duplicates per §2.6 step 4.
    vm.eval(
        "var f = function () {};
         el.addEventListener('click', f, false);
         el.addEventListener('click', f, true);",
    )
    .unwrap();

    let listeners = listeners_on(&dom, el);
    let entries = listeners.matching_all("click");
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|e| e.capture));
    assert!(entries.iter().any(|e| !e.capture));

    vm.unbind();
}

#[test]
fn null_callback_is_silently_ignored() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    // §2.6 step 2: "If callback is null, then return."
    vm.eval("el.addEventListener('click', null);").unwrap();
    vm.eval("el.addEventListener('click', undefined);").unwrap();

    let listeners = listeners_on(&dom, el);
    assert!(listeners.matching_all("click").is_empty());

    vm.unbind();
}

#[test]
fn non_callable_callback_throws_type_error() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let _el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    let result = vm.eval("el.addEventListener('click', 42);");
    assert!(
        result.is_err(),
        "non-callable callback must throw TypeError"
    );

    vm.unbind();
}

#[test]
fn calls_on_non_host_object_silently_no_op() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    vm.install_host_data(HostData::new());
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&mut session as *mut _, &mut dom as *mut _, doc);
    }

    // Pull `addEventListener` off the EventTarget prototype (reached
    // via `document`'s chain since `EventTarget` global doesn't exist
    // until PR5a) and invoke it with a plain `{}` as `this`.  The
    // `entity_from_this` extractor returns None for any receiver
    // that isn't `ObjectKind::HostObject`, and the native must
    // silently no-op (return undefined) — must not panic, must not
    // throw, must not allocate any ECS component on the bogus
    // receiver.
    let result = vm.eval(
        "document.addEventListener.call(
             {},
             'click',
             function () {}
         );",
    );
    assert!(
        result.is_ok(),
        "non-HostObject receiver must silently no-op, got {result:?}"
    );

    vm.unbind();
}

#[test]
fn calls_after_unbind_are_silent_no_op() {
    // Regression: JS retains a `HostObject` wrapper across
    // `Vm::unbind()` (e.g. via `globalThis.savedDoc = document`)
    // and later invokes `addEventListener` on it.  The native used
    // to panic in `host.dom()` because the bound dom pointer is
    // null after unbind.  `entity_from_this` now early-returns None
    // when HostData is unbound, so the native silently no-ops.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let _el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    // First eval: stash document into a global, then unbind.
    vm.eval("globalThis.savedDoc = document;").unwrap();
    vm.unbind();

    // Re-bind for a fresh eval cycle, but keep the saved document
    // reference around.  Calling addEventListener through a
    // wrapper that was created in a prior bind cycle should still
    // work (entity is the same; bound state is restored).
    #[allow(unsafe_code)]
    unsafe {
        vm.bind(&mut session as *mut _, &mut dom as *mut _, doc);
    }
    let result = vm.eval("savedDoc.addEventListener('click', function () {});");
    assert!(
        result.is_ok(),
        "addEventListener on a wrapper retained across unbind/bind must succeed: {result:?}"
    );

    // Now actually unbind and try again — must silently no-op,
    // not panic.  Have to use an unsafe construction since the
    // public API doesn't expose a "unbound eval" path; reach in
    // and clear the bound pointers directly via `unbind()`, then
    // dispatch a native via the saved wrapper.  We can't `eval`
    // unbound (eval doesn't auto-bind, but document global was
    // installed by the most recent bind) — so we install a fresh
    // unbound HostData, eval, and verify no panic.
    vm.unbind();
    // After unbind, the native invocation path:
    //   savedDoc.addEventListener(...)
    //   → method lookup via prototype → native fn
    //   → entity_from_this: host_data still installed but
    //     `is_bound()` returns false → None → silent no-op
    let result = vm.eval("savedDoc.addEventListener('click', function () {});");
    assert!(
        result.is_ok(),
        "addEventListener on a HostObject after Vm::unbind() must \
         silently no-op (not panic), got {result:?}"
    );
}

#[test]
fn registered_listener_stored_in_listener_store() {
    // After addEventListener succeeds, HostData::listener_store must
    // contain the function ObjectId so dispatch can resolve it.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };

    vm.eval(
        "globalThis.handler = function () {};
         el.addEventListener('click', globalThis.handler);",
    )
    .unwrap();

    let JsValue::Object(handler_id) = vm.get_global("handler").unwrap() else {
        panic!("handler must be an Object");
    };

    let listeners = listeners_on(&dom, el);
    let listener_id = listeners.matching_all("click")[0].id;
    let stored = vm
        .host_data()
        .expect("HostData installed")
        .get_listener(listener_id);
    assert_eq!(
        stored,
        Some(handler_id),
        "listener_store entry must point at handler"
    );

    vm.unbind();
}
