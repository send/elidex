//! Custom element upgrade algorithm (WHATWG HTML §4.13.5 "upgrade an
//! element") — engine-indep state-machine half.
//!
//! The constructor invocation itself is engine-bound (it runs user
//! JS), so this module ships the surrounding state transitions +
//! reaction-queue manipulation + connectedness-driven Connected
//! enqueue. The VM-side `invoke_upgrade` (in
//! `elidex-js/src/vm/host/custom_elements/upgrade.rs`) is now a thin
//! shim that:
//!
//! 1. Calls [`prepare_upgrade`] to early-return for already-Custom /
//!    Failed entities or unregistered definitions, and to retrieve
//!    the `constructor_id` + `observed_attributes`.
//! 2. Performs the engine-bound steps (constructor.prototype Object
//!    validation, element-wrapper allocation, `ctx.vm.call`).
//! 3. Brackets the constructor call with [`enter_constructor`] +
//!    [`finalize_success`] / [`finalize_failure`] for the state
//!    transitions and the post-upgrade reaction enqueue.

use std::collections::VecDeque;

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

use crate::reaction::{scrub_entity_reactions, CustomElementReaction};
use crate::registry::CustomElementRegistry;
use crate::state::{CEState, CustomElementState};

/// Outcome of [`prepare_upgrade`] — whether the caller should run the
/// engine-bound constructor invocation or short-circuit.
pub enum UpgradeResolution {
    /// Entity is not eligible — already Custom / Failed,
    /// has no `CustomElementState`, or its definition is not
    /// registered. Caller should return without invoking the
    /// constructor.
    Skip,
    /// Engine-bound caller should look up the constructor via
    /// `constructor_id` and run the upgrade algorithm; pass
    /// `observed_attributes` straight through to
    /// [`finalize_success`].
    Proceed {
        constructor_id: u64,
        observed_attributes: Vec<String>,
    },
}

/// Phase 1 of HTML §4.13.5 "upgrade an element": resolve the
/// registered definition for `entity` and short-circuit when the
/// entity is already in a terminal state.
///
/// Pure read — does not mutate the world or the registry. The
/// returned `Vec<String>` clones the registry's observed-attribute
/// list because the registry mutex is released as soon as this
/// function returns.
#[must_use]
pub fn prepare_upgrade(
    dom: &EcsDom,
    registry: &CustomElementRegistry,
    entity: Entity,
) -> UpgradeResolution {
    let (definition_name, current_state, registry_assoc) =
        match dom.world().get::<&CustomElementState>(entity) {
            Ok(state) => (state.definition_name.clone(), state.state, state.registry),
            Err(_) => return UpgradeResolution::Skip,
        };
    if matches!(current_state, CEState::Custom | CEState::Failed) {
        return UpgradeResolution::Skip;
    }
    // A null-registry element is outside every registry — DOM §4.9:
    // the definition lookup against its (null) registry is always
    // null, so NO upgrade path (creation, define()-drain, insertion,
    // customElements.upgrade(), clone) may resolve a definition for
    // it. Gated here defensively so every caller of the upgrade
    // machinery inherits the rule.
    if matches!(registry_assoc, crate::RegistryAssociation::Null) {
        return UpgradeResolution::Skip;
    }
    let Some(def) = registry.get(&definition_name) else {
        return UpgradeResolution::Skip;
    };
    // Reject a mismatched `is=` candidate the parser legitimately marked
    // (e.g. `<div is="plastic-button">` under `{ extends: "button" }`, or a
    // `<div is="my-el">` under an autonomous `define("my-el", …)`). The
    // local-name match rule lives on the definition
    // (`CustomElementDefinition::upgrade_matches_local_name`) so this VM gate
    // and the boa shell's upgrade walks share one implementation. The parser
    // cannot pre-filter it: `extends` is unknown until `define()` runs.
    let local_name = dom
        .world()
        .get::<&TagType>(entity)
        .map(|t| t.0.clone())
        .unwrap_or_default();
    if !def.upgrade_matches_local_name(&local_name) {
        return UpgradeResolution::Skip;
    }
    UpgradeResolution::Proceed {
        constructor_id: def.constructor_id,
        observed_attributes: def.observed_attributes().to_vec(),
    }
}

