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
fn mutation_observer_constructor_without_host_data_throws() {
    // Regression: prior implementation called `ctx.host()`
    // unconditionally, panicking when JS executed before
    // `Vm::install_host_data` (e.g. embedder ergonomics tests or
    // any pre-bind `vm.eval`).  The constructor must surface a
    // TypeError instead so `try { new MutationObserver(...) }
    // catch (e) {}` works pre-init.
    let mut vm = Vm::new();
    let err = vm
        .eval("new MutationObserver(function(){})")
        .expect_err("constructor must error pre-install_host_data");
    let err_text = format!("{err:?}");
    assert!(
        err_text.contains("host environment is not initialised"),
        "expected pre-init TypeError, got: {err_text}"
    );
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
fn mutation_observer_observe_requires_two_arguments() {
    // WebIDL: `observe(Node target, MutationObserverInit options)` —
    // both required.  Match Chrome/Firefox arg-count error message
    // before falling through to per-argument coercion errors.
    let err_zero = run_throws("var mo = new MutationObserver(function(){}); mo.observe();");
    assert!(
        err_zero.contains("2 arguments required") && err_zero.contains("only 0 present"),
        "expected '2 arguments required, but only 0 present', got: {err_zero}"
    );
    let err_one = run_throws("var mo = new MutationObserver(function(){}); mo.observe(document);");
    assert!(
        err_one.contains("2 arguments required") && err_one.contains("only 1 present"),
        "expected '2 arguments required, but only 1 present', got: {err_one}"
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
fn mutation_observer_observe_explicit_attributes_false_with_old_value_throws() {
    // WHATWG DOM §4.3.2 step 6: `attributeOldValue: true` requires
    // `attributes: true` (or absent).  Browser-aligned: Chrome /
    // Firefox throw a TypeError citing both fields.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, attributes:false, attributeOldValue:true});",
    );
    assert!(
        err.contains("'attributeOldValue'") && err.contains("'attributes'"),
        "expected attributeOldValue/attributes mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_explicit_attributes_false_with_filter_throws() {
    // §4.3.2 step 7: `attributeFilter` requires `attributes: true`
    // (or absent).
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, attributes:false, attributeFilter:['class']});",
    );
    assert!(
        err.contains("'attributeFilter'") && err.contains("'attributes'"),
        "expected attributeFilter/attributes mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_explicit_character_data_false_with_old_value_throws() {
    // §4.3.2 step 8: `characterDataOldValue: true` requires
    // `characterData: true` (or absent).
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true, characterData:false, characterDataOldValue:true});",
    );
    assert!(
        err.contains("'characterDataOldValue'") && err.contains("'characterData'"),
        "expected characterDataOldValue/characterData mismatch TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_attribute_filter_non_iterable_throws() {
    // WebIDL §3.10.20 sequence conversion: a non-iterable
    // `attributeFilter` must TypeError, not silently fall through to
    // a stale-empty filter.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributeFilter: 'class'});",
    );
    assert!(
        err.contains("'attributeFilter' is not iterable"),
        "expected attributeFilter non-iterable TypeError, got: {err}"
    );
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, {attributeFilter: 42});",
    );
    assert!(
        err.contains("'attributeFilter' is not iterable"),
        "expected attributeFilter non-iterable TypeError, got: {err}"
    );
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
fn mutation_observer_record_properties_are_readonly() {
    // WHATWG DOM §4.3.5: every `MutationRecord` member is a
    // `readonly attribute` — non-strict assignment should silently
    // fail, strict-mode assignment should TypeError.  Regression for
    // the prior `PropertyAttrs::DATA` (writable) installation.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
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
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    // Engine semantics: writes to a non-writable property throw
    // TypeError ("Cannot assign to read only property") regardless
    // of strict mode.  The test asserts the WEBIDL_RO descriptor is
    // in effect on every MutationRecord member; the prior
    // `PropertyAttrs::DATA` would silently allow the write to land
    // and `record.type === 'mutated'` would read back the new value.
    for member in [
        "type",
        "target",
        "addedNodes",
        "removedNodes",
        "previousSibling",
        "nextSibling",
        "attributeName",
        "oldValue",
    ] {
        let err = vm.eval(&format!("records[0].{member} = 'x';")).unwrap_err();
        let err_text = format!("{err:?}");
        assert!(
            err_text.contains("read only") || err_text.contains("read-only"),
            "expected read-only TypeError for `records[0].{member}`, got: {err_text}"
        );
    }
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

// --- Plan §G #17 — characterData record ----------------------------

#[test]
fn mutation_observer_delivers_character_data_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let text_node = dom.create_text("hello");
    assert!(dom.append_child(root, text_node));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {characterData:true, characterDataOldValue:true, subtree:true});",
    )
    .unwrap();

    let record = SessionRecord {
        kind: MutationKind::CharacterData,
        target: text_node,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: Some("hello".to_string()),
    };
    vm.deliver_mutation_records(std::slice::from_ref(&record));

    let length = vm.eval("records.length").unwrap();
    assert_eq!(length, JsValue::Number(1.0));
    let type_v = vm.eval("records[0].type").unwrap();
    let JsValue::String(sid) = type_v else {
        panic!("expected string, got {type_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "characterData");
    let old_v = vm.eval("records[0].oldValue").unwrap();
    let JsValue::String(sid) = old_v else {
        panic!("expected oldValue string, got {old_v:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "hello");
    vm.unbind();
}

// --- Plan §G #26 — records array survives GC during callback --------

#[test]
fn mutation_observer_records_array_survives_gc() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // Callback forces an explicit GC mid-delivery via a stress allocation
    // sequence + a direct `collect_garbage` shot from Rust below.  The
    // `push_temp_root` rooting in `build_mutation_records_array` /
    // `Vm::deliver_mutation_records` must keep the records array and its
    // embedded element wrappers alive across the cycle.
    vm.eval(
        "globalThis.recordsLen = -1; \
         globalThis.firstAddedTag = ''; \
         var mo = new MutationObserver(function(rec){ \
           /* stress allocation so any un-rooted intermediate would be \
              collectable before we read the record fields */ \
           var noise = []; \
           for (var i = 0; i < 64; i++) noise.push({i: i, s: 'x' + i}); \
           globalThis.recordsLen = rec.length; \
           globalThis.firstAddedTag = rec[0].addedNodes[0].tagName; \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("section", elidex_ecs::Attributes::default());
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
    // Force GC immediately before delivery so the rooting path is the
    // only thing holding the just-allocated records array.
    vm.inner.collect_garbage();
    vm.deliver_mutation_records(std::slice::from_ref(&record));
    // And again after, to catch any post-callback dangling references.
    vm.inner.collect_garbage();

    let len = vm.eval("recordsLen").unwrap();
    assert_eq!(len, JsValue::Number(1.0));
    let tag = vm.eval("firstAddedTag").unwrap();
    let JsValue::String(sid) = tag else {
        panic!("expected tag string, got {tag:?}")
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "SECTION");
    vm.unbind();
}

// --- Plan §H R4 #28 — re-entrancy: callback mutates registry --------

#[test]
fn mutation_observer_callback_can_re_observe_other_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let other = dom.create_element("aside", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, other));
    let other_wrapper = vm.inner.create_element_wrapper(other);
    vm.set_global("other", JsValue::Object(other_wrapper));

    // First delivery: callback adds `other` as a second observe target.
    // Verifies the registry borrow is released between iterations so a
    // re-entrant `observe` from inside the callback does not deadlock or
    // panic (Plan §L Finding 2).
    vm.eval(
        "globalThis.firstCount = 0; \
         globalThis.secondCount = 0; \
         globalThis.mo = new MutationObserver(function(rec){ \
           if (firstCount === 0) { \
             firstCount = rec.length; \
             mo.observe(other, {attributes:true}); \
           } else { \
             secondCount = rec.length; \
           } \
         }); \
         mo.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    let r1 = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r1));

    // Second delivery — `other` is now observed because the first
    // callback called `mo.observe(other, ...)`.
    let r2 = SessionRecord {
        kind: MutationKind::Attribute,
        target: other,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r2));

    let first = vm.eval("firstCount").unwrap();
    assert_eq!(first, JsValue::Number(1.0));
    let second = vm.eval("secondCount").unwrap();
    assert_eq!(
        second,
        JsValue::Number(1.0),
        "second delivery must include the re-attached target"
    );
    vm.unbind();
}

// --- Multi-observer / repeat-observe semantics -----------------------

#[test]
fn mutation_observer_two_observers_both_receive_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.aLen = 0; globalThis.bLen = 0; \
         var moA = new MutationObserver(function(rec){ aLen = rec.length; }); \
         var moB = new MutationObserver(function(rec){ bLen = rec.length; }); \
         moA.observe(root, {childList:true}); \
         moB.observe(root, {childList:true});",
    )
    .unwrap();

    let added = dom.create_element("p", elidex_ecs::Attributes::default());
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

    let a = vm.eval("aLen").unwrap();
    let b = vm.eval("bLen").unwrap();
    assert_eq!(a, JsValue::Number(1.0));
    assert_eq!(b, JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn mutation_observer_observe_replaces_existing_options_for_same_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(rec){ globalThis.records = rec; }); \
         mo.observe(root, {childList:true}); \
         mo.observe(root, {attributes:true});",
    )
    .unwrap();

    // ChildList mutation must NOT fire because the second observe
    // replaced childList:true with attributes:true (WHATWG §4.3.3
    // step 7).
    let added = dom.create_element("p", elidex_ecs::Attributes::default());
    let r1 = SessionRecord {
        kind: MutationKind::ChildList,
        target: root,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r1));
    assert!(matches!(vm.eval("records").unwrap(), JsValue::Null));

    // Attribute mutation MUST fire under the replaced options.
    let r2 = SessionRecord {
        kind: MutationKind::Attribute,
        target: root,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    vm.deliver_mutation_records(std::slice::from_ref(&r2));
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "second mutation must fire under the replacing options"
    );
    vm.unbind();
}

