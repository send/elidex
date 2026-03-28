use crate::child_node::{ensure_pre_insertion_validity, ensure_replace_validity};
use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::DomApiErrorKind;

// ---- ensure_pre_insertion_validity ----

#[test]
fn ensure_validity_invalid_parent() {
    let mut dom = EcsDom::new();
    let text = dom.create_text("hello");
    let child = dom.create_element("span", Attributes::default());

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
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
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
    let parent = dom.create_element("div", Attributes::default());
    let node = dom.create_element("span", Attributes::default());
    let unrelated = dom.create_element("p", Attributes::default());

    let result = ensure_pre_insertion_validity(parent, node, Some(unrelated), &dom);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
}

#[test]
fn ensure_validity_valid_insertion() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let node = dom.create_element("span", Attributes::default());
    let existing = dom.create_element("p", Attributes::default());
    dom.append_child(parent, existing);

    let result = ensure_pre_insertion_validity(parent, node, Some(existing), &dom);
    assert!(result.is_ok());
}

#[test]
fn ensure_validity_document_rejects_text() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let text = dom.create_text("hello");

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
    let parent = dom.create_element("div", Attributes::default());
    let doctype = dom.create_document_type("html", "", "");
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
    let doctype = dom.create_document_type("html", "", "");
    let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
    assert!(result.is_ok());
}

// ---- ensure_pre_insertion_validity: step 6 Document constraints ----

#[test]
fn ensure_validity_document_rejects_second_element() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let second = dom.create_element("body", Attributes::default());
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
    let html = dom.create_element("html", Attributes::default());
    let result = ensure_pre_insertion_validity(doc, html, None, &dom);
    assert!(result.is_ok());
}

#[test]
fn ensure_validity_document_rejects_element_before_doctype() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type("html", "", "");
    dom.append_child(doc, doctype);
    let html = dom.create_element("html", Attributes::default());
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
    let dt1 = dom.create_document_type("html", "", "");
    dom.append_child(doc, dt1);
    let dt2 = dom.create_document_type("html", "", "");
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
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let doctype = dom.create_document_type("html", "", "");
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
    let frag = dom.create_document_fragment();
    let text = dom.create_text("hello");
    dom.append_child(frag, text);
    let result = ensure_pre_insertion_validity(doc, frag, None, &dom);
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
    let parent = dom.create_element("div", Attributes::default());
    let old = dom.create_element("span", Attributes::default());
    let new_el = dom.create_element("p", Attributes::default());
    dom.append_child(parent, old);
    let result = ensure_replace_validity(parent, new_el, old, &dom);
    assert!(result.is_ok());
}

#[test]
fn replace_validity_cycle_rejected() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
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
    let parent = dom.create_element("div", Attributes::default());
    let node = dom.create_element("span", Attributes::default());
    let unrelated = dom.create_element("p", Attributes::default());
    let result = ensure_replace_validity(parent, node, unrelated, &dom);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
}
