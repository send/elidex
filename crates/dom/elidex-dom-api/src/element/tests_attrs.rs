use super::*;
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiErrorKind, DomApiHandler, SessionCore};

fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    // Pre-register entities so we can pass ObjectRef args.
    session.get_or_create_wrapper(parent, ComponentKind::Element);
    session.get_or_create_wrapper(child, ComponentKind::Element);
    (dom, parent, child, session)
}

// -----------------------------------------------------------------------
// hasAttribute tests
// -----------------------------------------------------------------------

#[test]
fn has_attribute_true() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("id", "test");
    }
    let result = HasAttribute
        .invoke(
            parent,
            &[JsValue::String("id".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn has_attribute_false() {
    let (mut dom, parent, _, mut session) = setup();
    let result = HasAttribute
        .invoke(
            parent,
            &[JsValue::String("id".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

// -----------------------------------------------------------------------
// toggleAttribute tests
// -----------------------------------------------------------------------

#[test]
fn toggle_attribute_adds_when_absent() {
    let (mut dom, parent, _, mut session) = setup();
    let result = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("hidden".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert_eq!(attrs.get("hidden"), Some(""));
}

#[test]
fn toggle_attribute_removes_when_present() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("hidden", "");
    }
    let result = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("hidden".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(!attrs.contains("hidden"));
}

#[test]
fn toggle_attribute_force_true() {
    let (mut dom, parent, _, mut session) = setup();
    let result = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("hidden".into()), JsValue::Bool(true)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(attrs.contains("hidden"));
}

#[test]
fn toggle_attribute_force_false() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("hidden", "");
    }
    let result = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("hidden".into()), JsValue::Bool(false)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(!attrs.contains("hidden"));
}

#[test]
fn toggle_attribute_rejects_invalid_name() {
    let (mut dom, parent, _, mut session) = setup();
    let err = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
}

// -----------------------------------------------------------------------
// getAttributeNames tests
// -----------------------------------------------------------------------

#[test]
fn get_attribute_names() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("id", "x");
        attrs.set("class", "y");
    }
    let result = GetAttributeNames
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap();
    if let JsValue::String(s) = result {
        let names: Vec<&str> = s.split('\0').collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"class"));
    } else {
        panic!("expected string");
    }
}

#[test]
fn get_attribute_names_empty() {
    let (mut dom, parent, _, mut session) = setup();
    let result = GetAttributeNames
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String(String::new()));
}

// -----------------------------------------------------------------------
// className getter/setter tests
// -----------------------------------------------------------------------

