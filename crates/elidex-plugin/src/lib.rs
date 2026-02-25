//! Plugin system traits and registry for elidex.
//!
//! This crate defines the core plugin traits, spec-level enums, and the
//! generic `PluginRegistry` used throughout the elidex browser engine.

use std::fmt;

mod registry;
mod spec_level;
mod traits;

pub use registry::PluginRegistry;
pub use spec_level::{CssSpecLevel, DomSpecLevel, EsSpecLevel, HtmlSpecLevel, WebApiSpecLevel};
pub use traits::{CssPropertyHandler, HtmlElementHandler, LayoutModel, NetworkMiddleware};

// ---------------------------------------------------------------------------
// Placeholder types used by plugin traits.
// These will be replaced with concrete types in Phase 1.
// ---------------------------------------------------------------------------

/// Define a unit error type with `Display` and `Error` impls.
macro_rules! define_error_type {
    ($(#[$meta:meta])* $name:ident, $msg:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
        pub struct $name;

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str($msg)
            }
        }

        impl std::error::Error for $name {}
    };
}

define_error_type!(
    /// Error returned when CSS value parsing fails.
    ///
    /// Phase 1 will include source span, property name, and the invalid input.
    ParseError,
    "CSS parse error"
);

define_error_type!(
    /// Error returned during HTML element creation or insertion.
    ///
    /// Phase 1 will include the tag name, error kind, and source location.
    HtmlParseError,
    "HTML parse error"
);

define_error_type!(
    /// Error returned by network middleware when a request or response is rejected.
    NetworkError,
    "Network error"
);

/// A parsed CSS value (e.g. `Length(10, Px)`, `Color(#fff)`).
///
/// Phase 1 will expand this into a rich enum covering all CSS value types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CssValue;

/// Context available during CSS value resolution (inherited values,
/// viewport size, font metrics, etc.).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct StyleContext;

/// The fully-resolved value of a CSS property after cascade and inheritance.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ComputedValue;

/// An opaque handle to a DOM node used by element handlers.
///
/// Phase 1 will replace this with a reference into the ECS world.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NodeHandle;

/// Context available to a layout algorithm (containing block, viewport, etc.).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LayoutContext;

/// A box in the layout tree that a [`LayoutModel`] operates on.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LayoutBox;

/// The output of a layout pass (position, dimensions, overflow, etc.).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LayoutResult;

/// An outgoing HTTP request that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HttpRequest;

/// An incoming HTTP response that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HttpResponse;
