//! PR4f C2: `document.create*` owner wiring +
//! `Node.prototype.ownerDocument` backed by the ECS
//! `AssociatedDocument` component.
//!
//! The observable bug (PR4e R12 F1) was that a cloned Document's
//! `createElement(...)` reported the *bound* document as its
//! `ownerDocument`, not the clone.  These tests lock in the corrected
//! behaviour.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

fn build_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

#[test]
fn create_element_owner_document_is_receiver_document() {
    // Baseline: `document.createElement(...).ownerDocument` reports
    // the bound document even before insertion.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.createElement('div').ownerDocument === document;")
            .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn create_text_owner_document_is_receiver_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.createTextNode('hi').ownerDocument === document;")
            .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn create_comment_owner_document_is_receiver_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.createComment('c').ownerDocument === document;")
            .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn create_document_fragment_owner_document_is_receiver_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.createDocumentFragment().ownerDocument === document;")
            .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_document_create_text_reports_clone_not_bound_document() {
    // Same shape as `createElement` clone test, for the
    // `createTextNode` migration — pins the bridge `this`-passing
    // contract so a future regression that drops `Some(this)` in
    // the handler doesn't slip past the test suite.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval(
            "var cloneDoc = document.cloneNode(true);\
             var t = cloneDoc.createTextNode('hi');\
             t.ownerDocument === cloneDoc && t.ownerDocument !== document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_document_create_comment_reports_clone_not_bound_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval(
            "var cloneDoc = document.cloneNode(true);\
             var c = cloneDoc.createComment('c');\
             c.ownerDocument === cloneDoc && c.ownerDocument !== document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_document_create_document_fragment_reports_clone_not_bound_document() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval(
            "var cloneDoc = document.cloneNode(true);\
             var f = cloneDoc.createDocumentFragment();\
             f.ownerDocument === cloneDoc && f.ownerDocument !== document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_document_set_title_anchors_synthesised_nodes_to_clone() {
    // PR #156 R1 + R7: `cloneDoc.title = 'x'` must anchor the
    // synthesised `<title>` + text-node children to `cloneDoc`, not
    // the bound document.  Without this end-to-end test, a future
    // bridge regression that drops the cloneDoc receiver in the
    // VM-side dispatch would silently revert to bound-document
    // anchoring and slip past the suite.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    // Need a `<head>` for SetTitle to land — `build_fixture`'s
    // body-only tree makes the setter a no-op.
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval(
            "var cloneDoc = document.cloneNode(true);\
             cloneDoc.title = 'X';\
             var titleEl = cloneDoc.head.firstChild;\
             titleEl !== null && \
             titleEl.ownerDocument === cloneDoc && \
             titleEl.ownerDocument !== document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_document_create_element_reports_clone_not_bound_document() {
    // PR4e R12 F1 — the observable bug we're fixing.
    //
    // `document.cloneNode(true).createElement("p").ownerDocument`
    // must point at the *cloned* document, not the bound global.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Identity check: `cloneDoc.createElement(...).ownerDocument` ===
    // cloneDoc, and !== the bound document.
    assert!(matches!(
        vm.eval(
            "var cloneDoc = document.cloneNode(true);\
             var el = cloneDoc.createElement('p');\
             el.ownerDocument === cloneDoc && el.ownerDocument !== document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn cloned_subtree_owner_document_matches_src_subtree() {
    // Non-Document clone: descendants inherit the *src* subtree's
    // owner document, not a new one.  `el.cloneNode(true)` on an
    // element rooted in the main document therefore keeps every
    // cloned descendant pointing at `document`.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval(
            "var root = document.createElement('section');\
             root.appendChild(document.createElement('h1'));\
             var copy = root.cloneNode(true);\
             copy.ownerDocument === document && copy.firstChild.ownerDocument === document;"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn associated_document_set_by_create_native_observable_via_ecs() {
    // White-box: after `document.createElement("span")` the ECS
    // AssociatedDocument component must resolve directly to the bound
    // document entity without falling back to the tree-root walk.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let handle = vm.eval("document.createElement('span');").unwrap();
    let JsValue::Object(id) = handle else {
        panic!("expected HostObject wrapper")
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        panic!("expected HostObject")
    };
    vm.unbind();

    let entity = elidex_ecs::Entity::from_bits(entity_bits).unwrap();
    assert_eq!(dom.get_associated_document(entity), Some(doc));
    assert_eq!(dom.owner_document(entity), Some(doc));
}
