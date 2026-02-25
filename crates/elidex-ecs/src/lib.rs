//! ECS-based DOM storage for elidex (Ch.12).
//!
//! Uses `hecs` to store DOM nodes as entities with component data,
//! providing a cache-friendly, archetype-based representation.

mod components;
mod dom;

pub use components::{Attributes, InlineStyle, TagType, TextContent, TreeRelation};
pub use dom::EcsDom;

// Re-export hecs Entity for downstream consumers.
pub use hecs::Entity;
