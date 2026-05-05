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

/// Richer fixture for querySelector / getElementsBy* tests:
/// ```text
/// doc > html > head > title("Hello World")
///             > body#b > div.box#target > span
///                      > p.box.highlight
/// ```
fn build_query_fixture(
    dom: &mut EcsDom,
) -> (
    elidex_ecs::Entity, // doc
    elidex_ecs::Entity, // div.box#target
    elidex_ecs::Entity, // span
    elidex_ecs::Entity, // p.box.highlight
) {
    let (doc, _html, _head, body, _title) = build_fixture(dom);
    let div = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("id", "target");
        a.set("class", "box");
        a
    });
    let span = dom.create_element("span", Attributes::default());
    let p = dom.create_element("p", {
        let mut a = Attributes::default();
        a.set("class", "box highlight");
        a
    });
    assert!(dom.append_child(body, div));
    assert!(dom.append_child(div, span));
    assert!(dom.append_child(body, p));
    (doc, div, span, p)
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
fn document_get_element_by_id_ignores_non_descendant() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _, _) = build_fixture(&mut dom);

    // Create an entity with id="orphan" that is NOT a child of the
    // document tree — getElementById must not find it.
    let _orphan = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("id", "orphan");
        a
    });

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert!(matches!(
        vm.eval("document.getElementById('orphan');").unwrap(),
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
fn document_create_comment_stores_data_and_reports_node_type() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // nodeType 8 = Comment per WHATWG §4.4.
    let JsValue::Number(node_type) = vm.eval("document.createComment('hi').nodeType;").unwrap()
    else {
        panic!()
    };
    assert!((node_type - 8.0).abs() < f64::EPSILON);

    // data field survives round trip (via nodeValue, which reads
    // CommentData on Node.prototype).
    let JsValue::Object(id) = vm.eval("document.createComment('world');").unwrap() else {
        panic!()
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    let entity = elidex_ecs::Entity::from_bits(entity_bits).unwrap();
    let data = {
        let dom_ref = vm.host_data().unwrap().dom();
        dom_ref
            .world()
            .get::<&elidex_ecs::CommentData>(entity)
            .unwrap()
            .0
            .clone()
    };
    assert_eq!(data, "world");
    vm.unbind();
}

#[test]
fn document_create_document_fragment_reports_node_type_and_no_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // nodeType 11 = DocumentFragment.
    let JsValue::Number(node_type) = vm
        .eval("document.createDocumentFragment().nodeType;")
        .unwrap()
    else {
        panic!()
    };
    assert!((node_type - 11.0).abs() < f64::EPSILON);

    // Freshly created fragment has no parent.
    let JsValue::Null = vm
        .eval("document.createDocumentFragment().parentNode;")
        .unwrap()
    else {
        panic!("expected null parentNode for fresh fragment");
    };
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
fn document_title_preserves_non_ascii_whitespace_through_bridge() {
    // End-to-end pin for WHATWG HTML §dom-document-title "strip and
    // collapse ASCII whitespace": NBSP / ideographic space are not in
    // the spec's whitespace set and must round-trip through the
    // VM-handler bridge unchanged.  The handler-layer test
    // (`title_get_preserves_non_ascii_whitespace` in elidex-dom-api)
    // alone wouldn't catch a future bridge ToString-coercion or
    // intern-table bug that re-applied Unicode collapsing.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let title = dom.create_element("title", Attributes::default());
    let title_text = dom.create_text("a\u{00A0}b\u{3000}c");
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(head, title));
    assert!(dom.append_child(title, title_text));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::String(id) = vm.eval("document.title;").unwrap() else {
        panic!("title getter must return a string")
    };
    assert_eq!(vm.get_string(id), "a\u{00A0}b\u{3000}c");
    vm.unbind();
}

#[test]
fn document_active_element_picks_first_body_or_frameset_in_document_order() {
    // Guard against a two-pass fallback that scans for `<body>`
    // first and `<frameset>` second — that would resolve a later
    // `<body>` over an earlier `<frameset>`, disagreeing with
    // `document.body`'s WHATWG "first body-or-frameset child" rule.
    // Build `<html><frameset/><body/></html>` (frameset first) and
    // verify both accessors agree on the frameset.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let frameset = dom.create_element("frameset", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, frameset));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // `document.body` and `document.activeElement` must agree, and
    // both must be the frameset (which appears first under html).
    assert!(matches!(
        vm.eval(
            "document.body === document.activeElement && \
             document.body.tagName.toLowerCase() === 'frameset';"
        )
        .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn document_active_element_falls_back_to_frameset_when_no_body() {
    // After the body-frameset alignment in R4, `document.body`
    // returns the `<frameset>` element when no `<body>` exists.
    // The `activeElement` fallback chain must agree — otherwise a
    // frameset document observes
    // `document.body !== document.activeElement` when nothing is
    // focused.  Pin the consistency at the JS layer.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let frameset = dom.create_element("frameset", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, frameset));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert!(matches!(
        vm.eval("document.body === document.activeElement;")
            .unwrap(),
        JsValue::Boolean(true),
    ));
    vm.unbind();
}

#[test]
fn document_body_returns_frameset_when_no_body_present() {
    // Post-arch-hoist-c, `document.body` accepts `<frameset>` as well
    // as `<body>` per WHATWG HTML §dom-document-body — pre-PR VM
    // filter only accepted `<body>`.  Pin the new behaviour at the
    // JS layer to catch any future bridge regression that re-narrows
    // it.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let frameset = dom.create_element("frameset", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, frameset));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Object(id) = vm.eval("document.body;").unwrap() else {
        panic!("document.body must return the frameset element");
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    assert_eq!(entity_bits, frameset.to_bits().get());
    vm.unbind();
}

#[test]
fn document_element_returns_first_element_child_regardless_of_tag() {
    // Post-arch-hoist-c, `document.documentElement` returns the first
    // Element child of the Document per WHATWG DOM §document-element
    // — no `<html>` tag filter.  Pre-PR VM walked specifically for
    // `<html>`.  Pin the spec-correct behaviour for documents whose
    // root element is something other than `<html>` (e.g. SVG-rooted
    // synthesised fixtures).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let svg = dom.create_element("svg", Attributes::default());
    assert!(dom.append_child(doc, svg));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Object(id) = vm.eval("document.documentElement;").unwrap() else {
        panic!("documentElement must return the first element child");
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        unreachable!()
    };
    assert_eq!(entity_bits, svg.to_bits().get());
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

// ---------------------------------------------------------------------------
// querySelector / querySelectorAll
// ---------------------------------------------------------------------------

#[test]
fn query_selector_by_tag() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.querySelector('div');").unwrap();
    assert!(matches!(v, JsValue::Object(_)));
    vm.unbind();
}

#[test]
fn query_selector_by_id() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.querySelector('#target');").unwrap();
    match v {
        JsValue::Object(id) => match vm.inner.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => {
                // #target is the div — verify it resolves to a valid entity.
                assert_ne!(entity_bits, 0);
            }
            _ => panic!("expected HostObject"),
        },
        _ => panic!("expected Object"),
    }
    vm.unbind();
}

