//! ParentNode mixin tests (WHATWG DOM §4.2.6).  Mutation methods
//! (`prepend` / `append` / `replaceChildren`) and read surface
//! (`children` / `firstElementChild` / `lastElementChild` /
//! `childElementCount` / `querySelector` / `querySelectorAll`)
//! installed on Element.prototype + DocumentFragment.prototype +
//! the Document wrapper.  ShadowRoot inherits via the
//! DocumentFragment.prototype chain.

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
fn element_append_user_node_survives_later_arg_throw() {
    // Atomicity guard: a throw during argument conversion must leave the DOM
    // untouched. The VM marshalling step (`normalize_mixin_arg`) is
    // side-effect-free — it classifies each arg (keep a Node wrapper, or
    // ToString-coerce a non-Node to a string) WITHOUT touching the tree — so for
    // `target.append(existingUserNode, Symbol())` the `ToString(Symbol())` throws
    // BEFORE `invoke_dom_api` runs any insertion, and `existingUserNode` stays
    // attached. (Pre-B1.2b the VM built a wrapper fragment first and could strand
    // the user node on a later-arg throw; the convergence onto the dom-api handler
    // removed that hazard by deferring all tree mutation past arg normalisation.)
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Build two parents; move `userNode` into `other` first, then
    // try to append it into `p` along with a throwing Symbol arg.
    vm.eval(
        "globalThis.p = document.createElement('p');\n\
         globalThis.other = document.createElement('other');\n\
         globalThis.userNode = document.createElement('u');\n\
         other.appendChild(userNode);",
    )
    .unwrap();
    let ok = vm
        .eval(
            "var err = null;\n\
             try { p.append(userNode, Symbol()); } catch (e) { err = e; }\n\
             var thrown = err !== null;\n\
             var still_in_other = userNode.parentNode === other;\n\
             thrown + ':' + still_in_other;",
        )
        .unwrap();
    let sid = match ok {
        JsValue::String(id) => id,
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(vm.inner.strings.get_utf8(sid), "true:true");
    vm.unbind();
}

#[test]
fn element_prepend_ancestor_cycle_throws_before_mutation() {
    // Pre-insertion validity (WHATWG §4.2.3): if a later arg is an
    // ancestor of the receiver, `prepend(firstArg, ancestor)` must
    // throw BEFORE firstArg is inserted. no partial mutation.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval(
        "globalThis.grandparent = document.createElement('gp');\n\
         grandparent.appendChild(p);\n\
         globalThis.x = document.createElement('x');",
    )
    .unwrap();
    let threw = vm
        .eval(
            "var err = null;\n\
             try { p.prepend(x, grandparent); } catch (e) { err = e; }\n\
             err !== null && err.name === 'HierarchyRequestError' \
             && err instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    // `x` must NOT have been inserted. throw happened before
    // the first insertion.
    let JsValue::Boolean(x_in_p) = vm
        .eval("Array.from(p.childNodes).indexOf(x) !== -1;")
        .unwrap()
    else {
        panic!()
    };
    assert!(!x_in_p);
    // Original children intact.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    vm.unbind();
}

#[test]
fn element_replace_children_does_not_clear_parent_on_ancestor_cycle() {
    // Pre-insertion validity (WHATWG §4.2.3): if the flattened child
    // list contains a node that is an ancestor of (or equal to) the
    // receiver, `replaceChildren` must throw BEFORE the "replace all"
    // step clears the parent.  Before the fix, the parent was cleared
    // first and we attempted a rollback; nodes that normalization had
    // already moved into the wrapper fragment were lost.
    //
    // We validate the user-observable invariant: children that were
    // *not* passed as args (here `b`) are still children of `p` after
    // the throw. they were never removed because the clear step is
    // gated behind pre-validation.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    make_parent_with_children(&mut vm);
    vm.eval(
        "globalThis.grandparent = document.createElement('gp');\n\
         grandparent.appendChild(p);",
    )
    .unwrap();
    let threw = vm
        .eval(
            "var err = null;\n\
             try { p.replaceChildren(a, grandparent); } catch (e) { err = e; }\n\
             err !== null && err.name === 'HierarchyRequestError' \
             && err instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    // `b` was never an argument and must still be p's child.  (Spec
    // allows `a` to be moved into the ephemeral wrapper fragment as
    // a side effect of "convert nodes into a node".)
    let JsValue::Boolean(b_in_p) = vm
        .eval("Array.from(p.childNodes).indexOf(b) !== -1;")
        .unwrap()
    else {
        panic!()
    };
    assert!(b_in_p);
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
        "globalThis.f = document.createDocumentFragment();\n\
         f.appendChild(document.createElement('x'));\n\
         f.appendChild(document.createElement('y'));\n\
         p.append(f);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 4.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[2].tagName;"), "X");
    assert_eq!(eval_str(&mut vm, "p.childNodes[3].tagName;"), "Y");
    // WHATWG §4.2.3: after pre-insert, the fragment must be empty.
    assert_eq!(eval_num(&mut vm, "f.childNodes.length;"), 0.0);
    vm.unbind();
}

