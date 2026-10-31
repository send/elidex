//! Unit cells U7, U8, U10, U11, U12 (umbrella §6.1; plan §4.3): the bind pass
//! (`EventHandlerAttributeConsumer::initialize_from_tree`, T-a1).
//!
//! Pre-bind shape (plan §4.1): the tree is built with **no** dispatcher
//! installed, then `initialize_from_tree` runs on a fresh consumer; every
//! second call uses another fresh consumer (umbrella H13).
//!
//! U8 (iii) asserts a delivery the pre-fix base cannot make (plan §0.6 C-1),
//! so it is a separate red-first test beside the U8 (i), (ii) guard.

use elidex_ecs::{EcsDom, Entity, ShadowRootMode, MAX_ANCESTOR_DEPTH};

use super::test_support::{
    append, compat, elem, entry, init_fresh, is_marked, page, raw, take_uncompiled, Entry,
};
use super::PreBindHandlerAttrsProcessed;

fn doc_marked(dom: &EcsDom, doc: Entity) -> bool {
    dom.world()
        .get::<&PreBindHandlerAttrsProcessed>(doc)
        .is_ok()
}

// ---- U7 (red, G1 / K-4): the first bind delivers; a later bind processes nothing ----
#[test]
fn u7_first_bind_delivers_second_bind_is_noop() {
    let mut dom = EcsDom::new();
    let (doc, body) = page(&mut dom);
    let button = elem(&mut dom, "button", &[("onclick", "A")]);
    append(&mut dom, body, button);
    init_fresh(&mut dom, doc, compat());
    assert_eq!(
        entry(&dom, button, "click"),
        raw("A"),
        "U7: raw A after the first bind"
    );
    assert!(is_marked(&dom, button), "U7: button marked");
    assert!(doc_marked(&dom, doc), "U7: document marked");
    take_uncompiled(&mut dom, button, "click");
    init_fresh(&mut dom, doc, compat());
    assert_eq!(
        entry(&dom, button, "click"),
        Entry::NoSource,
        "U7: the second bind processes nothing"
    );
}

/// U8's world: the bound document holds a `<template>` whose contents hold
/// `<div onclick>` and a shadow host whose shadow root holds
/// `<button onclick>`; a second document holds `<div onclick>`.
/// Returns `(doc, other_doc_div, template_div, shadow_button)`.
fn u8_world(dom: &mut EcsDom) -> (Entity, Entity, Entity, Entity) {
    let (doc, body) = page(dom);
    let d2 = dom.create_document_node();
    let other = elem(dom, "div", &[("onclick", "window.hit = 1")]);
    append(dom, d2, other);
    let tpl = elem(dom, "template", &[]);
    append(dom, body, tpl);
    let frag = dom.attach_template_contents(tpl, Some(doc));
    let in_tpl = elem(dom, "div", &[("onclick", "window.hit = 2")]);
    append(dom, frag, in_tpl);
    let host = elem(dom, "div", &[]);
    append(dom, body, host);
    let sr = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");
    let in_shadow = elem(dom, "button", &[("onclick", "window.hit = 3")]);
    append(dom, sr, in_shadow);
    (doc, other, in_tpl, in_shadow)
}

// ---- U8 (i), (ii) (guard, K-5): another document and template contents are not reached ----
#[test]
fn u8_i_ii_bind_skips_other_document_and_template_contents() {
    let mut dom = EcsDom::new();
    let (doc, other, in_tpl, _in_shadow) = u8_world(&mut dom);
    init_fresh(&mut dom, doc, compat());
    assert_eq!(
        entry(&dom, other, "click"),
        Entry::Absent,
        "U8 (i): no entry"
    );
    assert!(!is_marked(&dom, other), "U8 (i): unmarked");
    assert_eq!(
        entry(&dom, in_tpl, "click"),
        Entry::Absent,
        "U8 (ii): no entry"
    );
    assert!(!is_marked(&dom, in_tpl), "U8 (ii): unmarked");
}

// ---- U8 (iii) (red-first, C-1, K-6): the bind pass walks shadow trees ----
#[test]
fn u8_iii_bind_delivers_inside_shadow_tree() {
    let mut dom = EcsDom::new();
    let (doc, _other, _in_tpl, in_shadow) = u8_world(&mut dom);
    init_fresh(&mut dom, doc, compat());
    assert_eq!(
        entry(&dom, in_shadow, "click"),
        raw("window.hit = 3"),
        "U8 (iii): delivered"
    );
}