#[test]
fn query_selector_by_class() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, div, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // .box matches div.box (first in document order), not p.box.highlight
    let v = vm.eval("document.querySelector('.box');").unwrap();
    match v {
        JsValue::Object(id) => match vm.inner.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => {
                assert_eq!(entity_bits, div.to_bits().get());
            }
            _ => panic!("expected HostObject"),
        },
        _ => panic!("expected Object"),
    }
    vm.unbind();
}

#[test]
fn query_selector_descendant_combinator() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm.eval("document.querySelector('body div');").unwrap();
    assert!(matches!(v, JsValue::Object(_)));
    vm.unbind();
}

#[test]
fn query_selector_no_match() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert!(matches!(
        vm.eval("document.querySelector('article');").unwrap(),
        JsValue::Null
    ));
    vm.unbind();
}

#[test]
fn query_selector_invalid_throws() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Invalid selector throws — eval returns Err.
    let result = vm.eval("document.querySelector('>>>');");
    assert!(result.is_err());
    vm.unbind();
}

#[test]
fn query_selector_all_returns_array() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // .box matches div.box and p.box.highlight
    let v = vm
        .eval("document.querySelectorAll('.box').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 2.0));
    vm.unbind();
}

#[test]
fn query_selector_all_empty() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm
        .eval("document.querySelectorAll('article').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 0.0));
    vm.unbind();
}

#[test]
fn query_selector_all_invalid_throws_dom_exception_syntax_error() {
    // `document.querySelectorAll` opts out of `invoke_dom_api` (the
    // standalone-fn path can't return Vec<Entity> through the
    // handler protocol) and uses the
    // `dom_bridge::query_selector_all_snapshot` helper that maps
    // `DomApiError -> VmError` directly.  Pin the SyntaxError /
    // DOMException mapping at the JS layer so a regression in that
    // helper-specific path surfaces as a test failure.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Invalid selector — SyntaxError DOMException expected.
    let result = vm.eval(
        "try { document.querySelectorAll('>>>'); 'no-throw'; } \
         catch (e) { (e instanceof DOMException) + ':' + e.name; }",
    );
    let JsValue::String(sid) = result.unwrap() else {
        panic!("expected string from try/catch")
    };
    assert_eq!(vm.get_string(sid), "true:SyntaxError");

    // Shadow-pseudo `:host` — same SyntaxError DOMException.
    let result = vm.eval(
        "try { document.querySelectorAll(':host'); 'no-throw'; } \
         catch (e) { (e instanceof DOMException) + ':' + e.name; }",
    );
    let JsValue::String(sid) = result.unwrap() else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "true:SyntaxError");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// getElementsByTagName / getElementsByClassName
// ---------------------------------------------------------------------------

#[test]
fn get_elements_by_tag_name_finds() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // fixture has one <div>
    let v = vm
        .eval("document.getElementsByTagName('div').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 1.0));
    vm.unbind();
}

#[test]
fn get_elements_by_tag_name_wildcard() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // fixture: html, head, title, body, div, span, p = 7 elements
    let v = vm
        .eval("document.getElementsByTagName('*').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 7.0));
    vm.unbind();
}

#[test]
fn get_elements_by_tag_name_case_insensitive() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm
        .eval("document.getElementsByTagName('DIV').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 1.0));
    vm.unbind();
}

#[test]
fn get_elements_by_tag_name_no_match() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm
        .eval("document.getElementsByTagName('article').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 0.0));
    vm.unbind();
}

#[test]
fn get_elements_by_class_name_single() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // "box" matches div.box and p.box.highlight
    let v = vm
        .eval("document.getElementsByClassName('box').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 2.0));
    vm.unbind();
}

#[test]
fn get_elements_by_class_name_multiple() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // "box highlight" — both classes must be present; only p matches
    let v = vm
        .eval("document.getElementsByClassName('box highlight').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 1.0));
    vm.unbind();
}

#[test]
fn get_elements_by_class_name_no_match() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _, _, _) = build_query_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let v = vm
        .eval("document.getElementsByClassName('nonexistent').length;")
        .unwrap();
    assert!(matches!(v, JsValue::Number(n) if n == 0.0));
    vm.unbind();
}
