use super::*;

#[test]
fn get_doctype() {
    let (mut dom, doc, mut session) = setup_document();
    let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn get_doctype_name() {
    let (mut dom, doc, mut session) = setup_document();
    let dt_entity = find_doctype(&dom, doc).unwrap();
    let result = GetDoctypeName
        .invoke(dt_entity, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("html".into()));
}

#[test]
fn get_doctype_public_id() {
    let (mut dom, doc, mut session) = setup_document();
    let dt_entity = find_doctype(&dom, doc).unwrap();
    let result = GetDoctypePublicId
        .invoke(dt_entity, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("-//W3C//DTD HTML 4.01//EN".into()));
}

#[test]
fn get_doctype_system_id() {
    let (mut dom, doc, mut session) = setup_document();
    let dt_entity = find_doctype(&dom, doc).unwrap();
    let result = GetDoctypeSystemId
        .invoke(dt_entity, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(
        result,
        JsValue::String("http://www.w3.org/TR/html4/strict.dtd".into())
    );
}

#[test]
fn get_doctype_none() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(doc, html);
    let mut session = SessionCore::new();
    let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn get_doctype_legacy_doc_type_data_without_node_kind() {
    // Legacy html5ever-era fixtures attach `DocTypeData` payload but
    // not the explicit `NodeKind` component.  `find_doctype` must
    // route through `EcsDom::node_kind_inferred` so the entity still
    // surfaces — same fallback policy as `HostData::prototype_kind_for`
    // / `require_node_arg`.  Pinned at the dom-api layer so a future
    // refactor that reverts to a strict `NodeKind` component check
    // surfaces here rather than only via VM-level integration tests.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let dt = dom.create_document_type("html", "", "");
    dom.append_child(doc, dt);
    let _ = dom.world_mut().remove_one::<NodeKind>(dt);
    assert!(dom.node_kind(dt).is_none());
    let mut session = SessionCore::new();
    let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}