// ---- U10 (red, K-6 depth): a chain deeper than MAX_ANCESTOR_DEPTH ----
/// Stack for U10's bind thread (plan §4.6, "bind walk recursive" row). The
/// iterative walk passes with 32 KiB in a debug build (measured 2026-10-28),
/// so this leaves 8x headroom; a recursive walk overflows it at this depth.
const U10_BIND_STACK_BYTES: usize = 256 * 1024;

#[test]
fn u10_bind_reaches_past_the_depth_cap() {
    let mut dom = EcsDom::new();
    let (doc, body) = page(&mut dom);
    // Bottom-up (the `dom/tests/destroy.rs` precedent): each append links a
    // parentless node, O(1).
    let deepest = elem(&mut dom, "div", &[("onclick", "A")]);
    let mut top = deepest;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let parent = elem(&mut dom, "div", &[]);
        append(&mut dom, parent, top);
        top = parent;
    }
    append(&mut dom, body, top);
    // The bind runs on a thread with a fixed, small stack, so the cell does
    // not depend on the test harness's default stack size: an iterative walk
    // keeps its work list on the heap and fits, while a recursive one needs a
    // frame per level and overflows (which aborts the test binary).
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(U10_BIND_STACK_BYTES)
            .spawn_scoped(s, || init_fresh(&mut dom, doc, compat()))
            .expect("U10: spawn the bind thread")
            .join()
            .expect("U10: the bind thread completes");
    });
    assert!(
        is_marked(&dom, deepest),
        "U10: the deepest element is marked"
    );
    assert_eq!(entry(&dom, deepest, "click"), raw("A"), "U10: raw A");
}

// ---- U11 (red, K-6 width): a parent wider than MAX_ANCESTOR_DEPTH ----
#[test]
fn u11_bind_reaches_past_the_breadth_cap() {
    let mut dom = EcsDom::new();
    let (doc, body) = page(&mut dom);
    let parent = elem(&mut dom, "div", &[]);
    append(&mut dom, body, parent);
    let n = MAX_ANCESTOR_DEPTH + 5;
    let mut children = Vec::with_capacity(n);
    for _ in 0..n {
        let c = elem(&mut dom, "span", &[("onclick", "A")]);
        append(&mut dom, parent, c);
        children.push(c);
    }
    init_fresh(&mut dom, doc, compat());
    assert!(
        is_marked(&dom, children[0]),
        "U11: the first child is marked"
    );
    assert!(
        is_marked(&dom, children[n - 1]),
        "U11: the last child is marked"
    );
    let marked = children.iter().filter(|c| is_marked(&dom, **c)).count();
    assert_eq!(marked, n, "U11: every child is marked");
}

// ---- U12 (red, K-6 / D-5): the pass is keyed on the bound document, not document_root() ----
#[test]
fn u12_bind_keyed_on_bound_document() {
    let mut dom = EcsDom::new();
    let (root, root_body) = page(&mut dom);
    let d2 = dom.create_document_node();
    assert_ne!(dom.document_root(), Some(d2), "d2 is not document_root()");
    assert_eq!(dom.document_root(), Some(root));
    let html2 = elem(&mut dom, "html", &[]);
    let body2 = elem(&mut dom, "body", &[]);
    append(&mut dom, d2, html2);
    append(&mut dom, html2, body2);
    let in_d2 = elem(&mut dom, "button", &[("onclick", "A")]);
    append(&mut dom, body2, in_d2);
    let in_root = elem(&mut dom, "button", &[("onclick", "R")]);
    append(&mut dom, root_body, in_root);
    init_fresh(&mut dom, d2, compat());
    assert_eq!(
        entry(&dom, in_d2, "click"),
        raw("A"),
        "U12: d2's button delivered"
    );
    assert!(doc_marked(&dom, d2), "U12: d2 marked");
    assert!(
        !is_marked(&dom, in_root),
        "U12: the root tree's button is unmarked"
    );
    assert_eq!(
        entry(&dom, in_root, "click"),
        Entry::Absent,
        "U12: the root tree's button has no entry"
    );
}
