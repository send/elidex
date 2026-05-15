//! Integration tests for `TreeWalker` / `NodeIterator` / `NodeFilter`
//! (WHATWG DOM §6).  Slot `#11-traversal-and-range-pr-a2-bindings`.
//!
//! Covers: filter callback dispatch + Accept / Skip / Reject;
//! NodeFilter constant namespace; document factories;
//! null-filter walker; ToUnsignedShort coercion on return value;
//! filter re-entrancy InvalidStateError; spec §6.1 pre-removing-steps
//! adjustment via mutation hook.

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

#[allow(unsafe_code)]
unsafe fn bind(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, doc: elidex_ecs::Entity) {
    unsafe { bind_vm(vm, session, dom, doc) };
}

/// Build a 3-level tree under `documentElement`:
/// `documentElement → [<section> → [<p>, <em>], <aside>]`
fn build_simple_tree(vm: &mut Vm) {
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.section = document.createElement('section');\
         globalThis.p = document.createElement('p');\
         globalThis.em = document.createElement('em');\
         globalThis.aside = document.createElement('aside');\
         root.appendChild(section);\
         section.appendChild(p);\
         section.appendChild(em);\
         root.appendChild(aside);",
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// NodeFilter namespace
// ---------------------------------------------------------------------------

#[test]
fn node_filter_constants_installed() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    assert_eq!(eval_num(&mut vm, "NodeFilter.FILTER_ACCEPT"), 1.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.FILTER_REJECT"), 2.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.FILTER_SKIP"), 3.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_ELEMENT"), 1.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_TEXT"), 4.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_COMMENT"), 128.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_DOCUMENT"), 256.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_DOCUMENT_TYPE"), 512.0);
    assert_eq!(eval_num(&mut vm, "NodeFilter.SHOW_ALL"), 4_294_967_295.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// TreeWalker — no filter
// ---------------------------------------------------------------------------

#[test]
fn tree_walker_walks_elements_in_order() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    // First `nextNode` from documentElement goes to first child = section.
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "SECTION");
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "P");
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "EM");
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "ASIDE");
    let v = vm.eval("w.nextNode()").unwrap();
    assert!(matches!(v, super::super::value::JsValue::Null));
    vm.unbind();
}

#[test]
fn tree_walker_first_child_descends_into_first() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.firstChild().tagName"), "SECTION");
    assert_eq!(eval_str(&mut vm, "w.firstChild().tagName"), "P");
    vm.unbind();
}

#[test]
fn tree_walker_parent_returns_root_when_filter_accepts() {
    // Copilot R19: WHATWG §6.4 parentNode allows the walker's
    // root to be returned when it passes whatToShow + filter.
    // Browsers (Chrome / Firefox) return the root in this case.
    // The earlier (R7) implementation short-circuited on
    // `parent == root` before filtering, returning null
    // incorrectly.  Per spec, only an attempt to ascend ABOVE
    // root yields null.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);\
         w.currentNode = p;",
    )
    .unwrap();
    // p → section (Element, accepted).
    assert_eq!(eval_str(&mut vm, "w.parentNode().tagName"), "SECTION");
    // section → root (DIV, Element, accepted) — returns root.
    assert_eq!(eval_str(&mut vm, "w.parentNode().tagName"), "DIV");
    // From root itself, parentNode returns null (would ascend above root).
    let v = vm.eval("w.parentNode()").unwrap();
    assert!(matches!(v, super::super::value::JsValue::Null));
    vm.unbind();
}

#[test]
fn tree_walker_parent_skips_root_when_filter_rejects() {
    // Copilot R19: when the root does NOT pass whatToShow, the
    // ascent from a descendant must skip over the root and end
    // with null (the loop's `node != root` guard then exits the
    // loop).
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    // SHOW_TEXT alone — root (DIV element) does NOT match.
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_TEXT);\
         w.currentNode = p;",
    )
    .unwrap();
    // p → ascends to section (skip, not Text), then root (skip),
    // exits loop without setting current — null.
    let v = vm.eval("w.parentNode()").unwrap();
    assert!(matches!(v, super::super::value::JsValue::Null));
    vm.unbind();
}

