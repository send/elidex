//! Custom Elements v1 (WHATWG HTML §4.13).
//!
//! Provides the `CustomElementRegistry` for defining custom elements,
//! the `CustomElementReaction` queue for lifecycle callback batching,
//! and `CustomElementState` for tracking per-element upgrade status.

mod construction_stack;
mod consumer;
mod entity_spawn;
mod reaction;
mod registry;
mod state;
mod upgrade;
mod validation;

pub use construction_stack::ConstructionStackEntry;
pub use consumer::CustomElementReactionConsumer;
pub use entity_spawn::spawn_custom_element_entity;
pub use reaction::{scrub_entity_reactions, CustomElementReaction};
pub use registry::{
    collect_undefined_entities, CustomElementDefinition, CustomElementRegistry, DefineError,
};
pub use state::{CEState, CustomElementState, RegistryAssociation};
pub use upgrade::{
    apply_clone_creation_ce_semantics, clear_is_value_for_sync_autonomous, enter_constructor,
    finalize_failure, finalize_success, prepare_upgrade, sync_autonomous_definition_matches,
    UpgradeResolution,
};
pub use validation::is_valid_custom_element_name;

#[cfg(test)]
mod tests;
