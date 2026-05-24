//! Custom element registry (WHATWG HTML §4.13.4).
//!
//! Stores custom element definitions and manages the pending upgrade queue.

use std::collections::{HashMap, HashSet};

use elidex_ecs::Entity;

use crate::state::{CEState, CustomElementState};
use crate::validation::is_valid_custom_element_name;

/// A registered custom element definition.
///
/// `observed_attributes` preserves the developer's declared name list
/// (useful for serialization / external inspection). A parallel private
/// `observed_set` provides O(1) membership for the **mutation hot path**
/// (every `setAttribute` runs the consumer's filter, so an O(N)
/// `Vec::contains` would scale poorly with large `observedAttributes`
/// declarations). Membership is exposed via [`Self::observes`]; the
/// `observed_set` field is private so the Vec/Set pair can only desync
/// inside this module — external code cannot construct
/// `CustomElementDefinition` via a struct literal.
#[derive(Clone, Debug)]
pub struct CustomElementDefinition {
    /// Custom element name (e.g., `"my-element"`).
    pub name: String,
    /// ID referencing the JS constructor stored in `HostBridge`.
    pub constructor_id: u64,
    /// Attribute names observed by `attributeChangedCallback`, preserved
    /// in the order the developer returned them from the static
    /// `observedAttributes` getter. The upgrade-time initial
    /// `attributeChangedCallback` enqueue (`finalize_success`) walks the
    /// element's own attribute list per HTML §4.13.5 step 12 ("for each
    /// attribute in element's attribute list, in order") and filters by
    /// membership via [`Self::observes`] — so the enqueue order is
    /// element-attribute-insertion order, NOT this Vec's declared order.
    pub observed_attributes: Vec<String>,
    /// Built-in element tag to extend (e.g., `"div"` for customized built-in).
    /// `None` for autonomous custom elements.
    pub extends: Option<String>,
    /// Parallel `HashSet` over `observed_attributes` for O(1) hot-path
    /// membership tests. Private: callers must use [`Self::observes`]
    /// so the Vec/Set pair cannot desync via a struct literal.
    observed_set: HashSet<String>,
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

impl CustomElementDefinition {
    /// Construct a definition with the private `observed_set`
    /// materialized from `observed_attributes`. Single construction
    /// site for the Vec/Set parallel-invariant — the private field
    /// makes a struct literal a compile error from outside the module,
    /// so the only way in is via this constructor.
    #[must_use]
    pub fn new(
        name: String,
        constructor_id: u64,
        observed_attributes: Vec<String>,
        extends: Option<String>,
    ) -> Self {
        let observed_set: HashSet<String> = observed_attributes.iter().cloned().collect();
        Self {
            name,
            constructor_id,
            observed_attributes,
            extends,
            observed_set,
        }
    }

    /// O(1) `observedAttributes` membership test — the mutation-consumer
    /// hot path. Delegates to the private parallel `HashSet` so callers
    /// cannot read the set directly and accidentally drift it.
    #[must_use]
    pub fn observes(&self, name: &str) -> bool {
        self.observed_set.contains(name)
    }
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

    /// Iterate the registered custom element names. Caller-owned
    /// snapshot use case: `customElements.upgrade(root)`'s shadow-
    /// inclusive walk snapshots the name set once before the walk so
    /// the closure can do O(1) `HashSet::contains` instead of locking
    /// the registry mutex per descendant.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.definitions.keys().map(String::as_str)
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

    /// Drop every definition + pending-upgrade entity. Used by
    /// `Vm::unbind` to scrub cross-DOM references on the way out of a
    /// bind cycle (`CustomElementDefinition::constructor_id` indexes
    /// into per-VM `HostData::ce_constructors`; pending-upgrade
    /// entities reference the outgoing DOM world).
    pub fn clear(&mut self) {
        self.definitions.clear();
        self.pending_upgrade.clear();
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

/// World-wide query for every entity carrying
/// `CustomElementState` with `state == CEState::Undefined` for `name`, minus the entities
/// in `skip_already_pending`. Used by `customElements.define()` to
/// drain elements that the parser / cross-document `createElement`
/// path attached state to but could not enqueue via
/// [`CustomElementRegistry::queue_for_upgrade`] (the parser cannot
/// reach the per-VM registry).
///
/// Walks the hecs world directly so detached + DocumentFragment
/// subtrees + future multi-document worlds are covered (a
/// document-rooted DOM walk would silently miss them).
#[must_use]
pub fn collect_undefined_entities(
    world: &hecs::World,
    name: &str,
    skip_already_pending: &[Entity],
) -> Vec<Entity> {
    // O(pending) HashSet build → O(world) walk with O(1) membership
    // tests, replacing the O(world × pending) `[Entity]::contains`
    // scan. Pending size can grow with parser-baked + createElement-
    // queued elements waiting for define() so the structural fix
    // matters for large CE component pages.
    let skip: std::collections::HashSet<Entity> = skip_already_pending.iter().copied().collect();
    let mut out = Vec::new();
    let mut query = world.query::<(Entity, &CustomElementState)>();
    for (entity, state) in &mut query {
        if matches!(state.state, CEState::Undefined)
            && state.definition_name == name
            && !skip.contains(&entity)
        {
            out.push(entity);
        }
    }
    out
}
