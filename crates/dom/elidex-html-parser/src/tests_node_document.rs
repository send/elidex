//! C0b U8 / U9 (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`
//! §6.2): every node a document parse or a fragment parse creates is created
//! in its node document (HTML §13.2.6.1 *create an element for the token*
//! step 6; DOM §4.4 *node document*), so no link walks: the walk counter
//! (`EcsDom::adopt_walk_count`, `fixtures`) is unchanged across the parse.
//! The end state alone cannot show it (an ownerless node appended under the
//! document is re-homed silently, U2).
//!
//! Red-first base formulation of U8 (§6.5): the counter does not exist on the
//! base; the HEAD proxy was that non-foreign parser nodes read `None` (H-4).

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

use crate::{parse_fragment_strict, parse_html_fragment, ParseFragmentOptions};

const MARKUP: &str = "<!DOCTYPE html><html><head></head><body>\
    <template><b>t</b></template>\
    <div><template shadowrootmode=\"open\"><i>s</i></template></div>\
    <svg><circle></circle></svg><!--c--></body></html>";

/// Every node under `document` (light tree, template contents and shadow
/// trees), the Document excluded, each with whether it lies in a template
/// contents (the contents fragment, its descendants and the shadow trees under
/// them): those read the contents' owner document `f(D)` (HTML §4.12.3
/// *appropriate template contents owner document*, C1 INV-C), the others `D`.
fn all_nodes_with_contents_flag(dom: &EcsDom, document: Entity) -> Vec<(Entity, bool)> {
    let mut out = Vec::new();
    let mut stack = vec![(document, false)];
    while let Some((n, in_contents)) = stack.pop() {
        if n != document {
            out.push((n, in_contents));
        }
        stack.extend(dom.children(n).into_iter().map(|c| (c, in_contents)));
        if let Some(contents) = dom.template_contents_fragment(n) {
            stack.push((contents, true));
        }
        if let Some(sr) = dom.get_shadow_root(n) {
            stack.push((sr, in_contents));
        }
    }
    out
}

fn all_nodes(dom: &EcsDom, document: Entity) -> Vec<Entity> {
    all_nodes_with_contents_flag(dom, document)
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

fn tag_of(dom: &EcsDom, n: Entity) -> Option<String> {
    dom.world().get::<&TagType>(n).ok().map(|t| t.0.clone())
}

/// The nodes of `parsed` not stamped with their expected document (`D`, or
/// `f(D)` for a node in template contents): (count, tags).
fn unstamped(dom: &mut EcsDom, document: Entity) -> (usize, Vec<Option<String>>) {
    let inert = dom.appropriate_template_contents_owner_document(document);
    let off: Vec<Entity> = all_nodes_with_contents_flag(dom, document)
        .into_iter()
        .filter(|&(n, in_contents)| {
            dom.node_document(n) != Some(if in_contents { inert } else { document })
        })
        .map(|(n, _)| n)
        .collect();
    (off.len(), off.iter().map(|&n| tag_of(dom, n)).collect())
}

// The parse builds into a fresh `EcsDom` (counter 0 before the parse).
#[test]
fn u8_strict_document_parse_creates_every_node_in_the_document() {
    let mut parsed = crate::parse_strict(MARKUP).expect("conforming markup");
    assert_eq!(parsed.dom.adopt_walk_count(), 0, "no link walked");
    assert_eq!(unstamped(&mut parsed.dom, parsed.document), (0, vec![]));
}

#[test]
fn u8_tolerant_document_parse_creates_every_node_in_the_document() {
    let mut parsed = crate::parse_html(MARKUP);
    assert_eq!(parsed.dom.adopt_walk_count(), 0, "no link walked");
    assert_eq!(unstamped(&mut parsed.dom, parsed.document), (0, vec![]));
}

#[test]
fn u8_control_strict_foreign_elements_read_the_document() {
    let parsed = crate::parse_strict(MARKUP).expect("conforming markup");
    let svg = all_nodes(&parsed.dom, parsed.document)
        .into_iter()
        .find(|&n| tag_of(&parsed.dom, n).as_deref() == Some("svg"))
        .expect("svg parsed");
    assert_eq!(parsed.dom.node_document(svg), Some(parsed.document));
}

#[test]
fn u8_control_tolerant_template_contents_read_the_inert_template_document() {
    let mut parsed = crate::parse_html(MARKUP);
    let dom = &mut parsed.dom;
    let template = all_nodes(dom, parsed.document)
        .into_iter()
        .find(|&n| {
            tag_of(dom, n).as_deref() == Some("template")
                && dom.template_contents_fragment(n).is_some()
        })
        .expect("template parsed");
    let contents = dom.template_contents_fragment(template).unwrap();
    let inert = dom.appropriate_template_contents_owner_document(parsed.document);
    assert_eq!(dom.node_document(contents), Some(inert));
    assert_ne!(inert, parsed.document);
}

// ---- U9 (guard, ×2): a fragment parse into a context in `D` walks nothing
// (D-8: no finishing walk; every build link is a compare), returns nodes
// stamped `D`, and leaves no parse root behind: `FragmentParseRoot` is
// crate-private, so its absence is read as "root and throwaway despawned" —
// the entity count grows by exactly the returned subtrees (success) or not at
// all (strict rollback). The direct component check is elidex-ecs's U9 leg.

const FRAGMENT: &str =
    "<p>a<b>b</b></p><template><i>c</i></template><svg><circle></circle></svg><!--d-->";

const FRAGMENT_OPTS: ParseFragmentOptions = ParseFragmentOptions {
    allow_declarative_shadow: false,
    scripting_disabled: false,
};

/// `D` with a `<div>` context in its tree; returns (dom, D, context).
fn fragment_setup() -> (EcsDom, Entity, Entity) {
    let mut dom = EcsDom::new();
    let d = dom.create_document_root();
    let context = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(d),
    );
    assert!(dom.append_child(d, context));
    (dom, d, context)
}

