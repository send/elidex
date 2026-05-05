//! ECS-based DOM storage for elidex (Ch.12).
//!
//! Uses `hecs` to store DOM nodes as entities with component data,
//! providing a cache-friendly, archetype-based representation.

mod components;
mod dom;

pub use components::{
    AnonymousTableMarker, AssociatedDocument, AttrData, AttrEntityCache, Attributes,
    BackgroundImages, CommentData, DocTypeData, ElementState, IframeData, ImageData, InlineStyle,
    LoadingAttribute, NodeKind, PseudoElementMarker, ScrollState, ShadowHost, ShadowRoot,
    ShadowRootMode, SlotAssignment, SlotAssignmentMode, SlottedMarker, TagType, TemplateContent,
    TextContent,
};
pub use dom::equality::{
    DOCUMENT_POSITION_CONTAINED_BY, DOCUMENT_POSITION_CONTAINS, DOCUMENT_POSITION_DISCONNECTED,
    DOCUMENT_POSITION_FOLLOWING, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
    DOCUMENT_POSITION_PRECEDING,
};
pub use dom::{EcsDom, MAX_ANCESTOR_DEPTH};

// Re-export hecs Entity for downstream consumers.
pub use hecs::Entity;
