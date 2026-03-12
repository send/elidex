//! Accessibility tree builder for elidex.
//!
//! Converts an ECS DOM tree into an AccessKit accessibility tree
//! for screen reader and assistive technology support.

mod names;
mod roles;
mod tree;

pub use tree::{build_tree_update, entity_to_node_id};
