//! PR3 C7: `EventTarget.prototype.addEventListener` integration tests.
//!
//! Drives the native via real JS (`el.addEventListener('click', fn)`)
//! and verifies the resulting state in the ECS `EventListeners`
//! component + `HostData::listener_store`.
//!
//! Compiled only under the `engine` feature.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::{EventListeners, SessionCore};

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

/// Bootstrap a Vm with HostData bound, an element wrapper installed
/// at `globalThis.el`, and the entity returned for direct DOM
/// inspection.
#[allow(unsafe_code)]
fn setup_with_element(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
) -> Entity {
    let el = dom.create_element("div", Attributes::default());
    vm.install_host_data(HostData::new());
    unsafe {
        vm.bind(session as *mut _, dom as *mut _, doc);
    }
    let wrapper_id = vm.inner.create_element_wrapper(el);
    vm.set_global("el", JsValue::Object(wrapper_id));
    el
}

/// Read the EventListeners component for `entity`, returning a clone
/// (so we can drop the world borrow before further VM work).
fn listeners_on(dom: &EcsDom, entity: Entity) -> EventListeners {
    match dom.world().get::<&EventListeners>(entity) {
        Ok(r) => (*r).clone(),
        Err(_) => EventListeners::default(),
    }
}

#[test]
fn add_event_listener_inserts_into_ecs_component() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
    let _el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
fn registered_listener_stored_in_listener_store() {
    // After addEventListener succeeds, HostData::listener_store must
    // contain the function ObjectId so dispatch can resolve it.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

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