#[test]
fn append_child_never_nests_a_document_fragment() {
    // A DocumentFragment is NEVER linked as a child node: `appendChild` of an
    // EMPTY fragment is a no-op (WHATWG DOM §4.2.3 insert step 3 — count 0 →
    // return), and of a non-empty fragment expands its children (step 1/7). So a
    // fragment cannot be nested inside another via `appendChild` — the prior elidex
    // behavior that linked an empty fragment as a child node was a bug (it made the
    // recursive fragment-flatten path reachable from JS). This is the B1.2-fragment fix.
    //
    // `outer.appendChild(inner)` runs while `inner` is empty ⇒ no-op (outer stays
    // empty, inner not linked). Populating `inner` afterward and `append`-ing it to
    // `p` then flattens `inner`'s child into `p` and empties `inner`.
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.p = document.createElement('p');\n\
         globalThis.outer = document.createDocumentFragment();\n\
         globalThis.inner = document.createDocumentFragment();\n\
         outer.appendChild(inner);\n\
         globalThis.outerLenAfterAppendChild = outer.childNodes.length;\n\
         globalThis.appendChildReturnedInner = (outer.appendChild(inner) === inner);\n\
         inner.appendChild(document.createElement('leaf'));\n\
         p.append(inner);",
    )
    .unwrap();
    // appendChild of the (empty) fragment did NOT nest it as a child of `outer`.
    assert_eq!(
        eval_num(&mut vm, "outerLenAfterAppendChild;"),
        0.0,
        "appendChild of an empty fragment must be a no-op, not link it as a child"
    );
    assert_eq!(
        vm.eval("appendChildReturnedInner").unwrap(),
        JsValue::Boolean(true),
        "appendChild still returns its argument (the fragment) per WebIDL"
    );
    // The populated fragment then flattens into `p` and empties.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 1.0);
    assert_eq!(eval_str(&mut vm, "p.childNodes[0].tagName;"), "LEAF");
    assert_eq!(eval_num(&mut vm, "inner.childNodes.length;"), 0.0);
    assert_eq!(eval_num(&mut vm, "outer.childNodes.length;"), 0.0);
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
fn document_replace_children_with_element_child_throws_hierarchy_error() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Seed the document with its single element child. This first
    // `document.appendChild` is the documentElement — valid even under the
    // B1.2b-3 pre-insertion-validity fold (§4.2.3 step 6 allows ONE element child);
    // a *second* element via `document.appendChild` would now throw
    // HierarchyRequestError too, exactly like the `replaceChildren` below.
    vm.eval("document.appendChild(document.createElement('a'));")
        .unwrap();
    // `replaceChildren` runs "ensure pre-insertion validity" (WHATWG DOM §4.2.3
    // step 6, Element branch: "parent has an element child") BEFORE the
    // "replace all" step clears the children — so a Document that already has an
    // element child rejects a replacement element with HierarchyRequestError.
    // This matches the spec + boa; the VM previously skipped the check (a lax
    // path removed by the B1.2b convergence onto the dom-api handler).
    let threw = eval_num(
        &mut vm,
        "var err = null;\n\
         try { document.replaceChildren(document.createElement('root')); }\n\
         catch (e) { err = e; }\n\
         (err !== null && err.name === 'HierarchyRequestError') ? 1 : 0;",
    );
    assert_eq!(
        threw, 1.0,
        "replaceChildren must throw HierarchyRequestError"
    );
    // Validity throws before "replace all" mutates, so the document is unchanged.
    assert_eq!(eval_num(&mut vm, "document.childNodes.length;"), 1.0);
    assert_eq!(eval_str(&mut vm, "document.childNodes[0].tagName;"), "A");
    vm.unbind();
}

#[test]
fn document_append_child_second_element_throws_hierarchy_error() {
    // B1.2b-3 (the headline JS-observable fix): `document.appendChild` now runs
    // §4.2.3 ensure-pre-insertion-validity (step 6, Element branch: a Document may
    // have at most one element child), so a SECOND element throws
    // HierarchyRequestError. Before the validity fold the lax empty-record-list
    // path on the core Node method silently appended it (the mixins already threw).
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // First element = the documentElement → valid.
    vm.eval("document.appendChild(document.createElement('html'));")
        .unwrap();
    let threw = eval_num(
        &mut vm,
        "var err = null;\n\
         try { document.appendChild(document.createElement('body')); }\n\
         catch (e) { err = e; }\n\
         (err !== null && err.name === 'HierarchyRequestError' \
          && err instanceof DOMException) ? 1 : 0;",
    );
    assert_eq!(
        threw, 1.0,
        "a second document.appendChild element must throw HierarchyRequestError"
    );
    // The throw precedes any mutation — the document keeps its single element child.
    assert_eq!(eval_num(&mut vm, "document.childNodes.length;"), 1.0);
    assert_eq!(eval_str(&mut vm, "document.childNodes[0].tagName;"), "HTML");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Reader surface — DocumentFragment.prototype (covers
// `createDocumentFragment()` + `<template>.content` + ShadowRoot
// via the inherited chain).
// ---------------------------------------------------------------------------

fn seed_fragment_with_mixed_children(vm: &mut Vm) {
    vm.eval(
        "globalThis.f = document.createDocumentFragment();\n\
         f.appendChild(document.createTextNode('lead'));\n\
         globalThis.a = document.createElement('a');\n\
         a.id = 'first';\n\
         f.appendChild(a);\n\
         f.appendChild(document.createTextNode('mid'));\n\
         globalThis.b = document.createElement('b');\n\
         b.className = 'pick';\n\
         f.appendChild(b);\n\
         f.appendChild(document.createTextNode('tail'));",
    )
    .unwrap();
}

#[test]
fn fragment_first_element_child_skips_leading_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    assert_eq!(eval_str(&mut vm, "f.firstElementChild.tagName;"), "A");
    vm.unbind();
}

#[test]
fn fragment_last_element_child_skips_trailing_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    assert_eq!(eval_str(&mut vm, "f.lastElementChild.tagName;"), "B");
    vm.unbind();
}

