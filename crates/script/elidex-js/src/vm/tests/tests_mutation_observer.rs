//! M4-12 #11-mutation-observer — `MutationObserver` thin VM binding tests.
//!
//! Phase C2 surface: constructor + brand check + 3 method stubs.
//! Phase C3 surface: `observe` / `disconnect` / `takeRecords` semantics.
//! Phase C4 surface: `mutation_record_to_js` + `Vm::deliver_mutation_records`.
//! Phase C5 surface: post-unbind tolerance + `Vm::unbind` cleanup.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

fn run_throws(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let err = vm.eval(script).expect_err("expected an error");
    vm.unbind();
    format!("{err:?}")
}

// --- C2 — prototype + constructor brand check -----------------------

#[test]
fn mutation_observer_prototype_installed() {
    let mut vm = Vm::new();
    assert!(
        vm.inner.mutation_observer_prototype.is_some(),
        "MutationObserver.prototype must be allocated during register_globals"
    );
    // global binding present
    assert!(vm.eval("typeof MutationObserver === 'function'").is_ok());
}

#[test]
fn mutation_observer_constructor_creates_instance() {
    let out = run("var mo = new MutationObserver(function(){}); typeof mo;");
    assert_eq!(out, "object");
}

#[test]
fn mutation_observer_constructor_requires_callable() {
    let err = run_throws("new MutationObserver(123);");
    assert!(
        err.contains("not of type 'Function'"),
        "expected MutationObserver callable TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_constructor_bare_call_throws() {
    let err = run_throws("MutationObserver(function(){});");
    assert!(
        err.contains("'new' operator"),
        "expected bare-call TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_instanceof_works() {
    let out = run("var mo = new MutationObserver(function(){}); \
         (mo instanceof MutationObserver) ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn mutation_observer_method_brand_check_disconnect() {
    let err = run_throws("MutationObserver.prototype.disconnect.call({});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_method_brand_check_take_records() {
    let err = run_throws("MutationObserver.prototype.takeRecords.call({});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_method_brand_check_observe() {
    let err =
        run_throws("MutationObserver.prototype.observe.call({}, document, {childList:true});");
    assert!(
        err.contains("Illegal invocation"),
        "expected brand-check TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_take_records_initially_empty() {
    let out = run("var mo = new MutationObserver(function(){}); \
         var r = mo.takeRecords(); \
         Array.isArray(r) + ':' + r.length;");
    assert_eq!(out, "true:0");
}

#[test]
fn mutation_observer_disconnect_returns_undefined() {
    let out = run("var mo = new MutationObserver(function(){}); \
         typeof mo.disconnect();");
    assert_eq!(out, "undefined");
}

// --- C3 — observe / init parsing / TypeErrors ----------------------

#[test]
fn mutation_observer_observe_returns_undefined() {
    let out = run("var mo = new MutationObserver(function(){}); \
         typeof mo.observe(document, {childList:true});");
    assert_eq!(out, "undefined");
}

#[test]
fn mutation_observer_observe_requires_at_least_one_flag() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {});",
    );
    assert!(
        err.contains("at least one"),
        "expected 'at least one' TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_target_must_be_node() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe({}, {childList:true});",
    );
    assert!(
        err.contains("not of type 'Node'"),
        "expected non-Node TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attributes_implicit_via_old_value() {
    // attributeOldValue alone should be sufficient (spec §4.3.2 step 3).
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributeOldValue:true}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn mutation_observer_observe_character_data_implicit_via_old_value() {
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {characterDataOldValue:true}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

#[test]
fn mutation_observer_observe_attribute_filter_implies_attributes() {
    let out = run("var mo = new MutationObserver(function(){}); \
         try { mo.observe(document, {attributeFilter: ['class']}); 'ok' } \
         catch(e) { 'threw: ' + e.message; }");
    assert_eq!(out, "ok");
}

// --- C4 — delivery (`Vm::deliver_mutation_records`) ------------------

use elidex_script_session::{MutationKind, MutationRecord as SessionRecord};

/// Build a typical document tree with a `<div id="root">` returned for
/// targeted mutations, and bind the VM.  Exposes the root-element
/// JS wrapper as `globalThis.root`.
fn setup_with_root(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> (elidex_ecs::Entity, elidex_ecs::Entity) {
    let doc = build_doc(dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let root = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, root));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(wrapper));
    (doc, root)
}

#[test]
fn mutation_observer_delivers_child_list_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec, obs){ \
           globalThis.records = rec; \
           globalThis.observerArg = obs; \
           globalThis.mo = mo; \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    // Simulate a child append.
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    // Inspect the resulting JS records array.
    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let type_v = vm.eval("records[0].type").unwrap();
    let JsValue::String(sid) = type_v else {
        panic!("expected string, got {type_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "childList");

    let added_count = vm.eval("records[0].addedNodes.length").unwrap();
    assert_eq!(added_count, JsValue::Number(1.0));
    let target_eq = vm.eval("records[0].target === root").unwrap();
    assert_eq!(target_eq, JsValue::Boolean(true));
    let observer_eq = vm.eval("observerArg === mo").unwrap();
    assert_eq!(observer_eq, JsValue::Boolean(true));
    vm.unbind();
}

#[test]
fn mutation_observer_delivers_attribute_record_with_old_value() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true, attributeOldValue:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: Some("old-class".to_string()),
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let attr_name = vm.eval("records[0].attributeName").unwrap();
    let JsValue::String(sid) = attr_name else {
        panic!("expected attribute_name string, got {attr_name:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "class");

    let old_value = vm.eval("records[0].oldValue").unwrap();
    let JsValue::String(sid) = old_value else {
        panic!("expected old_value string, got {old_value:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "old-class");
    vm.unbind();
}

#[test]
fn mutation_observer_attribute_filter_excludes_unmatched() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributeFilter:['class']});",
    )
    .unwrap();

    let id_record = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("id".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&id_record));

    // No records should have been queued because `id` doesn't match the filter.
    let records_v = vm.eval("records").unwrap();
    assert!(matches!(records_v, JsValue::Null));
    vm.unbind();
}

#[test]
fn mutation_observer_subtree_observes_descendant() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let child = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, child));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true, subtree:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records ? records.length : 0").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn mutation_observer_subtree_off_skips_descendant() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let child = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, child));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let records_v = vm.eval("records").unwrap();
    assert!(matches!(records_v, JsValue::Null));
    vm.unbind();
}

