//! PR3 C8: `EventTarget.prototype.removeEventListener` integration tests.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::{EventListeners, SessionCore};

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::Vm;

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

fn listeners_on(dom: &EcsDom, entity: Entity) -> EventListeners {
    match dom.world().get::<&EventListeners>(entity) {
        Ok(r) => (*r).clone(),
        Err(_) => EventListeners::default(),
    }
}

#[test]
fn remove_drops_matching_listener() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "globalThis.h = function () {};
         el.addEventListener('click', globalThis.h);",
    )
    .unwrap();
    assert_eq!(listeners_on(&dom, el).matching_all("click").len(), 1);

    vm.eval("el.removeEventListener('click', globalThis.h);")
        .unwrap();
    assert!(
        listeners_on(&dom, el).matching_all("click").is_empty(),
        "removeEventListener must clear the matching entry"
    );

    vm.unbind();
}

#[test]
fn remove_also_clears_listener_store_entry() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "globalThis.h = function () {};
         el.addEventListener('click', globalThis.h);",
    )
    .unwrap();
    let listener_id = listeners_on(&dom, el).matching_all("click")[0].id;

    vm.eval("el.removeEventListener('click', globalThis.h);")
        .unwrap();

    assert_eq!(
        vm.host_data().unwrap().get_listener(listener_id),
        None,
        "listener_store entry must be cleared"
    );

    vm.unbind();
}

#[test]
fn remove_capture_phase_only_affects_capture_listener() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "globalThis.h = function () {};
         el.addEventListener('click', globalThis.h, false);
         el.addEventListener('click', globalThis.h, true);",
    )
    .unwrap();
    assert_eq!(listeners_on(&dom, el).matching_all("click").len(), 2);

    // Remove the capture-phase listener only — bubble-phase entry survives.
    vm.eval("el.removeEventListener('click', globalThis.h, true);")
        .unwrap();
    let entries = listeners_on(&dom, el)
        .matching_all("click")
        .iter()
        .map(|e| (**e).clone())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].capture, "remaining listener is the bubble one");

    vm.unbind();
}

#[test]
fn remove_with_unmatching_callback_is_silent_no_op() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "el.addEventListener('click', function () {});
         el.removeEventListener('click', function () {});",
    )
    .unwrap();

    // Removed function is a different identity — original stays.
    assert_eq!(listeners_on(&dom, el).matching_all("click").len(), 1);

    vm.unbind();
}

#[test]
fn remove_finds_correct_entry_among_multiple_same_type_capture() {
    // Regression: two listeners on the same (type, capture) tuple
    // with DIFFERENT callbacks.  removeEventListener for the second
    // listener must locate it specifically — earlier code picked the
    // first (type, capture) match and bailed when its callback didn't
    // match the candidate, missing the actually-matching later entry.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "globalThis.h1 = function () {};
         globalThis.h2 = function () {};
         el.addEventListener('click', globalThis.h1);
         el.addEventListener('click', globalThis.h2);",
    )
    .unwrap();
    assert_eq!(listeners_on(&dom, el).matching_all("click").len(), 2);

    // Remove h2 specifically.  h1 must remain.
    vm.eval("el.removeEventListener('click', globalThis.h2);")
        .unwrap();

    let listeners = listeners_on(&dom, el);
    let entries = listeners.matching_all("click");
    assert_eq!(entries.len(), 1, "h1 must remain after removing h2");
    let surviving_id = entries[0].id;

    // Cross-check listener_store: the surviving entry must point at h1.
    let JsValue::Object(h1_id) = vm.get_global("h1").unwrap() else {
        panic!("h1 must be an Object");
    };
    let stored = vm.host_data().unwrap().get_listener(surviving_id);
    assert_eq!(stored, Some(h1_id), "surviving listener must be h1, not h2");

    vm.unbind();
}

#[test]
fn remove_does_not_read_once_or_passive_from_options() {
    // WHATWG DOM §2.7.7 only flattens `capture` for removal — reading
    // `once` / `passive` would fire user getters / Proxy traps that
    // browsers don't trigger.  Use getter-equipped options bag to
    // detect the unwanted reads.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let _el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "globalThis.once_read = false;
         globalThis.passive_read = false;
         globalThis.capture_read = false;
         var opts = {
             get capture() { globalThis.capture_read = true; return false; },
             get once() { globalThis.once_read = true; return false; },
             get passive() { globalThis.passive_read = true; return false; },
         };
         var fn = function () {};
         el.addEventListener('click', fn);
         // Reset sentinels so addEventListener's reads don't pollute
         // the signal.
         globalThis.once_read = false;
         globalThis.passive_read = false;
         globalThis.capture_read = false;
         el.removeEventListener('click', fn, opts);",
    )
    .unwrap();

    assert_eq!(
        vm.get_global("capture_read").unwrap(),
        JsValue::Boolean(true),
        "removeEventListener MUST read .capture from options bag"
    );
    assert_eq!(
        vm.get_global("once_read").unwrap(),
        JsValue::Boolean(false),
        "removeEventListener must NOT read .once from options bag \
         (WHATWG DOM §2.7.7 only flattens capture)"
    );
    assert_eq!(
        vm.get_global("passive_read").unwrap(),
        JsValue::Boolean(false),
        "removeEventListener must NOT read .passive from options bag"
    );

    vm.unbind();
}

#[test]
fn remove_with_null_callback_is_silent_no_op() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = setup_with_element(&mut vm, &mut session, &mut dom, doc);

    vm.eval(
        "el.addEventListener('click', function () {});
         el.removeEventListener('click', null);
         el.removeEventListener('click', undefined);",
    )
    .unwrap();
    assert_eq!(listeners_on(&dom, el).matching_all("click").len(), 1);

    vm.unbind();
}
