//! Custom element registry (WHATWG HTML §4.13.4).
//!
//! Stores custom element definitions and manages the pending upgrade queue.

use std::collections::HashMap;

use elidex_ecs::Entity;

use crate::validation::is_valid_custom_element_name;

/// A registered custom element definition.
#[derive(Clone, Debug)]
pub struct CustomElementDefinition {
    /// Custom element name (e.g., `"my-element"`).
    pub name: String,
    /// ID referencing the JS constructor stored in `HostBridge`.
    pub constructor_id: u64,
    /// Attribute names observed by `attributeChangedCallback`.
    pub observed_attributes: Vec<String>,
    /// Built-in element tag to extend (e.g., `"div"` for customized built-in).
    /// `None` for autonomous custom elements.
    pub extends: Option<String>,
}

/// Registry of custom element definitions per realm (WHATWG HTML §4.13.4).
///
/// Stored in `HostBridge` (per content thread / per tab). Each navigation
/// creates a new bridge, naturally resetting the registry.
#[derive(Debug, Default)]
pub struct CustomElementRegistry {
    /// Name → definition mapping.
    definitions: HashMap<String, CustomElementDefinition>,
    /// Elements awaiting upgrade (created before `define()` was called).
    /// Key: custom element name, Value: entities waiting for that definition.
    pending_upgrade: HashMap<String, Vec<Entity>>,
}

impl CustomElementRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom element definition.
    ///
    /// Returns `Err` if the name is invalid or already defined.
    /// On success, returns the list of entities pending upgrade for this name.
    pub fn define(
        &mut self,
        definition: CustomElementDefinition,
    ) -> Result<Vec<Entity>, DefineError> {
        if !is_valid_custom_element_name(&definition.name) {
            return Err(DefineError::InvalidName(definition.name.clone()));
        }
        if self.definitions.contains_key(&definition.name) {
            return Err(DefineError::AlreadyDefined(definition.name.clone()));
        }
        let name = definition.name.clone();
        self.definitions.insert(name.clone(), definition);
        // Return pending upgrades for this name (drain the queue).
        Ok(self.pending_upgrade.remove(&name).unwrap_or_default())
    }

    /// Look up a custom element definition by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        self.definitions.get(name)
    }

    /// Check whether a name has been defined.
    #[must_use]
    pub fn is_defined(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }

    /// Queue an entity for upgrade when the definition becomes available.
    ///
    /// Called when an element with a custom element name is created before
    /// `customElements.define()` has been called for that name.
    pub fn queue_for_upgrade(&mut self, name: &str, entity: Entity) {
        self.pending_upgrade
            .entry(name.to_string())
            .or_default()
            .push(entity);
    }

    /// Look up a definition by the `is` attribute value and the tag name.
    ///
    /// For customized built-in elements: `is="my-div"` on a `<div>` matches
    /// a definition with `name="my-div"` and `extends=Some("div")`.
    #[must_use]
    pub fn lookup_by_is(&self, is_value: &str, tag: &str) -> Option<&CustomElementDefinition> {
        self.definitions.get(is_value).filter(|def| {
            def.extends
                .as_ref()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(tag))
        })
    }
}

/// Error returned by [`CustomElementRegistry::define`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefineError {
    /// Name is not a valid custom element name.
    InvalidName(String),
    /// Name is already defined.
    AlreadyDefined(String),
}

impl std::fmt::Display for DefineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "'{name}' is not a valid custom element name"),
            Self::AlreadyDefined(name) => {
                write!(f, "'{name}' has already been defined as a custom element")
            }
        }
    }
}

impl std::error::Error for DefineError {}