#[test]
fn mutation_observer_take_records_drains_pending() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){ /* never called via takeRecords */ }); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

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
    // Notify but do NOT drive the callback; instead use takeRecords()
    // from JS.  Mimic the no-callback branch by feeding the record
    // straight into the registry (skip deliver, which would call the
    // callback).
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&record, &|_, _| false);

    let drained = vm.eval("mo.takeRecords().length").unwrap();
    assert_eq!(drained, JsValue::Number(1.0));
    let drained_again = vm.eval("mo.takeRecords().length").unwrap();
    assert_eq!(drained_again, JsValue::Number(0.0));
    vm.unbind();
}

#[test]
fn mutation_observer_disconnect_clears_pending() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.mo = new MutationObserver(function(){ calls++; }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    // Notify but don't deliver yet; then disconnect.
    vm.inner
        .host_data
        .as_deref_mut()
        .unwrap()
        .mutation_observers
        .notify(&record, &|_, _| false);
    vm.eval("mo.disconnect();").unwrap();
    vm.deliver_mutation_records(&[]);
    let calls = vm.eval("calls").unwrap();
    assert_eq!(calls, JsValue::Number(0.0));
    vm.unbind();
}

// --- C5 — post-unbind tolerance + cleanup -----------------------------

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
    let rebound_root = build_doc(&mut rebound_dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(
            &mut vm,
            &mut rebound_session,
            &mut rebound_dom,
            rebound_root,
        );
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
fn mutation_observer_unbind_clears_callbacks() {
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
    assert_eq!(host.mutation_observer_callbacks.len(), 0);
    assert_eq!(host.mutation_observer_instances.len(), 0);
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

#[test]
fn mutation_observer_added_nodes_are_element_wrappers() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    let record = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let added_node_kind = vm.eval("typeof records[0].addedNodes[0]").unwrap();
    let JsValue::String(sid) = added_node_kind else {
        panic!("expected string, got {added_node_kind:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "object");

    // tagName accessor confirms it's an Element wrapper, not a raw number/null.
    let tag = vm.eval("records[0].addedNodes[0].tagName").unwrap();
    let JsValue::String(sid) = tag else {
        panic!("expected tagName string, got {tag:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "SPAN");
    vm.unbind();
}
