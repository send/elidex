//! ECS-based DOM storage for elidex (Ch.12).
//!
//! Uses `hecs` to store DOM nodes as entities with component data,
//! providing a cache-friendly, archetype-based representation.

mod components;
mod dom;
mod fragment_tree;

/// The placeholder `about:blank` base URL used while
/// `#11-document-url-real-navigation` slot is pending (no real
/// `DocumentUrl` ECS state yet).  Pre-parsed once on first call.
///
/// Consumers: [`EcsDom::create_document_root`] initial
/// [`DocumentBaseUrl`] + `elidex_dom_api::BaseUrlMaintainer`
/// `compute_frozen_url` fallback.
#[must_use]
pub fn about_blank_url() -> url::Url {
    static URL: std::sync::OnceLock<url::Url> = std::sync::OnceLock::new();
    URL.get_or_init(|| url::Url::parse("about:blank").expect("about:blank parses"))
        .clone()
}

pub use components::{
    AnonymousTableMarker, AssociatedDocument, AttrData, AttrEntityCache, Attributes,
    BackgroundImages, BaseFrozenUrl, CommentData, DialogReturnValue, DocTypeData, DocumentBaseUrl,
    ElementState, IframeData, ImageData, InlineFlow, InlineFlowLine, InlineFlowRun, InlineFragment,
    InlineStyle, IsModalDialog, LinkStylesheet, ListItemMarker, LoadingAttribute, Namespace,
    NodeKind, OutputDefaultValue, OutputValueOverride, PseudoElementMarker, ScrollState,
    ShadowHost, ShadowRoot, ShadowRootMode, SlotAssignment, SlotAssignmentMode, SlottedMarker,
    TagType, TemplateContent, TextContent,
};
pub use dom::equality::{
    DOCUMENT_POSITION_CONTAINED_BY, DOCUMENT_POSITION_CONTAINS, DOCUMENT_POSITION_DISCONNECTED,
    DOCUMENT_POSITION_FOLLOWING, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
    DOCUMENT_POSITION_PRECEDING,
};
pub use dom::shadow::{ShadowAttachError, ShadowInit, SlotAssignError};
pub use dom::{EcsDom, MutationDispatcher, MutationEvent, MAX_ANCESTOR_DEPTH};
pub use fragment_tree::{BoxFragment, FragmentContent, FragmentId, FragmentNode, FragmentTree};

// Re-export hecs Entity for downstream consumers.
pub use hecs::Entity;
