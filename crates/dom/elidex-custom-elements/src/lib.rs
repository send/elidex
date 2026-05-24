//! Custom Elements v1 (WHATWG HTML §4.13).
//!
//! Provides the `CustomElementRegistry` for defining custom elements,
//! the `CustomElementReaction` queue for lifecycle callback batching,
//! and `CustomElementState` for tracking per-element upgrade status.

mod consumer;
mod reaction;
mod registry;
mod state;
mod upgrade;
mod validation;

pub use consumer::CustomElementReactionConsumer;
pub use reaction::{scrub_entity_reactions, CustomElementReaction};
pub use registry::{
    collect_undefined_entities, CustomElementDefinition, CustomElementRegistry, DefineError,
};
pub use state::{CEState, CustomElementState};
pub use upgrade::{
    enter_constructor, finalize_failure, finalize_success, prepare_upgrade, UpgradeResolution,
};
pub use validation::is_valid_custom_element_name;

#[cfg(test)]
mod tests;
