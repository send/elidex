use super::*;

// -----------------------------------------------------------------------
// IsConnected
// -----------------------------------------------------------------------

#[test]
fn is_connected_true() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(doc, div);

    let r = IsConnected
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(true));
}

#[test]
fn is_connected_false() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    let r = IsConnected
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

#[test]
fn is_connected_detached() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(doc, div);
    dom.remove_child(doc, div);

    let r = IsConnected
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

// -----------------------------------------------------------------------
// GetRootNode
// -----------------------------------------------------------------------

#[test]
fn get_root_node_connected() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(doc, div);
    wrap(doc, &mut session);

    let r = GetRootNode
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (root, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(root, doc);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn get_root_node_detached() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(div, child);

    let r = GetRootNode
        .invoke(child, &[], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (root, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(root, div);
    } else {
        panic!("expected ObjectRef");
    }
}

// -----------------------------------------------------------------------
// GetRootNode — composed support (M5)
// -----------------------------------------------------------------------

#[test]
fn get_root_node_non_composed_default() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(doc, div);
    wrap(div, &mut session);

    let result = GetRootNode
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn get_root_node_composed_true() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(doc, host);
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    let result = GetRootNode
        .invoke(inner, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    // Composed root should cross shadow boundary and reach the document.
    if let JsValue::ObjectRef(ref_id) = result {
        let (root, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(root, doc);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn get_root_node_non_composed_stops_at_shadow() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(doc, host);
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    let result = GetRootNode
        .invoke(inner, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    // Non-composed root should stop at the shadow root.
    if let JsValue::ObjectRef(ref_id) = result {
        let (root, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(root, sr);
    } else {
        panic!("expected ObjectRef");
    }
}

// -----------------------------------------------------------------------
// OwnerDocument
// -----------------------------------------------------------------------

#[test]
fn owner_document_element() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(doc, div);
    wrap(doc, &mut session);

    let r = OwnerDocument
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (owner, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_eq!(owner, doc);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn owner_document_doc_null() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();

    let r = OwnerDocument
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Null);
}

// -----------------------------------------------------------------------
// OwnerDocument — disconnected nodes (H5)
// -----------------------------------------------------------------------

#[test]
fn owner_document_orphan() {
    let (mut dom, mut session) = setup();
    let _doc = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    // div is orphaned (not appended to doc).
    wrap(div, &mut session);

    let result = OwnerDocument
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    // Should still return the document, not null.
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn owner_document_null_for_document() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    wrap(doc, &mut session);

    let result = OwnerDocument
        .invoke(doc, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

// -----------------------------------------------------------------------
// IsSameNode
// -----------------------------------------------------------------------

#[test]
fn is_same_node_true() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    wrap(div, &mut session);

    let r = IsSameNode
        .invoke(
            div,
            &[obj_ref_arg(div, &mut session)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(r, JsValue::Bool(true));
}

#[test]
fn is_same_node_false() {
    let (mut dom, mut session) = setup();
    let a = dom.create_element("div", Attributes::default());
    let b = dom.create_element("div", Attributes::default());
    wrap(a, &mut session);
    wrap(b, &mut session);

    let r = IsSameNode
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

#[test]
fn is_same_node_null() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    let r = IsSameNode
        .invoke(div, &[JsValue::Null], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}

// -----------------------------------------------------------------------
// IsEqualNode
// -----------------------------------------------------------------------

#[test]
fn is_equal_node_true() {
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("id", "x");
    let a = dom.create_element("div", attrs.clone());
    let t1 = dom.create_text("hello");
    dom.append_child(a, t1);

    let b = dom.create_element("div", attrs);
    let t2 = dom.create_text("hello");
    dom.append_child(b, t2);

    wrap(a, &mut session);
    wrap(b, &mut session);

    let r = IsEqualNode
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(true));
}

#[test]
fn is_equal_node_false() {
    let (mut dom, mut session) = setup();
    let a = dom.create_element("div", Attributes::default());
    let b = dom.create_element("span", Attributes::default());
    wrap(a, &mut session);
    wrap(b, &mut session);

    let r = IsEqualNode
        .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
        .unwrap();
    assert_eq!(r, JsValue::Bool(false));
}
