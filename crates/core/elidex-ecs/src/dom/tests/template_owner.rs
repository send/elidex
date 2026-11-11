//! C1 unit cells (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
//! §6.2): the inert template contents owner document (WHATWG HTML §4.12.3
//! *appropriate template contents owner document*), its lifetime, its clone
//! policy, its root exclusion, and C0's adopt walk mapping template contents
//! to it (HTML §4.12.3 template *adopting steps*).
//!
//! U3, U4 (its parser half), U5 and U12 need a parser and live in
//! `elidex-html-parser` (`tests_template_owner.rs`); U14 needs the dom-api
//! clone post-pass and lives in `elidex-dom-api`
//! (`node_methods/tests/clone_template_owner.rs`).
//!
//! Red-first: these cells name C1's NEW API (the rule, the predicate, the two
//! components), so on the base they are red by absence (compile error); U10 is
//! a guard.

use super::*;
use crate::components::{AssociatedInertTemplateDocument, InertTemplateDocument};
use crate::dom::MAX_ANCESTOR_DEPTH;

/// How many entities carry [`InertTemplateDocument`].
fn marked(dom: &EcsDom) -> usize {
    dom.world().query::<&InertTemplateDocument>().iter().count()
}

/// An element of `tag` created in `document`.
fn el_in(dom: &mut EcsDom, tag: &'static str, document: Entity) -> Entity {
    let owner = dom.node_document_for_new_nodes(document);
    dom.create_element_with_owner(tag, Attributes::default(), owner)
}

/// A `<template>` created in `document`, with its contents established
/// (HTML §4.12.3 establishing steps). Returns (template, contents).
fn template_in(dom: &mut EcsDom, document: Entity) -> (Entity, Entity) {
    let tpl = el_in(dom, "template", document);
    let owner = dom.node_document_for_new_nodes(tpl);
    let contents = dom.attach_template_contents(tpl, owner);
    (tpl, contents)
}

// ---- U1: f(D) is one marked entity, linked from D ----
#[test]
fn u1_rule_returns_one_linked_marked_document() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let i1 = dom.appropriate_template_contents_owner_document(d);
    let i2 = dom.appropriate_template_contents_owner_document(d);
    assert_eq!(i1, i2);
    assert_ne!(i1, d);
    assert!(dom.is_inert_template_document(i1));
    assert!(!dom.is_inert_template_document(d));
    assert!(matches!(dom.node_kind(i1), Some(NodeKind::Document)));
    assert_eq!(
        dom.world()
            .get::<&AssociatedInertTemplateDocument>(d)
            .map(|l| l.0)
            .ok(),
        Some(i1)
    );
}

// ---- U2: idempotence, no I of I ----
#[test]
fn u2_rule_is_idempotent_over_the_pair() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let i = dom.appropriate_template_contents_owner_document(d);
    assert_eq!(dom.appropriate_template_contents_owner_document(i), i);
    assert_eq!(marked(&dom), 1);
    assert!(dom
        .world()
        .get::<&AssociatedInertTemplateDocument>(i)
        .is_err());
}

/// U4's ECS half: the HTML §13.4 build both parser tiers run (the root
/// redirects to the context's document), with the throwaway read as a
/// component before its destruction. The parser half (100 parses, both
/// tiers) is `elidex-html-parser`'s.
#[test]
fn u4_fragment_build_never_links_the_throwaway() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let context = el_in(&mut dom, "div", d);
    assert!(dom.append_child(d, context));
    let (throwaway, root, saved) = dom.begin_detached_fragment(context);
    let tpl = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(root),
    );
    assert!(dom.append_child(root, tpl));
    let contents = dom.attach_template_contents(tpl, dom.node_document_for_new_nodes(tpl));
    assert!(dom
        .world()
        .get::<&AssociatedInertTemplateDocument>(throwaway)
        .is_err());
    let i = dom.appropriate_template_contents_owner_document(d);
    assert_eq!(dom.node_document(contents), Some(i));
    let _ = dom.finish_detached_fragment(root, throwaway);
    assert!(saved.is_none());
    assert_eq!(marked(&dom), 1);
}

