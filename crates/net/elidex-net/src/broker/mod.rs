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
//! - `register` — slot #10.6c `RegisterRenderer` ack handshake
//!   (`REGISTER_ACK_TIMEOUT` + `register_with_ack`).  Split out
//!   of `handle` once the helper + its unit tests pushed the
//!   parent file past the project's ~1000-line file-split
//!   convention (slot #10.5).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::sse::SseEvent;
use crate::ws::{WsCommand, WsEvent};
use crate::{Request, Response};

mod buffered;
mod cancel;
mod dispatch;
mod handle;
mod register;

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
    /// Internal back-edge: the broker has finished tearing down
    /// this renderer's per-client state and is about to drop its
    /// `clients` entry.  Never surfaced to JS / embedder code —
    /// [`NetworkHandle::drain_events`] /
    /// [`NetworkHandle::drain_fetch_responses_only`] consume it
    /// to flip the renderer-side `unregistered` flag and
    /// synthesise terminal `Err` replies for any race-window
    /// fetches that the broker dropped via its `handle_request`
    /// stale-cid gate.  See slot #10.6b
    /// (`m4-12-pr-broker-unregistered-handle-back-edge-plan.md`)
    /// for the layered defence: the back-edge closes the
    /// `synthesise_aborted_replies_for_client → cancel →
    /// clients.remove` race window where a fetch submitted
    /// between steps 1 and 4 had no terminal event delivered.
    RendererUnregistered,
}

/// Control messages from the Browser thread to the Network Process.
///
/// **Breaking change (slot #10.6c R10/R11)**: this enum was
/// `pub` in releases prior to slot #10.6c.  It has been
/// narrowed to `pub(crate)` because its variants now carry
/// implementation-detail payloads — the shared
/// [`std::sync::Arc<std::sync::atomic::AtomicBool>`] introduced
/// by slot #10.6c R9 for cross-handle unregister observability,
/// and the `crossbeam_channel::Sender<()>` ack introduced by
/// the slot #10.6c handshake.  Either field addition would
/// have been a breaking change for downstream code that
/// exhaustively matched the variant or constructed it
/// directly, and the broker module needs the freedom to evolve
/// these payloads without committing to a semver-stable
/// contract for an internal coordination protocol.  This is an
/// intentional break consistent with the project's
/// "後方互換性は維持しない" rule (`CLAUDE.md`).  The remaining
/// public surface of the broker module (`NetworkHandle`,
/// `NetworkProcessHandle`, `spawn_network_process`, and the
/// data message types `RendererToNetwork` /
/// `NetworkToRenderer` — the latter two ARE observed in
/// `elidex-js` / `elidex-js-boa` realtime bridges) stays `pub`
/// and unchanged.  No external embedder constructs or matches
/// this enum: control messages flow only via the
/// `pub(super)`-fielded `NetworkProcessHandle::control_tx` /
/// `NetworkHandle::control_tx` channels held inside the broker
/// module, so the visibility narrowing has zero blast radius
/// in the real workspace.
#[derive(Debug)]
pub(crate) enum NetworkProcessControl {
    /// Register a new renderer (content thread).
    RegisterRenderer {
        /// Unique client identifier.
        client_id: u64,
        /// Channel to send responses/events to this renderer.
        response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
        /// Slot #10.6c R9: shared with the renderer's
        /// [`NetworkHandle`] `unregistered` flag.  The
        /// broker stores this clone in its `clients` map and
        /// flips it to `true` (Release) BEFORE emitting the
        /// [`NetworkToRenderer::RendererUnregistered`] marker on
        /// `response_tx` from `emit_renderer_unregistered` —
        /// gives any concurrent
        /// `NetworkHandle::create_sibling_handle` against this
        /// cid an O(1) `Acquire` load fast-path for the common
        /// case (parent observably unregistered before sibling
        /// creation).  R13 strengthening: the load alone is a
        /// snapshot, not exclusion — see `parent_client_id`.
        unregistered: std::sync::Arc<std::sync::atomic::AtomicBool>,
        /// Slot #10.6c R13: when registering a **sibling** of
        /// an existing renderer (via
        /// [`NetworkHandle::create_sibling_handle`]), this
        /// carries the parent's `client_id` so the broker can
        /// check `parent_id ∈ clients` BEFORE inserting the
        /// sibling.  Closes the TOCTOU between
        /// `create_sibling_handle`'s atomic-load fast-path and
        /// the broker's `emit_renderer_unregistered` for the
        /// parent: with this field, the broker's FIFO drain of
        /// `control_rx` serialises the sibling's Register
        /// against any preceding `UnregisterRenderer` for the
        /// parent — if the parent's `clients.remove` has
        /// happened first (FIFO order in control_tx), the
        /// broker drops the sibling's `ack_tx` without sending
        /// and the renderer's `register_with_ack` recv returns
        /// `Disconnected`, falling through to the
        /// pre-unregistered fallback.  `None` for top-level
        /// `create_renderer_handle` (no parent — that path is
        /// always allowed).
        parent_client_id: Option<u64>,
        /// Slot #10.6c: one-shot ack so the caller of
        /// [`NetworkProcessHandle::create_renderer_handle`]
        /// (and [`NetworkHandle::create_sibling_handle`])
        /// can block until the broker has actually inserted
        /// `client_id` into its `clients` map.  Closes the
        /// cross-channel race where a `Fetch` on `request_tx`
        /// could be observed by the broker BEFORE the matching
        /// `RegisterRenderer` on `control_tx`: the broker drains
        /// control before request within an iteration, but a
        /// renderer that calls `fetch_async` immediately after
        /// `create_renderer_handle` returns can still post a
        /// Fetch into a request-drain loop already in progress
        /// — the Fetch is then silently dropped by the broker's
        /// stale-cid gate (`dispatch::handle_request`
        /// early-return) because Register hasn't been processed
        /// yet.  The handshake makes that race impossible: the
        /// renderer doesn't get a usable `NetworkHandle` until
        /// the broker has acknowledged `clients.insert`, so any
        /// subsequent `request_tx.send` is happens-after the
        /// insert by transitive program order.  The receiver
        /// side is held by the factory function and dropped
        /// after `recv_timeout`; broker `send` is best-effort
        /// (`bounded(1)`, fire-and-forget).
        ack_tx: crossbeam_channel::Sender<()>,
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