#[test]
fn tree_walker_sibling_walks() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);\
         w.currentNode = section;",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.nextSibling().tagName"), "ASIDE");
    assert_eq!(eval_str(&mut vm, "w.previousSibling().tagName"), "SECTION");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// TreeWalker — filter
// ---------------------------------------------------------------------------

#[test]
fn tree_walker_filter_accept_returns_node() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.visited = [];\
         globalThis.filter = function(n) { visited.push(n.tagName); return NodeFilter.FILTER_ACCEPT; };\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filter);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "SECTION");
    // Filter sees one Accept call for SECTION.
    let len = eval_num(&mut vm, "visited.length");
    assert!(len >= 1.0);
    vm.unbind();
}

#[test]
fn tree_walker_filter_reject_prunes_subtree() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "var filter = function(n) { \
            if (n.tagName === 'SECTION') return NodeFilter.FILTER_REJECT; \
            return NodeFilter.FILTER_ACCEPT; \
         };\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filter);",
    )
    .unwrap();
    // Reject 'SECTION' must skip <p> and <em>, jump to <aside>.
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "ASIDE");
    vm.unbind();
}

#[test]
fn tree_walker_filter_skip_descends_into_subtree() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "var filter = function(n) { \
            if (n.tagName === 'SECTION') return NodeFilter.FILTER_SKIP; \
            return NodeFilter.FILTER_ACCEPT; \
         };\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filter);",
    )
    .unwrap();
    // Skip 'SECTION' must still visit its descendants.
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "P");
    vm.unbind();
}

#[test]
fn tree_walker_filter_object_with_accept_node() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.filterObj = { acceptNode: function(n) { return NodeFilter.FILTER_ACCEPT; } };\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filterObj);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "SECTION");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// TreeWalker — accessor read-back
// ---------------------------------------------------------------------------

#[test]
fn tree_walker_accessors_reflect_state() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.root.tagName"), "DIV");
    assert_eq!(eval_num(&mut vm, "w.whatToShow"), 1.0);
    let f = vm.eval("w.filter").unwrap();
    assert!(matches!(f, super::super::value::JsValue::Null));
    assert_eq!(eval_str(&mut vm, "w.currentNode.tagName"), "DIV");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// NodeIterator
// ---------------------------------------------------------------------------

#[test]
fn node_iterator_walks_in_pre_order() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.it = document.createNodeIterator(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "DIV");
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "SECTION");
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "P");
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "EM");
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "ASIDE");
    let v = vm.eval("it.nextNode()").unwrap();
    assert!(matches!(v, super::super::value::JsValue::Null));
    vm.unbind();
}

#[test]
fn node_iterator_detach_is_noop() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.it = document.createNodeIterator(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    let v = vm.eval("it.detach()").unwrap();
    assert!(matches!(v, super::super::value::JsValue::Undefined));
    // After detach() the iterator still works.
    assert_eq!(eval_str(&mut vm, "it.nextNode().tagName"), "DIV");
    vm.unbind();
}

#[test]
fn tree_walker_filter_with_accessor_accept_node() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    // Copilot R2 — accessor (getter-defined) `acceptNode` on the
    // filter object must resolve via WebIDL `Get` semantics rather
    // than being treated as non-callable.
    vm.eval(
        "globalThis.filterObj = Object.defineProperty({}, 'acceptNode', {\
            get: function() { return function(n) { return NodeFilter.FILTER_ACCEPT; }; }\
         });\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filterObj);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "SECTION");
    vm.unbind();
}

#[test]
fn node_iterator_detach_brand_check_throws_on_non_iterator() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R1 — WebIDL brand check on no-op operation.
    let res = vm.eval("NodeIterator.prototype.detach.call({});");
    assert!(
        res.is_err(),
        "detach.call(non-NodeIterator) must throw TypeError"
    );
    vm.unbind();
}