// ---- U6: I is not a render / layout / hit-test root ----
#[test]
fn u6_root_entities_excludes_the_inert_document() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let html = el_in(&mut dom, "html", d);
    assert!(dom.append_child(d, html));
    let i = dom.appropriate_template_contents_owner_document(d);
    let child = el_in(&mut dom, "div", i);
    assert!(dom.append_child(i, child));
    let roots = dom.root_entities();
    assert!(!roots.contains(&i));
    assert!(roots.contains(&d), "control: the document is a root");
}

// ---- U7: destroying D destroys its I ----
#[test]
fn u7_destroying_the_document_destroys_its_inert_document() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let (_tpl, contents) = template_in(&mut dom, d);
    let p = el_in(&mut dom, "p", d);
    assert!(dom.append_child(contents, p));
    let i = dom.appropriate_template_contents_owner_document(d);
    assert_eq!(dom.node_document(p), Some(i));
    assert!(dom.destroy_entity(d));
    assert!(!dom.contains(i));
    assert_eq!(dom.node_document(p), None);
    assert_eq!(marked(&dom), 0);
}

// ---- U8: neither component is copied by a clone ----
#[test]
fn u8_clone_copies_neither_link_nor_marker() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let i = dom.appropriate_template_contents_owner_document(d);
    let mut pairs = Vec::new();
    let dc = dom
        .clone_subtree(d, &mut pairs, dom.node_document_for_new_nodes(d))
        .unwrap();
    assert!(dom
        .world()
        .get::<&AssociatedInertTemplateDocument>(dc)
        .is_err());
    let ic = dom
        .clone_subtree(i, &mut pairs, dom.node_document_for_new_nodes(i))
        .unwrap();
    assert!(!dom.is_inert_template_document(ic));
    assert_eq!(marked(&dom), 1);
}

// ---- U9: adopt maps contents (nested, with shadow trees) to f(X) ----
#[test]
fn u9_adopt_maps_contents_to_the_targets_inert_document() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let x = dom.create_document_node();
    // Light tree: tpl > host_l (shadow root sr_l > s_l).
    let (tpl, f) = template_in(&mut dom, d);
    let host_l = el_in(&mut dom, "div", d);
    assert!(dom.append_child(tpl, host_l));
    let sr_l = dom.attach_shadow(host_l, ShadowRootMode::Open).unwrap();
    let s_l = el_in(&mut dom, "span", d);
    assert!(dom.append_child(sr_l, s_l));
    // Contents: nested template (contents nf > p) and a shadow host h
    // (sr > span).
    let (ntpl, nf) = template_in(&mut dom, d);
    assert!(dom.append_child(f, ntpl));
    let p = el_in(&mut dom, "p", d);
    assert!(dom.append_child(nf, p));
    let h = el_in(&mut dom, "div", d);
    assert!(dom.append_child(f, h));
    let sr = dom.attach_shadow(h, ShadowRootMode::Open).unwrap();
    let span = el_in(&mut dom, "span", d);
    assert!(dom.append_child(sr, span));

    dom.adopt(tpl, x);
    let ix = dom.appropriate_template_contents_owner_document(x);
    for n in [tpl, host_l, sr_l, s_l] {
        assert_eq!(dom.node_document(n), Some(x), "light {n:?}");
    }
    for n in [f, ntpl, nf, p, h, sr, span] {
        assert_eq!(dom.node_document(n), Some(ix), "contents {n:?}");
    }
}

