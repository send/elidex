//! Tests for [`AssociatedDocument`] — WHATWG DOM §4.4 "node document" —
//! and the corresponding [`EcsDom::owner_document`] / clone propagation
//! policy (§4.4 "clone a node").

use super::*;

#[test]
fn associated_document_on_create_element_with_owner() {
    // `create_element_with_owner(Some(doc))` attaches the component so
    // that `owner_document` reports `doc` before insertion.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    );
    assert_eq!(dom.get_associated_document(div), Some(doc));
    assert_eq!(dom.owner_document(div), Some(doc));
}

#[test]
fn associated_document_for_text_comment_fragment() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let text = dom.create_text_with_owner("hi", dom.node_document_for_new_nodes(doc));
    let comment = dom.create_comment_with_owner("c", dom.node_document_for_new_nodes(doc));
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(doc));
    assert_eq!(dom.owner_document(text), Some(doc));
    assert_eq!(dom.owner_document(comment), Some(doc));
    assert_eq!(dom.owner_document(frag), Some(doc));
}

#[test]
fn fragment_child_is_adopted_into_the_fragments_node_document() {
    // A node appended under a `DocumentFragment` is adopted into the
    // fragment's node document (DOM §4.2.3 insert step 7.1), its subtree
    // included; nothing is derived from the tree root afterwards.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let frag = dom.create_document_fragment_with_owner(dom.node_document_for_new_nodes(doc));
    // Ownerless fixture nodes (production cannot create them).
    let child =
        dom.create_element_ownerless_fixture("span", crate::components::Attributes::default());
    let grandchild = dom.create_text_ownerless_fixture("x");
    assert!(dom.append_child(child, grandchild));
    assert!(dom.append_child(frag, child));
    assert_eq!(dom.owner_document(child), Some(doc), "adopted on insert");
    assert_eq!(
        dom.owner_document(grandchild),
        Some(doc),
        "the adopt walk reaches the descendants"
    );
    // A fragment with NO node document leaves its descendants without one.
    let orphan_frag = dom.create_document_fragment_ownerless_fixture();
    let orphan_child =
        dom.create_element_ownerless_fixture("p", crate::components::Attributes::default());
    assert!(dom.append_child(orphan_frag, orphan_child));
    assert_eq!(dom.owner_document(orphan_child), None);
}

#[test]
fn ownerless_node_is_adopted_on_insert_and_a_detached_one_has_none() {
    // A fixture node created without a node document is adopted when it is
    // inserted into a document's tree; a detached one has none (no
    // position-based derivation).
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let attached = elem(&mut dom, "div");
    assert_eq!(dom.node_document(attached), None);
    assert!(dom.append_child(doc, attached));
    assert_eq!(dom.node_document(attached), Some(doc));
    assert_eq!(dom.owner_document(attached), Some(doc));

    let orphan = elem(&mut dom, "span");
    assert_eq!(dom.owner_document(orphan), None);
}

#[test]
fn owner_document_document_itself_is_none() {
    // WHATWG: `Document.ownerDocument` is `null`.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    assert_eq!(dom.owner_document(doc), None);
}

#[test]
fn clone_subtree_non_document_propagates_src_owner() {
    // WHATWG DOM §4.4 "clone a node" step: for non-Document clones, every copied node
    // inherits the *source's* node document.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    );
    let span = dom.create_element_with_owner(
        "span",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    );
    let text = dom.create_text_with_owner("hi", dom.node_document_for_new_nodes(doc));
    assert!(dom.append_child(div, span));
    assert!(dom.append_child(span, text));

    let clone = dom
        .clone_subtree(div, &mut Vec::new(), dom.node_document_for_new_nodes(div))
        .expect("clone_subtree");
    assert_eq!(dom.owner_document(clone), Some(doc));
    let cloned_span = dom.children(clone)[0];
    assert_eq!(dom.owner_document(cloned_span), Some(doc));
    let cloned_text = dom.children(cloned_span)[0];
    assert_eq!(dom.owner_document(cloned_text), Some(doc));
    // The clones are distinct entities from their sources.
    assert_ne!(clone, div);
    assert_ne!(cloned_span, span);
    assert_ne!(cloned_text, text);
}