#[test]
fn node_iterator_accessors_reflect_state() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.it = document.createNodeIterator(\
             root, NodeFilter.SHOW_ELEMENT);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "it.root.tagName"), "DIV");
    assert_eq!(eval_num(&mut vm, "it.whatToShow"), 1.0);
    assert_eq!(eval_str(&mut vm, "it.referenceNode.tagName"), "DIV");
    assert_eq!(
        eval_str(
            &mut vm,
            "it.pointerBeforeReferenceNode ? 'before' : 'after'"
        ),
        "before"
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Constructor — Illegal
// ---------------------------------------------------------------------------

#[test]
fn tree_walker_constructor_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    let res = vm.eval("new TreeWalker()");
    assert!(res.is_err());
    vm.unbind();
}

#[test]
fn node_iterator_constructor_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    let res = vm.eval("new NodeIterator()");
    assert!(res.is_err());
    vm.unbind();
}

// ---------------------------------------------------------------------------
// WebIDL coercion on return value — ToUnsignedShort
// ---------------------------------------------------------------------------

#[test]
fn filter_return_value_coerces_via_to_uint16() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "var filter = function(n) { \
            if (n.tagName === 'SECTION') return -1; /* -> 65535 -> Skip */\
            return NodeFilter.FILTER_ACCEPT; \
         };\
         globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT, filter);",
    )
    .unwrap();
    // -1 wraps to 65535 → Skip (descend) so 'P' is visited.
    assert_eq!(eval_str(&mut vm, "w.nextNode().tagName"), "P");
    vm.unbind();
}

#[test]
fn tree_walker_current_node_setter_throws_when_detached() {
    // Copilot R12: setter must (a) consult walker state BEFORE
    // coercing the node argument so it cannot panic on
    // `ctx.host().dom()` after `Vm::unbind()`, and (b) surface the
    // same detached-walker error as the getter / traversal methods
    // for cross-method consistency.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.alt = document.createElement('span');\
         globalThis.w = document.createTreeWalker(root, 0xFFFFFFFF, null);",
    )
    .unwrap();
    vm.unbind();
    // Retained walker + retained node: setter must throw rather
    // than panic on `host().dom()` access during node coercion.
    let res = vm.eval("w.currentNode = alt;");
    assert!(
        res.is_err(),
        "currentNode setter on retained walker after unbind must throw, not panic"
    );
}

#[test]
fn tree_walker_siblings_at_root_return_null() {
    // Copilot R15: WHATWG §6.4 traverseSiblings step 2 — when
    // `currentNode === root`, both `nextSibling()` and
    // `previousSibling()` must return null without escaping the
    // walker's subtree.  Without the early-out, `get_next_sibling`
    // on the root would walk OUT of the walker's view and return
    // a node that is technically a sibling of root in the wider
    // document.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.outer = document.createElement('div');\
         globalThis.root = document.createElement('section');\
         globalThis.outerSib = document.createElement('aside');\
         /* outer's children: [root, outerSib].  If the walker's
            root looked beyond its subtree, nextSibling() would
            return `outerSib`. */\
         outer.appendChild(root);\
         outer.appendChild(outerSib);\
         /* Inner children so descend paths exist but shouldn't fire. */\
         root.appendChild(document.createElement('p'));\
         globalThis.w = document.createTreeWalker(\
             root, 0xFFFFFFFF, null);\
         /* currentNode is root by default. */",
    )
    .unwrap();
    assert_eq!(
        eval_str(&mut vm, "w.nextSibling() === null ? 'null' : 'non'"),
        "null"
    );
    assert_eq!(
        eval_str(&mut vm, "w.previousSibling() === null ? 'null' : 'non'"),
        "null"
    );
    // currentNode unchanged after the rejected step.
    assert_eq!(
        eval_str(&mut vm, "w.currentNode === root ? 'root' : 'moved'"),
        "root"
    );
    vm.unbind();
}

