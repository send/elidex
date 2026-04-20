//! Tests for the Element.prototype method suite and the
//! Node-common members installed on `Node.prototype`.
//!
//! The natives live across `vm/host/element_proto.rs` and
//! `vm/host/node_proto.rs`, but they share a small fixture and
//! common assertions — the tests are grouped here so one file holds
//! the full Element / Node surface verification.
//!
//! Compiled only with `feature = "engine"` (HostData bridging is a
//! stub otherwise, so the natives no-op).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// ```text
/// doc > html > head
///             > body#root > p.intro (text="hi")
///                         > "raw-text"  (bare text child)
///                         > div.box
///                           > span
///                         > !-- comment --
/// ```
///
/// Covers the shape every test needs — mixed element/text/comment
/// siblings, nested element, tag/class/id attributes — without
/// requiring each test to rebuild the tree.
pub(super) fn build_element_fixture(
    dom: &mut EcsDom,
) -> (
    Entity, // doc
    Entity, // body
    Entity, // p.intro (has text "hi")
    Entity, // div.box
    Entity, // span (inside div)
    Entity, // text node "raw-text" (direct child of body)
    Entity, // comment direct child of body
) {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", {
        let mut a = Attributes::default();
        a.set("id", "root");
        a
    });
    let p = dom.create_element("p", {
        let mut a = Attributes::default();
        a.set("class", "intro");
        a
    });
    let p_text = dom.create_text("hi");
    let raw_text = dom.create_text("raw-text");
    let div = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("class", "box");
        a
    });
    let span = dom.create_element("span", Attributes::default());
    let comment = dom.create_comment("note");
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, p));
    assert!(dom.append_child(p, p_text));
    assert!(dom.append_child(body, raw_text));
    assert!(dom.append_child(body, div));
    assert!(dom.append_child(div, span));
    assert!(dom.append_child(body, comment));
    (doc, body, p, div, span, raw_text, comment)
}

// ---------------------------------------------------------------------------
// Prototype chain sanity
// ---------------------------------------------------------------------------

#[test]
fn element_prototype_chain_reaches_event_target() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Element wrapper's addEventListener resolves through the
    // chain: wrapper → Element.prototype → Node.prototype →
    // EventTarget.prototype.
    let t = vm
        .eval("typeof document.getElementById('root').addEventListener;")
        .unwrap();
    let JsValue::String(sid) = t else {
        panic!("typeof returned {t:?}");
    };
    assert_eq!(vm.get_string(sid), "function");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Node common — parentNode / nextSibling / previousSibling
// ---------------------------------------------------------------------------

