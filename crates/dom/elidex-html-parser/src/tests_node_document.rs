//! C0b U8 (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.2):
//! every node a document parse creates is created in the parse document (HTML
//! §13.2.6.1 *create an element for the token* step 3; DOM §4.4 *node
//! document*), so no link walks (the walk counter of C0b's head).
//!
//! Red-first base formulation (§6.5): the counter does not exist on the base;
//! the HEAD proxy is that non-foreign parser nodes read `None` (H-4).
//! `get_associated_document` stands for `node_document`.

use elidex_ecs::{EcsDom, Entity, TagType};

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
        .filter(|&n| dom.get_associated_document(n) != Some(document))
        .collect();
    (off.len(), off.iter().map(|&n| tag_of(dom, n)).collect())
}

#[test]
fn u8_strict_document_parse_creates_every_node_in_the_document() {
    let parsed = crate::parse_strict(MARKUP).expect("conforming markup");
    assert_eq!(unstamped(&parsed.dom, parsed.document), (0, vec![]));
}

#[test]
fn u8_tolerant_document_parse_creates_every_node_in_the_document() {
    let parsed = crate::parse_html(MARKUP);
    assert_eq!(unstamped(&parsed.dom, parsed.document), (0, vec![]));
}

#[test]
fn u8_control_strict_foreign_elements_read_the_document() {
    let parsed = crate::parse_strict(MARKUP).expect("conforming markup");
    let svg = all_nodes(&parsed.dom, parsed.document)
        .into_iter()
        .find(|&n| tag_of(&parsed.dom, n).as_deref() == Some("svg"))
        .expect("svg parsed");
    assert_eq!(
        parsed.dom.get_associated_document(svg),
        Some(parsed.document)
    );
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
    assert_eq!(dom.get_associated_document(contents), Some(parsed.document));
}
