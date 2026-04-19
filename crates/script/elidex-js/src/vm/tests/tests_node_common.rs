//! PR4e C3.5 / C3.6 / C4: Node common members —
//! `ownerDocument`, `isSameNode`, `getRootNode`, `isEqualNode`,
//! `compareDocumentPosition`.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

// ---------------------------------------------------------------------------
// ownerDocument
// ---------------------------------------------------------------------------

#[test]
fn owner_document_of_element_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.createElement('p').ownerDocument === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn owner_document_of_text_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.createTextNode('x').ownerDocument === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn owner_document_of_document_is_null() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Null = vm.eval("document.ownerDocument;").unwrap() else {
        panic!("expected null");
    };
    vm.unbind();
}

#[test]
fn owner_document_of_cloned_node_is_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.cloneNode(false).ownerDocument === document;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// isSameNode
// ---------------------------------------------------------------------------

#[test]
fn is_same_node_same_wrapper_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.isSameNode(p);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn is_same_node_distinct_nodes_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Two separately created elements → different wrappers.
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.isSameNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_same_node_null_arg_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.isSameNode(null);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// getRootNode
// ---------------------------------------------------------------------------

#[test]
fn get_root_node_detached_returns_self() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Detached element: root is itself.
    let JsValue::Boolean(b) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.getRootNode() === p;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// isEqualNode
// ---------------------------------------------------------------------------

#[test]
fn is_equal_node_same_tag_same_attrs_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.setAttribute('id', 'x');\n\
             b.setAttribute('id', 'x');\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_attrs_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var b = document.createElement('p');\n\
             a.setAttribute('id', 'x');\n\
             b.setAttribute('id', 'y');\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_children_order_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.appendChild(document.createElement('span'));\n\
             a.appendChild(document.createElement('em'));\n\
             var b = document.createElement('p');\n\
             b.appendChild(document.createElement('em'));\n\
             b.appendChild(document.createElement('span'));\n\
             a.isEqualNode(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_different_kind_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             var t = document.createTextNode('hi');\n\
             a.isEqualNode(t);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_null_arg_false() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.isEqualNode(null);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(!b);
    vm.unbind();
}

#[test]
fn is_equal_node_self_true() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var a = document.createElement('p');\n\
             a.isEqualNode(a);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// compareDocumentPosition
// ---------------------------------------------------------------------------

fn setup_fixture() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    (vm, session, dom, doc)
}

#[test]
fn compare_document_position_same_node_zero() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval("document.body.compareDocumentPosition(document.body);")
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 0.0);
    vm.unbind();
}

#[test]
fn compare_document_position_sibling_following() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Add two siblings a, b under body; a.compareDocumentPosition(b)
    // should set FOLLOWING (0x04).
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             document.body.appendChild(a);\n\
             document.body.appendChild(b);\n\
             a.compareDocumentPosition(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 4.0);
    vm.unbind();
}

#[test]
fn compare_document_position_sibling_preceding() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // b.compareDocumentPosition(a) → PRECEDING (0x02).
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             document.body.appendChild(a);\n\
             document.body.appendChild(b);\n\
             b.compareDocumentPosition(a);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 2.0);
    vm.unbind();
}

#[test]
fn compare_document_position_ancestor_contains() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // body contains child → child.compareDocumentPosition(body) =
    // CONTAINS | PRECEDING = 0x08 | 0x02 = 0x0a = 10.
    let JsValue::Number(n) = vm
        .eval(
            "var child = document.createElement('child');\n\
             document.body.appendChild(child);\n\
             child.compareDocumentPosition(document.body);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 10.0);
    vm.unbind();
}

#[test]
fn compare_document_position_descendant_contained_by() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // body.compareDocumentPosition(child) = CONTAINED_BY | FOLLOWING
    // = 0x10 | 0x04 = 0x14 = 20.
    let JsValue::Number(n) = vm
        .eval(
            "var child = document.createElement('child');\n\
             document.body.appendChild(child);\n\
             document.body.compareDocumentPosition(child);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 20.0);
    vm.unbind();
}

#[test]
fn compare_document_position_disconnected() {
    let (mut vm, mut session, mut dom, doc) = setup_fixture();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Two detached nodes → DISCONNECTED (0x01) | PRECEDING (0x02) = 3.
    let JsValue::Number(n) = vm
        .eval(
            "var a = document.createElement('a');\n\
             var b = document.createElement('b');\n\
             a.compareDocumentPosition(b);",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 3.0);
    vm.unbind();
}

#[test]
fn get_root_node_attached_returns_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    // Build: doc > html > p
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let p = dom.create_element("p", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, p));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval("document.documentElement.firstChild.getRootNode() === document;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}
