//! Custom Elements methods for `HostBridge`.
//!
//! Manages custom element definitions, constructors, lifecycle reactions,
//! `whenDefined()` promises, and the `is` attribute lookup.

use boa_engine::{JsObject, JsValue};
use elidex_custom_elements::CustomElementReaction;
use elidex_ecs::Entity;

use super::HostBridge;

impl HostBridge {
    /// Register a custom element definition.
    ///
    /// Stores the constructor and calls `registry.define()`.
    /// Upgrade-candidate discovery is the caller's job via
    /// [`Self::ce_collect_upgrade_candidates`].
    pub fn register_custom_element(
        &self,
        name: &str,
        constructor: JsObject,
        observed_attrs: Vec<String>,
        extends: Option<String>,
    ) -> Result<(), elidex_custom_elements::DefineError> {
        let mut inner = self.inner.borrow_mut();

        // Validate before allocating constructor ID to avoid leaking an ID
        // and storing the constructor on define() failure.
        if !elidex_custom_elements::is_valid_custom_element_name(name) {
            return Err(elidex_custom_elements::DefineError::InvalidName(
                name.to_string(),
            ));
        }
        if inner.custom_element_registry.is_defined(name) {
            return Err(elidex_custom_elements::DefineError::AlreadyDefined(
                name.to_string(),
            ));
        }

        let id = inner.ce_next_constructor_id;
        inner.ce_next_constructor_id += 1;
        inner.custom_element_constructors.insert(id, constructor);

        let def = elidex_custom_elements::CustomElementDefinition::new(
            name.to_string(),
            id,
            observed_attrs,
            extends,
        );
        inner.custom_element_registry.define(def)
    }

    /// Retrieve the JS constructor for a custom element definition by name.
    pub fn get_custom_element_constructor(&self, name: &str) -> Option<JsObject> {
        let inner = self.inner.borrow();
        let def = inner.custom_element_registry.get(name)?;
        inner
            .custom_element_constructors
            .get(&def.constructor_id)
            .cloned()
    }

    /// Enqueue a custom element lifecycle reaction.
    pub fn enqueue_ce_reaction(&self, reaction: CustomElementReaction) {
        self.inner
            .borrow_mut()
            .custom_element_reactions
            .push(reaction);
    }

    /// Drain all pending custom element reactions.
    pub fn drain_ce_reactions(&self) -> Vec<CustomElementReaction> {
        std::mem::take(&mut self.inner.borrow_mut().custom_element_reactions)
    }

    /// Check whether a custom element name has been defined.
    #[must_use]
    pub fn is_custom_element_defined(&self, name: &str) -> bool {
        self.inner.borrow().custom_element_registry.is_defined(name)
    }

    /// WHATWG HTML §4.13.4 define()-time upgrade-candidate discovery —
    /// thin accessor over the engine-indep
    /// `elidex_custom_elements::collect_upgrade_candidates` (shadow-
    /// including descendants of the document in tree order, with the
    /// Undefined + document-registry + local-name/is-value match), so
    /// the candidate rule is one function shared with the VM engine.
    /// Detached elements upgrade on insertion, not here.
    #[must_use]
    pub fn ce_collect_upgrade_candidates(
        &self,
        dom: &elidex_ecs::EcsDom,
        name: &str,
    ) -> Vec<Entity> {
        let inner = self.inner.borrow();
        let Some(document) = inner.document_entity else {
            return Vec::new();
        };
        elidex_custom_elements::collect_upgrade_candidates(
            dom,
            document,
            &inner.custom_element_registry,
            name,
        )
    }

    /// DOM §4.4 clone-time creation pass over a fresh clone subtree:
    /// delegates to the engine-indep
    /// `elidex_custom_elements::apply_clone_creation_ce_semantics`
    /// (async-autonomous null-is rule + upgrade candidacy via
    /// `prepare_upgrade`) and enqueues an `Upgrade` reaction per
    /// candidate (Codex PR331 R14 — shared rule with the VM path).
    pub fn apply_clone_ce_creation_pass(&self, dom: &mut elidex_ecs::EcsDom, clone_root: Entity) {
        let candidates = {
            let inner = self.inner.borrow();
            elidex_custom_elements::apply_clone_creation_ce_semantics(
                dom,
                &inner.custom_element_registry,
                clone_root,
            )
        };
        for entity in candidates {
            self.enqueue_ce_reaction(CustomElementReaction::Upgrade(entity));
        }
    }