// ---- U10 (guard): the walk past the depth and breadth caps ----
#[test]
fn u10_adopt_walk_rehomes_past_the_caps() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let x = dom.create_document_node();
    // A chain deeper than the cap, built bottom-up, with a template at the
    // bottom whose contents hold another over-deep chain and a node wider
    // than the cap.
    let (tpl, f) = template_in(&mut dom, d);
    let mut contents_nodes = vec![f];
    let deep_leaf = el_in(&mut dom, "i", d);
    contents_nodes.push(deep_leaf);
    let mut top = deep_leaf;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let parent = el_in(&mut dom, "div", d);
        assert!(dom.append_child(parent, top));
        contents_nodes.push(parent);
        top = parent;
    }
    assert!(dom.append_child(f, top));
    let wide = el_in(&mut dom, "ul", d);
    assert!(dom.append_child(f, wide));
    contents_nodes.push(wide);
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let li = el_in(&mut dom, "li", d);
        assert!(dom.append_child(wide, li));
        contents_nodes.push(li);
    }
    let mut light_nodes = vec![tpl];
    let mut top = tpl;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let parent = el_in(&mut dom, "div", d);
        assert!(dom.append_child(parent, top));
        light_nodes.push(parent);
        top = parent;
    }

    dom.adopt(top, x);
    let ix = dom.appropriate_template_contents_owner_document(x);
    assert!(light_nodes.iter().all(|&n| dom.node_document(n) == Some(x)));
    assert!(contents_nodes
        .iter()
        .all(|&n| dom.node_document(n) == Some(ix)));
}

// ---- U11: the new-node entry inside contents answers f(D) ----
#[test]
fn u11_entry_inside_contents_answers_the_inert_document() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let (_tpl, f) = template_in(&mut dom, d);
    let i = dom.appropriate_template_contents_owner_document(d);
    let div = el_in(&mut dom, "div", d);
    assert!(dom.append_child(f, div));
    let (ntpl, nf) = template_in(&mut dom, d);
    assert!(dom.append_child(f, ntpl));
    let sr = dom.attach_shadow(div, ShadowRootMode::Open).unwrap();
    for source in [f, div, ntpl, nf, sr] {
        assert_eq!(
            dom.node_document_for_new_nodes(source).document(),
            Some(i),
            "{source:?}"
        );
    }
    // A template built under the HTML §13.4 fragment-parse root with its
    // context in D: its contents are stamped f(D), never f(throwaway).
    let context = el_in(&mut dom, "div", d);
    assert!(dom.append_child(d, context));
    let (throwaway, root, _saved) = dom.begin_detached_fragment(context);
    let btpl = dom.create_element_with_owner(
        "template",
        Attributes::default(),
        dom.node_document_for_new_nodes(root),
    );
    assert!(dom.append_child(root, btpl));
    let bf = dom.attach_template_contents(btpl, dom.node_document_for_new_nodes(btpl));
    let _ = dom.finish_detached_fragment(root, throwaway);
    assert_eq!(dom.node_document(bf), Some(i));
    assert_eq!(marked(&dom), 1);
}

// ---- U13: f(target) is created by the adopt iff it reaches contents ----
#[test]
fn u13_adopt_creates_the_targets_inert_document_lazily() {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let x = dom.create_document_node();
    let y = dom.create_document_node();
    let (tpl, f) = template_in(&mut dom, d);
    let c = el_in(&mut dom, "i", d);
    assert!(dom.append_child(f, c));
    let before = marked(&dom);
    dom.adopt(tpl, x);
    let ix = dom
        .world()
        .get::<&AssociatedInertTemplateDocument>(x)
        .map(|l| l.0)
        .expect("the adopt linked X to an inert document");
    assert!(dom.is_inert_template_document(ix));
    assert_eq!(marked(&dom), before + 1);
    assert_eq!([f, c].map(|n| dom.node_document(n)), [Some(ix); 2]);
    // No template: the target stays unlinked.
    let div = el_in(&mut dom, "div", d);
    let span = el_in(&mut dom, "span", d);
    assert!(dom.append_child(div, span));
    dom.adopt(div, y);
    assert!(dom
        .world()
        .get::<&AssociatedInertTemplateDocument>(y)
        .is_err());
    assert_eq!(marked(&dom), before + 1);
}
