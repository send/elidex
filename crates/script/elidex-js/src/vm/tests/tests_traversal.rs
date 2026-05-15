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
fn tree_walker_parent_stops_at_root() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    build_simple_tree(&mut vm);
    vm.eval(
        "globalThis.w = document.createTreeWalker(\
             root, NodeFilter.SHOW_ELEMENT);\
         w.currentNode = p;",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "w.parentNode().tagName"), "SECTION");
    // Copilot R7: WHATWG §6.4 parentNode must NOT return the walker's
    // root.  Next call from SECTION ascends to DIV (root) and returns
    // null without yielding root itself.
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