#[test]
fn document_factory_call_on_plain_object_throws() {
    // Copilot R16: `document.createRange.call({})` (and the
    // sibling traversal factories) must throw a `TypeError`
    // "Illegal invocation" because the WebIDL brand check
    // requires `this` to be a Document instance.  Prior to R16
    // the factories swallowed the non-HostObject receiver as the
    // unbound-VM silent-null fallback.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    let res1 = vm.eval("document.createRange.call({});");
    assert!(res1.is_err(), "createRange.call({{}}) must throw");
    let res2 = vm.eval("document.createTreeWalker.call({}, document.body, 0xFFFFFFFF, null);");
    assert!(res2.is_err(), "createTreeWalker.call({{}}, ...) must throw");
    let res3 = vm.eval("document.createNodeIterator.call({}, document.body, 0xFFFFFFFF, null);");
    assert!(
        res3.is_err(),
        "createNodeIterator.call({{}}, ...) must throw"
    );
    // Primitive `this` should also throw.
    let res4 = vm.eval("document.createRange.call(42);");
    assert!(res4.is_err(), "createRange.call(42) must throw");
    vm.unbind();
}

#[test]
fn document_create_walker_rejects_non_object_filter() {
    // Copilot R17: WebIDL `NodeFilter?` callback interface
    // conversion (§3.10) — null/undefined → null, Object →
    // callback, anything else → TypeError.  Prior to R17,
    // primitives silently became null and created an unfiltered
    // walker / iterator.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.root = document.createElement('div');")
        .unwrap();
    // Number primitive — must throw.
    let r1 = vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, 42);");
    assert!(r1.is_err(), "createTreeWalker(.., .., 42) must throw");
    let r2 = vm.eval("document.createNodeIterator(root, NodeFilter.SHOW_ALL, 42);");
    assert!(r2.is_err(), "createNodeIterator(.., .., 42) must throw");
    // String primitive — must throw.
    let r3 = vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, 'x');");
    assert!(r3.is_err(), "createTreeWalker(.., .., 'x') must throw");
    // Boolean primitive — must throw.
    let r4 = vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, true);");
    assert!(r4.is_err(), "createTreeWalker(.., .., true) must throw");
    // null / undefined still accepted as "no filter".
    vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, null);")
        .unwrap();
    vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, undefined);")
        .unwrap();
    // Plain object accepted (lazy acceptNode lookup at dispatch).
    vm.eval("document.createTreeWalker(root, NodeFilter.SHOW_ALL, {});")
        .unwrap();
    vm.unbind();
}

#[test]
fn node_iterator_ancestor_of_root_removed_does_not_escape() {
    // Copilot R18: WHATWG §6.1 pre-removing steps run on every
    // NodeIterator whose root.node_document matches the removed
    // subtree's document.  When an ANCESTOR of the iterator's
    // root is removed, the `descendants` snapshot includes
    // `state.root` itself.  The previous impl ran the fallback
    // candidate path which selected from parent's siblings —
    // OUTSIDE the iterator's configured subtree.  Subsequent
    // `nextNode` / `previousNode` would then walk OUT of the
    // iterator's intended view.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.holder = document.createElement('main');\
         globalThis.outer = document.createElement('div');\
         globalThis.iterRoot = document.createElement('section');\
         globalThis.child = document.createElement('p');\
         globalThis.outerSib = document.createElement('aside');\
         holder.appendChild(outer);\
         holder.appendChild(outerSib);\
         outer.appendChild(iterRoot);\
         iterRoot.appendChild(child);\
         globalThis.it = document.createNodeIterator(\
             iterRoot, 0xFFFFFFFF, null);\
         /* Advance into the subtree so reference != root. */\
         it.nextNode();\
         it.nextNode();",
    )
    .unwrap();
    // Remove `outer` — an ancestor of `iterRoot`.  descendants
    // snapshot = [outer, iterRoot, child] (inclusive).  The
    // iterator's reference (`child`) and root (`iterRoot`) are
    // both in descendants.  Without the R18 guard, the fallback
    // would select `outerSib` (outer's sibling) as a follower.
    vm.eval("holder.removeChild(outer);").unwrap();
    // The iterator's referenceNode should NOT have moved to
    // `outerSib` — it should remain inside the (now-detached)
    // iterRoot subtree.
    let escaped = vm.eval("it.referenceNode === outerSib").unwrap();
    assert!(
        !matches!(escaped, super::super::value::JsValue::Boolean(true)),
        "iterator must not escape its configured root subtree"
    );
    vm.unbind();
}
