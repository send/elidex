//! Engine-independent WebSocket and EventSource protocol types.
//!
//! Provides connection state, URL validation, and readyState constants
//! shared across script engine bindings. Does NOT depend on any JS engine.

mod event_source;
mod websocket;

pub use event_source::{SseReadyState, SSE_READYSTATE_CONSTANTS};
pub use websocket::{is_mixed_content, validate_ws_url, WsReadyState, WS_READYSTATE_CONSTANTS};
