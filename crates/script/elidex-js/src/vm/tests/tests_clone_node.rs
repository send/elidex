//! `Node.prototype.cloneNode(deep?)` tests — shallow/deep clone,
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
fn clone_node_document_methods_operate_on_clone_not_bound_doc() {
    // After cloning a document, methods like querySelector /
    // getElementById must search the cloned subtree, not the
    // still-bound document.  Add an `id=bound-only` element to the
    // bound document AFTER cloning, so the clone cannot see it.
    let (mut vm, mut session, mut dom, doc) = setup();
    // Seed the bound doc with a <html> root so the clone has
    // something to search in.
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.cloned = document.cloneNode(true);")
        .unwrap();
    // Append a new element to the BOUND document AFTER the clone —
    // the clone must not see it.
    vm.eval(
        "var fresh = document.createElement('div');\n\
         fresh.setAttribute('id', 'bound-only');\n\
         document.documentElement.appendChild(fresh);",
    )
    .unwrap();
    // `cloned.getElementById('bound-only')` searches the clone and
    // should miss; `document.getElementById('bound-only')` finds it.
    let cloned_hit = vm.eval("cloned.getElementById('bound-only');").unwrap();
    assert!(
        matches!(cloned_hit, JsValue::Null),
        "cloned document should not see the bound doc's new element",
    );
    let bound_hit = vm.eval("document.getElementById('bound-only');").unwrap();
    assert!(matches!(bound_hit, JsValue::Object(_)));
    vm.unbind();
}

#[test]
fn clone_node_document_wrapper_has_document_methods() {
    // A cloned Document must carry the same own-property suite as
    // the bound document — `createElement`, `getElementById`,
    // `body` accessor, etc.  Without the post-clone install these
    // would silently be `undefined` on the clone.
    let (mut vm, mut session, mut dom, doc) = setup();
    // Seed the bound document with a bit of structure so the deep
    // clone has something to traverse.
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Force the bound document's methods to install by accessing one.
    vm.eval("document.getElementById('dummy');").unwrap();
    // Now clone the document and confirm methods survive on the clone.
    vm.eval("globalThis.cloned = document.cloneNode(true);")
        .unwrap();
    let JsValue::String(sid) = vm.eval("typeof cloned.createElement;").unwrap() else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "function");
    let JsValue::String(sid2) = vm.eval("typeof cloned.getElementById;").unwrap() else {
        panic!()
    };
    assert_eq!(vm.get_string(sid2), "function");
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

// ---------------------------------------------------------------------------
// arch-hoist-d pre-emptive regression tests (skill lesson #149).
// ---------------------------------------------------------------------------

#[test]
fn cloned_element_does_not_acquire_document_methods() {
    // Pin install_document_methods_if_cloned_doc gate: cloning a
    // non-Document must NOT install document-only methods on the
    // resulting wrapper.  Reaching `createElement` on an Element
    // clone should be undefined (not a function), which `typeof`
    // reports as "undefined".
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var el = document.createElement('div').cloneNode(true);\n\
             typeof el.createElement === 'undefined';",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

#[test]
fn cloned_document_clone_node_owner_propagates() {
    // Chained cloneNode must keep installing document methods on
    // each cloned wrapper — pin that c2.createElement still resolves
    // and reports c2 as ownerDocument.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var c1 = document.cloneNode(true);\n\
             var c2 = c1.cloneNode(true);\n\
             // c2 must itself be a Document with a working createElement
             // (proves install_document_methods_if_cloned_doc fired on c2).
             typeof c2.createElement === 'function' && \
             c2.createElement('p').ownerDocument === c2;",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// Note: `cloneNode` ShadowRoot rejection is covered at the handler
// layer in `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs::
// clone_node_shadow_root_error`.  A JS-layer mirror would require
// `Element.prototype.attachShadow`, which is not yet exposed (defer
// slot `#11-arch-hoist-e` / PR5b).