/// DOM §4.4 "clone a single node" routes every copy through *create
/// an element* with `synchronousCustomElements = false` — apply the
/// creation-time custom-element semantics that depend on the registry
/// to a FRESH clone subtree (Codex PR331 R14):
///
/// - §4.9 step 5.2 (async **autonomous** definition-found branch):
///   the copy is created with a **null is value** — so a source that
///   legitimately retained a creation-time `is` (created before
///   `define()`) must NOT leak it onto a clone made after the
///   definition registered. The customized-built-in branch (step 4.2)
///   passes `is` through and keeps it.
/// - step 5.2.2 / 4.4: enqueue a custom element upgrade reaction for
///   every copy whose definition lookup is non-null — returned to the
///   caller, which owns the engine-side reaction queue.
///
/// Candidacy is resolved via [`prepare_upgrade`] (registry-association
/// gate + `upgrade_matches_local_name` included), so null-registry
/// clones and `is` mismatches are skipped by the same rule every other
/// upgrade path uses.
///
/// MUST only be called on a freshly cloned subtree: every visited
/// element is a just-created copy, which is what makes the is-value
/// clear safe (a define()-time walk over pre-existing elements must
/// NOT clear — those legitimately retain their creation-time `is`).
pub fn apply_clone_creation_ce_semantics(
    dom: &mut EcsDom,
    registry: &CustomElementRegistry,
    clone_root: Entity,
) -> Vec<Entity> {
    let mut candidates: Vec<Entity> = Vec::new();
    dom.for_each_shadow_inclusive_descendant(clone_root, &mut |e| {
        if matches!(
            prepare_upgrade(dom, registry, e),
            UpgradeResolution::Proceed { .. }
        ) {
            candidates.push(e);
        }
    });
    for &entity in &candidates {
        let autonomous = {
            let Ok(state) = dom.world().get::<&CustomElementState>(entity) else {
                continue;
            };
            let Ok(tag) = dom.world().get::<&TagType>(entity) else {
                continue;
            };
            crate::sync_autonomous_definition_matches(registry, &state.definition_name, &tag.0)
        };
        if autonomous {
            if let Ok(mut state) = dom.world_mut().get::<&mut CustomElementState>(entity) {
                state.is_value = None;
            }
        }
    }
    candidates
}

/// Transition `entity` to [`CEState::Precustomized`] before the
/// engine-bound constructor invocation. Spec §4.13.5 step 4.
pub fn enter_constructor(dom: &mut EcsDom, entity: Entity) {
    set_state(dom, entity, CEState::Precustomized);
}

/// Post-constructor success path (spec steps 5-9): transition to
/// [`CEState::Custom`], enqueue `attributeChangedCallback` reactions
/// for each already-present attribute in `observed_attributes`, and
/// enqueue `connectedCallback` when the element is connected.
pub fn finalize_success(
    dom: &mut EcsDom,
    queue: &mut VecDeque<CustomElementReaction>,
    entity: Entity,
    observed_attributes: &[String],
) {
    set_state(dom, entity, CEState::Custom);
    if !observed_attributes.is_empty() {
        // O(attrs) membership via HashSet — observed_attributes is
        // bounded by MAX_OBSERVED_ATTRIBUTES=1000 and typical N<20
        // so a per-upgrade HashSet alloc is cheaper than the O(attrs
        // × observed) nested linear search.
        let observed_set: std::collections::HashSet<&str> =
            observed_attributes.iter().map(String::as_str).collect();
        let to_enqueue: Vec<(String, String)> = match dom.world().get::<&Attributes>(entity) {
            Ok(attrs) => attrs
                .iter()
                .filter(|(name, _)| observed_set.contains(*name))
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            Err(_) => Vec::new(),
        };
        for (name, value) in to_enqueue {
            queue.push_back(CustomElementReaction::AttributeChanged {
                entity,
                name,
                old_value: None,
                new_value: Some(value),
            });
        }
    }
    if dom.is_connected(entity) {
        queue.push_back(CustomElementReaction::Connected(entity));
    }
}

/// Post-constructor failure path (spec step 8): transition to
/// [`CEState::Failed`] and drop every queued reaction targeting
/// `entity` so re-enqueued Upgrade attempts short-circuit via
/// `prepare_upgrade`'s early-return.
pub fn finalize_failure(
    dom: &mut EcsDom,
    queue: &mut VecDeque<CustomElementReaction>,
    entity: Entity,
) {
    set_state(dom, entity, CEState::Failed);
    scrub_entity_reactions(queue, entity);
}

fn set_state(dom: &mut EcsDom, entity: Entity, new_state: CEState) {
    if let Ok(mut state) = dom.world_mut().get::<&mut CustomElementState>(entity) {
        state.state = new_state;
    }
}

