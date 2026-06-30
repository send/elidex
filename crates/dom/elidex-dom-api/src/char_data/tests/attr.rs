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
fn set_attribute_node_self_fires_no_record() {
    // WHATWG DOM §4.9 "set an attribute" step 4: `el.setAttributeNode(
    // el.getAttributeNode('x'))` returns the same Attr with NO write and NO
    // MutationObserver record — the entity-backed dom-api path must short-circuit
    // (oldAttr == attr via `AttrEntityCache`) before `apply_set_attribute`, which
    // records every successful write. Mirrors the VM native's identity guard.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("data-x", "42");
    let div = dom.create_element("div", attrs);
    let mut session = SessionCore::new();
    // Materialize + cache the element's canonical Attr node for "data-x".
    let attr_ref = GetAttributeNode
        .invoke(
            div,
            &[JsValue::String("data-x".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let JsValue::ObjectRef(want) = attr_ref else {
        panic!("getAttributeNode should return an ObjectRef");
    };
    let _ = session.take_notify_records(); // isolate the setAttributeNode call

    let result = SetAttributeNode
        .invoke(div, &[JsValue::ObjectRef(want)], &mut session, &mut dom)
        .unwrap();

    // Step-4 return = the same Attr; no record; attribute unchanged.
    assert!(matches!(result, JsValue::ObjectRef(r) if r == want));
    assert!(
        session.take_notify_records().is_empty(),
        "self setAttributeNode (oldAttr==attr) must emit no MutationObserver record"
    );
    let attrs = dom.world().get::<&Attributes>(div).unwrap();
    assert_eq!(attrs.get("data-x"), Some("42"));
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

/// Codex #335 R5 F15 (attribute-node removal): removing a `style` Attr node
/// must route through the `EcsDom::remove_attribute` chokepoint so a
/// lazily-hydrated `InlineStyle` cache is invalidated. Mirrors the
/// `removeAttribute` handler fix for the Attr-node path.
#[test]
fn remove_attribute_node_style_invalidates_inline_style_cache() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: red");
    let div = dom.create_element("div", attrs);
    // Simulate a prior `el.style.*` read that hydrated the cache.
    let mut style = elidex_ecs::InlineStyle::default();
    style.set("color", "red");
    dom.world_mut().insert_one(div, style).unwrap();

    let attr = dom.create_attribute("style");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "color: red".into();
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

    assert!(dom
        .world()
        .get::<&Attributes>(div)
        .map_or(true, |a| !a.contains("style")));
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(div).is_err(),
        "stale InlineStyle cache survived removeAttributeNode(style)"
    );
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

/// Codex #335 R6 F20: when the receiver is not a live Element,
/// `EcsDom::set_attribute` returns `false`; `setAttributeNode` must
/// surface that as an error and NOT mark the Attr as owned by a dead
/// receiver (mirrors `SetAttribute`'s `NotFoundError`).
#[test]
fn set_attribute_node_on_non_element_errors_without_owning() {
    let mut dom = EcsDom::new();
    // A Document node is a non-Element receiver: `set_attribute` short-
    // circuits to `false` for it.
    let doc = dom.create_document_root();
    let attr = dom.create_attribute("id");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "x".into();
    }
    let mut session = SessionCore::new();
    let attr_ref = session
        .get_or_create_wrapper(attr, ComponentKind::Element)
        .to_raw();

    let result =
        SetAttributeNode.invoke(doc, &[JsValue::ObjectRef(attr_ref)], &mut session, &mut dom);
    assert!(result.is_err());
    // Ownership must be untouched — no phantom success.
    let ad = dom.world().get::<&AttrData>(attr).unwrap();
    assert_eq!(ad.owner_element, None);
}

/// Codex #335 R9 F30: `removeAttributeNode` on a stale/non-Element receiver
/// (the Attr still recording it as owner) must error BEFORE detaching the
/// Attr from its owner — `remove_attribute` returns `()` and silently
/// no-ops, so the up-front receiver-liveness guard surfaces the error and
/// the recorded owner is left intact.
#[test]
fn remove_attribute_node_on_non_element_errors_without_detaching() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let attr = dom.create_attribute("id");
    {
        let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
        ad.value = "x".into();
        ad.owner_element = Some(doc);
    }
    let mut session = SessionCore::new();
    let attr_ref = session
        .get_or_create_wrapper(attr, ComponentKind::Element)
        .to_raw();

    let result =
        RemoveAttributeNode.invoke(doc, &[JsValue::ObjectRef(attr_ref)], &mut session, &mut dom);
    assert!(result.is_err());
    // The Attr must NOT have been detached from its recorded owner.
    let ad = dom.world().get::<&AttrData>(attr).unwrap();
    assert_eq!(ad.owner_element, Some(doc));
}
