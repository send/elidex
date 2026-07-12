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

// `rebound_doc` (the document-root entity) and `rebound_dom` (the
// EcsDom) are both keyed by the rebind operation; the names are
// intentionally near-twin to mirror that pairing.  R1 PR #168
// renamed `rebound_root` → `rebound_doc` for clarity (it's a doc-
// root entity, not the mutation root element), which moved the
// suffix from `_root` to `_doc` and made it edit-distance-1 from
// `_dom`.  Allow the resulting `similar_names` trip rather than
// re-renaming through a third spelling.
#[allow(clippy::similar_names)]
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
    // Only `clear_pending_records` drains the per-observer record
    // queues (which hold `Entity` refs); observation targets are
    // world-scoped `MutationObservedBy` components needing no scrub.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval("globalThis.mo = new MutationObserver(function(){});")
        .unwrap();
    let host = vm.inner.host_data.as_deref().unwrap();
    assert_eq!(host.mutation_observer_bindings.len(), 1);
    vm.unbind();
    let host = vm.inner.host_data.as_deref().unwrap();
    assert_eq!(
        host.mutation_observer_bindings.len(),
        1,
        "binding must persist across unbind so retained `mo` can re-observe"
    );
}

#[test]
fn mutation_observer_unbind_drains_pending_records() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(root, {childList:true, attributes:true, characterData:true, subtree:true});",
    )
    .unwrap();

    // Queue a pending record, then unbind.
    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
        parent_was_connected: false,
    };
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&dom, &record);
    assert!(
        vm.inner
            .host_data
            .as_deref()
            .unwrap()
            .mutation_observers
            .has_pending_records(),
        "record should be queued before unbind"
    );

    vm.unbind();

    // `unbind` drains the per-observer record queues (they hold `Entity`
    // refs from the outgoing world).  The observation itself is a
    // `MutationObservedBy` component scoped to `dom`'s world — despawned
    // with that world on rebind — so no Entity-index-collision can occur
    // and no manual target scrub is needed.
    assert!(
        !vm.inner
            .host_data
            .as_deref()
            .unwrap()
            .mutation_observers
            .has_pending_records(),
        "pending records must be drained on unbind"
    );
}

// --- Rebind to same DOM ---------------------------------------------

#[test]
fn mutation_observer_methods_after_unbind_then_rebind_to_same_dom() {
    // Retained `mo` across `unbind()` then `bind(same_doc)` — observer
    // IDs persist in the registry (`clear_pending_records` only drains
    // pending records), so a fresh `observe` after rebind must work
    // end-to-end.
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
        parent_was_connected: false,
    }]);
    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}