/// DOM §4.9 "create an element" step 5.1.3.10: the **synchronous**
/// autonomous-definition branch (a matching autonomous definition
/// already registered when `createElement` runs) sets the result's
/// *is value* to **null** — unlike the async branch (definition
/// registered later), where the creation-time is value is retained
/// through the upgrade. Engine hosts call this on the
/// defined-at-creation route, immediately before invoking/enqueuing
/// the upgrade; the upgrade machinery itself must NOT clear the slot
/// (that would also strip async-created elements, diverging from
/// spec).
///
/// The clear is gated on the registered definition actually MATCHING
/// as autonomous for this element (HTML §4.13.3 "look up a custom
/// element definition" requires the definition's local name to equal
/// the element's): a customized-built-in definition that merely
/// shares the name (e.g. `define('plastic-button', C, {extends:
/// 'button'})` vs `createElement('plastic-button', {is})`) does NOT
/// match, so create-an-element takes the no-definition branch and the
/// is value is retained. No-op for customized built-ins (step 4
/// passes the is value through untouched) and for entities without a
/// `CustomElementState`.
pub fn clear_is_value_for_sync_autonomous(
    registry: &crate::CustomElementRegistry,
    dom: &mut elidex_ecs::EcsDom,
    entity: hecs::Entity,
) {
    let local_name = match dom.world().get::<&elidex_ecs::TagType>(entity) {
        Ok(tag) => tag.0.clone(),
        Err(_) => return,
    };
    if let Ok(mut state) = dom
        .world_mut()
        .get::<&mut crate::CustomElementState>(entity)
    {
        if sync_autonomous_definition_matches(registry, &state.definition_name, &local_name) {
            state.is_value = None;
        }
    }
}

/// The decision half of [`clear_is_value_for_sync_autonomous`] (single
/// home — hosts that cannot hold the registry guard and a mutable DOM
/// borrow simultaneously phase the read/decide/clear steps but must
/// route the decision through here): true iff the element is an
/// autonomous candidate (definition keyed on the tag) AND the
/// registered definition matches this local name as autonomous per
/// HTML §4.13.3 lookup semantics.
#[must_use]
pub fn sync_autonomous_definition_matches(
    registry: &crate::CustomElementRegistry,
    definition_name: &str,
    local_name: &str,
) -> bool {
    definition_name == local_name
        && registry
            .get(definition_name)
            .is_some_and(|def| def.upgrade_matches_local_name(local_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CustomElementDefinition;
    use elidex_ecs::Attributes;

    fn proceeds(r: &UpgradeResolution) -> bool {
        matches!(r, UpgradeResolution::Proceed { .. })
    }

    fn mark(dom: &mut EcsDom, entity: Entity, definition_name: &str) {
        let _ = dom
            .world_mut()
            .insert_one(entity, CustomElementState::undefined(definition_name));
    }

    #[test]
    fn customized_builtin_upgrades_only_matching_base_local_name() {
        // Codex #329 R5 (P2): the parser marks any `is=`-bearing element as a
        // candidate, but a customized built-in upgrades only when its local
        // name matches the definition's `extends` base (HTML §4.13.1.4).
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let button = dom.create_element("button", Attributes::default());
        mark(&mut dom, div, "plastic-button");
        mark(&mut dom, button, "plastic-button");

        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "plastic-button".to_string(),
                1,
                vec![],
                Some("button".to_string()),
            ))
            .expect("define plastic-button");

        assert!(
            !proceeds(&prepare_upgrade(&dom, &registry, div)),
            "<div is=plastic-button> must NOT upgrade (base is <button>)"
        );
        assert!(
            proceeds(&prepare_upgrade(&dom, &registry, button)),
            "<button is=plastic-button> must upgrade"
        );
    }

    #[test]
    fn autonomous_upgrades_only_matching_tag() {
        let mut dom = EcsDom::new();
        let widget = dom.create_element("my-widget", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        mark(&mut dom, widget, "my-widget");
        mark(&mut dom, div, "my-widget");

        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "my-widget".to_string(),
                2,
                vec![],
                None,
            ))
            .expect("define my-widget");

        assert!(
            proceeds(&prepare_upgrade(&dom, &registry, widget)),
            "<my-widget> must upgrade"
        );
        assert!(
            !proceeds(&prepare_upgrade(&dom, &registry, div)),
            "<div is=my-widget> must NOT upgrade an autonomous definition"
        );
    }
}

#[cfg(test)]
mod clone_creation_pass_tests {
    use super::*;
    use crate::registry::CustomElementDefinition;

    fn marked(dom: &mut EcsDom, tag: &str, is: Option<&str>) -> Entity {
        let el = dom.create_element(tag, Attributes::default());
        dom.world_mut()
            .insert_one(
                el,
                CustomElementState::for_created_element(tag, is, elidex_ecs::Namespace::Html)
                    .unwrap(),
            )
            .unwrap();
        el
    }