#[test]
fn mutation_observer_observes_multiple_distinct_targets() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let other = dom.create_element("aside", elidex_ecs::Attributes::default());
    assert!(dom.append_child(root, other));
    let other_wrapper = vm.inner.create_element_wrapper(other);
    vm.set_global("other", JsValue::Object(other_wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         var mo = new MutationObserver(function(){ calls++; }); \
         mo.observe(root, {childList:true}); \
         mo.observe(other, {attributes:true});",
    )
    .unwrap();

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
    vm.deliver_mutation_records(&[SessionRecord {
        kind: MutationKind::Attribute,
        target: other,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("data-x".to_string()),
        old_value: None,
    }]);

    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(2.0));
    vm.unbind();
}

// --- Argument validation edge cases ----------------------------------

#[test]
fn mutation_observer_observe_null_target_throws() {
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(null, {childList:true});",
    );
    assert!(
        err.contains("not of type 'Node'"),
        "expected null-target TypeError, got: {err}"
    );
}

#[test]
fn mutation_observer_observe_null_options_uses_defaults() {
    // WebIDL §3.10.7: null and undefined both yield the default-init
    // dictionary; the subsequent at-least-one-flag check then fires.
    let err = run_throws(
        "var mo = new MutationObserver(function(){}); \
         mo.observe(document, null);",
    );
    assert!(
        err.contains("at least one"),
        "null options should default-init then fail at-least-one-flag, got: {err}"
    );
}