    /// Access a custom element definition by name via a closure.
    ///
    /// Returns `false` if the definition does not exist, otherwise returns the
    /// closure's result.
    #[must_use]
    pub fn with_ce_definition<F>(&self, name: &str, f: F) -> bool
    where
        F: FnOnce(&elidex_custom_elements::CustomElementDefinition) -> bool,
    {
        self.inner
            .borrow()
            .custom_element_registry
            .get(name)
            .is_some_and(f)
    }

    /// Look up the `extends` tag for a custom element by name.
    ///
    /// Returns `None` if the definition does not exist or does not extend
    /// a built-in element (autonomous custom element).
    #[must_use]
    pub fn ce_extends_tag(&self, name: &str) -> Option<String> {
        self.inner
            .borrow()
            .custom_element_registry
            .get(name)
            .and_then(|def| def.extends.clone())
    }

    /// Look up the observed attributes for a custom element by name.
    pub fn ce_observed_attributes(&self, name: &str) -> Vec<String> {
        self.inner
            .borrow()
            .custom_element_registry
            .get(name)
            .map(|def| def.observed_attributes().to_vec())
            .unwrap_or_default()
    }

    /// Check if a specific attribute is observed for a custom element (non-allocating).
    pub fn ce_is_observed_attribute(&self, ce_name: &str, attr_name: &str) -> bool {
        self.inner
            .borrow()
            .custom_element_registry
            .get(ce_name)
            .is_some_and(|def| def.observes(attr_name))
    }

    /// Store a `whenDefined()` resolve function for a not-yet-defined custom element.
    pub fn store_when_defined_resolver(
        &self,
        name: &str,
        resolver: boa_engine::object::builtins::JsFunction,
    ) {
        self.inner
            .borrow_mut()
            .when_defined_resolvers
            .entry(name.to_string())
            .or_default()
            .push(resolver);
    }

    /// Take all pending `whenDefined()` resolve functions for a name.
    pub fn take_when_defined_resolvers(
        &self,
        name: &str,
    ) -> Vec<boa_engine::object::builtins::JsFunction> {
        self.inner
            .borrow_mut()
            .when_defined_resolvers
            .remove(name)
            .unwrap_or_default()
    }

    /// Store a cached pending `whenDefined()` promise for a custom element name.
    pub fn store_when_defined_promise(&self, name: &str, promise: JsValue) {
        self.inner
            .borrow_mut()
            .when_defined_promises
            .insert(name.to_string(), promise);
    }

    /// Get a cached pending `whenDefined()` promise for a custom element name.
    pub fn get_when_defined_promise(&self, name: &str) -> Option<JsValue> {
        self.inner.borrow().when_defined_promises.get(name).cloned()
    }

    /// Clear the cached pending `whenDefined()` promise after resolution.
    pub fn clear_when_defined_promise(&self, name: &str) {
        self.inner.borrow_mut().when_defined_promises.remove(name);
    }

    /// Look up a customized built-in element by `is` attribute value and tag name.
    pub fn ce_lookup_by_is(&self, is_value: &str, tag: &str) -> bool {
        self.inner
            .borrow()
            .custom_element_registry
            .lookup_by_is(is_value, tag)
            .is_some()
    }

    /// DOM §4.9 step 5.1.3.10 — delegate to the engine-indep
    /// registry-gated is-value clear (see
    /// `elidex_custom_elements::clear_is_value_for_sync_autonomous`).
    pub fn ce_clear_is_value_for_sync_autonomous(
        &self,
        dom: &mut elidex_ecs::EcsDom,
        entity: elidex_ecs::Entity,
    ) {
        let inner = self.inner.borrow();
        elidex_custom_elements::clear_is_value_for_sync_autonomous(
            &inner.custom_element_registry,
            dom,
            entity,
        );
    }
}