#[test]
fn clone_subtree_document_self_reference_and_descendants() {
    // WHATWG DOM §4.4 "clone a node" Document branch: the cloned Document becomes its
    // own node document, and descendants adopt the *clone* as their
    // owner document — not the source document.
    let mut dom = EcsDom::new();
    let src_doc = dom.create_document_root();
    // documentElement + descendant, both owned by src_doc.
    let html = dom.create_element_with_owner(
        "html",
        Attributes::default(),
        dom.node_document_for_new_nodes(src_doc),
    );
    let body = dom.create_element_with_owner(
        "body",
        Attributes::default(),
        dom.node_document_for_new_nodes(src_doc),
    );
    assert!(dom.append_child(src_doc, html));
    assert!(dom.append_child(html, body));

    let clone_doc = dom
        .clone_subtree(
            src_doc,
            &mut Vec::new(),
            dom.node_document_for_new_nodes(src_doc),
        )
        .expect("clone_subtree");
    // Document.ownerDocument is always null per spec, so the accessor
    // returns None; the clone is its own node document.
    assert_eq!(dom.node_document(clone_doc), Some(clone_doc));
    assert_eq!(dom.owner_document(clone_doc), None);

    let kids = dom.children(clone_doc);
    assert_eq!(kids.len(), 1);
    let cloned_html = kids[0];
    // Descendants point at the *clone*, not at the source document.
    assert_eq!(dom.owner_document(cloned_html), Some(clone_doc));
    let cloned_body = dom.children(cloned_html)[0];
    assert_eq!(dom.owner_document(cloned_body), Some(clone_doc));
}

#[test]
fn create_with_owner_skips_component_when_owner_is_not_a_document() {
    // Copilot R3 F7 lock-in: `create_*_with_owner` must not seed an
    // `AssociatedDocument` pointing at a non-Document entity — that
    // would leave invalid data the `owner_document` read path
    // silently drops.  Release builds silently skip the component;
    // debug builds would fire `debug_assert!` (not exercised here so
    // the test runs in both profiles).
    let mut dom = EcsDom::new();
    let fake_owner = dom.create_element_ownerless_fixture("html", Attributes::default());
    // Skip the debug_assert by reaching in via a test-only pattern:
    // we can't directly pass `Some(fake_owner)` because it would
    // trip the assertion in debug.  Instead, confirm the RELEASE
    // fallback path by manually attaching a bogus component and
    // verifying `owner_document` refuses to return it.
    let node = dom.create_element_with_owner("div", Attributes::default(), NewNodeOwner::NONE);
    let _ = dom
        .world_mut()
        .insert_one(node, crate::components::AssociatedDocument(fake_owner));
    // Read-time validation drops the non-Document pointer, yielding None.
    assert_eq!(dom.owner_document(node), None);
}

#[test]
fn owner_document_of_a_destroyed_document_is_none() {
    // If the node document has been destroyed, `node_document` must not hand
    // back a ghost Entity, and nothing falls back to the tree root: a node in
    // a live document's tree whose stored document is gone reads `None`.
    let mut dom = EcsDom::new();
    let real_doc = dom.create_document_root();
    let ghost_doc = dom
        .world_mut()
        .spawn((TreeRelation::default(), NodeKind::Document));
    let div = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(real_doc),
    );
    assert!(dom.append_child(real_doc, div));
    // The stored document points at the ghost (a state only a later
    // destruction produces), then the ghost is destroyed.
    assert!(dom.set_associated_document(div, ghost_doc));
    dom.destroy_entity(ghost_doc);
    assert_eq!(dom.node_document(div), None);
    assert_eq!(dom.owner_document(div), None);
}
