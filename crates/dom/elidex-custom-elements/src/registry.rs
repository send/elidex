//! Custom element registry (WHATWG HTML §4.13.4).
//!
//! Stores custom element definitions and manages the pending upgrade queue.

use std::collections::{HashMap, HashSet};

use elidex_ecs::Entity;

use crate::construction_stack::ConstructionStackEntry;
use crate::state::{CEState, CustomElementState};
use crate::validation::is_valid_custom_element_name;

/// A registered custom element definition.
///
/// The observed-attribute Vec and its parallel `HashSet` are BOTH
/// private — that is the only seal that holds the Vec/Set invariant.
/// Read access is exposed via [`Self::observed_attributes`] (returns
/// `&[String]` so callers cannot mutate the Vec) and
/// [`Self::observes`] (O(1) membership via the parallel `HashSet`).
/// The fields are derived from the constructor argument and are
/// effectively immutable post-`new`; making them `pub` would expose a
/// `&mut def.observed_attributes` mutation path that could silently
/// desync the Set (R11 closure of the seal that R9 #2 started).
#[derive(Clone, Debug)]
pub struct CustomElementDefinition {
    /// Custom element name (e.g., `"my-element"`).
    pub name: String,
    /// ID referencing the JS constructor stored in `HostBridge`.
    pub constructor_id: u64,
    /// Built-in element tag to extend (e.g., `"div"` for customized built-in).
    /// `None` for autonomous custom elements.
    pub extends: Option<String>,
    /// Attribute names observed by `attributeChangedCallback`, in
    /// the order the developer returned them from the static
    /// `observedAttributes` getter. Private — read via
    /// [`Self::observed_attributes`]. Spec (HTML §4.13.5 step 12) for
    /// the upgrade-time initial enqueue walks the element's own
    /// attribute list and filters by membership, so enqueue order is
    /// element-attribute-insertion order, NOT this Vec's order.
    observed_attributes: Vec<String>,
    /// Parallel `HashSet` over `observed_attributes` for O(1) hot-path
    /// membership tests. Private — read via [`Self::observes`].
    observed_set: HashSet<String>,
    /// Per-definition construction stack (\[C2\] WHATWG HTML §4.13.3).
    /// Pushed by the upgrade algorithm (\[C4\] step 6), drained by the
    /// HTMLElement constructor's upgrade branch (\[C1\] step 15:
    /// replace-with-marker). Private — read/write only via the
    /// `*_construction_stack*` registry helpers so the
    /// element-vs-marker invariant is enforced by construction.
    construction_stack: Vec<ConstructionStackEntry>,
}