#[test]
fn classname_get_set() {
    let (mut dom, parent, _, mut session) = setup();
    // Initially empty.
    let result = GetClassName
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String(String::new()));

    // Set.
    SetClassName
        .invoke(
            parent,
            &[JsValue::String("foo bar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = GetClassName
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(result, JsValue::String("foo bar".into()));
}

// -----------------------------------------------------------------------
// id getter/setter tests
// -----------------------------------------------------------------------

#[test]
fn id_get_set() {
    let (mut dom, parent, _, mut session) = setup();
    let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String(String::new()));

    SetId
        .invoke(
            parent,
            &[JsValue::String("main".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = GetId.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("main".into()));
}

// -----------------------------------------------------------------------
// data_attr_to_camel / camel_to_data_attr tests
// -----------------------------------------------------------------------

#[test]
fn data_attr_to_camel_basic() {
    assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
    assert_eq!(data_attr_to_camel("data-x"), "x");
    assert_eq!(data_attr_to_camel("data-foo-bar-baz"), "fooBarBaz");
}

#[test]
fn camel_to_data_attr_basic() {
    assert_eq!(camel_to_data_attr("fooBar"), "data-foo-bar");
    assert_eq!(camel_to_data_attr("x"), "data-x");
    assert_eq!(camel_to_data_attr("fooBarBaz"), "data-foo-bar-baz");
}

#[test]
fn data_attr_roundtrip() {
    let camel = data_attr_to_camel("data-my-value");
    let attr = camel_to_data_attr(&camel);
    assert_eq!(attr, "data-my-value");
}

// -----------------------------------------------------------------------
// dataset tests
// -----------------------------------------------------------------------

#[test]
fn dataset_set_and_get() {
    let (mut dom, parent, _, mut session) = setup();
    DatasetSet
        .invoke(
            parent,
            &[
                JsValue::String("fooBar".into()),
                JsValue::String("42".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let result = DatasetGet
        .invoke(
            parent,
            &[JsValue::String("fooBar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::String("42".into()));

    // Verify it's stored as data-foo-bar.
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert_eq!(attrs.get("data-foo-bar"), Some("42"));
}

#[test]
fn dataset_get_missing() {
    let (mut dom, parent, _, mut session) = setup();
    let result = DatasetGet
        .invoke(
            parent,
            &[JsValue::String("missing".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn dataset_delete() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("data-foo-bar", "val");
    }
    DatasetDelete
        .invoke(
            parent,
            &[JsValue::String("fooBar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let attrs = dom.world().get::<&Attributes>(parent).unwrap();
    assert!(!attrs.contains("data-foo-bar"));
}

#[test]
fn dataset_keys() {
    let (mut dom, parent, _, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("data-x", "1");
        attrs.set("data-foo-bar", "2");
        attrs.set("class", "ignore");
    }
    let result = DatasetKeys
        .invoke(parent, &[], &mut session, &mut dom)
        .unwrap();
    if let JsValue::String(s) = result {
        let keys: Vec<&str> = s.split('\0').collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"x"));
        assert!(keys.contains(&"fooBar"));
    } else {
        panic!("expected string");
    }
}

#[test]
fn toggle_attribute_rev_version() {
    let (mut dom, parent, _child, mut session) = setup();
    let v1 = dom.inclusive_descendants_version(parent);
    ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("hidden".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let v2 = dom.inclusive_descendants_version(parent);
    assert_ne!(v1, v2);
}

#[test]
fn set_class_name_rev_version() {
    let (mut dom, parent, _child, mut session) = setup();
    let v1 = dom.inclusive_descendants_version(parent);
    SetClassName
        .invoke(
            parent,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let v2 = dom.inclusive_descendants_version(parent);
    assert_ne!(v1, v2);
}

#[test]
fn set_id_rev_version() {
    let (mut dom, parent, _child, mut session) = setup();
    let v1 = dom.inclusive_descendants_version(parent);
    SetId
        .invoke(
            parent,
            &[JsValue::String("myid".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let v2 = dom.inclusive_descendants_version(parent);
    assert_ne!(v1, v2);
}

#[test]
fn dataset_set_rev_version() {
    let (mut dom, parent, _child, mut session) = setup();
    let v1 = dom.inclusive_descendants_version(parent);
    DatasetSet
        .invoke(
            parent,
            &[JsValue::String("foo".into()), JsValue::String("bar".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let v2 = dom.inclusive_descendants_version(parent);
    assert_ne!(v1, v2);
}

#[test]
fn dataset_delete_rev_version() {
    let (mut dom, parent, _child, mut session) = setup();
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(parent).unwrap();
        attrs.set("data-foo", "bar");
    }
    let v1 = dom.inclusive_descendants_version(parent);
    DatasetDelete
        .invoke(
            parent,
            &[JsValue::String("foo".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let v2 = dom.inclusive_descendants_version(parent);
    assert_ne!(v1, v2);
}

#[test]
fn data_attr_to_camel_non_lowercase() {
    // Dash followed by non-lowercase should preserve dash + char.
    assert_eq!(data_attr_to_camel("data-foo-Bar"), "foo-Bar");
    assert_eq!(data_attr_to_camel("data-foo-1"), "foo-1");
    assert_eq!(data_attr_to_camel("data-foo-bar"), "fooBar");
    // Trailing dash should be preserved.
    assert_eq!(data_attr_to_camel("data-foo-"), "foo-");
}

// -----------------------------------------------------------------------
// InlineStyle cache invalidation on attribute mutation (Codex #335 R5 F15)
// -----------------------------------------------------------------------

/// Seed a `style` attribute plus a hydrated `InlineStyle` cache, mimicking
/// a prior `el.style.*` read that materialized the component.
fn seed_hydrated_style(dom: &mut EcsDom, entity: Entity, css_decl: (&str, &str)) {
    {
        let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).unwrap();
        attrs.set("style", format!("{}: {}", css_decl.0, css_decl.1));
    }
    let mut style = elidex_ecs::InlineStyle::default();
    style.set(css_decl.0, css_decl.1);
    dom.world_mut().insert_one(entity, style).unwrap();
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(entity).is_ok(),
        "precondition: InlineStyle cache present"
    );
}

/// Codex #335 R5 F15: the `RemoveAttribute` handler must route through the
/// `EcsDom::remove_attribute` chokepoint so removing the `style` attribute
/// invalidates the lazily-hydrated `InlineStyle` cache — otherwise a prior
/// read leaves a stale component that resurrects the removed declaration.
/// This pins the engine-independent handler (the path boa's generic
/// `removeAttribute` and the VM's reflected-attribute removals share).
#[test]
fn remove_attribute_style_invalidates_inline_style_cache() {
    let (mut dom, parent, _, mut session) = setup();
    seed_hydrated_style(&mut dom, parent, ("color", "red"));
    RemoveAttribute
        .invoke(
            parent,
            &[JsValue::String("style".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert!(
        dom.world()
            .get::<&Attributes>(parent)
            .map_or(true, |a| !a.contains("style")),
        "style attribute should be removed"
    );
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(parent).is_err(),
        "stale InlineStyle cache survived removeAttribute('style')"
    );
}

/// Codex #335 R6 F21: when the receiver is not a live Element,
/// `EcsDom::set_attribute` returns `false`; `toggleAttribute` must surface
/// that as an error rather than claim a phantom add (mirrors
/// `SetAttribute`'s `NotFoundError`).
#[test]
fn toggle_attribute_on_non_element_errors() {
    let mut dom = EcsDom::new();
    // A Document node is a non-Element receiver: `set_attribute` returns
    // `false` for it, so the add branch must error.
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = ToggleAttribute.invoke(
        doc,
        &[JsValue::String("hidden".into())],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
    assert!(dom.world().get::<&Attributes>(doc).is_err());
}

/// Codex #335 R9 F29: `toggleAttribute(name, false)` (forced removal) on a
/// stale/non-Element receiver must also error — the `has` probe collapses
/// to false and the forced-removal branch reaches no chokepoint, so the
/// up-front receiver-liveness guard is what surfaces the error.
#[test]
fn toggle_attribute_force_false_on_non_element_errors() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();
    let result = ToggleAttribute.invoke(
        doc,
        &[JsValue::String("hidden".into()), JsValue::Bool(false)],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
}

/// F15 audit: `toggleAttribute('style')` (removal branch) shares the same
/// chokepoint-bypass class and must also invalidate the cache.
#[test]
fn toggle_attribute_style_off_invalidates_inline_style_cache() {
    let (mut dom, parent, _, mut session) = setup();
    seed_hydrated_style(&mut dom, parent, ("color", "red"));
    let result = ToggleAttribute
        .invoke(
            parent,
            &[JsValue::String("style".into()), JsValue::Bool(false)],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
    assert!(
        dom.world().get::<&elidex_ecs::InlineStyle>(parent).is_err(),
        "stale InlineStyle cache survived toggleAttribute('style', false)"
    );
}
