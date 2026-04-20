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
fn build_element_fixture(
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

// ---------------------------------------------------------------------------
// Attributes: tagName / getAttribute / setAttribute / …
// ---------------------------------------------------------------------------

#[test]
fn element_tag_name_is_upper_case() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.getElementById('root').tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "BODY");

    vm.unbind();
}

#[test]
fn element_get_attribute_present_and_missing() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // body has id="root" and no class.
    let v = vm
        .eval("document.getElementById('root').getAttribute('id');")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "root");

    assert!(matches!(
        vm.eval("document.getElementById('root').getAttribute('nonexistent');")
            .unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

#[test]
fn element_set_attribute_then_get_and_has() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "var el = document.getElementById('root'); \
         el.setAttribute('data-x', 'hello');",
    )
    .unwrap();

    let v = vm
        .eval("document.getElementById('root').getAttribute('data-x');")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "hello");

    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('data-x');")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('missing');")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_remove_attribute_is_silent_when_missing() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Grab the wrapper before removing the id — id-based lookup
    // would fail after removal.  Run the whole scenario in one
    // script so locals survive between statements.
    let v = vm
        .eval(
            "var el = document.getElementById('root');\n\
             el.removeAttribute('id');\n\
             el.removeAttribute('missing');\n\
             el.hasAttribute('id') ? 'bug' : 'ok';",
        )
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "ok");

    vm.unbind();
}

#[test]
fn element_get_attribute_names_is_array_in_insertion_order() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // div has class="box" (only one attr).  Add two more via setAttribute.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.eval("_div.setAttribute('data-a', '1'); _div.setAttribute('data-b', '2');")
        .unwrap();
    let len = vm.eval("_div.getAttributeNames().length;").unwrap();
    let JsValue::Number(n) = len else { panic!() };
    assert!((n - 3.0).abs() < f64::EPSILON, "got {n}");

    // Each entry is a string.  Verify the first (original) slot.
    let first = vm.eval("_div.getAttributeNames()[0];").unwrap();
    let JsValue::String(sid) = first else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "class");

    vm.unbind();
}

#[test]
fn element_toggle_attribute_without_force_toggles_presence() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // First call: absent → add → returns true.  Value is empty string.
    let on = vm
        .eval("document.getElementById('root').toggleAttribute('hidden');")
        .unwrap();
    assert!(matches!(on, JsValue::Boolean(true)));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('hidden');")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    // Second call: present → remove → returns false.
    let off = vm
        .eval("document.getElementById('root').toggleAttribute('hidden');")
        .unwrap();
    assert!(matches!(off, JsValue::Boolean(false)));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('hidden');")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_toggle_attribute_with_force_is_idempotent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // force=true both times — still present, returns true.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', true);")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', true);")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    // force=false while present → remove.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', false);")
            .unwrap(),
        JsValue::Boolean(false)
    ));
    // force=false while absent → still absent.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', false);")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_id_reflected_getter_setter() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.getElementById('root').id;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "root");

    vm.eval("document.getElementById('root').id = 'new-id';")
        .unwrap();
    let v = vm.eval("document.getElementById('new-id').id;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "new-id");

    vm.unbind();
}

