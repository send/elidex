//! Custom Elements v1 (WHATWG HTML §4.13).
//!
//! Provides the `CustomElementRegistry` for defining custom elements,
//! the `CustomElementReaction` queue for lifecycle callback batching,
//! and `CustomElementState` for tracking per-element upgrade status.

mod reaction;
mod registry;
mod state;
mod validation;

pub use reaction::CustomElementReaction;
pub use registry::{CustomElementDefinition, CustomElementRegistry};
pub use state::{CEState, CustomElementState};
pub use validation::is_valid_custom_element_name;

#[cfg(test)]
mod tests;
