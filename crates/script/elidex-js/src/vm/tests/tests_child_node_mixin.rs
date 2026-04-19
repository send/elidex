//! ChildNode mixin tests (WHATWG DOM §5.2.2) — `before` / `after` /
//! `replaceWith` / `remove` installed on `Element.prototype` and
//! `CharacterData.prototype`.

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
// before
// ---------------------------------------------------------------------------

#[test]
fn element_before_inserts_node_before_this() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("globalThis.c = document.createElement('c');")
        .unwrap();
    vm.eval("b.before(c);").unwrap();
    // Expected order: a, c, b.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "C");
    vm.unbind();
}

#[test]
fn element_before_with_string_creates_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("b.before('hello');").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    // The new node is a Text → nodeType 3.
    assert_eq!(eval_num(&mut vm, "p.childNodes[1].nodeType;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].data;"), "hello");
    vm.unbind();
}

#[test]
fn element_before_no_parent_is_noop() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Detached element → before() is a no-op, does not throw.
    vm.eval(
        "var orphan = document.createElement('p');\n\
         orphan.before(document.createElement('span'));",
    )
    .unwrap();
    vm.unbind();
}

// ---------------------------------------------------------------------------
// after
// ---------------------------------------------------------------------------

#[test]
fn element_after_inserts_node_after_this() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("globalThis.c = document.createElement('c');")
        .unwrap();
    vm.eval("a.after(c);").unwrap();
    // Expected: a, c, b.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "C");
    vm.unbind();
}

#[test]
fn element_after_at_end_appends() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    // b is the last child → after() appends.
    vm.eval("b.after(document.createElement('c'));").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "C");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// replaceWith
// ---------------------------------------------------------------------------

#[test]
fn element_replace_with_single_node() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("globalThis.c = document.createElement('c');")
        .unwrap();
    vm.eval("a.replaceWith(c);").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "C");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "B");
    vm.unbind();
}

#[test]
fn element_replace_with_no_args_detaches() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.replaceWith();").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 1.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "B");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// remove (on CharacterData prototype)
// ---------------------------------------------------------------------------

#[test]
fn text_remove_detaches_from_parent() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.p = document.createElement('p');\n\
         var t = document.createTextNode('hi');\n\
         p.appendChild(t);\n\
         t.remove();",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 0.0);
    vm.unbind();
}

#[test]
fn comment_remove_detaches_from_parent() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.p = document.createElement('p');\n\
         var c = document.createComment('note');\n\
         p.appendChild(c);\n\
         c.remove();",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 0.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Multi-arg / DocumentFragment handling
// ---------------------------------------------------------------------------

#[test]
fn element_before_multi_arg_inserts_all_in_order() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("b.before(document.createElement('x'), document.createElement('y'), 'z');")
        .unwrap();
    // Expected: a, x, y, textNode('z'), b.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 5.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "Y");
    assert_eq!(eval_num(&mut vm, "p.childNodes[3].nodeType;"), 3.0);
    vm.unbind();
}

#[test]
fn element_after_document_fragment_flattens() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    // Pass a single DocumentFragment — it should not be re-wrapped.
    // Its children should move into the tree at the insertion point.
    vm.eval(
        "var f = document.createDocumentFragment();\n\
         f.appendChild(document.createElement('x'));\n\
         f.appendChild(document.createElement('y'));\n\
         a.after(f);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "Y");
    vm.unbind();
}

#[test]
fn element_replace_with_multi_arg_replaces_with_all() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.replaceWith(document.createElement('x'), document.createElement('y'));")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "Y");
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "B");
    vm.unbind();
}

#[test]
fn element_after_mixed_string_and_node() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.after('hello', document.createElement('x'));")
        .unwrap();
    // Expected: a, text('hello'), x, b.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_num(&mut vm, "p.childNodes[1].nodeType;"), 3.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].data;"), "hello");
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "X");
    vm.unbind();
}

#[test]
fn before_empty_args_is_noop() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("b.before();").unwrap();
    // Still just [a, b].
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    vm.unbind();
}