/// Registry of custom element definitions per realm (WHATWG HTML §4.13.4).
///
/// Stored in `HostBridge` (per content thread / per tab). Each navigation
/// creates a new bridge, naturally resetting the registry.
#[derive(Debug, Default)]
pub struct CustomElementRegistry {
    /// Name → definition mapping.
    definitions: HashMap<String, CustomElementDefinition>,
    /// Reverse index: `constructor_id` → registered name. Maintained
    /// alongside `definitions` for O(1) reverse lookup ([C1] §3.2.3
    /// step 5 reverse-mapping is on the hot path of every CE
    /// `[[Construct]]` — the prior O(N) `values()` scan would scale
    /// poorly on CE-heavy pages (e.g. SSR rehydration with hundreds
    /// of definitions). Both maps are private + only mutated through
    /// `define`/`clear`, so the parallel-invariant is enforced by
    /// construction (R11 #1 / D-17 R9 #2 sealing precedent).
    constructor_id_to_name: HashMap<u64, String>,
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
            extends,
            observed_attributes,
            observed_set,
            construction_stack: Vec::new(),
        }
    }

    /// Whether an element with local name `local_name` matches this
    /// definition for upgrade (WHATWG HTML §4.13.5 "upgrade an element",
    /// via the §4.13.3 "look up a custom element definition" matching):
    ///
    /// - **Customized built-in** (`extends: Some(base)`) — the element's
    ///   local name must equal `base` (HTML's customized built-in syntax
    ///   ties the definition to the retained base tag).
    /// - **Autonomous** (`extends: None`) — the element's local name must
    ///   equal the definition `name` itself.
    ///
    /// This is the single match rule shared by every upgrade gate (the VM's
    /// `prepare_upgrade` and the boa shell's subtree-upgrade walks), so a
    /// mismatched `is=`-marked candidate — e.g. `<div is="my-el">` under an
    /// autonomous `define("my-el", …)`, or under `{ extends: "button" }` —
    /// is never upgraded. (A parse-time marker cannot pre-filter this: the
    /// definition's `extends` is unknown until `define()` runs.)
    #[must_use]
    pub fn upgrade_matches_local_name(&self, local_name: &str) -> bool {
        match &self.extends {
            Some(base) => base.eq_ignore_ascii_case(local_name),
            None => self.name.eq_ignore_ascii_case(local_name),
        }
    }

    /// O(1) `observedAttributes` membership test — the mutation-consumer
    /// hot path. Delegates to the private parallel `HashSet` so callers
    /// cannot read the set directly and accidentally drift it.
    #[must_use]
    pub fn observes(&self, name: &str) -> bool {
        self.observed_set.contains(name)
    }

    /// Read-only view of the developer-declared `observedAttributes`
    /// name list (preserved in declaration order). Returns `&[String]`
    /// so callers cannot mutate the backing Vec — combined with the
    /// private struct field, this is the seal that keeps the Vec/Set
    /// pair from desyncing.
    #[must_use]
    pub fn observed_attributes(&self) -> &[String] {
        &self.observed_attributes
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
        let constructor_id = definition.constructor_id;
        self.definitions.insert(name.clone(), definition);
        // Maintain reverse index for O(1) lookup_by_constructor.
        // Both maps are updated under a single fn so the
        // parallel-invariant holds by construction. The reverse
        // insert MUST return None: callers mint `constructor_id`
        // monotonically (D-17 binding-layer counter) so duplicates
        // are unreachable today, but a future refactor that injects
        // ids from elsewhere would silently overwrite the reverse
        // index and corrupt `lookup_by_constructor`. Asserting
        // None here makes the invariant explicit and catches drift
        // in debug builds before it can alias to a wrong definition.
        let prev = self
            .constructor_id_to_name
            .insert(constructor_id, name.clone());
        debug_assert!(
            prev.is_none(),
            "duplicate constructor_id {constructor_id} (previous name {prev:?})"
        );
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
        // Queue admission is gated on name validity (the element's
        // Undefined MARKING is not — DOM §4.9 step 6.3 is
        // validity-free): `define()` rejects invalid names, so a
        // bucket keyed by one is undrainable forever and would grow
        // unboundedly.  Owning the gate here (not at the engine call
        // sites) protects every present and future caller.
        if !is_valid_custom_element_name(name) {
            return;
        }
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
        self.constructor_id_to_name.clear();
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

    /// Look up a definition by the `constructor_id` it carries
    /// (\[C1\] §3.2.3 step 5 + \[C4\] §4.13.5 step 9.3: NewTarget's
    /// constructor ID identifies the definition during upgrade /
    /// HTMLElement-constructor brand-check). Reverse of the
    /// name → definition_id direction served by [`Self::get`].
    /// O(1) via the private `constructor_id_to_name` parallel
    /// index — the hot path of every CE `[[Construct]]` on pages
    /// with many CE definitions. The parallel-invariant is sealed
    /// by `define` being the only entry path mutating both maps
    /// (R11 #1 / D-17 R9 #2 sealing precedent).
    #[must_use]
    pub fn lookup_by_constructor(&self, constructor_id: u64) -> Option<&CustomElementDefinition> {
        let name = self.constructor_id_to_name.get(&constructor_id)?;
        let def = self.definitions.get(name);
        debug_assert!(
            def.is_some(),
            "lookup_by_constructor: reverse-index name '{name}' missing from definitions \
             (parallel-invariant violated for constructor_id={constructor_id})"
        );
        def
    }

    /// Push an element entry onto `name`'s construction stack
    /// (\[C2\] field + \[C4\] §4.13.5 step 6). Returns `false` when
    /// `name` is not a registered definition (caller-error
    /// short-circuit; the upgrade path holds the registry lock so
    /// this should never happen in practice).
    pub fn push_construction_stack(&mut self, name: &str, entity: Entity) -> bool {
        let Some(def) = self.definitions.get_mut(name) else {
            return false;
        };
        def.construction_stack
            .push(ConstructionStackEntry::Element(entity));
        true
    }

    /// Peek the top entry of `name`'s construction stack (\[C1\]
    /// §3.2.3 step 12: "Let element be the last entry in
    /// definition's construction stack"). `None` if the stack is
    /// empty (sync construct path per \[C1\] step 9) or the
    /// definition is not registered.
    #[must_use]
    pub fn peek_construction_stack(&self, name: &str) -> Option<&ConstructionStackEntry> {
        self.definitions.get(name)?.construction_stack.last()
    }

    /// Replace the top entry with [`ConstructionStackEntry::AlreadyConstructed`]
    /// (\[C1\] §3.2.3 step 15). Returns the entity that was at the
    /// top (so the caller can correlate against the SameValue
    /// invariant) or `None` if the stack is empty / definition is
    /// not registered. Does NOT pop — \[C4\] step 9 cleanup is the
    /// driver of the final pop.
    pub fn replace_construction_stack_top_with_marker(&mut self, name: &str) -> Option<Entity> {
        let stack = &mut self.definitions.get_mut(name)?.construction_stack;
        let last = stack.last_mut()?;
        match std::mem::replace(last, ConstructionStackEntry::AlreadyConstructed) {
            ConstructionStackEntry::Element(entity) => Some(entity),
            ConstructionStackEntry::AlreadyConstructed => None,
        }
    }

    /// Pop the top entry off `name`'s construction stack (\[C4\]
    /// §4.13.5 step 9 cleanup — runs after the constructor returns
    /// regardless of Ok / Err). Returns the popped entry so the
    /// caller can assert against expected shape during testing.
    pub fn pop_construction_stack(&mut self, name: &str) -> Option<ConstructionStackEntry> {
        self.definitions.get_mut(name)?.construction_stack.pop()
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
        // Null-registry elements are outside every registry — the
        // define()-time drain must not upgrade them (DOM §4.9:
        // definition lookup in a null registry is always null).
        if matches!(state.state, CEState::Undefined)
            && matches!(state.registry, crate::RegistryAssociation::Document)
            && state.definition_name == name
            && !skip.contains(&entity)
        {
            out.push(entity);
        }
    }
    out
}

