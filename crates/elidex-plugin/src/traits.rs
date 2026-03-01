//! Core plugin traits (Ch.7 §7.2).
//!
//! These traits define the extension points for the elidex browser engine.
//! All traits require `Send + Sync` for safe use in async and multi-threaded
//! contexts.

use crate::{HttpRequest, HttpResponse, NetworkError, WebApiSpecLevel};

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
