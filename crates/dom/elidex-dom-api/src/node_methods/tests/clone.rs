use super::*;

// -----------------------------------------------------------------------
// CloneNode
// -----------------------------------------------------------------------

#[test]
fn clone_node_shallow() {
    let (mut dom, mut session) = setup();
    let mut attrs = Attributes::default();
    attrs.set("class", "test");
    let div = dom.create_element("div", attrs);
    let child = dom.create_text("hello");
    dom.append_child(div, child);
    wrap(div, &mut session);

    let r = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        // Tag preserved.
        assert_eq!(dom.world().get::<&TagType>(cloned).unwrap().0, "div");
        // Attributes preserved.
        assert_eq!(
            dom.world().get::<&Attributes>(cloned).unwrap().get("class"),
            Some("test")
        );
        // No children (shallow).
        assert!(dom.children(cloned).is_empty());
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_deep() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let text = dom.create_text("hello");
    dom.append_child(div, text);
    wrap(div, &mut session);

    let r = CloneNode
        .invoke(div, &[JsValue::Bool(true)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        let children = dom.children(cloned);
        assert_eq!(children.len(), 1);
        let child_text = dom
            .world()
            .get::<&TextContent>(children[0])
            .unwrap()
            .0
            .clone();
        assert_eq!(child_text, "hello");
        // Cloned child is a different entity.
        assert_ne!(children[0], text);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_no_identity() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    wrap(div, &mut session);

    let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        // Cloned entity is different from original.
        assert_ne!(cloned, div);
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_no_inline_style() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(div, InlineStyle::default())
        .unwrap();
    wrap(div, &mut session);

    let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        // InlineStyle should NOT be copied.
        assert!(dom.world().get::<&InlineStyle>(cloned).is_err());
    } else {
        panic!("expected ObjectRef");
    }
}

#[test]
fn clone_node_shadow_root_error() {
    let (mut dom, mut session) = setup();
    let host = dom.create_element("div", Attributes::default());
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    wrap(sr, &mut session);

    let r = CloneNode.invoke(sr, &[], &mut session, &mut dom);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().kind, DomApiErrorKind::NotSupportedError);
}

#[test]
fn clone_node_document_type() {
    let (mut dom, mut session) = setup();
    let dt = dom.create_document_type("html", "-//W3C", "http://example.com");
    wrap(dt, &mut session);

    let r = CloneNode.invoke(dt, &[], &mut session, &mut dom).unwrap();
    if let JsValue::ObjectRef(ref_id) = r {
        let (cloned, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        let data = dom.world().get::<&DocTypeData>(cloned).unwrap();
        assert_eq!(data.name, "html");
        assert_eq!(data.public_id, "-//W3C");
        assert_eq!(data.system_id, "http://example.com");
        assert_ne!(cloned, dt);
    } else {
        panic!("expected ObjectRef");
    }
}

// -----------------------------------------------------------------------
// CloneNode — ComponentKind (M6)
// -----------------------------------------------------------------------

#[test]
fn clone_node_component_kind_element() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    wrap(div, &mut session);

    let result = CloneNode
        .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn clone_node_component_kind_document() {
    let (mut dom, mut session) = setup();
    let doc = dom.create_document_root();
    wrap(doc, &mut session);

    let result = CloneNode
        .invoke(doc, &[JsValue::Bool(false)], &mut session, &mut dom)
        .unwrap();
    if let JsValue::ObjectRef(ref_id) = result {
        let (cloned, kind) = session
            .identity_map()
            .get(JsObjectRef::from_raw(ref_id))
            .unwrap();
        assert_ne!(cloned, doc);
        assert_eq!(kind, ComponentKind::Document);
    } else {
        panic!("expected ObjectRef");
    }
}
