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

#[test]
fn element_replace_with_self_is_noop() {
    // WHATWG §5.2.2 `replaceWith`: if an arg equals `this`, the
    // viable-next-sibling walk skips over it and the remove/insert
    // cycle restores the node at its original position — effectively
    // a no-op.  Must not throw (the old impl tripped
    // `insert_before(parent, this, this)` and raised
    // HierarchyRequestError).
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.replaceWith(a);").unwrap();
    // Tree unchanged: p > (a, b) in original order.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "A");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "B");
    vm.unbind();
}

#[test]
fn element_replace_with_ancestor_cycle_preserves_tree() {
    // Pre-insertion validity (WHATWG §5.2.2 step 3): if an arg is
    // an ancestor of the receiver's parent (which would create a
    // cycle on insert), the method must throw BEFORE removing
    // the receiver.  Before this fix `replaceWith` detached
    // `entity` first and threw after the failed insert, leaving
    // the tree partially mutated.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    // Build: grandparent > p > (a, b).  `a.replaceWith(grandparent)`
    // — grandparent is p's ancestor, so insert would cycle.
    vm.eval(
        "globalThis.grandparent = document.createElement('gp');\n\
         grandparent.appendChild(p);",
    )
    .unwrap();
    let threw = vm
        .eval(
            "var err = null;\n\
             try { a.replaceWith(grandparent); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    // `a` must still be p's child: the throw happened before the
    // detach step, so the tree is unchanged.
    let JsValue::Boolean(a_still_child) = vm
        .eval("Array.prototype.slice.call(p.childNodes).indexOf(a) !== -1;")
        .unwrap()
    else {
        panic!()
    };
    assert!(a_still_child);
    vm.unbind();
}

#[test]
fn child_node_brand_check_rejects_document_fragment_receiver() {
    // WebIDL branding: ChildNode mixin methods installed on
    // `Element.prototype` / `CharacterData.prototype` must throw
    // TypeError when `.call`'d with a DocumentFragment receiver
    // (which doesn't implement ChildNode).
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let threw = vm
        .eval(
            "var el = document.createElement('p');\n\
             var f = document.createDocumentFragment();\n\
             var err = null;\n\
             try { el.before.call(f); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
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
fn element_before_nested_fragment_is_recursively_flattened() {
    // When an outer DocumentFragment contains another fragment as a
    // child, every level must be flattened so only concrete nodes
    // land in the tree.  Reach this by passing a single outer
    // fragment that has a nested fragment child.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval(
        "var outer = document.createDocumentFragment();\n\
         var inner = document.createDocumentFragment();\n\
         inner.appendChild(document.createElement('x'));\n\
         inner.appendChild(document.createElement('y'));\n\
         outer.appendChild(inner);\n\
         b.before(outer);",
    )
    .unwrap();
    // Expected: [a, x, y, b].
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "Y");
    vm.unbind();
}

#[test]
fn element_before_self_insert_throws() {
    // Inserting an ancestor as a child of its own subtree creates a
    // cycle — EcsDom rejects, we surface TypeError (matching the
    // Node.appendChild convention).
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    let threw = vm
        .eval(
            "var err = null;\n\
             try { a.before(p); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
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
fn element_before_self_is_noop() {
    // WHATWG: `el.before(el)` is a no-op (the receiver is already
    // its own "viable previous sibling").  Must not throw, order
    // unchanged.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("b.before(b);").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "A");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "B");
    vm.unbind();
}

#[test]
fn element_after_self_is_noop() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.after(a);").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "A");
    vm.unbind();
}

#[test]
fn element_after_next_sibling_is_noop() {
    // `a.after(b)` where b is already a's next sibling → no-op.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval("a.after(b);").unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "A");
    assert_eq!(eval_str(&mut vm, "p.childNodes[1].tagName;"), "B");
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
