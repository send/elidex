//! PR4b C9: `document` method/accessor tests —
//! `getElementById`, `createElement`, `createTextNode`, `body`,
//! `head`, `documentElement`, `title`, `URL`, `documentURI`,
//! `readyState`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

/// Build a minimal `<html><head><title>…</title></head><body id='b'/>`
/// tree, returning the created entities.
fn build_fixture(
    dom: &mut EcsDom,
) -> (
    elidex_ecs::Entity, // doc
    elidex_ecs::Entity, // html
    elidex_ecs::Entity, // head
    elidex_ecs::Entity, // body
    elidex_ecs::Entity, // title element
) {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let title = dom.create_element("title", Attributes::default());
    let title_text = dom.create_text("Hello World");
    let body = dom.create_element("body", {
        let mut a = Attributes::default();
        a.set("id", "b");
        a
    });
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(head, title));
    assert!(dom.append_child(title, title_text));
    assert!(dom.append_child(html, body));
    (doc, html, head, body, title)
}

// ---------------------------------------------------------------------------

#[test]
fn document_get_element_by_id_returns_wrapper() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _html, _head, body, _title) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.getElementById('b');").unwrap();
    match v {
        JsValue::Object(id) => match vm.inner.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => {
                assert_eq!(entity_bits, body.to_bits().get());
            }
            _ => panic!("expected HostObject"),
        },
        other => panic!("unexpected: {other:?}"),
    }
    vm.unbind();
}

#[test]
fn document_get_element_by_id_returns_null_for_miss() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _, _) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert!(matches!(
        vm.eval("document.getElementById('nonexistent');").unwrap(),
        JsValue::Null
    ));
    vm.unbind();
}

#[test]
fn document_get_element_by_id_returns_identity_on_repeat() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _, _) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm
        .eval("document.getElementById('b') === document.getElementById('b');")
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn document_create_element_lowercases_tag() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.createElement('DIV');").unwrap();
    let JsValue::Object(id) = v else { panic!() };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    let entity = elidex_ecs::Entity::from_bits(entity_bits).unwrap();
    let tag = {
        let dom_ref = vm.host_data().unwrap().dom();
        dom_ref
            .world()
            .get::<&elidex_ecs::TagType>(entity)
            .unwrap()
            .0
            .clone()
    };
    assert_eq!(tag, "div");
    vm.unbind();
}

#[test]
fn document_create_text_node_stores_data() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let JsValue::Object(id) = vm.eval("document.createTextNode('hello world');").unwrap() else {
        panic!()
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    let entity = elidex_ecs::Entity::from_bits(entity_bits).unwrap();
    let text = {
        let dom_ref = vm.host_data().unwrap().dom();
        dom_ref
            .world()
            .get::<&elidex_ecs::TextContent>(entity)
            .unwrap()
            .0
            .clone()
    };
    assert_eq!(text, "hello world");
    vm.unbind();
}

#[test]
fn document_document_element_returns_html() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, html, _, _, _) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Object(id) = vm.eval("document.documentElement;").unwrap() else {
        panic!()
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    assert_eq!(entity_bits, html.to_bits().get());
    vm.unbind();
}

#[test]
fn document_body_and_head_return_correct_entities() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, head, body, _) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let body_bits = match vm.eval("document.body;").unwrap() {
        JsValue::Object(id) => match vm.inner.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => entity_bits,
            _ => unreachable!(),
        },
        _ => panic!(),
    };
    assert_eq!(body_bits, body.to_bits().get());
    let head_bits = match vm.eval("document.head;").unwrap() {
        JsValue::Object(id) => match vm.inner.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => entity_bits,
            _ => unreachable!(),
        },
        _ => panic!(),
    };
    assert_eq!(head_bits, head.to_bits().get());
    vm.unbind();
}

#[test]
fn document_body_null_without_tree() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert!(matches!(vm.eval("document.body;").unwrap(), JsValue::Null));
    assert!(matches!(vm.eval("document.head;").unwrap(), JsValue::Null));
    vm.unbind();
}

#[test]
fn document_title_reads_first_title_text() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _, _) = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.title;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "Hello World"),
        _ => panic!(),
    }
    vm.unbind();
}

#[test]
fn document_title_is_empty_string_without_title_element() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.title;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), ""),
        _ => panic!(),
    }
    vm.unbind();
}

#[test]
fn document_url_reflects_navigation_state() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Default.
    let v = vm.eval("document.URL;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "about:blank"),
        _ => panic!(),
    }
    // After navigation.
    vm.eval("location.href = 'https://example.com/a';").unwrap();
    let v = vm.eval("document.URL;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "https://example.com/a"),
        _ => panic!(),
    }
    // documentURI is the same thing.
    let v = vm.eval("document.documentURI;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "https://example.com/a"),
        _ => panic!(),
    }
    vm.unbind();
}

#[test]
fn document_methods_install_per_document_entity() {
    // Regression: `document_methods_installed` must track *which*
    // document entity has been patched.  A VM-wide boolean would
    // leave `getElementById` absent on every document bound after
    // the first — observable as a missing method on the new wrapper.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();

    // First document.
    let (doc1, _, _, _, _) = build_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc1);
    }
    assert_eq!(
        vm.eval("typeof document.getElementById;").unwrap(),
        JsValue::String(vm.inner.well_known.function_type),
    );
    vm.unbind();

    // Build a second, independent document tree and bind to it.
    let doc2 = dom.create_document_root();
    let html2 = dom.create_element("html", Attributes::default());
    assert!(dom.append_child(doc2, html2));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc2);
    }
    // The new wrapper must also have `getElementById` installed.
    // Before the per-entity fix, this would evaluate to `"undefined"`.
    assert_eq!(
        vm.eval("typeof document.getElementById;").unwrap(),
        JsValue::String(vm.inner.well_known.function_type),
    );
    // And be usable — document.documentElement must resolve to html2.
    let JsValue::Object(id) = vm.eval("document.documentElement;").unwrap() else {
        panic!()
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    assert_eq!(entity_bits, html2.to_bits().get());
    vm.unbind();
}

#[test]
fn document_ready_state_is_complete() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.readyState;").unwrap();
    match v {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "complete"),
        _ => panic!(),
    }
    vm.unbind();
}
