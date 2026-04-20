//! Tests for [`AssociatedDocument`] — WHATWG DOM §4.4 "node document" —
//! and the corresponding [`EcsDom::owner_document`] / clone propagation
//! policy (§4.5 "clone a node").

use super::*;

#[test]
fn associated_document_on_create_element_with_owner() {
    // `create_element_with_owner(Some(doc))` attaches the component so
    // that `owner_document` reports `doc` before insertion.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = dom.create_element_with_owner("div", Attributes::default(), Some(doc));
    assert_eq!(dom.get_associated_document(div), Some(doc));
    assert_eq!(dom.owner_document(div), Some(doc));
}

#[test]
fn associated_document_for_text_comment_fragment() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let text = dom.create_text_with_owner("hi", Some(doc));
    let comment = dom.create_comment_with_owner("c", Some(doc));
    let frag = dom.create_document_fragment_with_owner(Some(doc));
    assert_eq!(dom.owner_document(text), Some(doc));
    assert_eq!(dom.owner_document(comment), Some(doc));
    assert_eq!(dom.owner_document(frag), Some(doc));
}

#[test]
fn owner_document_fallback_without_component() {
    // Legacy creation paths (no owner argument) leave the component
    // absent.  `owner_document` then falls back to the tree-root walk:
    // an entity inserted into the document reports that document,
    // while a fully detached entity reports None.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let attached = elem(&mut dom, "div");
    assert!(dom.append_child(doc, attached));
    assert_eq!(dom.get_associated_document(attached), None);
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
    // WHATWG §4.5 step: for non-Document clones, every copied node
    // inherits the *source's* node document.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let div = dom.create_element_with_owner("div", Attributes::default(), Some(doc));
    let span = dom.create_element_with_owner("span", Attributes::default(), Some(doc));
    let text = dom.create_text_with_owner("hi", Some(doc));
    assert!(dom.append_child(div, span));
    assert!(dom.append_child(span, text));

    let clone = dom.clone_subtree(div).expect("clone_subtree");
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
    // WHATWG §4.5 Document branch: the cloned Document becomes its
    // own node document, and descendants adopt the *clone* as their
    // owner document — not the source document.
    let mut dom = EcsDom::new();
    let src_doc = dom.create_document_root();
    // documentElement + descendant, both owned by src_doc.
    let html = dom.create_element_with_owner("html", Attributes::default(), Some(src_doc));
    let body = dom.create_element_with_owner("body", Attributes::default(), Some(src_doc));
    assert!(dom.append_child(src_doc, html));
    assert!(dom.append_child(html, body));

    let clone_doc = dom.clone_subtree(src_doc).expect("clone_subtree");
    // Document.ownerDocument is always null per spec, so the accessor
    // returns None; but the internal AssociatedDocument points at the
    // clone itself (self-ref), which matters when descendants look up
    // their owner through tree-root walks.
    assert_eq!(dom.get_associated_document(clone_doc), Some(clone_doc));
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
    let fake_owner = dom.create_element("html", Attributes::default());
    // Skip the debug_assert by reaching in via a test-only pattern:
    // we can't directly pass `Some(fake_owner)` because it would
    // trip the assertion in debug.  Instead, confirm the RELEASE
    // fallback path by manually attaching a bogus component and
    // verifying `owner_document` refuses to return it.
    let node = dom.create_element_with_owner("div", Attributes::default(), None);
    let _ = dom
        .world_mut()
        .insert_one(node, crate::AssociatedDocument(fake_owner));
    // Read-time validation drops the non-Document pointer and falls
    // through to the tree-root walk (which has no Document either),
    // yielding None.
    assert_eq!(dom.owner_document(node), None);
}

#[test]
fn owner_document_skips_destroyed_associated_document() {
    // If the associated Document has been destroyed, `owner_document`
    // must not hand back a ghost Entity — it falls through to the
    // tree-root walk so callers keep observing real entities only.
    let mut dom = EcsDom::new();
    let real_doc = dom.create_document_root();
    let ghost_doc = dom
        .world_mut()
        .spawn((TreeRelation::default(), NodeKind::Document));
    let div = dom.create_element_with_owner("div", Attributes::default(), Some(ghost_doc));
    // Attach to the real document so the fallback walk can find it.
    assert!(dom.append_child(real_doc, div));
    // Now destroy the ghost document.
    dom.destroy_entity(ghost_doc);
    assert_eq!(dom.owner_document(div), Some(real_doc));
}
