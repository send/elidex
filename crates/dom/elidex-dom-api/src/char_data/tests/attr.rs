use super::*;
use elidex_ecs::{AttrData, Attributes};
use elidex_script_session::{ComponentKind, DomApiErrorKind, JsObjectRef};

#[test]
fn create_attribute() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = CreateAttribute
        .invoke(
            doc,
            &[JsValue::String("Data-X".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));

    if let JsValue::ObjectRef(id) = result {
        let (entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(id))
            .unwrap();
        let ad = dom.world().get::<&AttrData>(entity).unwrap();
        assert_eq!(ad.local_name, "data-x");
    }
}

#[test]
fn get_attribute_node_exists() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "active");
    let div = dom.create_element("div", attrs);
    let mut session = SessionCore::new();

    let result = GetAttributeNode
        .invoke(
            div,
            &[JsValue::String("class".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));

    if let JsValue::ObjectRef(id) = result {
        let (entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(id))
            .unwrap();
        let ad = dom.world().get::<&AttrData>(entity).unwrap();
        assert_eq!(ad.local_name, "class");
        assert_eq!(ad.value, "active");
        assert_eq!(ad.owner_element, Some(div));
    }
}

#[test]
fn get_attribute_node_missing() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();

    let result = GetAttributeNode
        .invoke(
            div,
            &[JsValue::String("nonexistent".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn get_attribute_node_identity_cache() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("id", "test");
    let div = dom.create_element("div", attrs);
    let mut session = SessionCore::new();

    // First call creates and caches the Attr entity.
    let r1 = GetAttributeNode
        .invoke(div, &[JsValue::String("id".into())], &mut session, &mut dom)
        .unwrap();

    // Second call should return the same ObjectRef (same entity).
    let r2 = GetAttributeNode
        .invoke(div, &[JsValue::String("id".into())], &mut session, &mut dom)
        .unwrap();

    assert_eq!(r1, r2, "repeated getAttributeNode must return same entity");
}

#[test]
fn set_attribute_node() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let attr = dom.create_attribute("data-x");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "42".into();
    }
    let mut session = SessionCore::new();
    let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

    SetAttributeNode
        .invoke(
            div,
            &[JsValue::ObjectRef(attr_ref.to_raw())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let attrs = dom.world().get::<&Attributes>(div).unwrap();
    assert_eq!(attrs.get("data-x"), Some("42"));

    let ad = dom.world().get::<&AttrData>(attr).unwrap();
    assert_eq!(ad.owner_element, Some(div));
}

#[test]
fn set_attribute_node_in_use_error() {
    let mut dom = EcsDom::new();
    let div1 = dom.create_element("div", Attributes::default());
    let div2 = dom.create_element("div", Attributes::default());
    let attr = dom.create_attribute("data-x");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "42".into();
        ad.owner_element = Some(div1);
    }
    let mut session = SessionCore::new();
    let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

    let result = SetAttributeNode.invoke(
        div2,
        &[JsValue::ObjectRef(attr_ref.to_raw())],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        DomApiErrorKind::InUseAttributeError
    );
}

#[test]
fn remove_attribute_node() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("data-x", "42");
    let div = dom.create_element("div", attrs);
    let attr = dom.create_attribute("data-x");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "42".into();
        ad.owner_element = Some(div);
    }
    let mut session = SessionCore::new();
    let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

    RemoveAttributeNode
        .invoke(
            div,
            &[JsValue::ObjectRef(attr_ref.to_raw())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
    assert!(!el_attrs.contains("data-x"));

    let ad = dom.world().get::<&AttrData>(attr).unwrap();
    assert_eq!(ad.owner_element, None);
}

#[test]
fn attr_name() {
    let mut dom = EcsDom::new();
    let attr = dom.create_attribute("class");
    let mut session = SessionCore::new();
    let result = GetAttrName
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("class".into()));
}

#[test]
fn attr_value_get_set() {
    let mut dom = EcsDom::new();
    let attr = dom.create_attribute("class");
    let mut session = SessionCore::new();

    let result = GetAttrValue
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String(String::new()));

    SetAttrValue
        .invoke(
            attr,
            &[JsValue::String("active".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = GetAttrValue
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("active".into()));
}

#[test]
fn attr_value_syncs_to_owner() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "old");
    let div = dom.create_element("div", attrs);
    let attr = dom.create_attribute("class");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "old".into();
        ad.owner_element = Some(div);
    }
    let mut session = SessionCore::new();

    SetAttrValue
        .invoke(
            attr,
            &[JsValue::String("new".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
    assert_eq!(el_attrs.get("class"), Some("new"));
}

#[test]
fn attr_owner_element() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let attr = dom.create_attribute("id");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.owner_element = Some(div);
    }
    let mut session = SessionCore::new();

    let result = GetOwnerElement
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn attr_owner_element_null() {
    let mut dom = EcsDom::new();
    let attr = dom.create_attribute("id");
    let mut session = SessionCore::new();
    let result = GetOwnerElement
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn attr_specified() {
    let mut dom = EcsDom::new();
    let attr = dom.create_attribute("id");
    let mut session = SessionCore::new();
    let result = GetAttrSpecified
        .invoke(attr, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn create_attribute_validates_name() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = CreateAttribute.invoke(
        doc,
        &[JsValue::String("invalid name".into())],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
}

#[test]
fn remove_attribute_node_wrong_owner() {
    let mut dom = EcsDom::new();
    let elem1 = dom.create_element("div", Attributes::default());
    let elem2 = dom.create_element("span", Attributes::default());
    {
        let mut a1 = dom.world_mut().get::<&mut Attributes>(elem1).unwrap();
        a1.set("foo", "bar");
    }
    let mut session = SessionCore::new();
    session.get_or_create_wrapper(elem1, ComponentKind::Element);
    session.get_or_create_wrapper(elem2, ComponentKind::Element);

    let attr_result = GetAttributeNode
        .invoke(
            elem1,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let result = RemoveAttributeNode.invoke(elem2, &[attr_result], &mut session, &mut dom);
    assert!(result.is_err());
}

#[test]
fn set_attribute_node_returns_null() {
    let mut dom = EcsDom::new();
    let elem = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();
    session.get_or_create_wrapper(elem, ComponentKind::Element);

    let attr = dom.create_attribute("foo");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "bar".into();
    }
    let attr_ref = session
        .get_or_create_wrapper(attr, ComponentKind::Element)
        .to_raw();

    let result = SetAttributeNode
        .invoke(
            elem,
            &[JsValue::ObjectRef(attr_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Null);
}