#[test]
fn element_class_name_reflects_class_attribute() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm.eval("_p.className;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "intro");

    vm.eval("_p.className = 'foo bar';").unwrap();
    assert!(matches!(
        vm.eval("_p.getAttribute('class');").unwrap(),
        JsValue::String(_)
    ));
    let v = vm.eval("_p.className;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "foo bar");

    vm.unbind();
}

#[test]
fn element_id_on_text_node_is_undefined() {
    // `id` / `className` live on Element.prototype, so Text wrappers
    // (which inherit via Node.prototype, not Element.prototype) must
    // NOT expose them.
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
    let t = vm.eval("typeof _raw.id;").unwrap();
    let JsValue::String(sid) = t else { panic!() };
    assert_eq!(vm.get_string(sid), "undefined");

    vm.unbind();
}

// ---------------------------------------------------------------------------
// DOM mutation
// ---------------------------------------------------------------------------

#[test]
fn element_append_child_adds_new_element_and_returns_it() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm
        .eval(
            "var el = document.createElement('section'); \
             document.getElementById('root').appendChild(el) === el;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // Count elements on body (originally 2 — p, div; now 3 with new section).
    assert!(matches!(
        vm.eval("document.getElementById('root').childElementCount;")
            .unwrap(),
        JsValue::Number(n) if (n - 3.0).abs() < f64::EPSILON
    ));

    vm.unbind();
}

#[test]
fn element_remove_child_detaches_and_returns_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm
        .eval("document.getElementById('root').removeChild(_p) === _p;")
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // `_p.parentNode` is now null.
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_remove_child_of_non_child_throws() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // span is a grandchild, not a direct child of body.  PR5a C4
    // upgrades the throw from TypeError to
    // `DOMException("NotFoundError")` (legacy code 8) per WHATWG
    // DOM §4.4.
    let span_wrapper = vm.inner.create_element_wrapper(span);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    let check = vm
        .eval(
            "var root = document.getElementById('root');\
             var thrown = null;\
             try { root.removeChild(_span); } catch (e) { thrown = e; }\
             thrown && thrown.name === 'NotFoundError' \
             && thrown instanceof DOMException && thrown.code === 8;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_append_child_rejects_non_node_argument() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').appendChild({});");
    assert!(r.is_err());
    let r = vm.eval("document.getElementById('root').appendChild(null);");
    assert!(r.is_err());

    vm.unbind();
}

#[test]
fn element_insert_before_places_new_child_ahead_of_ref() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Insert a new section before div.  Ordering: p, text, section, div, comment.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    let _ = p;
    let v = vm
        .eval(
            "var s = document.createElement('section'); \
             document.getElementById('root').insertBefore(s, _div); \
             _div.previousElementSibling === s;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_insert_before_with_null_ref_appends() {
    // insertBefore(new, null) behaves like appendChild.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm
        .eval(
            "var s = document.createElement('section'); \
             document.getElementById('root').insertBefore(s, null); \
             document.getElementById('root').lastElementChild === s;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_replace_child_returns_old_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm
        .eval(
            "var h = document.createElement('h1'); \
             document.getElementById('root').replaceChild(h, _p) === _p;",
        )
        .unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));

    // _p is now detached from body; h is in its place.
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_replace_child_rejects_non_child_with_not_found_error() {
    // WHATWG DOM §4.4: `replaceChild` throws
    // `DOMException("NotFoundError")` when `old` is not a child of
    // the receiver.  PR5a C4 upgrade from the pre-C4 TypeError
    // surface.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let span_wrapper = vm.inner.create_element_wrapper(span);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    let check = vm
        .eval(
            "var root = document.getElementById('root');\
             var h = document.createElement('h1');\
             var thrown = null;\
             try { root.replaceChild(h, _span); } catch (e) { thrown = e; }\
             thrown && thrown.name === 'NotFoundError' \
             && thrown instanceof DOMException && thrown.code === 8;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));

    vm.unbind();
}

#[test]
fn element_remove_detaches_from_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    vm.eval("_p.remove();").unwrap();
    assert!(matches!(vm.eval("_p.parentNode;").unwrap(), JsValue::Null));

    vm.unbind();
}

#[test]
fn element_remove_on_detached_node_is_no_op() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // `createElement` produces a detached element.  `.remove()` is silent.
    vm.eval("document.createElement('aside').remove();")
        .unwrap();

    vm.unbind();
}

// ---------------------------------------------------------------------------
// matches / closest
// ---------------------------------------------------------------------------

#[test]
fn element_matches_tag_class_id() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("_div.matches('div');").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.matches('.box');").unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("_div.matches('.nonexistent');").unwrap(),
        JsValue::Boolean(false)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').matches('#root');")
            .unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_matches_throws_on_invalid_selector() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').matches('!!!');");
    assert!(r.is_err(), "invalid selector must throw SyntaxError");

    vm.unbind();
}

#[test]
fn element_matches_rejects_shadow_pseudos() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let r = vm.eval("document.getElementById('root').matches(':host');");
    assert!(r.is_err());

    vm.unbind();
}

