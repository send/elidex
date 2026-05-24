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

use elidex_ecs::{Attributes, EcsDom, Entity};

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
    let (definition_name, current_state) = match dom.world().get::<&CustomElementState>(entity) {
        Ok(state) => (state.definition_name.clone(), state.state),
        Err(_) => return UpgradeResolution::Skip,
    };
    if matches!(current_state, CEState::Custom | CEState::Failed) {
        return UpgradeResolution::Skip;
    }
    let Some(def) = registry.get(&definition_name) else {
        return UpgradeResolution::Skip;
    };
    UpgradeResolution::Proceed {
        constructor_id: def.constructor_id,
        observed_attributes: def.observed_attributes.clone(),
    }
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
        let to_enqueue: Vec<(String, String)> = match dom.world().get::<&Attributes>(entity) {
            Ok(attrs) => attrs
                .iter()
                .filter(|(name, _)| observed_attributes.iter().any(|n| n == *name))
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