#[test]
fn fragment_children_returns_live_collection_excluding_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    assert_eq!(eval_num(&mut vm, "f.children.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "f.children[0].tagName;"), "A");
    // Live semantics: appending a third element bumps the count
    // observed via a fresh `f.children` access.
    vm.eval("f.appendChild(document.createElement('c'));")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "f.children.length;"), 3.0);
    vm.unbind();
}

#[test]
fn fragment_child_element_count_matches_children_length() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    assert_eq!(eval_num(&mut vm, "f.childElementCount;"), 2.0);
    assert_eq!(
        eval_str(
            &mut vm,
            "(f.childElementCount === f.children.length) ? 'eq' : 'neq';",
        ),
        "eq",
    );
    vm.unbind();
}

#[test]
fn fragment_query_selector_finds_descendant() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    assert_eq!(eval_str(&mut vm, "f.querySelector('#first').tagName;"), "A");
    assert_eq!(eval_str(&mut vm, "f.querySelector('.pick').tagName;"), "B");
    assert_eq!(
        eval_str(
            &mut vm,
            "(f.querySelector('.missing') === null) ? 'null' : 'not-null';",
        ),
        "null",
    );
    vm.unbind();
}

#[test]
fn fragment_query_selector_all_returns_static_snapshot() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    seed_fragment_with_mixed_children(&mut vm);
    vm.eval("globalThis.snap = f.querySelectorAll('a, b');")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "snap.length;"), 2.0);
    // Static: a subsequent append does NOT affect the captured snapshot.
    vm.eval("f.appendChild(document.createElement('a'));")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "snap.length;"), 2.0);
    assert_eq!(eval_num(&mut vm, "f.querySelectorAll('a').length;"), 2.0);
    vm.unbind();
}

#[test]
fn template_content_query_selector_finds_descendant() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // `<template>.content` is a DocumentFragment per HTML §4.12.3, so
    // the ParentNode reader surface installed on
    // DocumentFragment.prototype must reach it.
    vm.eval(
        "globalThis.t = document.createElement('template');\n\
         t.content.appendChild(document.createElement('p'));",
    )
    .unwrap();
    assert_eq!(
        eval_str(&mut vm, "t.content.querySelector('p').tagName;"),
        "P"
    );
    assert_eq!(eval_num(&mut vm, "t.content.childElementCount;"), 1.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Reader surface — Document wrapper (per-bind install via
// `DOCUMENT_RO_ACCESSORS`).
// ---------------------------------------------------------------------------

fn setup_with_html_root() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    (vm, session, dom, doc)
}

#[test]
fn document_first_element_child_is_document_element() {
    let (mut vm, mut session, mut dom, doc) = setup_with_html_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert_eq!(
        eval_str(
            &mut vm,
            "(document.firstElementChild === document.documentElement) ? 'eq' : 'neq';",
        ),
        "eq",
    );
    vm.unbind();
}

#[test]
fn document_children_contains_only_document_element() {
    let (mut vm, mut session, mut dom, doc) = setup_with_html_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    assert_eq!(eval_num(&mut vm, "document.children.length;"), 1.0);
    assert_eq!(eval_num(&mut vm, "document.childElementCount;"), 1.0);
    assert_eq!(
        eval_str(
            &mut vm,
            "(document.children[0] === document.documentElement) ? 'eq' : 'neq';",
        ),
        "eq",
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Reader surface — ShadowRoot (implicit win via the ShadowRoot →
// DocumentFragment.prototype chain).
// ---------------------------------------------------------------------------

#[test]
fn shadow_root_reader_mixin_reaches_via_document_fragment_chain() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.host = document.createElement('div');\n\
         globalThis.sr = host.attachShadow({mode: 'open'});\n\
         sr.appendChild(document.createTextNode('lead'));\n\
         sr.appendChild(document.createElement('span'));\n\
         sr.appendChild(document.createElement('em'));",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "sr.children.length;"), 2.0);
    assert_eq!(eval_str(&mut vm, "sr.firstElementChild.tagName;"), "SPAN");
    assert_eq!(eval_str(&mut vm, "sr.querySelector('em').tagName;"), "EM");
    vm.unbind();
}
