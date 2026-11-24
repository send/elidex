use crate::child_node::{ensure_pre_insertion_validity, ensure_replace_validity};
use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::DomApiErrorKind;

// ---- ensure_pre_insertion_validity ----

#[test]
fn ensure_validity_invalid_parent() {
    let mut dom = EcsDom::new();
    let text = dom.create_text_ownerless_fixture("hello");
    let child = dom.create_element_ownerless_fixture("span", Attributes::default());

    let result = ensure_pre_insertion_validity(text, child, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_ancestor_cycle() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let child = dom.create_element_ownerless_fixture("span", Attributes::default());
    dom.append_child(parent, child);

    // Trying to insert parent under child should be rejected.
    let result = ensure_pre_insertion_validity(child, parent, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_ref_child_not_child_of_parent() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let node = dom.create_element_ownerless_fixture("span", Attributes::default());
    let unrelated = dom.create_element_ownerless_fixture("p", Attributes::default());

    let result = ensure_pre_insertion_validity(parent, node, Some(unrelated), &dom);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn ensure_validity_valid_insertion() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let node = dom.create_element_ownerless_fixture("span", Attributes::default());
    let existing = dom.create_element_ownerless_fixture("p", Attributes::default());
    dom.append_child(parent, existing);

    let result = ensure_pre_insertion_validity(parent, node, Some(existing), &dom);
    assert!(result.is_ok());
}

#[test]
fn ensure_validity_document_rejects_text() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let text = dom.create_text_ownerless_fixture("hello");

    let result = ensure_pre_insertion_validity(doc, text, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

// ---- ensure_pre_insertion_validity: step 5 DocumentType ----

#[test]
fn ensure_validity_doctype_under_element_rejected() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let result = ensure_pre_insertion_validity(parent, doctype, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_doctype_under_document_ok() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
    assert!(result.is_ok());
}

// ---- ensure_pre_insertion_validity: Document constraints (DOM §4.2.3 steps 6–11) ----

#[test]
fn ensure_validity_document_rejects_second_element() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    dom.append_child(doc, html);
    let second = dom.create_element_ownerless_fixture("body", Attributes::default());
    let result = ensure_pre_insertion_validity(doc, second, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_document_allows_first_element() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    let result = ensure_pre_insertion_validity(doc, html, None, &dom);
    assert!(result.is_ok());
}

#[test]
fn ensure_validity_document_rejects_element_before_doctype() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    dom.append_child(doc, doctype);
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    // Inserting element before doctype should fail.
    let result = ensure_pre_insertion_validity(doc, html, Some(doctype), &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_document_rejects_second_doctype() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let dt1 = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    dom.append_child(doc, dt1);
    let dt2 = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let result = ensure_pre_insertion_validity(doc, dt2, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_document_rejects_doctype_after_element() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    dom.append_child(doc, html);
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    // Appending doctype after element should fail.
    let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_document_fragment_rejects_text_child() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let frag = dom.create_document_fragment_ownerless_fixture();
    let text = dom.create_text_ownerless_fixture("hello");
    dom.append_child(frag, text);
    let result = ensure_pre_insertion_validity(doc, frag, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

// ---- Ensure pre-insert validity steps 9.1 / 11: NO self-exclusion in pre-insert (childrenToExclude = « ») (Codex R1) ----
// Re-appending the document element or doctype fails because pre-insert passes
// an empty exclusion list to the validity check.

#[test]
fn ensure_validity_document_rejects_existing_element_reappend() {
    // `document.appendChild(document.documentElement)`: step 9.1 rejects the existing root
    // because pre-insert passes an empty exclusion list.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    dom.append_child(doc, html);
    let result = ensure_pre_insertion_validity(doc, html, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn ensure_validity_document_rejects_existing_doctype_reappend() {
    // Doctype sibling of the above: re-appending the document's existing doctype throws
    // at step 11 because the doctype is outside the empty exclusion list.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    dom.append_child(doc, doctype);
    let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

// ---- ensure_replace_validity ----

#[test]
fn replace_validity_ok() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let old = dom.create_element_ownerless_fixture("span", Attributes::default());
    let new_el = dom.create_element_ownerless_fixture("p", Attributes::default());
    dom.append_child(parent, old);
    let result = ensure_replace_validity(parent, new_el, old, &dom);
    assert!(result.is_ok());
}

#[test]
fn replace_validity_cycle_rejected() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let child = dom.create_element_ownerless_fixture("span", Attributes::default());
    dom.append_child(parent, child);
    // Trying to replace child with parent (ancestor) should be rejected.
    let result = ensure_replace_validity(parent, parent, child, &dom);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::HierarchyRequestError
    );
}

#[test]
fn replace_validity_child_not_found() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let node = dom.create_element_ownerless_fixture("span", Attributes::default());
    let unrelated = dom.create_element_ownerless_fixture("p", Attributes::default());
    let result = ensure_replace_validity(parent, node, unrelated, &dom);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn replace_validity_document_element_swap_ok() {
    // Replace excludes the old documentElement. With no other element child,
    // the new element passes the existing-element check in step 9.1.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let old_html = dom.create_element_ownerless_fixture("html", Attributes::default());
    dom.append_child(doc, old_html);
    let new_html = dom.create_element_ownerless_fixture("html", Attributes::default());
    let result = ensure_replace_validity(doc, new_html, old_html, &dom);
    assert!(result.is_ok());
}

// ---- NodeKind-only contract ----

#[test]
fn ensure_validity_parent_with_tag_but_no_node_kind_rejected() {
    // `validate_parent_kind` reads `NodeKind` only: an entity with a
    // `TagType` but no `NodeKind` is not an Element.
    let mut dom = EcsDom::new();
    let parent = dom
        .world_mut()
        .spawn((elidex_ecs::TagType("div".to_string()),));
    let child = dom.create_element_ownerless_fixture("span", Attributes::default());
    let err = ensure_pre_insertion_validity(parent, child, None, &dom).unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

fn spawn_cdata(dom: &mut EcsDom) -> elidex_ecs::Entity {
    dom.world_mut().spawn((
        elidex_ecs::NodeKind::CdataSection,
        elidex_ecs::TextContent("x".to_string()),
    ))
}

#[test]
fn ensure_validity_document_rejects_cdata_as_text_node() {
    // DOM: a CDATASection is a Text node (only the *exclusive* Text node
    // excludes it), so it cannot be inserted under a Document.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let cdata = spawn_cdata(&mut dom);
    let err = ensure_pre_insertion_validity(doc, cdata, None, &dom).unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}

#[test]
fn ensure_validity_document_rejects_fragment_with_cdata_child() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let frag = dom.create_document_fragment_ownerless_fixture();
    let cdata = spawn_cdata(&mut dom);
    assert!(dom.append_child(frag, cdata));
    let err = ensure_pre_insertion_validity(doc, frag, None, &dom).unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::HierarchyRequestError);
}
