//! PR4e C3: `Node.prototype.cloneNode(deep?)` — shallow/deep clone,
//! attribute copy, parent isolation, listener non-duplication.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

#[test]
fn clone_node_shallow_has_no_children() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.appendChild(document.createElement('span'));\n\
             var c = p.cloneNode(false);\n\
             c.childNodes.length;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 0.0);
    vm.unbind();
}

#[test]
fn clone_node_shallow_copies_attributes() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::String(sid) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.setAttribute('id', 'hero');\n\
             p.setAttribute('data-x', '42');\n\
             var c = p.cloneNode(false);\n\
             c.getAttribute('id') + '|' + c.getAttribute('data-x');",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "hero|42");
    vm.unbind();
}

#[test]
fn clone_node_deep_copies_children() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Number(n) = vm
        .eval(
            "var p = document.createElement('p');\n\
             p.appendChild(document.createElement('span'));\n\
             p.appendChild(document.createElement('em'));\n\
             var c = p.cloneNode(true);\n\
             c.childNodes.length;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(n, 2.0);
    vm.unbind();
}

#[test]
fn clone_node_text_copies_data() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::String(sid) = vm
        .eval(
            "var t = document.createTextNode('greetings');\n\
             t.cloneNode(false).nodeValue;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "greetings");
    vm.unbind();
}

#[test]
fn clone_node_has_no_parent() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Null = vm
        .eval(
            "var parent = document.createElement('section');\n\
             var child = document.createElement('p');\n\
             parent.appendChild(child);\n\
             child.cloneNode(false).parentNode;",
        )
        .unwrap()
    else {
        panic!("expected null parent on clone");
    };
    vm.unbind();
}

#[test]
fn clone_node_allocates_distinct_entity() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Clone must not share entity bits with the original.  Use
    // globalThis assignment so the original wrapper survives into a
    // second script eval — plain `var` in one script does not persist
    // to the next eval.
    vm.eval("globalThis.orig = document.createElement('p');")
        .unwrap();
    vm.eval("globalThis.clone = globalThis.orig.cloneNode(false);")
        .unwrap();
    let orig = vm.eval("globalThis.orig;").unwrap();
    let clone = vm.eval("globalThis.clone;").unwrap();
    let JsValue::Object(orig_id) = orig else {
        panic!()
    };
    let JsValue::Object(clone_id) = clone else {
        panic!()
    };
    assert_ne!(orig_id, clone_id);
    let o = match &vm.inner.get_object(orig_id).kind {
        ObjectKind::HostObject { entity_bits } => *entity_bits,
        _ => unreachable!(),
    };
    let c = match &vm.inner.get_object(clone_id).kind {
        ObjectKind::HostObject { entity_bits } => *entity_bits,
        _ => unreachable!(),
    };
    assert_ne!(o, c);
    vm.unbind();
}