#[test]
fn node_parent_node_returns_parent_wrapper() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // `div.parentNode === body`.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.set_global("_body", JsValue::Object(body_wrapper));
    assert!(matches!(
        vm.eval("_div.parentNode === _body;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn node_parent_node_root_is_null() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.parentNode;").unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

#[test]
fn node_next_and_previous_sibling_include_non_elements() {
    // `nextSibling` / `previousSibling` traverse every Node — including
    // Text and Comment — per WHATWG §4.4.  This matters: body's
    // children are `p`, text, div, comment.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // p.nextSibling points at the raw-text node (not the div).
    let p_wrapper = vm.inner.create_element_wrapper(p);
    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    assert!(matches!(
        vm.eval("_p.nextSibling === _raw;").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_raw.previousSibling === _p;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

// ---------------------------------------------------------------------------
// nodeType / nodeName / nodeValue
// ---------------------------------------------------------------------------

#[test]
fn node_type_numeric_values_match_spec() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Element = 1, Text = 3, Comment = 8, Document = 9.
    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    let com_wrapper = vm.inner.create_element_wrapper(com);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    vm.set_global("_com", JsValue::Object(com_wrapper));
    assert!(matches!(
        vm.eval("document.getElementById('root').nodeType;").unwrap(),
        JsValue::Number(n) if (n - 1.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("_raw.nodeType;").unwrap(),
        JsValue::Number(n) if (n - 3.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("_com.nodeType;").unwrap(),
        JsValue::Number(n) if (n - 8.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("document.nodeType;").unwrap(),
        JsValue::Number(n) if (n - 9.0).abs() < f64::EPSILON
    ));

    vm.unbind();
}

#[test]
fn node_name_uppercase_tag_and_hash_names() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    let com_wrapper = vm.inner.create_element_wrapper(com);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    vm.set_global("_com", JsValue::Object(com_wrapper));

    let cases = [
        ("document.getElementById('root').nodeName;", "BODY"),
        ("_raw.nodeName;", "#text"),
        ("_com.nodeName;", "#comment"),
        ("document.nodeName;", "#document"),
    ];
    for (expr, expected) in cases {
        let v = vm.eval(expr).unwrap();
        let JsValue::String(sid) = v else {
            panic!("{expr}: unexpected {v:?}");
        };
        assert_eq!(vm.get_string(sid), expected, "expr: {expr}");
    }

    vm.unbind();
}

#[test]
fn node_value_data_for_text_and_comment_else_null() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    let com_wrapper = vm.inner.create_element_wrapper(com);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    vm.set_global("_com", JsValue::Object(com_wrapper));

    let raw_val = vm.eval("_raw.nodeValue;").unwrap();
    let JsValue::String(sid) = raw_val else {
        panic!();
    };
    assert_eq!(vm.get_string(sid), "raw-text");

    let com_val = vm.eval("_com.nodeValue;").unwrap();
    let JsValue::String(sid) = com_val else {
        panic!();
    };
    assert_eq!(vm.get_string(sid), "note");

    // Elements have `null` nodeValue.
    assert!(matches!(
        vm.eval("document.getElementById('root').nodeValue;")
            .unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

#[test]
fn node_value_setter_replaces_text_data() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));

    // Setter returns whatever JS assigned (the right-hand side).
    vm.eval("_raw.nodeValue = 'replaced';").unwrap();
    let v = vm.eval("_raw.nodeValue;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "replaced");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// textContent
// ---------------------------------------------------------------------------

#[test]
fn text_content_getter_concatenates_descendant_text() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // body's descendant text: "hi" (inside <p>) + "raw-text" direct.
    let v = vm
        .eval("document.getElementById('root').textContent;")
        .unwrap();
    let JsValue::String(sid) = v else {
        panic!("unexpected {v:?}")
    };
    assert_eq!(vm.get_string(sid), "hiraw-text");

    vm.unbind();
}

#[test]
fn text_content_getter_text_node_is_own_data() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    let v = vm.eval("_raw.textContent;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "raw-text");

    vm.unbind();
}

#[test]
fn text_content_setter_replaces_children_with_single_text_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval("document.getElementById('root').textContent = 'fresh';")
        .unwrap();
    // After assignment: body has a single Text child, "fresh".
    let v = vm
        .eval("document.getElementById('root').textContent;")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "fresh");
    // Prior children (p / raw-text / div / comment) are detached —
    // unbind first, then inspect `dom` directly (it outlives the VM
    // bind since this test owns it on the stack).
    vm.unbind();
    assert_eq!(
        dom.children_iter(body).count(),
        1,
        "body should have exactly one Text child after textContent setter"
    );
}

#[test]
fn text_content_on_document_is_null_and_setter_is_noop() {
    // WHATWG §4.4: `document.textContent` returns `null` (not the
    // concatenated descendant text), and `document.textContent = x`
    // is a no-op — the document tree must not be rewritten.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.textContent;").unwrap(),
        JsValue::Null
    ));
    // Setter is a no-op: children and descendants unchanged.
    vm.eval("document.textContent = 'ignored';").unwrap();
    assert!(matches!(
        vm.eval("document.textContent;").unwrap(),
        JsValue::Null
    ));
    // body / #root still exists (would be gone if the setter wiped
    // the document tree).
    assert!(matches!(
        vm.eval("document.getElementById('root') !== null;")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn text_content_setter_null_becomes_empty() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval("document.getElementById('root').textContent = null;")
        .unwrap();
    let v = vm
        .eval("document.getElementById('root').textContent;")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Element-only tree navigation
// ---------------------------------------------------------------------------

#[test]
fn element_parent_element_returns_null_when_parent_not_element() {
    // body's parent is <html>, which IS an element — sanity check.
    // documentElement's parent is the document root (no TagType), so
    // .parentElement must be null even though .parentNode is not.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.documentElement.parentElement;").unwrap(),
        JsValue::Null
    ));
    // But parentNode points at the document.
    assert!(matches!(
        vm.eval("document.documentElement.parentNode === document;")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_child_nodes_and_children_differ_on_text_children() {
    // body has mixed children: p, text, div, comment (4 nodes total;
    // 2 of them are elements).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.getElementById('root').childNodes.length;")
            .unwrap(),
        JsValue::Number(n) if (n - 4.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').children.length;")
            .unwrap(),
        JsValue::Number(n) if (n - 2.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').childElementCount;")
            .unwrap(),
        JsValue::Number(n) if (n - 2.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasChildNodes();")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_first_and_last_element_child_skip_text() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("document.getElementById('root').firstElementChild === _p;")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').lastElementChild === _div;")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_sibling_accessors_skip_non_elements() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("_p.nextElementSibling === _div;").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.previousElementSibling === _p;").unwrap(),
        JsValue::Boolean(true)
    ));
    // Last element in the chain — next is null.
    assert!(matches!(
        vm.eval("_div.nextElementSibling;").unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

#[test]
fn element_is_connected_respects_document_root() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);
    let detached = dom.create_element("aside", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let det_wrapper = vm.inner.create_element_wrapper(detached);
    vm.set_global("_det", JsValue::Object(det_wrapper));
    assert!(matches!(
        vm.eval("document.getElementById('root').isConnected;")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_det.isConnected;").unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_contains_self_and_descendants_and_rejects_null() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    let span_wrapper = vm.inner.create_element_wrapper(span);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.set_global("_span", JsValue::Object(span_wrapper));
    assert!(matches!(
        vm.eval("_div.contains(_div);").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.contains(_span);").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_span.contains(_div);").unwrap(),
        JsValue::Boolean(false)
    ));
    // `null` and `undefined` are the only non-Node values that do
    // NOT throw — WebIDL `Node?` allows them and they map to `false`.
    assert!(matches!(
        vm.eval("_div.contains(null);").unwrap(),
        JsValue::Boolean(false)
    ));
    assert!(matches!(
        vm.eval("_div.contains();").unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_contains_throws_for_non_node_arguments() {
    // WebIDL `boolean contains(Node? other)`: any non-Node that is
    // not `null` / `undefined` must throw `TypeError`.  This
    // includes plain objects, Window, and non-Node HostObjects.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    for expr in [
        "document.getElementById('root').contains({});",
        "document.getElementById('root').contains(42);",
        "document.getElementById('root').contains('text');",
        "document.getElementById('root').contains(window);",
    ] {
        assert!(
            vm.eval(expr).is_err(),
            "{expr} must throw TypeError for a non-Node argument"
        );
    }

    vm.unbind();
}

// Attribute tests → tests_element_attributes.rs
// DOM mutation / matches / prototype separation → tests_element_mutation.rs