/// Every node of the returned subtrees (template contents included).
fn subtree_nodes(dom: &EcsDom, roots: &[Entity]) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack: Vec<Entity> = roots.to_vec();
    while let Some(n) = stack.pop() {
        out.push(n);
        stack.extend(dom.children(n));
        if let Some(contents) = dom.template_contents_fragment(n) {
            stack.push(contents);
        }
    }
    out
}

/// The nodes of the returned subtrees that lie in a template contents.
fn contents_nodes(dom: &EcsDom, roots: &[Entity]) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack: Vec<(Entity, bool)> = roots.iter().map(|&r| (r, false)).collect();
    while let Some((n, in_contents)) = stack.pop() {
        if in_contents {
            out.push(n);
        }
        stack.extend(dom.children(n).into_iter().map(|c| (c, in_contents)));
        if let Some(contents) = dom.template_contents_fragment(n) {
            stack.push((contents, true));
        }
    }
    out
}

fn u9_run(strict: bool) {
    let (mut dom, d, context) = fragment_setup();
    let walks = dom.adopt_walk_count();
    let entities = dom.world().len();
    let roots = if strict {
        parse_fragment_strict(FRAGMENT, context, &mut dom, FRAGMENT_OPTS).expect("conforming")
    } else {
        parse_html_fragment(FRAGMENT, context, &mut dom, FRAGMENT_OPTS)
    };
    assert_eq!(dom.adopt_walk_count(), walks, "strict={strict}: no walk");
    let nodes = subtree_nodes(&dom, &roots);
    assert!(nodes.len() >= 8, "strict={strict}: the fragment parsed");
    // A node in template contents reads `f(D)` (HTML §4.12.3, C1 INV-C).
    let inert = dom.appropriate_template_contents_owner_document(d);
    let in_contents = contents_nodes(&dom, &roots);
    let off: Vec<Option<String>> = nodes
        .iter()
        .filter(|&&n| {
            dom.node_document(n) != Some(if in_contents.contains(&n) { inert } else { d })
        })
        .map(|&n| tag_of(&dom, n))
        .collect();
    assert_eq!(
        off,
        Vec::<Option<String>>::new(),
        "strict={strict}: all in D"
    );
    assert_eq!(
        dom.world().len() as usize,
        // + 1: the fragment holds a `<template>`, so the parse creates D's
        // inert template document lazily (HTML §4.12.3, C1 D-C2).
        entities as usize + nodes.len() + 1,
        "strict={strict}: root and throwaway despawned"
    );
}

#[test]
fn u9_strict_fragment_parse_runs_no_walk() {
    u9_run(true);
}

#[test]
fn u9_tolerant_fragment_parse_runs_no_walk() {
    u9_run(false);
}

#[test]
fn u9_strict_rollback_leaves_the_dom_as_it_was() {
    let (mut dom, _d, context) = fragment_setup();
    let walks = dom.adopt_walk_count();
    let entities = dom.world().len();
    assert!(parse_fragment_strict("<p>a<b>b</i></p>", context, &mut dom, FRAGMENT_OPTS).is_err());
    assert_eq!(dom.adopt_walk_count(), walks, "no walk");
    assert_eq!(dom.world().len(), entities, "root and throwaway despawned");
}
