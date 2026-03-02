//! Plugin system traits and registry for elidex.
//!
//! This crate defines the core plugin traits, spec-level enums, and the
//! generic `PluginRegistry` used throughout the elidex browser engine.

mod computed_style;
mod error;
mod event_types;
mod js_value;
mod layout_types;
mod registry;
mod spec_level;
mod traits;
pub mod url_security;
mod values;

pub use computed_style::{
    AlignContent, AlignItems, AlignSelf, BorderCollapse, BorderStyle, BoxSizing, CaptionSide,
    ComputedStyle, ContentItem, ContentValue, Dimension, Display, FlexDirection, FlexWrap,
    GridAutoFlow, GridLine, JustifyContent, LineHeight, ListStyleType, Overflow, Position,
    TableLayout, TextAlign, TextDecorationLine, TextTransform, TrackBreadth, TrackSize, WhiteSpace,
};
pub use error::ParseError;
pub use event_types::{EventPayload, EventPhase, KeyboardEventInit, MouseEventInit};
pub use js_value::JsValue;
pub use layout_types::{EdgeSizes, LayoutBox, LayoutContext, LayoutResult, Rect, Size};
pub use registry::PluginRegistry;
pub use spec_level::{CssSpecLevel, DomSpecLevel, EsSpecLevel, HtmlSpecLevel, WebApiSpecLevel};
pub use traits::NetworkMiddleware;
pub use values::{CssColor, CssValue, LengthUnit};

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
