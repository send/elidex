//! Plugin system traits and registry for elidex.
//!
//! This crate defines the core plugin traits, spec-level enums, and the
//! generic `PluginRegistry` used throughout the elidex browser engine.

pub mod background;
mod computed_style;
pub mod css_resolve;
mod error;
mod event_types;
pub mod handlers;
mod js_value;
mod layout_types;
mod logical;
mod registry;
pub mod sandbox;
mod spec_level;
mod traits;
pub mod url_security;
mod values;

pub use computed_style::{
    AlignContent, AlignItems, AlignSelf, AutoRepeatMode, BorderCollapse, BorderSide, BorderStyle,
    BoxSizing, CaptionSide, Clear, ComputedStyle, ContentItem, ContentValue, Dimension, Direction,
    Display, FlexDirection, FlexWrap, Float, FontStyle, GridAutoFlow, GridLine, GridTrackList,
    JustifyContent, LineHeight, ListStyleType, Overflow, Position, TableLayout, TextAlign,
    TextDecorationLine, TextDecorationStyle, TextOrientation, TextTransform, TrackBreadth,
    TrackSize, UnicodeBidi, VerticalAlign, Visibility, WhiteSpace, WritingMode,
};
pub use error::ParseError;
pub use event_types::{
    AnimationEventInit, ClipboardEventInit, CompositionEventInit, EventPayload, EventPhase,
    FocusEventInit, InputEventInit, KeyboardEventInit, MouseEventInit, TransitionEventInit,
};
pub use js_value::JsValue;
pub use layout_types::{EdgeSizes, LayoutBox, LayoutContext, LayoutResult, Rect, Size};
pub use logical::{LogicalEdges, LogicalRect, LogicalSize, WritingModeContext};
pub use registry::PluginRegistry;
pub use spec_level::{CssSpecLevel, DomSpecLevel, EsSpecLevel, HtmlSpecLevel, WebApiSpecLevel};
pub use traits::{
    AccessibilityRole, Attributes, Constraints, CssPropertyHandler, CssPropertyRegistry, CssRule,
    ElementData, HtmlElementHandler, LayoutModel, LayoutNode, NetworkMiddleware, ParseBehavior,
    PropertyDeclaration, ResolveContext,
};
pub use values::{
    AngleOrDirection, CalcExpr, CssColor, CssColorStop, CssValue, GradientValue, LengthUnit,
};

// ---------------------------------------------------------------------------
// Process model
// ---------------------------------------------------------------------------

/// Describes how renderer (content) threads are allocated.
///
/// Phase 3.5 implements `SingleProcess` only (all content in one thread).
/// Other variants are defined for future multi-process support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProcessModel {
    /// Each site (origin) gets its own renderer.
    SiteIsolation,
    /// Each tab gets its own renderer.
    PerTab,
    /// Renderers are shared across tabs up to a maximum count.
    Shared {
        /// Maximum number of concurrent renderer threads.
        max_renderers: usize,
    },
    /// Everything runs in a single process (current default).
    SingleProcess,
}

// ---------------------------------------------------------------------------
// Network types
// ---------------------------------------------------------------------------

/// An outgoing HTTP request that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpRequest {
    /// HTTP method (e.g. `"GET"`, `"POST"`).
    pub method: String,
    /// Request URL.
    pub url: String,
    /// Header name-value pairs.
    pub headers: Vec<(String, String)>,
}

/// An incoming HTTP response that [`NetworkMiddleware`] may inspect or modify.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Header name-value pairs.
    pub headers: Vec<(String, String)>,
}

/// Error returned by network middleware when a request or response is rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkError {
    /// The kind of network error.
    pub kind: NetworkErrorKind,
    /// A human-readable error message.
    pub message: String,
}

impl Default for NetworkError {
    fn default() -> Self {
        Self {
            kind: NetworkErrorKind::Other,
            message: String::new(),
        }
    }
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }
}

impl std::error::Error for NetworkError {}

/// The kind of network error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetworkErrorKind {
    /// The remote host refused the connection.
    ConnectionRefused,
    /// The operation timed out.
    Timeout,
    /// DNS resolution failed.
    DnsFailure,
    /// TLS handshake or certificate error.
    TlsFailure,
    /// The request was blocked by SSRF protection.
    ///
    /// Emitted for three scenarios:
    /// - Unsupported URL scheme (only `http`/`https` allowed)
    /// - Host resolves to a private/reserved IP address
    /// - URL has no host component
    SsrfBlocked,
    /// An unclassified network error.
    #[default]
    Other,
}

impl std::fmt::Display for NetworkErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionRefused => f.write_str("connection refused"),
            Self::Timeout => f.write_str("timeout"),
            Self::DnsFailure => f.write_str("DNS failure"),
            Self::TlsFailure => f.write_str("TLS failure"),
            Self::SsrfBlocked => f.write_str("SSRF blocked"),
            Self::Other => f.write_str("network error"),
        }
    }
}

// ---------------------------------------------------------------------------
// CSS parse helpers (shared by CSS property handler crates)
// ---------------------------------------------------------------------------

/// Register a CSS property handler for all its declared property names.
///
/// This is a shared helper to avoid duplicating the identical registration
/// pattern across CSS property handler crates. The handler is cloned once per
/// property name; all handlers are unit structs (zero-sized), so the clone is
/// free.
#[allow(clippy::needless_pass_by_value)] // handler.clone() requires ownership or &H; by-value is idiomatic here
pub fn register_css_handler<H: CssPropertyHandler + Clone + 'static>(
    registry: &mut CssPropertyRegistry,
    handler: H,
) {
    // Collect names first to release the borrow on `handler` before cloning into boxes.
    let names: Vec<String> = handler
        .property_names()
        .iter()
        .map(ToString::to_string)
        .collect();
    for name in names {
        registry.register_dynamic(name, Box::new(handler.clone()));
    }
}

/// Parse a CSS keyword from the input, accepting only values in `allowed`.
///
/// Returns `CssValue::Keyword(lowercase)` on success. This is a shared helper
/// to avoid duplicating the same pattern across CSS property plugin crates.
pub fn parse_css_keyword(
    input: &mut cssparser::Parser<'_, '_>,
    allowed: &[&str],
) -> Result<CssValue, ParseError> {
    let ident = input.expect_ident().map_err(|_| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected identifier".into(),
    })?;
    let lower = ident.to_ascii_lowercase();
    if allowed.contains(&lower.as_str()) {
        Ok(CssValue::Keyword(lower))
    } else {
        Err(ParseError {
            property: String::new(),
            input: lower,
            message: "unexpected keyword".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_request_default() {
        let req = HttpRequest::default();
        assert!(req.method.is_empty());
        assert!(req.url.is_empty());
        assert!(req.headers.is_empty());
    }

    #[test]
    fn http_request_fields() {
        let req = HttpRequest {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![("Accept".into(), "text/html".into())],
        };
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.headers.len(), 1);
    }

    #[test]
    fn network_error_display() {
        let err = NetworkError {
            kind: NetworkErrorKind::Timeout,
            message: "request timed out after 30s".into(),
        };
        let s = err.to_string();
        assert!(s.contains("timeout"));
        assert!(s.contains("request timed out after 30s"));
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }
}