#[test]
fn element_closest_returns_self_when_self_matches() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    assert!(matches!(
        vm.eval("_div.closest('.box') === _div;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_closest_walks_up_to_matching_ancestor() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, _div, span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let span_wrapper = vm.inner.create_element_wrapper(span);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    vm.set_global("_span", JsValue::Object(span_wrapper));
    vm.set_global("_body", JsValue::Object(body_wrapper));
    assert!(matches!(
        vm.eval("_span.closest('#root') === _body;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn element_closest_returns_null_when_no_match() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.getElementById('root').closest('.nonexistent');")
            .unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Node / Window prototype separation (addresses Copilot #1 / #2)
// ---------------------------------------------------------------------------

#[test]
fn window_does_not_expose_node_members() {
    // WHATWG: Window is an EventTarget but NOT a Node.
    // `window.nodeType` / `window.parentNode` / `window.textContent`
    // must all be `undefined` — they live on `Node.prototype` which
    // is NOT in Window's prototype chain.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    for expr in [
        "typeof window.nodeType",
        "typeof window.parentNode",
        "typeof window.parentElement",
        "typeof window.textContent",
        "typeof window.firstChild",
        "typeof window.appendChild",
    ] {
        let v = vm.eval(&format!("{expr};")).unwrap();
        let JsValue::String(sid) = v else {
            panic!("{expr}: unexpected {v:?}");
        };
        assert_eq!(
            vm.get_string(sid),
            "undefined",
            "{expr} must be undefined on Window"
        );
    }
    // But window.addEventListener — an EventTarget method — is still present.
    let v = vm.eval("typeof window.addEventListener;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "function");

    vm.unbind();
}

#[test]
fn text_parent_element_returns_parent_element() {
    // `parentElement` is a Node member (WHATWG §4.4), not Element-
    // specific — so a Text wrapper must expose it and return the
    // parent element when its parent is one.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    vm.set_global("_body", JsValue::Object(body_wrapper));
    assert!(matches!(
        vm.eval("_raw.parentElement === _body;").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn text_wrapper_sees_node_members() {
    // Text nodes chain through `Node.prototype`, so Node-level
    // accessors and methods must all resolve — `firstChild` returns
    // null, `appendChild` exists, `textContent` returns own data.
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

    // firstChild / lastChild / childNodes on Text — exist, return
    // null / empty.
    assert!(matches!(
        vm.eval("_raw.firstChild;").unwrap(),
        JsValue::Null
    ));
    assert!(matches!(vm.eval("_raw.lastChild;").unwrap(), JsValue::Null));
    assert!(matches!(
        vm.eval("_raw.childNodes.length;").unwrap(),
        JsValue::Number(n) if n.abs() < f64::EPSILON
    ));
    // hasChildNodes → false on a text node.
    assert!(matches!(
        vm.eval("_raw.hasChildNodes();").unwrap(),
        JsValue::Boolean(false)
    ));
    // appendChild exists as a function.
    let v = vm.eval("typeof _raw.appendChild;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "function");

    vm.unbind();
}

#[test]
fn append_child_rejects_window_argument() {
    // Window is an EventTarget but not a Node in WHATWG — passing
    // it as the child argument to a mutation method must throw the
    // same "parameter is not of type 'Node'" TypeError as any other
    // non-Node.  Covers appendChild / removeChild / insertBefore /
    // replaceChild (they share `require_node_arg`).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    for expr in [
        "document.getElementById('root').appendChild(window);",
        "document.getElementById('root').removeChild(window);",
        "document.getElementById('root').insertBefore(window, null);",
        "document.getElementById('root').replaceChild(window, \
             document.getElementById('root').firstChild);",
    ] {
        assert!(
            vm.eval(expr).is_err(),
            "{expr} must throw TypeError when passed Window"
        );
    }

    vm.unbind();
}

#[test]
fn shadow_host_hides_shadow_root_from_light_tree_traversal() {
    // A host with an attached shadow root and one light-DOM child
    // must expose ONLY the light child via `firstChild` /
    // `lastChild` / `childNodes` / `hasChildNodes()`.  The shadow
    // root is internal and must not leak through any of these
    // accessors.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);
    // div currently has one child (span).  Attach a shadow root
    // and also place a node inside it.
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let shadow_el = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, shadow_el));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));

    // firstChild / lastChild resolve to the span, not the shadow root.
    let v = vm.eval("_div.firstChild.tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "SPAN");
    let v = vm.eval("_div.lastChild.tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "SPAN");

    // childNodes / childElementCount count the span only.
    assert!(matches!(
        vm.eval("_div.childNodes.length;").unwrap(),
        JsValue::Number(n) if (n - 1.0).abs() < f64::EPSILON
    ));
    assert!(matches!(
        vm.eval("_div.hasChildNodes();").unwrap(),
        JsValue::Boolean(true)
    ));

    vm.unbind();
}

#[test]
fn shadow_host_has_child_nodes_false_when_only_shadow_root() {
    // A host whose ONLY child is a shadow root reports
    // `hasChildNodes() === false` (light-tree empty), matching the
    // browser where `childNodes.length` is also 0.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let host = dom.create_element("section", {
        let mut a = Attributes::default();
        a.set("id", "shadow-host");
        a
    });
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, host));
    let _ = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').hasChildNodes();")
            .unwrap(),
        JsValue::Boolean(false)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').firstChild;")
            .unwrap(),
        JsValue::Null
    ));
    assert!(matches!(
        vm.eval("document.getElementById('shadow-host').childNodes.length;")
            .unwrap(),
        JsValue::Number(n) if n.abs() < f64::EPSILON
    ));

    vm.unbind();
}

#[test]
fn contains_stops_at_shadow_boundary() {
    // `host.contains(nodeInsideShadow)` must be false — the shadow
    // root is NOT a light-tree descendant of its host, even though
    // elidex stores it as a child for convenience.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let shadow_el = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, shadow_el));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let div_wrapper = vm.inner.create_element_wrapper(div);
    let shadow_el_wrapper = vm.inner.create_element_wrapper(shadow_el);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.set_global("_shadow_el", JsValue::Object(shadow_el_wrapper));

    assert!(matches!(
        vm.eval("_div.contains(_shadow_el);").unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn closest_stops_at_shadow_boundary() {
    // When walking ancestors from inside a shadow tree, `closest`
    // must stop at the shadow root (approximated by "non-Element
    // parent") and not return a match on the shadow host.
    use elidex_ecs::ShadowRootMode;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    // Give div a shadow root, put a child inside the shadow tree.
    let shadow_root = dom
        .attach_shadow(div, ShadowRootMode::Open)
        .expect("attach_shadow");
    let inner = dom.create_element("article", Attributes::default());
    assert!(dom.append_child(shadow_root, inner));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Set div.id = "host" so it would be matched by `#host` — if the
    // walk crossed the shadow boundary, `inner.closest('#host')`
    // would return `div`.  Correct behaviour: return null.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.eval("_div.setAttribute('id', 'host');").unwrap();
    let inner_wrapper = vm.inner.create_element_wrapper(inner);
    vm.set_global("_inner", JsValue::Object(inner_wrapper));

    // Matching its own tag succeeds (self-match).
    let v = vm.eval("_inner.closest('article') === _inner;").unwrap();
    assert!(matches!(v, JsValue::Boolean(true)));
    // But walking to the host is blocked.
    let v = vm.eval("_inner.closest('#host');").unwrap();
    assert!(
        matches!(v, JsValue::Null),
        "closest() must not cross the shadow boundary; got {v:?}"
    );

    vm.unbind();
}

// ---------------------------------------------------------------------------
// Element-only members should NOT surface on Text nodes
// ---------------------------------------------------------------------------

#[test]
fn text_wrapper_does_not_expose_element_placeholder_marker() {
    // Invariant: members installed on `Element.prototype` must be
    // `undefined` on Text wrappers — the Text branch skips
    // `Element.prototype` and inherits from `Node.prototype` (and
    // then `EventTarget.prototype`).  `firstElementChild` is an
    // Element-only accessor, so `typeof` must be `undefined` on a
    // Text node.
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
    let t = vm.eval("typeof _raw.firstElementChild;").unwrap();
    let JsValue::String(sid) = t else { panic!() };
    assert_eq!(
        vm.get_string(sid),
        "undefined",
        "firstElementChild must not resolve on Text wrappers"
    );

    vm.unbind();
}
