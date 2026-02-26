//! Core plugin traits (Ch.7 §7.2).
//!
//! These traits define the extension points for the elidex browser engine.
//! All traits require `Send + Sync` for safe use in async and multi-threaded
//! contexts.
//!
//! **Note:** [`EsSpecLevel`](crate::EsSpecLevel) is defined in `spec_level`
//! but has no corresponding trait yet. An `EsBindingHandler` trait for
//! JavaScript API bindings will be added in Phase 1 when the JS engine
//! integration is designed.

use crate::{
    ComputedValue, CssSpecLevel, CssValue, HtmlParseError, HtmlSpecLevel, HttpRequest,
    HttpResponse, LayoutBox, LayoutContext, LayoutResult, NetworkError, NodeHandle, ParseError,
    StyleContext, WebApiSpecLevel,
};

/// Handler for CSS property parsing and computation.
pub trait CssPropertyHandler: Send + Sync {
    /// Returns the CSS property name this handler is responsible for.
    fn property_name(&self) -> &str;

    /// Returns the specification level of this CSS property.
    fn spec_level(&self) -> CssSpecLevel {
        CssSpecLevel::Standard
    }

    /// Parse a raw CSS value string into a `CssValue`.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the value is not valid for this property.
    fn parse(&self, value: &str) -> Result<CssValue, ParseError>;

    /// Resolve the final computed value given a style context.
    ///
    /// Applies cascade rules (inherited values, viewport-relative units, etc.)
    /// from `context` to produce the value actually used during layout.
    fn resolve(&self, value: &CssValue, context: &StyleContext) -> ComputedValue;
}

/// Handler for HTML element parsing and behavior.
pub trait HtmlElementHandler: Send + Sync {
    /// Returns the HTML tag name this handler is responsible for.
    fn tag_name(&self) -> &str;

    /// Returns the specification level of this HTML element.
    fn spec_level(&self) -> HtmlSpecLevel {
        HtmlSpecLevel::Html5
    }

    /// Called when the element is created during parsing.
    ///
    /// # Errors
    ///
    /// Returns `HtmlParseError` if element initialization fails.
    fn on_create(&self, node: &NodeHandle) -> Result<(), HtmlParseError>;

    /// Called when the element is inserted into the DOM tree.
    ///
    /// # Errors
    ///
    /// Returns `HtmlParseError` if insertion-time validation fails.
    fn on_insert(&self, node: &NodeHandle) -> Result<(), HtmlParseError>;
}

/// Layout algorithm for a specific display type.
pub trait LayoutModel: Send + Sync {
    /// Returns the layout model name (e.g., "block", "flex", "grid").
    fn name(&self) -> &str;

    /// Returns the specification level of this display/layout model.
    fn spec_level(&self) -> CssSpecLevel {
        CssSpecLevel::Standard
    }

    /// Perform layout on the given box within the provided context.
    ///
    /// Computes the position and dimensions of `layout_box` and its children
    /// using the containing block and viewport information from `context`.
    fn layout(&self, layout_box: &LayoutBox, context: &LayoutContext) -> LayoutResult;
}

/// Middleware for intercepting and modifying network requests/responses.
pub trait NetworkMiddleware: Send + Sync {
    /// Returns the middleware name.
    fn name(&self) -> &str;

    /// Returns the specification level of this network middleware.
    fn spec_level(&self) -> WebApiSpecLevel {
        WebApiSpecLevel::Modern
    }

    /// Called before a request is sent. Can modify the request in-place.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError` if the request should be rejected.
    fn on_request(&self, request: &mut HttpRequest) -> Result<(), NetworkError>;

    /// Called after a response is received. Can modify the response in-place.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError` if the response should be rejected.
    fn on_response(&self, response: &mut HttpResponse) -> Result<(), NetworkError>;
}