    #[test]
    fn autonomous_defined_clone_clears_is_and_is_candidate() {
        // Codex PR331 R14 / DOM §4.9 step 5.2: the async autonomous
        // branch creates the copy with a NULL is value, and the copy
        // gets an upgrade reaction.
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "my-el".to_string(),
                1,
                vec![],
                None,
            ))
            .unwrap();
        let mut dom = EcsDom::new();
        let el = marked(&mut dom, "my-el", Some("other-el"));
        let candidates = apply_clone_creation_ce_semantics(&mut dom, &registry, el);
        assert_eq!(candidates, vec![el], "defined copy is an upgrade candidate");
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(
            state.is_value(),
            None,
            "async autonomous creation passes a null is value (§4.9 step 5.2)"
        );
    }

    #[test]
    fn customized_builtin_clone_keeps_is() {
        // The customized-built-in branch (§4.9 step 4.2) passes `is`
        // through — the clone keeps it.
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "my-btn".to_string(),
                1,
                vec![],
                Some("button".to_string()),
            ))
            .unwrap();
        let mut dom = EcsDom::new();
        let el = marked(&mut dom, "button", Some("my-btn"));
        let candidates = apply_clone_creation_ce_semantics(&mut dom, &registry, el);
        assert_eq!(candidates, vec![el]);
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(state.is_value(), Some("my-btn"), "built-in branch keeps is");
    }

    #[test]
    fn undefined_name_keeps_is_and_no_candidate() {
        let registry = CustomElementRegistry::new();
        let mut dom = EcsDom::new();
        let el = marked(&mut dom, "my-el", Some("other-el"));
        let candidates = apply_clone_creation_ce_semantics(&mut dom, &registry, el);
        assert!(candidates.is_empty(), "no definition → no reaction");
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(state.is_value(), Some("other-el"));
    }

    #[test]
    fn null_registry_clone_untouched() {
        // prepare_upgrade's registry gate excludes null-registry
        // copies — neither candidate nor is-clear.
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "my-el".to_string(),
                1,
                vec![],
                None,
            ))
            .unwrap();
        let mut dom = EcsDom::new();
        let el = marked(&mut dom, "my-el", Some("other-el"));
        dom.world_mut()
            .get::<&mut CustomElementState>(el)
            .unwrap()
            .registry = crate::RegistryAssociation::Null;
        let candidates = apply_clone_creation_ce_semantics(&mut dom, &registry, el);
        assert!(candidates.is_empty());
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(state.is_value(), Some("other-el"));
    }
}

#[cfg(test)]
mod sync_is_clear_tests {
    use super::*;
    use crate::registry::CustomElementDefinition;

    fn dom_with_marked_element(tag: &str, is: &str) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let el = dom.create_element(tag, Attributes::default());
        dom.world_mut()
            .insert_one(
                el,
                CustomElementState::for_created_element(tag, Some(is), elidex_ecs::Namespace::Html)
                    .unwrap(),
            )
            .unwrap();
        (dom, el)
    }

    #[test]
    fn clears_for_matching_autonomous_definition() {
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "my-el".to_string(),
                1,
                vec![],
                None,
            ))
            .unwrap();
        let (mut dom, el) = dom_with_marked_element("my-el", "other-el");
        clear_is_value_for_sync_autonomous(&registry, &mut dom, el);
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(state.is_value(), None, "DOM §4.9 step 5.1.3.10 clears");
    }

    #[test]
    fn name_sharing_builtin_definition_does_not_clear() {
        // Codex PR331 R6: a customized-built-in definition merely
        // sharing the name does NOT match the §4.13.3 lookup for this
        // local name — the element keeps its creation-time is value.
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new(
                "plastic-button".to_string(),
                1,
                vec![],
                Some("button".to_string()),
            ))
            .unwrap();
        let (mut dom, el) = dom_with_marked_element("plastic-button", "other-el");
        clear_is_value_for_sync_autonomous(&registry, &mut dom, el);
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(
            state.is_value(),
            Some("other-el"),
            "no matching definition for this local name — is value retained"
        );
    }

    #[test]
    fn unregistered_name_does_not_clear() {
        let registry = CustomElementRegistry::new();
        let (mut dom, el) = dom_with_marked_element("my-el", "other-el");
        clear_is_value_for_sync_autonomous(&registry, &mut dom, el);
        let state = dom.world().get::<&CustomElementState>(el).unwrap();
        assert_eq!(state.is_value(), Some("other-el"));
    }
}
