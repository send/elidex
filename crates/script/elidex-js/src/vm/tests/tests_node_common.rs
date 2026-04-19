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
