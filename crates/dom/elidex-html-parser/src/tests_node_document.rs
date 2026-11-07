//! C0b U8 / U9 (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`
//! §6.2): every node a document parse or a fragment parse creates is created
//! in its node document (HTML §13.2.6.1 *create an element for the token*
//! step 3; DOM §4.4 *node document*), so no link walks: the walk counter
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
/// trees), the Document excluded.
fn all_nodes(dom: &EcsDom, document: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![document];
    while let Some(n) = stack.pop() {
        if n != document {
            out.push(n);
        }
        stack.extend(dom.children(n));
        if let Some(contents) = dom.template_contents_fragment(n) {
            stack.push(contents);
        }
        if let Some(sr) = dom.get_shadow_root(n) {
            stack.push(sr);
        }
    }
    out
}

fn tag_of(dom: &EcsDom, n: Entity) -> Option<String> {
    dom.world().get::<&TagType>(n).ok().map(|t| t.0.clone())
}

/// The nodes of `parsed` not stamped with its document: (count, tags).
fn unstamped(dom: &EcsDom, document: Entity) -> (usize, Vec<Option<String>>) {
    let off: Vec<Entity> = all_nodes(dom, document)
        .into_iter()
        .filter(|&n| dom.node_document(n) != Some(document))
        .collect();
    (off.len(), off.iter().map(|&n| tag_of(dom, n)).collect())
}

// The parse builds into a fresh `EcsDom` (counter 0 before the parse).
#[test]
fn u8_strict_document_parse_creates_every_node_in_the_document() {
    let parsed = crate::parse_strict(MARKUP).expect("conforming markup");
    assert_eq!(parsed.dom.adopt_walk_count(), 0, "no link walked");
    assert_eq!(unstamped(&parsed.dom, parsed.document), (0, vec![]));
}

#[test]
fn u8_tolerant_document_parse_creates_every_node_in_the_document() {
    let parsed = crate::parse_html(MARKUP);
    assert_eq!(parsed.dom.adopt_walk_count(), 0, "no link walked");
    assert_eq!(unstamped(&parsed.dom, parsed.document), (0, vec![]));
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
fn u8_control_tolerant_template_contents_read_the_document() {
    let parsed = crate::parse_html(MARKUP);
    let dom = &parsed.dom;
    let template = all_nodes(dom, parsed.document)
        .into_iter()
        .find(|&n| {
            tag_of(dom, n).as_deref() == Some("template")
                && dom.template_contents_fragment(n).is_some()
        })
        .expect("template parsed");
    let contents = dom.template_contents_fragment(template).unwrap();
    assert_eq!(dom.node_document(contents), Some(parsed.document));
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
    let off: Vec<Option<String>> = nodes
        .iter()
        .filter(|&&n| dom.node_document(n) != Some(d))
        .map(|&n| tag_of(&dom, n))
        .collect();
    assert_eq!(
        off,
        Vec::<Option<String>>::new(),
        "strict={strict}: all in D"
    );
    assert_eq!(
        dom.world().len() as usize,
        entities as usize + nodes.len(),
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
