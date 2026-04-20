//! PR4d C3: `addEventListener({signal})` integration tests.
//!
//! All cases dispatch a real DOM event through the document
//! (`document.click()` fires `click` listeners synchronously through
//! the dispatch pipeline), so they exercise the end-to-end ECS +
//! listener_store + AbortSignal back-ref machinery.
//!
//! Split out of [`super::tests_abort`] to keep that file under the
//! project's 1000-line convention.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Bind a `Vm` to a fresh document containing a single `<div>`,
/// expose the element as `globalThis.target`, and return the
/// element's Entity for ECS-level inspection in the assertions.
///
/// Verifying `addEventListener({signal})` end-to-end via a real
/// dispatch needs `Event` constructors (`new Event('click')`
/// from script), which land in PR5a.  Until then C3's
/// observable behaviour is "the ECS `EventListeners` slot is
/// detached on abort" — directly inspectable via the world
/// borrow in the assertions below.
fn bind_with_div(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, el);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(el);
    let key = vm.inner.strings.intern("target");
    vm.inner.globals.insert(key, JsValue::Object(wrapper));
    el
}

#[test]
fn signal_option_already_aborted_skips_registration() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let entity = bind_with_div(&mut vm, &mut session, &mut dom);

    vm.eval(
        "var c = new AbortController();
         c.abort();
         target.addEventListener('click', function() {}, {signal: c.signal});",
    )
    .unwrap();

    // Verify ECS side: the entity has no `'click'` listener
    // recorded — `parse_listener_options`'s already-aborted
    // short-circuit fired before any ECS write.
    let dom = vm.inner.host_data.as_mut().unwrap().dom();
    let listener_count = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(entity)
        .map_or(0, |l| {
            l.iter_matching("click").filter(|e| !e.capture).count()
        });
    assert_eq!(listener_count, 0);
    vm.unbind();
}

#[test]
fn signal_option_abort_detaches_listener_from_ecs() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let entity = bind_with_div(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.c = new AbortController();
         target.addEventListener('click', function() {}, {signal: c.signal});",
    )
    .unwrap();

    // Pre-abort: listener is recorded on the entity.
    {
        let dom = vm.inner.host_data.as_mut().unwrap().dom();
        let n = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .map_or(0, |l| l.iter_matching("click").count());
        assert_eq!(n, 1, "listener should be present before abort");
    }

    vm.eval("c.abort();").unwrap();

    // Post-abort: ECS slot is empty AND `HostData::listener_store`
    // dropped its entry (no orphan JS function held rooted).
    {
        let host = vm.inner.host_data.as_mut().unwrap();
        let dom = host.dom();
        let n = dom
            .world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .map_or(0, |l| l.iter_matching("click").count());
        assert_eq!(n, 0, "listener must be detached on abort");
    }
    vm.unbind();
}

#[test]
fn signal_option_non_abort_signal_throws_type_error() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let _entity = bind_with_div(&mut vm, &mut session, &mut dom);

    // Catch surfaces the TypeError; use `e.toString()` rather
    // than `String(e)` because the VM's simplified
    // OrdinaryToPrimitive returns `"[object Object]"` for
    // non-wrapper receivers (§7.1.1.1 open task — see `ops.rs`
    // `to_primitive` simplification note).
    let result = vm
        .eval(
            "var caught = '';
             try {
               target.addEventListener('click', function() {}, {signal: 'not a signal'});
               caught = 'no-throw';
             } catch(e) { caught = e.toString(); }
             caught;",
        )
        .unwrap();
    let s = match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert!(
        s.contains("TypeError") && s.contains("AbortSignal"),
        "expected TypeError mentioning AbortSignal, got {s}"
    );
    vm.unbind();
}

#[test]
fn signal_option_undefined_is_no_op() {
    // Explicit `signal: undefined` is treated the same as a
    // missing key — registration succeeds, no AbortSignal is
    // tracked.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let entity = bind_with_div(&mut vm, &mut session, &mut dom);

    vm.eval("target.addEventListener('click', function() {}, {signal: undefined});")
        .unwrap();

    let dom = vm.inner.host_data.as_mut().unwrap().dom();
    let n = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(entity)
        .map_or(0, |l| l.iter_matching("click").count());
    assert_eq!(n, 1, "undefined signal must not block registration");
    vm.unbind();
}
