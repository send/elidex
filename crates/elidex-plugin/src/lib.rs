//! Plugin system traits and registry for elidex.
//!
//! This crate defines the core plugin traits, spec-level enums, and the
//! generic `PluginRegistry` used throughout the elidex browser engine.

mod computed_style;
mod context;
mod error;
mod layout_types;
mod registry;
mod spec_level;
mod traits;
mod values;

pub use computed_style::{BorderStyle, ComputedStyle, Dimension, Display, Position};
pub use context::StyleContext;
pub use error::{HtmlErrorKind, HtmlParseError, ParseError};
pub use layout_types::{EdgeSizes, LayoutBox, LayoutContext, LayoutResult, Rect, Size};
pub use registry::PluginRegistry;
pub use spec_level::{CssSpecLevel, DomSpecLevel, EsSpecLevel, HtmlSpecLevel, WebApiSpecLevel};
pub use traits::{CssPropertyHandler, HtmlElementHandler, LayoutModel, NetworkMiddleware};
pub use values::{ComputedValue, CssColor, CssValue, LengthUnit};

// ---------------------------------------------------------------------------
// Opaque DOM node handle
// ---------------------------------------------------------------------------

/// Opaque DOM node handle. Wraps a `u64` entity ID.
///
/// `elidex-parser` converts `hecs::Entity` to `NodeHandle` via [`from_bits`](NodeHandle::from_bits).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeHandle(u64);

impl NodeHandle {
    /// Create a `NodeHandle` from a raw `u64` entity ID.
    #[must_use]
    pub fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Extract the raw `u64` entity ID.
    #[must_use]
    pub fn to_bits(self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Placeholder types — will be replaced in Phase 2
// ---------------------------------------------------------------------------

/// An outgoing HTTP request that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HttpRequest;

/// An incoming HTTP response that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HttpResponse;

/// Error returned by network middleware when a request or response is rejected.
///
/// Phase 2 will expand this with status codes, URL, and error details.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NetworkError;

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Network error")
    }
}

impl std::error::Error for NetworkError {}
