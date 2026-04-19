//! ParentNode mixin tests (WHATWG DOM §5.2.4) — `prepend` /
//! `append` / `replaceChildren` installed on Element.prototype and
//! on the document wrapper.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

fn make_parent_with_children(vm: &mut Vm) {
    vm.eval(
        "globalThis.p = document.createElement('p');\n\
         globalThis.a = document.createElement('a');\n\
         globalThis.b = document.createElement('b');\n\
         p.appendChild(a);\n\
         p.appendChild(b);",
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// prepend
// ---------------------------------------------------------------------------

#[test]
fn element_prepend_inserts_at_start() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.prepend(document.createElement('z'));").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "Z");
    vm.unbind();
}

#[test]
fn element_prepend_multi_arg_preserves_order() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.prepend(document.createElement('x'), document.createElement('y'));")
        .unwrap();
    // Expected: x, y, a, b.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "Y");
    vm.unbind();
}

#[test]
fn element_prepend_empty_tree_appends() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.p = document.createElement('p');")
        .unwrap();
    vm.eval("p.prepend(document.createElement('z'));").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 1.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// append
// ---------------------------------------------------------------------------

#[test]
fn element_append_inserts_at_end() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.append(document.createElement('z'));").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "Z");
    vm.unbind();
}

#[test]
fn element_append_string_becomes_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.append('tail');").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_num(&mut vm, "p.childNodes[2].nodeType;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].data;"), "tail");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// replaceChildren
// ---------------------------------------------------------------------------

#[test]
fn element_replace_children_no_args_clears() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.replaceChildren();").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 0.0);
    vm.unbind();
}

#[test]
fn element_replace_children_with_args_substitutes() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.replaceChildren(document.createElement('x'), document.createElement('y'));")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "Y");
    vm.unbind();
}

#[test]
fn element_replace_children_preserves_tree_when_conversion_throws() {
    // Arg normalization runs BEFORE clearing the parent, so a
    // ToString throw on a Symbol leaves the existing children intact.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    let threw = vm
        .eval(
            "var err = null;\n\
             try { p.replaceChildren(Symbol()); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    // Original children still present.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    vm.unbind();
}

#[test]
fn element_prepend_own_first_child_is_noop() {
    // WHATWG pre-insert: `parent.prepend(parent.firstChild)` is a
    // no-op (the child is already at the position it would be
    // moved to).  Must not throw, tree unchanged.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("p.prepend(a);").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "A");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "B");
    vm.unbind();
}

#[test]
fn element_append_document_fragment_flattens() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval(
        "var f = document.createDocumentFragment();\n\
         f.appendChild(document.createElement('x'));\n\
         f.appendChild(document.createElement('y'));\n\
         p.append(f);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[3].tagName;"), "Y");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Document-level install
// ---------------------------------------------------------------------------

#[test]
fn document_append_adds_to_document_root() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Document.prototype.append lands a freshly-created element
    // straight onto the document root.  Count root children before +
    // after to confirm.
    let before = eval_num(&mut vm, "document.childNodes.length;");
    vm.eval("document.append(document.createElement('html'));")
        .unwrap();
    let after = eval_num(&mut vm, "document.childNodes.length;");
    assert_eq!(after, before + 1.0);
    vm.unbind();
}

#[test]
fn document_replace_children_single_element_replaces() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Seed the document with a couple of placeholder children, then
    // replaceChildren with exactly one element.
    vm.eval(
        "document.appendChild(document.createElement('a'));\n\
         document.appendChild(document.createElement('b'));",
    )
    .unwrap();
    vm.eval("document.replaceChildren(document.createElement('root'));")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "document.childNodes.length;"), 1.0);
    assert_eq!(eval_str(&mut vm, "document.childNodes[0].tagName;"), "ROOT");
    vm.unbind();
}
