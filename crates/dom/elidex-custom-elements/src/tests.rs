//! Tests for Custom Elements registry.

use super::*;
use elidex_ecs::EcsDom;

#[test]
fn define_and_get() {
    let mut registry = CustomElementRegistry::new();
    let def = CustomElementDefinition {
        name: "my-element".to_string(),
        constructor_id: 1,
        observed_attributes: vec!["title".to_string()],
        extends: None,
    };
    let pending = registry.define(def).unwrap();
    assert!(pending.is_empty());
    assert!(registry.is_defined("my-element"));
    assert!(registry.get("my-element").is_some());
    assert_eq!(registry.get("my-element").unwrap().constructor_id, 1);
}

#[test]
fn define_rejects_invalid_name() {
    let mut registry = CustomElementRegistry::new();
    let def = CustomElementDefinition {
        name: "div".to_string(), // no hyphen
        constructor_id: 1,
        observed_attributes: Vec::new(),
        extends: None,
    };
    assert!(registry.define(def).is_err());
}

#[test]
fn define_rejects_duplicate() {
    let mut registry = CustomElementRegistry::new();
    let def1 = CustomElementDefinition {
        name: "my-el".to_string(),
        constructor_id: 1,
        observed_attributes: Vec::new(),
        extends: None,
    };
    let def2 = CustomElementDefinition {
        name: "my-el".to_string(),
        constructor_id: 2,
        observed_attributes: Vec::new(),
        extends: None,
    };
    assert!(registry.define(def1).is_ok());
    assert!(matches!(
        registry.define(def2),
        Err(registry::DefineError::AlreadyDefined(_))
    ));
}

#[test]
fn pending_upgrade_queue() {
    let mut registry = CustomElementRegistry::new();
    let mut dom = EcsDom::new();
    let e1 = dom.create_element("my-el", elidex_ecs::Attributes::default());
    let e2 = dom.create_element("my-el", elidex_ecs::Attributes::default());

    registry.queue_for_upgrade("my-el", e1);
    registry.queue_for_upgrade("my-el", e2);

    let def = CustomElementDefinition {
        name: "my-el".to_string(),
        constructor_id: 1,
        observed_attributes: Vec::new(),
        extends: None,
    };
    let pending = registry.define(def).unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0], e1);
    assert_eq!(pending[1], e2);

    // Queue should be drained after define.
    let def2 = CustomElementDefinition {
        name: "other-el".to_string(),
        constructor_id: 2,
        observed_attributes: Vec::new(),
        extends: None,
    };
    let pending2 = registry.define(def2).unwrap();
    assert!(pending2.is_empty());
}

#[test]
fn lookup_by_is_attribute() {
    let mut registry = CustomElementRegistry::new();
    let def = CustomElementDefinition {
        name: "my-div".to_string(),
        constructor_id: 1,
        observed_attributes: Vec::new(),
        extends: Some("div".to_string()),
    };
    registry.define(def).unwrap();

    // Matching: is="my-div" on <div>.
    assert!(registry.lookup_by_is("my-div", "div").is_some());
    // Non-matching: is="my-div" on <span> (wrong base element).
    assert!(registry.lookup_by_is("my-div", "span").is_none());
    // Non-matching: unknown is value.
    assert!(registry.lookup_by_is("unknown-el", "div").is_none());
}