#[cfg(test)]
mod upgrade_match_tests {
    use super::CustomElementDefinition;

    fn def(name: &str, extends: Option<&str>) -> CustomElementDefinition {
        CustomElementDefinition::new(name.to_string(), 1, vec![], extends.map(str::to_string))
    }

    #[test]
    fn autonomous_matches_only_its_own_tag() {
        // Codex #329 R8 (P2): an autonomous definition upgrades only an
        // element whose local name IS the definition name — NOT a
        // `<div is="my-el">` candidate (the boa shell's `is_none_or` walk
        // wrongly upgraded any such entity).
        let d = def("my-el", None);
        assert!(d.upgrade_matches_local_name("my-el"));
        assert!(!d.upgrade_matches_local_name("div"));
    }

    #[test]
    fn customized_builtin_matches_only_extends_base() {
        let d = def("plastic-button", Some("button"));
        assert!(d.upgrade_matches_local_name("button"));
        assert!(d.upgrade_matches_local_name("BUTTON")); // ASCII case-insensitive
        assert!(!d.upgrade_matches_local_name("div"));
        assert!(!d.upgrade_matches_local_name("plastic-button"));
    }
}

#[cfg(test)]
mod queue_admission_tests {
    use super::CustomElementRegistry;

    #[test]
    fn queue_for_upgrade_rejects_invalid_names() {
        // The DoS guard this gate exists for: `define()` rejects
        // invalid names, so a pending bucket keyed by one could never
        // drain — admission must refuse it outright while valid
        // (merely not-yet-defined) names queue normally.
        let mut reg = CustomElementRegistry::new();
        let world = hecs::World::new();
        let entity = world.reserve_entity();
        // Genuinely invalid names: no hyphen ("input", "notvalid"),
        // uppercase ("My-El"). NB "x-invalid" would be VALID (lowercase
        // start + hyphen) — naming a thing "-invalid" doesn't make it so.
        reg.queue_for_upgrade("input", entity);
        reg.queue_for_upgrade("notvalid", entity);
        reg.queue_for_upgrade("My-El", entity);
        assert!(
            reg.pending_upgrade.is_empty(),
            "invalid names must not be admitted to the pending-upgrade queue"
        );
        reg.queue_for_upgrade("my-el", entity);
        assert_eq!(reg.pending_upgrade.len(), 1);
        assert_eq!(reg.pending_upgrade["my-el"], vec![entity]);
    }
}
