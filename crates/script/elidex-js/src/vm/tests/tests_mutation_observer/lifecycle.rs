//! Phase C5 — post-unbind tolerance + `Vm::unbind` cleanup, plus
//! the "rebind to the same DOM" scenario.
//!
//! Companion to [`super::setup`] (constructor + observe init parsing)
//! and [`super::delivery`] (record delivery + later additions).

use elidex_ecs::EcsDom;
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord, SessionCore};

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{build_doc, setup_with_root};

#[test]
fn mutation_observer_methods_after_unbind_do_not_panic() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();
    vm.unbind();

    // Re-bind to a fresh DOM and call methods on the retained `mo`.
    let mut rebound_session = SessionCore::new();
    let mut rebound_dom = EcsDom::new();
    let rebound_doc = build_doc(&mut rebound_dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut rebound_session, &mut rebound_dom, rebound_doc);
    }
    let r = vm
        .eval("typeof mo.disconnect() + ':' + typeof mo.takeRecords() + ':' + mo.takeRecords().length")
        .unwrap();
    let JsValue::String(sid) = r else {
        panic!("expected string, got {r:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "undefined:object:0");
    vm.unbind();
}

#[test]
fn mutation_observer_observe_after_unbind_does_not_panic() {
    // Regression: `observe()` previously ran `require_target_node`
    // and `parse_mutation_observer_init` BEFORE the `host_if_bound`
    // early-return, so an unbound retained `mo` calling
    // `mo.observe(retained_target, options)` would assert via
    // `ctx.host().dom()` inside `node_proto::require_node_arg`.
    // The contract documented at the top of `host/mutation_observer.rs`
    // is that all three natives no-op when unbound.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         globalThis.savedRoot = root;",
    )
    .unwrap();
    vm.unbind();

    // Call `observe(target, options)` while unbound — must no-op,
    // not panic via the `HostData accessed while unbound` assertion.
    let r = vm
        .eval("typeof mo.observe(savedRoot, {childList:true})")
        .unwrap();
    let JsValue::String(sid) = r else {
        panic!("expected string, got {r:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "undefined");
}

#[test]
fn mutation_observer_unbind_retains_callback_maps() {
    // Inverse contract from initial Phase C5 sketch: callbacks +
    // instance wrappers persist across `unbind()` so a retained
    // `mo` reference can re-observe after a `bind()` to the same
    // (or another) DOM and still have its callback fire.  The maps
    // are keyed by VM-monotonic `observer_id`, not by `Entity` or
    // recycled `ObjectId`, so cross-DOM aliasing cannot apply.
    // Only `clear_all_targets` drains target lists + record queues
    // (Entity-keyed state, where aliasing IS a risk).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval("globalThis.mo = new MutationObserver(function(){});")
        .unwrap();
    let host = vm.inner.host_data.as_deref().unwrap();
    assert_eq!(host.mutation_observer_callbacks.len(), 1);
    assert_eq!(host.mutation_observer_instances.len(), 1);
    vm.unbind();
    let host = vm.inner.host_data.as_deref().unwrap();
    assert_eq!(
        host.mutation_observer_callbacks.len(),
        1,
        "callbacks must persist across unbind so retained `mo` can re-observe"
    );
    assert_eq!(
        host.mutation_observer_instances.len(),
        1,
        "instance wrapper must persist across unbind"
    );
}

#[test]
fn mutation_observer_unbind_drains_registry_targets() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(root, {childList:true, attributes:true, characterData:true, subtree:true});",
    )
    .unwrap();
    vm.unbind();

    // Notify against an entity that would have matched: registry
    // must report no matches because targets were cleared.
    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&record, &|_, _| true);
    assert!(
        !vm.inner
            .host_data
            .as_deref()
            .unwrap()
            .mutation_observers
            .has_pending_records(),
        "registry must be empty after unbind"
    );
}

// --- Rebind to same DOM ---------------------------------------------

#[test]
fn mutation_observer_methods_after_unbind_then_rebind_to_same_dom() {
    // Retained `mo` across `unbind()` then `bind(same_doc)` — observer
    // IDs persist in the registry (`clear_all_targets` only drains
    // targets), so a fresh `observe` after rebind must work end-to-end.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.mo = new MutationObserver(function(){ calls++; });",
    )
    .unwrap();
    vm.unbind();

    // Rebind to the same dom + doc.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(wrapper));

    vm.eval("mo.observe(root, {childList:true});").unwrap();
    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }]);
    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}