// --- Callback exception isolation ------------------------------------

#[test]
fn mutation_observer_callback_throw_does_not_block_sibling_observer() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);

    // moA throws; moB must still fire (WHATWG §4.3.4 / §8.1.5
    // "report the exception" — does not abort sibling delivery).
    vm.eval(
        "globalThis.bRan = 0; \
         var moA = new MutationObserver(function(){ throw new Error('boom'); }); \
         var moB = new MutationObserver(function(){ bRan++; }); \
         moA.observe(root, {childList:true}); \
         moB.observe(root, {childList:true});",
    )
    .unwrap();

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

    assert_eq!(
        vm.eval("bRan").unwrap(),
        JsValue::Number(1.0),
        "sibling observer must run despite first observer throwing"
    );
    vm.unbind();
}

// --- takeRecords semantics ------------------------------------------

#[test]
fn mutation_observer_take_records_after_disconnect_is_empty() {
    let out = run("var mo = new MutationObserver(function(){}); \
         mo.observe(document, {childList:true}); \
         mo.disconnect(); \
         '' + mo.takeRecords().length;");
    assert_eq!(out, "0");
}

#[test]
fn mutation_observer_take_records_returns_fresh_array_each_call() {
    let out = run("var mo = new MutationObserver(function(){}); \
         var a = mo.takeRecords(); \
         var b = mo.takeRecords(); \
         (a !== b) ? 'fresh' : 'same';");
    assert_eq!(out, "fresh");
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
