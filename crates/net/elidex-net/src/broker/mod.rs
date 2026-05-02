//! Network Process broker (design doc §5.2, §5.3.3).
//!
//! Implements the Network Process as a singleton coordination thread that owns
//! the shared [`crate::NetClient`], cookie jar, and all WebSocket/SSE I/O loops.
//! Each HTTP fetch is executed on its own OS thread with a per-request tokio
//! runtime (see `dispatch::handle_fetch`).
//!
//! Content threads (Renderers) communicate exclusively through typed channels:
//! - [`RendererToNetwork`]: requests from content thread → Network Process
//! - [`NetworkToRenderer`]: responses/events from Network Process → content thread
//!
//! The broker is spawned once by the browser thread via [`spawn_network_process`].
//! Each content thread receives a [`NetworkHandle`] for IPC. All network access
//! is mediated through the broker — content threads never touch network APIs
//! directly, enabling OS-level sandbox enforcement (seccomp-bpf, etc.).
//!
//! # Cookie sharing
//!
//! The broker owns a single [`crate::NetClient`] (with shared `CookieJar`),
//! fixing the previous design where each content thread had its own
//! `FetchHandle` with an isolated cookie jar (spec violation — cookies must
//! be shared across browsing contexts within a profile).
//!
//! # Module layout
//!
//! Internal submodules (private; types are re-exported here):
//!
//! - `handle` — [`NetworkHandle`] + [`NetworkProcessHandle`] structs and
//!   their lifecycle methods, plus the [`spawn_network_process`] entry point.
//! - `dispatch` — broker thread main loop + per-renderer state machine
//!   (`NetworkProcessState`, `network_process_main`, `handle_fetch` worker
//!   spawn, WS/SSE forwarding).
//! - `cancel` — per-fetch cancellation token map (`CancelMap`) and the
//!   panic-safe RAII guards that keep it bounded.
//! - `buffered` — [`NetworkHandle::drain_events`] /
//!   [`NetworkHandle::drain_fetch_responses_only`] /
//!   [`NetworkHandle::rebuffer_events`] partial-drain helpers.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::sse::SseEvent;
use crate::ws::{WsCommand, WsEvent};
use crate::{Request, Response};

mod buffered;
mod cancel;
mod dispatch;
mod handle;

pub use handle::{spawn_network_process, NetworkHandle, NetworkProcessHandle};

// ---------------------------------------------------------------------------
// ID types
// ---------------------------------------------------------------------------

/// Unique fetch request identifier (globally monotonic).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FetchId(pub u64);

/// Monotonic counter for renderer client IDs.
pub(super) static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Monotonic counter for fetch request IDs.
static FETCH_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl FetchId {
    /// Generate a new unique fetch ID.
    #[must_use]
    pub fn next() -> Self {
        Self(FETCH_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

// ---------------------------------------------------------------------------
// Message types (design doc §5.3.3)
// ---------------------------------------------------------------------------

/// Messages from a Renderer (content thread) to the Network Process.
#[derive(Debug)]
pub enum RendererToNetwork {
    /// HTTP fetch request.
    Fetch(FetchId, Request),
    /// Cancel a pending fetch.
    CancelFetch(FetchId),
    /// Open a WebSocket connection.
    WebSocketOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// WebSocket URL (ws:// or wss://).
        url: url::Url,
        /// Requested sub-protocols.
        protocols: Vec<String>,
        /// Document origin for the `Origin` header.
        origin: String,
    },
    /// Send a WebSocket command (text/binary/close).
    WebSocketSend(u64, WsCommand),
    /// Close a WebSocket connection.
    WebSocketClose(u64),
    /// Open a Server-Sent Events connection.
    EventSourceOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// HTTP(S) URL for the event stream.
        url: url::Url,
        /// Last event ID for reconnection.
        last_event_id: Option<String>,
        /// Document origin for CORS (None = same-origin).
        origin: Option<String>,
        /// Whether to send credentials (cookies) cross-origin.
        with_credentials: bool,
    },
    /// Close an SSE connection (stop auto-reconnect).
    EventSourceClose(u64),
    /// Shutdown all connections for this renderer.
    Shutdown,
}

/// Messages from the Network Process to a Renderer (content thread).
#[derive(Debug)]
pub enum NetworkToRenderer {
    /// HTTP fetch response.
    FetchResponse(FetchId, Result<Response, String>),
    /// WebSocket event.
    WebSocketEvent(u64, WsEvent),
    /// SSE event.
    EventSourceEvent(u64, SseEvent),
}

/// Control messages from the Browser thread to the Network Process.
#[derive(Debug)]
pub enum NetworkProcessControl {
    /// Register a new renderer (content thread).
    RegisterRenderer {
        /// Unique client identifier.
        client_id: u64,
        /// Channel to send responses/events to this renderer.
        response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
    },
    /// Unregister a renderer (content thread shutting down).
    UnregisterRenderer {
        /// Client ID to remove.
        client_id: u64,
    },
    /// Shutdown the Network Process.
    Shutdown,
}

#[cfg(test)]
mod tests;
