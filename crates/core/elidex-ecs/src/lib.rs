//! ECS-based DOM storage for elidex (Ch.12).
//!
//! Uses `hecs` to store DOM nodes as entities with component data,
//! providing a cache-friendly, archetype-based representation.

mod components;
mod dom;

pub use components::{
    AttrData, Attributes, BackgroundImages, CommentData, DocTypeData, ElementState, ImageData,
    InlineStyle, NodeKind, PseudoElementMarker, ShadowHost, ShadowRoot, ShadowRootMode,
    SlotAssignment, SlotAssignmentMode, SlottedMarker, TagType, TemplateContent, TextContent,
};
pub use dom::{EcsDom, MAX_ANCESTOR_DEPTH};

// Re-export hecs Entity for downstream consumers.
pub use hecs::Entity;
