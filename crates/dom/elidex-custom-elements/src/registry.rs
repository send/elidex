//! Custom element registry (WHATWG HTML §4.13.4).
//!
//! Stores custom element definitions. Elements awaiting upgrade are
//! not tracked here — "awaiting upgrade under name N" is already
//! materialized as the per-entity `CustomElementState` component
//! (`state: Undefined`), so `define()`-time candidate discovery is a
//! world query ([`collect_upgrade_candidates`]), not a side-store.

use std::collections::{HashMap, HashSet};

use elidex_ecs::{EcsDom, Entity, TagType};

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
    /// Upgrade-candidate discovery is the caller's job via
    /// [`collect_upgrade_candidates`] (the per-entity
    /// `CustomElementState` component is the single source of truth
    /// for "awaiting upgrade").
    pub fn define(&mut self, definition: CustomElementDefinition) -> Result<(), DefineError> {
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
        Ok(())
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

    /// Drop every definition. Called from `Vm::teardown_document` at
    /// document destruction to release the document-lifetime registry
    /// (`CustomElementDefinition::constructor_id` indexes into per-VM
    /// `HostData::ce_constructors`, cleared in lockstep). The registry
    /// SURVIVES a per-turn `Vm::unbind`
    /// (`#11-per-batch-unbind-document-lifetime-state`).
    pub fn clear(&mut self) {
        self.definitions.clear();
        self.constructor_id_to_name.clear();
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

/// The `customElements.define()`-time *upgrade candidates* of `name`
/// (WHATWG HTML §4.13.4 define() step 18 → "upgrade particular
/// elements within a document" steps 1–2): the shadow-including
/// descendants of `document`, **in shadow-including tree order**,
/// whose `CustomElementState` is `Undefined` for `name` (the
/// per-entity component is the single "awaiting upgrade" record) and
/// whose local name matches the registered definition
/// ([`CustomElementDefinition::upgrade_matches_local_name`] — that AO
/// builds the local-name/is-value match into candidate collection,
/// and the is value is carried by `definition_name` for customized
/// built-ins). Returns an empty Vec when `name` has no definition.
///
/// Document-scoped + tree-ordered to match the spec AO exactly: a
/// whole-world query would (a) upgrade *detached* fragment/template
/// elements at `define()` time — which the spec defers to their later
/// insertion — and (b) enqueue connected candidates in ECS iteration
/// order rather than the JS-observable tree order. Detached elements
/// awaiting upgrade are caught by the insertion-time "try to upgrade"
/// path (`CustomElementReactionConsumer::handle_insert` reactions),
/// not here.
///
/// Both engines consume this one function, so the candidate rule
/// cannot drift between them; the executor-side [`prepare_upgrade`]
/// gate re-checks at flush time (state may legitimately change
/// between enqueue and flush via other reaction sources).
///
/// `define()` rejects invalid names, so this is never queried with a
/// name that cannot acquire a definition — `Undefined` components
/// carrying an invalid `is`-derived name (DOM §4.9 step 6.3 marking
/// is validity-free) are simply never matched.
///
/// [`prepare_upgrade`]: crate::prepare_upgrade
#[must_use]
pub fn collect_upgrade_candidates(
    dom: &EcsDom,
    document: Entity,
    registry: &CustomElementRegistry,
    name: &str,
) -> Vec<Entity> {
    let Some(def) = registry.get(name) else {
        return Vec::new();
    };
    let world = dom.world();
    let mut out = Vec::new();
    dom.for_each_shadow_inclusive_descendant(document, &mut |entity| {
        let Ok(state) = world.get::<&CustomElementState>(entity) else {
            return;
        };
        // Null-registry elements are outside every registry — the
        // define()-time walk must not upgrade them (DOM §4.9:
        // definition lookup in a null registry is always null).
        if matches!(state.state, CEState::Undefined)
            && matches!(state.registry, crate::RegistryAssociation::Document)
            && state.definition_name == name
            && world
                .get::<&TagType>(entity)
                .is_ok_and(|tag| def.upgrade_matches_local_name(&tag.0))
        {
            out.push(entity);
        }
    });
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
