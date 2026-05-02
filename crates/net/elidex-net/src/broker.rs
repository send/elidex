//! Network Process broker (design doc §5.2, §5.3.3).
//!
//! Implements the Network Process as a singleton coordination thread that owns
//! the shared [`NetClient`], cookie jar, and all WebSocket/SSE I/O loops.
//! Each HTTP fetch is executed on its own OS thread with a per-request tokio
//! runtime (see `handle_fetch`).
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
//! The broker owns a single [`NetClient`] (with shared `CookieJar`), fixing
//! the previous design where each content thread had its own `FetchHandle`
//! with an isolated cookie jar (spec violation — cookies must be shared
//! across browsing contexts within a profile).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{self, TryRecvError};

use crate::sse::{SseCommand, SseEvent, SseHandle};
use crate::ws::{WsCommand, WsEvent, WsHandle};
use crate::{NetClient, Request, Response};

// ---------------------------------------------------------------------------
// ID types
// ---------------------------------------------------------------------------

/// Unique fetch request identifier (globally monotonic).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FetchId(pub u64);

/// Monotonic counter for renderer client IDs.
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

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

// ---------------------------------------------------------------------------
// NetworkProcessHandle (Browser side)
// ---------------------------------------------------------------------------

/// Handle held by the browser thread to control the Network Process.
///
/// Creates [`NetworkHandle`]s for content threads and manages the Network
/// Process lifecycle.
pub struct NetworkProcessHandle {
    control_tx: crossbeam_channel::Sender<NetworkProcessControl>,
    request_tx: crossbeam_channel::Sender<(u64, RendererToNetwork)>,
    thread: Option<JoinHandle<()>>,
    /// Shared cookie jar from the `NetClient` (for `document.cookie` access).
    cookie_jar: Arc<crate::CookieJar>,
}

impl NetworkProcessHandle {
    /// Create a new renderer handle and register it with the Network Process.
    ///
    /// The returned [`NetworkHandle`] should be passed to the content thread.
    #[must_use]
    pub fn create_renderer_handle(&self) -> NetworkHandle {
        let client_id = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = crossbeam_channel::unbounded();

        let _ = self
            .control_tx
            .send(NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
            });

        NetworkHandle {
            client_id,
            request_tx: self.request_tx.clone(),
            control_tx: self.control_tx.clone(),
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            #[cfg(feature = "test-hooks")]
            mock_responses: None,
            #[cfg(feature = "test-hooks")]
            recorded_requests: None,
        }
    }

    /// Get a reference to the shared cookie jar (for `document.cookie`).
    #[must_use]
    pub fn cookie_jar(&self) -> &Arc<crate::CookieJar> {
        &self.cookie_jar
    }

    /// Unregister a renderer. All its connections will be closed.
    pub fn unregister_renderer(&self, client_id: u64) {
        let _ = self
            .control_tx
            .send(NetworkProcessControl::UnregisterRenderer { client_id });
    }

    /// Shutdown the Network Process and wait for the thread to finish.
    pub fn shutdown(mut self) {
        let _ = self.control_tx.send(NetworkProcessControl::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for NetworkProcessHandle {
    fn drop(&mut self) {
        let _ = self.control_tx.send(NetworkProcessControl::Shutdown);
        // Join the broker thread for deterministic cleanup (skip during panics).
        if !std::thread::panicking() {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NetworkHandle (Renderer / Content thread side)
// ---------------------------------------------------------------------------

/// Handle held by a content thread for communicating with the Network Process.
///
/// All network operations go through this handle. The content thread never
/// directly accesses `NetClient`, `FetchHandle`, or I/O thread handles.
///
/// Events received during a blocking fetch (WS/SSE messages) are buffered
/// internally and returned by the next call to [`drain_events`](Self::drain_events).
pub struct NetworkHandle {
    /// Unique client identifier for this renderer.
    client_id: u64,
    /// Shared request channel (all renderers → Network Process).
    request_tx: crossbeam_channel::Sender<(u64, RendererToNetwork)>,
    /// Control channel for registering sibling handles (e.g., for workers).
    control_tx: crossbeam_channel::Sender<NetworkProcessControl>,
    /// Dedicated response channel (Network Process → this renderer).
    response_rx: crossbeam_channel::Receiver<NetworkToRenderer>,
    /// Events buffered during blocking fetch (drained by `drain_events()`).
    /// Uses `RefCell` for interior mutability (content thread is single-threaded).
    buffered: std::cell::RefCell<Vec<NetworkToRenderer>>,
    /// Test-only: when `Some`, [`Self::fetch_blocking`] reads from the
    /// map instead of going through the broker.  Populated via
    /// [`Self::mock_with_responses`]; absent (`None`) for every
    /// production construction path.  The map keys on the URL
    /// serialisation — callers insert
    /// `(url::Url::parse(...).unwrap(), Ok(response) or Err(msg))`
    /// entries.  Each URL may hold **one** configured response
    /// because the backing store is a `HashMap` (duplicate keys
    /// overwrite at construction); that response is consumed on
    /// the first matching lookup (pop-then-return).  If a test
    /// needs the same URL to answer twice, either (a) use two
    /// distinct URLs, or (b) upgrade the store to
    /// `HashMap<String, VecDeque<...>>` first — R28.2.
    #[cfg(feature = "test-hooks")]
    mock_responses:
        Option<std::cell::RefCell<std::collections::HashMap<String, Result<Response, String>>>>,
    /// Test-only: log of every [`Request`] handed to
    /// [`Self::fetch_blocking`] on a mock handle.  Populated only
    /// when `mock_responses` is `Some` (i.e. the handle came from
    /// [`Self::mock_with_responses`]); production handles leave this
    /// `None` so we do not pay for the clone on the hot path.  Read
    /// out via [`Self::drain_recorded_requests`] — long-running
    /// tests should call that periodically because the log is
    /// unbounded.
    #[cfg(feature = "test-hooks")]
    recorded_requests: Option<std::cell::RefCell<Vec<Request>>>,
}

impl NetworkHandle {
    /// Create a disconnected `NetworkHandle` for tests and contexts where
    /// no Network Process is available (standalone pipelines, OOP iframes
    /// before proper handle wiring).
    ///
    /// `fetch_blocking()` will return an error; WS/SSE opens are silently dropped.
    #[must_use]
    pub fn disconnected() -> Self {
        // Create a channel pair and immediately drop the receiver,
        // making all sends on request_tx fail silently.
        let (request_tx, _request_rx) = crossbeam_channel::unbounded();
        let (control_tx, _control_rx) = crossbeam_channel::unbounded();
        let (_response_tx, response_rx) = crossbeam_channel::unbounded();
        Self {
            client_id: 0,
            request_tx,
            control_tx,
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            #[cfg(feature = "test-hooks")]
            mock_responses: None,
            #[cfg(feature = "test-hooks")]
            recorded_requests: None,
        }
    }

    /// Create a sibling handle sharing the same Network Process broker.
    ///
    /// Used to create handles for Web Workers spawned by this content thread.
    /// The sibling gets its own client ID and response channel but shares
    /// the request and control channels (same broker, same cookie jar).
    #[must_use]
    pub fn create_sibling_handle(&self) -> Self {
        let client_id = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = crossbeam_channel::unbounded();

        let _ = self
            .control_tx
            .send(NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
            });

        Self {
            client_id,
            request_tx: self.request_tx.clone(),
            control_tx: self.control_tx.clone(),
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            #[cfg(feature = "test-hooks")]
            mock_responses: None,
            #[cfg(feature = "test-hooks")]
            recorded_requests: None,
        }
    }

    /// Get this renderer's client ID.
    #[must_use]
    pub fn client_id(&self) -> u64 {
        self.client_id
    }

    /// Construct a mock handle that answers `fetch_blocking` from a
    /// pre-populated `URL → Result<Response, String>` map.  Intended
    /// for downstream unit tests (the Fetch surface in `elidex-js`)
    /// that need deterministic responses without a live Network
    /// Process.
    ///
    /// Entries are consumed on first match.  A request whose URL is
    /// not in the map resolves to `Err("mock: no response for {url}")`.
    #[cfg(feature = "test-hooks")]
    #[must_use]
    pub fn mock_with_responses(responses: Vec<(url::Url, Result<Response, String>)>) -> Self {
        let map: std::collections::HashMap<String, Result<Response, String>> = responses
            .into_iter()
            .map(|(u, r)| (u.to_string(), r))
            .collect();
        let mut handle = Self::disconnected();
        handle.mock_responses = Some(std::cell::RefCell::new(map));
        handle.recorded_requests = Some(std::cell::RefCell::new(Vec::new()));
        handle
    }

    /// Drain and return the [`Request`]s observed by this mock
    /// handle since the last drain.  Returns `Vec::new()` for
    /// non-mock handles.  Test-only.
    #[cfg(feature = "test-hooks")]
    #[must_use]
    pub fn drain_recorded_requests(&self) -> Vec<Request> {
        self.recorded_requests
            .as_ref()
            .map(|log| log.borrow_mut().drain(..).collect())
            .unwrap_or_default()
    }

    /// Async dispatch: enqueue a fetch and return its [`FetchId`]
    /// immediately.  The reply arrives via [`Self::drain_events`] as
    /// [`NetworkToRenderer::FetchResponse`] some time later.  Used by
    /// the elidex-js VM's async fetch path (M4-12 PR5) and by any
    /// embedder that drives its own event loop.
    ///
    /// Mock handles short-circuit identically to
    /// [`Self::fetch_blocking`] — the configured response is dropped
    /// onto the internal `buffered` queue under the freshly-allocated
    /// id, so a follow-up [`Self::drain_events`] returns the matching
    /// `FetchResponse(id, ...)`.  Each mock URL still answers exactly
    /// once (the `HashMap` `remove` is consumed on first match).
    pub fn fetch_async(&self, request: Request) -> FetchId {
        let fetch_id = FetchId::next();

        #[cfg(feature = "test-hooks")]
        if let Some(ref map) = self.mock_responses {
            if let Some(ref log) = self.recorded_requests {
                log.borrow_mut().push(request.clone());
            }
            let url_str = request.url.to_string();
            let result = map
                .borrow_mut()
                .remove(&url_str)
                .unwrap_or_else(|| Err(format!("mock: no response for {url_str}")));
            self.buffered
                .borrow_mut()
                .push(NetworkToRenderer::FetchResponse(fetch_id, result));
            return fetch_id;
        }

        // Send may fail when the broker has shut down or the handle
        // was created via `disconnected()`; in that case buffer a
        // synthetic `Err` reply so the renderer's `pending_fetches`
        // table can settle on the next `drain_events()` instead of
        // leaking the entry forever (R1.1).
        if !self.send(RendererToNetwork::Fetch(fetch_id, request)) {
            self.buffered
                .borrow_mut()
                .push(NetworkToRenderer::FetchResponse(
                    fetch_id,
                    Err("network process disconnected".into()),
                ));
        }
        fetch_id
    }

    /// Cancel an in-flight fetch.  Idempotent — calling on a
    /// completed / unknown id is harmless because the broker thread
    /// merely posts an `Err("aborted")` reply for that id, and the
    /// renderer's pending-fetch table already deduplicates late
    /// replies (the second arrival's `remove` returns `None` and is
    /// silently dropped).
    ///
    /// **Multi-reply contract** (R6.2): the broker emits the
    /// synthesised `FetchResponse(id, Err("aborted"))` immediately
    /// on the cancel, but the in-flight fetch thread continues
    /// running until its underlying tokio call returns and may
    /// post a *second* `FetchResponse` for the same `FetchId`.
    /// Direct embedders driving [`Self::drain_events`] themselves
    /// must therefore treat the first terminal reply per `FetchId`
    /// as authoritative and silently drop subsequent ones; the
    /// elidex-js VM does this via its `pending_fetches.remove`
    /// dedup.  Tightening the broker to suppress the late real
    /// reply would require per-`FetchId` cancellation state on
    /// the broker thread (currently kept stateless to bound
    /// memory under unbounded cancel-then-leak scenarios).
    ///
    /// **Concurrency-counter saturation** (R7.1): because the
    /// in-flight thread is not actually stopped, the per-broker
    /// `MAX_CONCURRENT_FETCHES` inflight counter stays bumped
    /// until the underlying network IO completes (success, error,
    /// or transport timeout — the latter ~30s for HTTP requests
    /// that connect but never respond).  A workload that issues
    /// many fetches and aborts each one immediately can therefore
    /// transiently saturate the global concurrency limit and
    /// starve subsequent un-cancelled fetches until the cancelled
    /// IO drains.  True request cancellation (passing a tokio
    /// cancellation token through `client.send`) belongs with
    /// the broader broker-state work in PR5-cors / PR5-streams.
    /// For now, embedders that anticipate cancel-heavy workloads
    /// should size `MAX_CONCURRENT_FETCHES` accordingly.
    ///
    /// Returns `true` if the cancel was queued, `false` if the
    /// broker is disconnected.
    pub fn cancel_fetch(&self, id: FetchId) -> bool {
        self.send(RendererToNetwork::CancelFetch(id))
    }

    /// Send a blocking fetch request.
    ///
    /// The content thread blocks until the fetch completes (or times out
    /// at 30 seconds). Any WS/SSE events received while waiting are
    /// buffered internally and returned by the next [`drain_events`](Self::drain_events)
    /// call.
    pub fn fetch_blocking(&self, request: Request) -> Result<Response, String> {
        // Test-hooks mock short-circuit: when populated, the map
        // answers directly and the broker is never contacted.  Keeps
        // the blocking-path semantics (sync return) identical to the
        // live path from the caller's perspective.
        #[cfg(feature = "test-hooks")]
        if let Some(ref map) = self.mock_responses {
            // Record the request for later inspection (Referer
            // header verification, etc.).  Cloned because the
            // request itself is consumed below by the URL lookup.
            if let Some(ref log) = self.recorded_requests {
                log.borrow_mut().push(request.clone());
            }
            let url_str = request.url.to_string();
            let mut guard = map.borrow_mut();
            return guard
                .remove(&url_str)
                .unwrap_or_else(|| Err(format!("mock: no response for {url_str}")));
        }

        let fetch_id = FetchId::next();
        let _ = self
            .request_tx
            .send((self.client_id, RendererToNetwork::Fetch(fetch_id, request)));

        let mut buf = self.buffered.borrow_mut();
        loop {
            match self.response_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(NetworkToRenderer::FetchResponse(id, result)) if id == fetch_id => {
                    return result;
                }
                Ok(other) => buf.push(other),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    return Err("fetch timeout (30s)".into());
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    return Err("network process disconnected".into());
                }
            }
        }
    }

    /// Non-blocking drain of all pending events (WS/SSE/fetch responses).
    ///
    /// Includes any events buffered during a prior [`fetch_blocking`](Self::fetch_blocking) call.
    pub fn drain_events(&self) -> Vec<NetworkToRenderer> {
        let mut events: Vec<_> = self.buffered.borrow_mut().drain(..).collect();
        while let Ok(msg) = self.response_rx.try_recv() {
            events.push(msg);
        }
        events
    }

    /// Partial drain: return only fetch replies, leaving every
    /// other event (WS / SSE) in the internal buffer for a sibling
    /// consumer's later [`Self::drain_events`].
    ///
    /// Replaces the `drain_events` + [`Self::rebuffer_events`]
    /// pattern that the elidex-js VM's `tick_network` previously
    /// used — that pattern stopped at the first non-fetch event
    /// and re-buffered the tail (including any later fetch
    /// replies) to preserve arrival order across a sibling
    /// consumer's drain.  This API instead walks every pending
    /// event in one pass and partitions them: fetch replies are
    /// returned, non-fetch events are kept in the same relative
    /// order in `self.buffered`.  The non-fetch ordering observed
    /// by the next [`Self::drain_events`] is therefore identical
    /// to what an unfiltered drain would have produced — fetch
    /// replies are the only thing the sibling no longer sees.
    ///
    /// Order guarantee: fetch replies appear in the returned `Vec`
    /// in arrival order across `(prior buffer, channel try_recv
    /// drain)`; non-fetch events stay in `self.buffered` in the
    /// same arrival order.
    pub fn drain_fetch_responses_only(&self) -> Vec<(FetchId, Result<Response, String>)> {
        let mut buf = self.buffered.borrow_mut();
        let prior = std::mem::take(&mut *buf);
        // Pre-size both partitions to `prior.len()`: the steady-
        // state per-tick caller (the elidex-js VM's `tick_network`)
        // typically sees a single-digit buffer + a single-digit
        // arrival batch, and we don't know the split a priori, so
        // sizing each bucket to the prior length avoids the early
        // `Vec::push` reallocations on the buffered branch.  The
        // channel branch may push past the reserve when arrivals
        // exceed `prior.len()`; that's still amortised O(1) per
        // push and is bounded by the broker's per-tick fan-in.
        let mut fetches: Vec<(FetchId, Result<Response, String>)> = Vec::with_capacity(prior.len());
        let mut kept: Vec<NetworkToRenderer> = Vec::with_capacity(prior.len());
        for event in prior {
            match event {
                NetworkToRenderer::FetchResponse(id, result) => fetches.push((id, result)),
                other => kept.push(other),
            }
        }
        while let Ok(event) = self.response_rx.try_recv() {
            match event {
                NetworkToRenderer::FetchResponse(id, result) => fetches.push((id, result)),
                other => kept.push(other),
            }
        }
        *buf = kept;
        fetches
    }

    /// Send a message to the Network Process without waiting for a response.
    ///
    /// Returns `true` if the message was queued, `false` if the broker is disconnected.
    pub fn send(&self, msg: RendererToNetwork) -> bool {
        self.request_tx.send((self.client_id, msg)).is_ok()
    }

    /// Push events back onto the internal buffer so the next
    /// [`Self::drain_events`] returns them.  Held over from the
    /// pre-[`Self::drain_fetch_responses_only`] partial-drain
    /// pattern: the elidex-js VM's `tick_network` once drained
    /// every event, settled fetch replies, and re-buffered WS/SSE
    /// for a sibling consumer.  That site now uses the partition-
    /// in-place API and no longer calls this method.  Retained
    /// for any direct embedder that still drives `drain_events`
    /// itself and needs to push events back; once no caller
    /// remains, this method can be removed in a follow-up PR.
    /// Events appear in front of any newly-arrived events on the
    /// channel; relative order within the re-buffered slice is
    /// preserved.
    pub fn rebuffer_events(&self, events: Vec<NetworkToRenderer>) {
        if events.is_empty() {
            return;
        }
        let mut buf = self.buffered.borrow_mut();
        // Re-buffered events come before anything arriving on the
        // channel since `drain_events` reads `buffered` first.
        buf.splice(0..0, events);
    }
}

impl Drop for NetworkHandle {
    fn drop(&mut self) {
        // Unregister from the broker so per-client resources are cleaned up.
        // Disconnected handles (client_id == 0) skip this.
        if self.client_id != 0 {
            let _ = self
                .control_tx
                .send(NetworkProcessControl::UnregisterRenderer {
                    client_id: self.client_id,
                });
        }
    }
}

impl std::fmt::Debug for NetworkHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkHandle")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Network Process thread
// ---------------------------------------------------------------------------

/// Spawn the Network Process thread.
///
/// Returns a [`NetworkProcessHandle`] for the browser thread to manage
/// renderer registrations and lifecycle.
#[must_use]
pub fn spawn_network_process(client: NetClient) -> NetworkProcessHandle {
    let (control_tx, control_rx) = crossbeam_channel::unbounded();
    let (request_tx, request_rx) = crossbeam_channel::unbounded();

    let cookie_jar = client.cookie_jar_arc();
    let client = Arc::new(client);
    let thread = std::thread::Builder::new()
        .name("elidex-network".into())
        .spawn(move || {
            network_process_main(client, request_rx, control_rx);
        })
        .expect("failed to spawn network process thread");

    NetworkProcessHandle {
        control_tx,
        request_tx,
        thread: Some(thread),
        cookie_jar,
    }
}

/// Main loop of the Network Process thread.
#[allow(clippy::needless_pass_by_value)] // Owned channels consumed by the thread.
fn network_process_main(
    client: Arc<NetClient>,
    request_rx: crossbeam_channel::Receiver<(u64, RendererToNetwork)>,
    control_rx: crossbeam_channel::Receiver<NetworkProcessControl>,
) {
    let mut state = NetworkProcessState::new();

    loop {
        // Event-driven wait: block until ANY channel has data.
        // Uses crossbeam's dynamic `Select` to multiplex control, request,
        // and all WS/SSE event channels. Wakes on any event with near-zero
        // latency; 1-second timeout ensures periodic cleanup when idle.
        {
            let mut sel = crossbeam_channel::Select::new();
            sel.recv(&control_rx);
            sel.recv(&request_rx);
            for handle in state.ws_handles.values() {
                sel.recv(&handle.event_rx);
            }
            for handle in state.sse_handles.values() {
                sel.recv(&handle.event_rx);
            }
            // Block until at least one channel is ready, or timeout for
            // periodic cleanup of finished I/O threads.
            // `ready_timeout` returns the index of a ready channel without
            // consuming the operation (unlike `select_timeout`).
            let _ = sel.ready_timeout(Duration::from_secs(1));
        }

        // Drain all channels (non-blocking). We don't care which channel
        // woke us — process everything that's available.

        // 1. Control messages first (register/unregister/shutdown).
        while let Ok(ctrl) = control_rx.try_recv() {
            if !state.handle_control(ctrl) {
                return;
            }
        }

        // 2. Renderer requests (up to 64 per iteration).
        for _ in 0..64 {
            match request_rx.try_recv() {
                Ok((cid, msg)) => state.handle_request(cid, msg, &client),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        // 3. Forward WS/SSE events from I/O threads to renderers.
        state.forward_ws_events();
        state.forward_sse_events();

        // 4. Clean up finished I/O threads.
        state.cleanup_finished();
    }
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Maximum concurrent fetch threads across all renderers.
const MAX_CONCURRENT_FETCHES: usize = 64;

/// RAII guard that decrements the inflight fetch counter on drop.
/// Ensures the counter is decremented even if the fetch thread panics.
struct FetchInflightGuard(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for FetchInflightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// RAII guard that removes the worker's `(cid, fetch_id)` entry
/// from [`CancelMap`] on drop — including unwind paths.  Without
/// this, a panic anywhere in the worker (tokio runtime build,
/// `block_on`, future internals, downstream `.expect`s) would
/// leave the entry in the map; over time those orphan entries
/// would grow the map beyond the documented `MAX_CONCURRENT_FETCHES`
/// bound (Copilot R2).  Uses `unwrap_or_else(into_inner)` so a
/// poisoned mutex during panic teardown still releases the
/// entry rather than double-panicking.
struct CancelMapEntryGuard {
    map: CancelMap,
    key: (u64, FetchId),
}

impl Drop for CancelMapEntryGuard {
    fn drop(&mut self) {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.key);
    }
}

/// Per-fetch cancellation token map.  Worker threads drop their
/// entry on completion (regardless of outcome) so the map is
/// bounded by `MAX_CONCURRENT_FETCHES` rather than the total
/// fetch count.  Wrapped in `Arc<Mutex<...>>` because the broker
/// thread inserts/cancels and worker threads remove on
/// completion.
///
/// **Key shape**: `(client_id, FetchId)` — keying on `FetchId`
/// alone would let one renderer cancel another renderer's
/// in-flight fetch by guessing/observing its id (the broker's
/// synthetic `Err("aborted")` reply would also be misrouted to
/// the *cancelling* client while the original client's promise
/// stays unresolved).  Pairing with `client_id` mirrors the
/// `ws_handles` / `sse_handles` ownership convention (Copilot
/// R1).
type CancelMap = Arc<std::sync::Mutex<HashMap<(u64, FetchId), crate::CancelHandle>>>;

/// Internal state of the Network Process.
struct NetworkProcessState {
    /// Registered renderer clients: `client_id` → response sender.
    clients: HashMap<u64, crossbeam_channel::Sender<NetworkToRenderer>>,
    /// Active WebSocket connections: `(client_id, conn_id)` → `WsHandle`.
    ws_handles: HashMap<(u64, u64), WsHandle>,
    /// Active SSE connections: `(client_id, conn_id)` → `SseHandle`.
    sse_handles: HashMap<(u64, u64), SseHandle>,
    /// Counter of in-flight fetch threads (for limiting concurrency).
    inflight_fetches: Arc<std::sync::atomic::AtomicUsize>,
    /// In-flight fetch cancellation tokens, keyed by
    /// `(client_id, FetchId)` (see [`CancelMap`] for why the
    /// composite key is required for cross-client cancel
    /// isolation).  `Fetch` inserts before spawning the worker;
    /// the worker removes on completion via
    /// [`CancelMapEntryGuard`]; `CancelFetch` looks up the key
    /// pair + triggers + removes (so the worker's later guard
    /// drop is a no-op).
    cancel_tokens: CancelMap,
}

impl NetworkProcessState {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            ws_handles: HashMap::new(),
            sse_handles: HashMap::new(),
            inflight_fetches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            cancel_tokens: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    fn handle_request(&mut self, cid: u64, msg: RendererToNetwork, client: &Arc<NetClient>) {
        match msg {
            RendererToNetwork::Fetch(fetch_id, request) => {
                self.handle_fetch(cid, fetch_id, request, client);
            }
            RendererToNetwork::CancelFetch(fetch_id) => {
                self.handle_cancel_fetch(cid, fetch_id);
            }
            RendererToNetwork::WebSocketOpen {
                conn_id,
                url,
                protocols,
                origin,
            } => {
                // SSRF validation at the broker boundary — the renderer is
                // sandboxed and cannot be trusted to validate URLs.
                // Convert ws→http / wss→https for validate_url (same as websocket.rs).
                let mut check_url = url.clone();
                let scheme_ok = match check_url.scheme() {
                    "ws" => check_url.set_scheme("http").is_ok(),
                    "wss" => check_url.set_scheme("https").is_ok(),
                    _ => false, // Reject non-WS schemes.
                };
                if !scheme_ok || elidex_plugin::url_security::validate_url(&check_url).is_err() {
                    // Send error + close so JS transitions to CLOSED state.
                    if let Some(tx) = self.clients.get(&cid) {
                        let _ = tx.send(NetworkToRenderer::WebSocketEvent(
                            conn_id,
                            WsEvent::Error("SSRF: URL blocked by security policy".into()),
                        ));
                        let _ = tx.send(NetworkToRenderer::WebSocketEvent(
                            conn_id,
                            WsEvent::Closed {
                                code: 1006,
                                reason: String::new(),
                                was_clean: false,
                            },
                        ));
                    }
                    return;
                }
                let handle = crate::ws::spawn_ws_thread(url, protocols, origin);
                self.ws_handles.insert((cid, conn_id), handle);
            }
            RendererToNetwork::WebSocketSend(conn_id, command) => {
                if let Some(handle) = self.ws_handles.get(&(cid, conn_id)) {
                    let _ = handle.command_tx.send(command);
                }
            }
            RendererToNetwork::WebSocketClose(conn_id) => {
                // Close with code 1000 (normal). JS-level close() uses
                // WebSocketSend(_, WsCommand::Close(code, reason)) instead.
                if let Some(handle) = self.ws_handles.get(&(cid, conn_id)) {
                    let _ = handle
                        .command_tx
                        .send(WsCommand::Close(1000, String::new()));
                }
            }
            RendererToNetwork::EventSourceOpen {
                conn_id,
                url,
                last_event_id,
                origin,
                with_credentials,
            } => {
                // SSRF validation at the broker boundary.
                if elidex_plugin::url_security::validate_url(&url).is_err() {
                    if let Some(tx) = self.clients.get(&cid) {
                        let _ = tx.send(NetworkToRenderer::EventSourceEvent(
                            conn_id,
                            SseEvent::FatalError("SSRF: URL blocked by security policy".into()),
                        ));
                    }
                    return;
                }
                // Attach cookies for same-origin requests (origin=None) and
                // cross-origin with withCredentials=true. Per WHATWG HTML §9.2,
                // same-origin requests always include credentials.
                let cookie_jar = if origin.is_none() || with_credentials {
                    Some(client.cookie_jar_arc())
                } else {
                    None
                };
                let handle = crate::sse::spawn_sse_thread(
                    url,
                    last_event_id,
                    cookie_jar,
                    origin,
                    with_credentials,
                );
                self.sse_handles.insert((cid, conn_id), handle);
            }
            RendererToNetwork::EventSourceClose(conn_id) => {
                if let Some(handle) = self.sse_handles.get(&(cid, conn_id)) {
                    let _ = handle.command_tx.send(SseCommand::Close);
                }
            }
            RendererToNetwork::Shutdown => {
                self.close_all_for_client(cid);
            }
        }
    }

    /// Trigger true cancellation for an in-flight fetch
    /// (R7.1): pull the per-fetch [`crate::CancelHandle`] out of
    /// the map and call `cancel()` so the worker thread's
    /// `tokio::select!` aborts the in-flight hyper future
    /// immediately.  The worker's `FetchInflightGuard` then
    /// drops on exit, releasing the `MAX_CONCURRENT_FETCHES`
    /// slot for subsequent fetches (no more saturation under
    /// cancel-spam workloads).
    ///
    /// The synthesised `Err("aborted")` reply still fires here
    /// so the renderer-side `pending_fetches.remove(id)`
    /// settles the JS Promise without waiting for the worker
    /// thread to finish its teardown.  The worker, on observing
    /// `NetErrorKind::Cancelled`, suppresses its own duplicate
    /// reply (see `handle_fetch`) so the renderer sees exactly
    /// one reply per fetch.
    ///
    /// **Owner check**: the cancel-token map is keyed by
    /// `(cid, fetch_id)` so the underlying [`crate::CancelHandle`]
    /// is only triggered when the requesting client owns the
    /// fetch.  Without this check a malicious or buggy renderer
    /// could cancel another renderer's in-flight fetch by
    /// guessing/observing its `FetchId`, leaving the owner's
    /// promise stuck waiting on a worker that has been aborted
    /// (Copilot R1).  The synthetic `Err("aborted")` reply still
    /// fires unconditionally to the *requesting* client so its
    /// own `pending_fetches.remove(id)` resolves promptly even
    /// for unknown ids — the renderer-side dedup table absorbs
    /// the no-op when the id was never registered locally.
    ///
    /// Cancel-then-completion ordering: if the worker has
    /// already finished and removed its own cancel-token entry,
    /// this `remove` returns `None` and the cancel-trigger
    /// becomes a no-op (the JS Promise was already settled by
    /// the real reply).
    fn handle_cancel_fetch(&self, cid: u64, fetch_id: FetchId) {
        // Tolerate poison: a worker panic while *holding* the
        // cancel-token lock would poison this mutex; bringing
        // down the broker thread on every subsequent
        // `CancelFetch` would amplify a single worker bug into
        // permanent fetch-cancel breakage.  Match
        // `CancelMapEntryGuard`'s recovery strategy (Copilot R5).
        if let Some(token) = self
            .cancel_tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(cid, fetch_id))
        {
            token.cancel();
        }
        if let Some(tx) = self.clients.get(&cid) {
            let _ = tx.send(NetworkToRenderer::FetchResponse(
                fetch_id,
                Err("aborted".into()),
            ));
        }
    }

    fn handle_fetch(&self, cid: u64, fetch_id: FetchId, request: Request, client: &Arc<NetClient>) {
        // Note: SSRF validation for fetch is handled by NetClient::send() which
        // checks validate_url() internally (respecting allow_private_ips config).
        // WS/SSE need broker-level SSRF because their I/O threads bypass NetClient.
        let client = Arc::clone(client);
        let tx = self.clients.get(&cid).cloned();
        let inflight = Arc::clone(&self.inflight_fetches);
        // Atomically increment and check — avoids TOCTOU between load and add.
        // Note: the broker is single-threaded so the race is theoretical, but
        // this pattern is correct regardless of future threading changes.
        let prev = inflight.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if prev >= MAX_CONCURRENT_FETCHES {
            inflight.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            if let Some(tx) = tx {
                let _ = tx.send(NetworkToRenderer::FetchResponse(
                    fetch_id,
                    Err("too many concurrent fetches".into()),
                ));
            }
            return;
        }
        // Register a cancel token for this fetch *before* spawning
        // the worker.  A `CancelFetch` arriving between this insert
        // and the worker's first await is observed via
        // `transport.send`'s pre-await `is_cancelled()` fast-path
        // (no wasted connection checkout).
        let cancel = crate::CancelHandle::new();
        let cancel_map = Arc::clone(&self.cancel_tokens);
        // Tolerate poison on the broker thread (Copilot R5) —
        // see the matching comment in `handle_cancel_fetch`.
        cancel_map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((cid, fetch_id), cancel.clone());
        std::thread::spawn(move || {
            // Drop guards ensure the counter is decremented and
            // the cancel-token entry is removed even on panic
            // (prevents permanent counter leak → fetch starvation,
            // and unbounded growth of the cancel_tokens map past
            // its `MAX_CONCURRENT_FETCHES` bound).
            let _guard = FetchInflightGuard(inflight);
            let _cancel_entry = CancelMapEntryGuard {
                map: cancel_map,
                key: (cid, fetch_id),
            };

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create fetch runtime");
            let outcome = rt.block_on(client.send_cancellable(request, Some(&cancel)));
            // The `CancelMapEntryGuard` drops at the end of this
            // scope (or on unwind) and removes our entry — a
            // late `CancelFetch` after this point is a no-op
            // (the JS Promise was already settled by the
            // worker's reply).
            // When the worker observes `Cancelled` it means the
            // `CancelFetch` handler has already pushed the
            // synthesised `Err("aborted")` reply to the renderer
            // — suppress this duplicate so the renderer sees
            // exactly one reply per fetch with a stable error
            // message.  Any other outcome (success, real error
            // from non-cancel paths) is forwarded as before.
            let result = match outcome {
                Err(ref e) if e.kind == crate::NetErrorKind::Cancelled => return,
                Ok(r) => Ok(r),
                Err(e) => Err(format!("{e:#}")),
            };
            if let Some(tx) = tx {
                let _ = tx.send(NetworkToRenderer::FetchResponse(fetch_id, result));
            }
        });
    }

    fn handle_control(&mut self, ctrl: NetworkProcessControl) -> bool {
        match ctrl {
            NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
            } => {
                self.clients.insert(client_id, response_tx);
                true
            }
            NetworkProcessControl::UnregisterRenderer { client_id } => {
                self.close_all_for_client(client_id);
                self.clients.remove(&client_id);
                true
            }
            NetworkProcessControl::Shutdown => false,
        }
    }

    fn forward_ws_events(&mut self) {
        let mut remove = Vec::new();
        for (&(cid, conn_id), handle) in &self.ws_handles {
            loop {
                match handle.event_rx.try_recv() {
                    Ok(event) => {
                        if let Some(tx) = self.clients.get(&cid) {
                            let _ = tx.send(NetworkToRenderer::WebSocketEvent(conn_id, event));
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        remove.push((cid, conn_id));
                        break;
                    }
                }
            }
        }
        for key in remove {
            self.ws_handles.remove(&key);
        }
    }

    fn forward_sse_events(&mut self) {
        let mut remove = Vec::new();
        for (&(cid, conn_id), handle) in &self.sse_handles {
            loop {
                match handle.event_rx.try_recv() {
                    Ok(event) => {
                        if let Some(tx) = self.clients.get(&cid) {
                            let _ = tx.send(NetworkToRenderer::EventSourceEvent(conn_id, event));
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        remove.push((cid, conn_id));
                        break;
                    }
                }
            }
        }
        for key in remove {
            self.sse_handles.remove(&key);
        }
    }

    fn cleanup_finished(&mut self) {
        self.ws_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
        self.sse_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
    }

    fn close_all_for_client(&mut self, client_id: u64) {
        // Close WebSocket connections.
        let ws_keys: Vec<_> = self
            .ws_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in ws_keys {
            if let Some(handle) = self.ws_handles.remove(&key) {
                let _ = handle
                    .command_tx
                    .send(WsCommand::Close(1001, "navigated away".into()));
            }
        }

        // Close SSE connections.
        let sse_keys: Vec<_> = self
            .sse_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in sse_keys {
            if let Some(handle) = self.sse_handles.remove(&key) {
                let _ = handle.command_tx.send(SseCommand::Close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetClient, NetClientConfig, TransportConfig};

    fn test_client() -> NetClient {
        NetClient::with_config(NetClientConfig {
            transport: TransportConfig {
                allow_private_ips: true,
                // Lift per-origin and global connection caps well
                // above `MAX_CONCURRENT_FETCHES` so cancel-spam
                // regression tests can keep ≥`MAX_CONCURRENT_FETCHES`
                // workers genuinely stalled on transport IO.  With
                // the production defaults (`6` per-origin, `256`
                // total) most workers in those tests would fail
                // fast on the per-origin cap inside
                // `pool::create_connection` — that's a different
                // error path than the cancel-vs-stall race those
                // tests are meant to exercise (Copilot R1).
                max_connections_per_origin: 256,
                max_total_connections: 1024,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn spawn_and_shutdown() {
        let handle = spawn_network_process(test_client());
        handle.shutdown();
    }

    #[test]
    fn create_renderer_handle() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        assert!(renderer.client_id() > 0);
        handle.shutdown();
    }

    #[test]
    fn unregister_renderer() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let cid = renderer.client_id();
        handle.unregister_renderer(cid);
        // Brief wait for unregistration to propagate.
        std::thread::sleep(Duration::from_millis(10));
        handle.shutdown();
    }

    #[test]
    fn fetch_blocking_connection_refused() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse("http://127.0.0.1:1/").unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };

        let result = renderer.fetch_blocking(request);
        assert!(result.is_err());

        handle.shutdown();
    }

    #[test]
    fn fetch_blocking_success() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        // Bind a sync TCP server — no race with thread startup.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(resp).unwrap();
        });

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };

        let result = renderer.fetch_blocking(request);
        let resp = result.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body.as_ref(), b"ok");

        server_thread.join().unwrap();
        np.shutdown();
    }

    #[test]
    fn drain_events_empty() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let events = renderer.drain_events();
        assert!(events.is_empty());
        handle.shutdown();
    }

    #[test]
    fn fetch_id_monotonic() {
        let a = FetchId::next();
        let b = FetchId::next();
        assert!(b.0 > a.0);
    }

    #[test]
    fn multiple_renderers() {
        let handle = spawn_network_process(test_client());
        let r1 = handle.create_renderer_handle();
        let r2 = handle.create_renderer_handle();
        assert_ne!(r1.client_id(), r2.client_id());
        handle.shutdown();
    }

    #[test]
    fn debug_network_handle() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let debug = format!("{renderer:?}");
        assert!(debug.contains("NetworkHandle"));
        handle.shutdown();
    }

    #[test]
    fn fetch_async_returns_id_and_drain_picks_up_response() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(resp).unwrap();
        });

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };

        let id = renderer.fetch_async(request);
        assert!(id.0 > 0);

        // Poll drain_events until the matching FetchResponse arrives.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut got = None;
        while std::time::Instant::now() < deadline {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == id {
                        got = Some(result);
                    }
                }
            }
            if got.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let resp = got.expect("FetchResponse not delivered").unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body.as_ref(), b"ok");

        server_thread.join().unwrap();
        np.shutdown();
    }

    #[test]
    fn cancel_fetch_delivers_aborted_reply() {
        // Bind a sync server that *never replies* — the only way the
        // renderer sees a FetchResponse is via the broker's CancelFetch
        // synthesised reply.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        // Hold the listener open for the whole test so the connect
        // succeeds; never accept-and-reply (so the real fetch hangs).
        let _listener = listener;

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };

        let id = renderer.fetch_async(request);
        assert!(renderer.cancel_fetch(id));

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut got = None;
        while std::time::Instant::now() < deadline {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == id && result.is_err() {
                        got = Some(result);
                    }
                }
            }
            if got.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        let err = got.expect("aborted reply not delivered").unwrap_err();
        assert!(err.contains("aborted"), "expected 'aborted' got: {err}");

        np.shutdown();
    }

    #[test]
    fn fetch_async_on_disconnected_handle_buffers_terminal_error() {
        // R1.1: when the request channel is closed (broker shut down,
        // or `NetworkHandle::disconnected()` test fixture), `fetch_async`
        // must still produce a `FetchResponse(id, Err(...))` so the
        // renderer's `pending_fetches` table can settle on the next
        // drain instead of leaking the entry.
        let renderer = NetworkHandle::disconnected();
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse("http://example.invalid/").unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };
        let id = renderer.fetch_async(request);
        let events = renderer.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            NetworkToRenderer::FetchResponse(rid, Err(msg)) => {
                assert_eq!(*rid, id);
                assert!(msg.contains("disconnected"), "got: {msg}");
            }
            other => panic!("expected disconnected error, got {other:?}"),
        }
    }

    /// True request cancellation: a `cancel_fetch` against a
    /// fetch dispatched to a server that never responds must
    /// release the in-flight slot promptly, so the next fetch
    /// is not blocked behind the stalled IO.
    ///
    /// Pre-PR-true-request-cancellation behaviour: the worker
    /// kept its `MAX_CONCURRENT_FETCHES` inflight slot until the
    /// underlying `request_timeout` (~30s) — a workload that
    /// cancelled aggressively could starve subsequent fetches.
    /// With the [`crate::CancelHandle`] wired through to
    /// `transport.send`, the hyper future is dropped immediately
    /// and the inflight counter decrements via the `FetchInflight
    /// Guard` drop.
    #[test]
    fn cancel_fetch_releases_inflight_slot_promptly() {
        // Bind a sync server that holds the connection open
        // forever (never replies), so any un-cancelled fetch
        // would wait for the request_timeout to fire.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let stall_addr = listener.local_addr().unwrap();
        let _stall_listener = listener; // hold open

        // Second listener: the post-cancel "did the slot free up"
        // probe.  Replies promptly so a successful fetch confirms
        // the inflight counter is below MAX after the cancel.
        let probe_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let probe_addr = probe_listener.local_addr().unwrap();
        let probe_thread = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let Ok((mut stream, _)) = probe_listener.accept() else {
                return;
            };
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        });

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        // Fire 1 fetch at the stalling server, then cancel it.
        let stall_request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };
        let stall_id = renderer.fetch_async(stall_request);
        // Yield briefly so the worker has a chance to enter
        // `transport.send` before the cancel arrives.
        std::thread::sleep(Duration::from_millis(20));
        assert!(renderer.cancel_fetch(stall_id));

        // Drain the synth aborted reply.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut saw_aborted = false;
        while std::time::Instant::now() < deadline && !saw_aborted {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, Err(msg)) = ev {
                    if rid == stall_id && msg.contains("aborted") {
                        saw_aborted = true;
                    }
                }
            }
            if !saw_aborted {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert!(saw_aborted, "synth aborted reply not delivered");

        // Now fire a probe fetch.  If the cancel actually
        // released the inflight slot, this should complete
        // promptly (well under the 30s request_timeout that
        // would gate a saturated counter).
        let probe_request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };
        let probe_id = renderer.fetch_async(probe_request);
        let probe_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut probe_got = None;
        while std::time::Instant::now() < probe_deadline && probe_got.is_none() {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == probe_id {
                        probe_got = Some(result);
                    }
                }
            }
            if probe_got.is_none() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let probe_resp = probe_got
            .expect("probe fetch did not complete — inflight slot likely not released")
            .unwrap();
        assert_eq!(probe_resp.status, 200);
        assert_eq!(probe_resp.body.as_ref(), b"ok");

        np.shutdown();
        probe_thread.join().unwrap();
    }

    /// Cancel-spam workload regression (R7.1): dispatch
    /// many fetches at a stalling server and cancel each
    /// immediately.  Without true cancellation, the inflight
    /// counter would saturate at `MAX_CONCURRENT_FETCHES` and
    /// later fetches would receive `"too many concurrent
    /// fetches"`.  With the [`crate::CancelHandle`] each cancel
    /// decrements the counter promptly.
    ///
    /// Sized at 100 (rather than the doc'd 1000) to keep the
    /// test under a few seconds of wall-clock; the assertion is
    /// the same — subsequent fetch must not be starved.
    #[test]
    fn cancel_spam_does_not_saturate_inflight_counter() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let stall_addr = listener.local_addr().unwrap();
        let _stall_listener = listener;

        let probe_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let probe_addr = probe_listener.local_addr().unwrap();
        let probe_thread = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let Ok((mut stream, _)) = probe_listener.accept() else {
                return;
            };
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        });

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        let mk_stall = || Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };

        let mut ids = Vec::new();
        for _ in 0..100 {
            let id = renderer.fetch_async(mk_stall());
            ids.push(id);
        }
        // Brief pause so a meaningful fraction of workers reach
        // the transport's await point before the cancels fire —
        // exercises the actual abort path more realistically
        // than a pure pre-dispatch cancel.
        std::thread::sleep(Duration::from_millis(50));
        for id in &ids {
            assert!(renderer.cancel_fetch(*id));
        }

        // Drain replies until all 100 cancels are observed.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut seen = std::collections::HashSet::new();
        while std::time::Instant::now() < deadline && seen.len() < ids.len() {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, _) = ev {
                    seen.insert(rid);
                }
            }
            if seen.len() < ids.len() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(seen.len(), ids.len(), "not all cancels acknowledged");

        // Probe: fresh fetch must succeed (counter not pinned at MAX).
        let probe_id = renderer.fetch_async(Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        });
        let probe_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut probe_got = None;
        while std::time::Instant::now() < probe_deadline && probe_got.is_none() {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == probe_id {
                        probe_got = Some(result);
                    }
                }
            }
            if probe_got.is_none() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let probe_resp = probe_got
            .expect("probe fetch starved — inflight counter saturated by cancel-spam")
            .unwrap();
        assert_eq!(probe_resp.status, 200);

        np.shutdown();
        probe_thread.join().unwrap();
    }

    #[test]
    fn cancel_fetch_unknown_id_is_idempotent() {
        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();
        // Allocate an id never sent as a Fetch — broker still posts an
        // aborted reply (renderer-side dedupe handles the mismatch).
        let id = FetchId::next();
        assert!(renderer.cancel_fetch(id));

        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let mut got = None;
        while std::time::Instant::now() < deadline {
            for ev in renderer.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == id {
                        got = Some(result);
                    }
                }
            }
            if got.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let err = got.expect("aborted reply not delivered").unwrap_err();
        assert!(err.contains("aborted"));

        np.shutdown();
    }

    /// Cross-client cancel isolation (Copilot R1, broker.rs):
    /// renderer A cannot cancel renderer B's in-flight fetch by
    /// sending `CancelFetch` with B's `FetchId`.  Pre-fix the
    /// `cancel_tokens` map was keyed only by `FetchId` so A's
    /// cancel triggered B's [`crate::CancelHandle`] — the worker
    /// suppressed its own reply on observing
    /// `NetErrorKind::Cancelled` and the synthetic `Err("aborted")`
    /// reply was misrouted to A, leaving B's promise permanently
    /// pending.  Post-fix the map is keyed by `(cid, FetchId)`,
    /// so A's cancel is a no-op against B's fetch and B receives
    /// the worker's real reply on completion.
    #[test]
    fn cancel_fetch_from_non_owner_is_isolated() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            // Brief delay so the cross-client cancel races
            // against an actually-in-flight request, not one that
            // already completed.
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            std::thread::sleep(Duration::from_millis(80));
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
            );
        });

        let np = spawn_network_process(test_client());
        let owner = np.create_renderer_handle();
        let attacker = np.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", server_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };
        let owner_id = owner.fetch_async(request);
        // Yield so the worker enters transport.send before the
        // cross-client cancel arrives.
        std::thread::sleep(Duration::from_millis(20));

        // Attacker tries to cancel owner's FetchId.  Broker
        // accepts the message but the (attacker, fetch_id) lookup
        // misses, so the underlying CancelHandle is NOT triggered.
        // The synthetic aborted reply goes back to the attacker
        // and is silently dropped (attacker has no matching
        // pending entry).
        assert!(attacker.cancel_fetch(owner_id));

        // Owner must observe a successful reply — the worker
        // wasn't actually cancelled.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut owner_result = None;
        while std::time::Instant::now() < deadline && owner_result.is_none() {
            for ev in owner.drain_events() {
                if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                    if rid == owner_id {
                        owner_result = Some(result);
                    }
                }
            }
            if owner_result.is_none() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let resp = owner_result
            .expect("owner did not receive any reply — cross-client cancel hit owner's fetch")
            .expect("owner saw aborted error — cross-client cancel triggered owner's CancelHandle");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body.as_ref(), b"hello");

        // Attacker may have observed the synthetic aborted reply
        // (harmless — its pending_fetches table never had this
        // id), but the *owner* must not have seen one in addition
        // to the success.
        np.shutdown();
        server_thread.join().unwrap();
    }

    /// Regression for Copilot R2 (broker.rs cancel_map leak on
    /// worker panic): the worker thread must remove its
    /// `(cid, fetch_id)` entry from [`CancelMap`] even on
    /// unwind, otherwise an `expect()` panic anywhere in the
    /// hot path leaks the entry and grows the map past its
    /// `MAX_CONCURRENT_FETCHES` bound.
    ///
    /// We can't easily force a panic inside the live worker
    /// without test-only inject points, so we exercise the
    /// guard directly: insert an entry, drop the guard via
    /// `catch_unwind` after deliberately panicking, then
    /// verify the map is empty.
    #[test]
    fn cancel_map_entry_guard_removes_on_panic() {
        let map: CancelMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let key = (42_u64, FetchId::next());
        map.lock().unwrap().insert(key, crate::CancelHandle::new());
        assert_eq!(map.lock().unwrap().len(), 1);

        let map_for_worker = Arc::clone(&map);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _entry = CancelMapEntryGuard {
                map: map_for_worker,
                key,
            };
            panic!("simulated worker panic");
        }));
        assert!(result.is_err(), "panic was caught");
        // Guard's Drop ran during unwind → entry removed.
        assert!(
            map.lock().unwrap().is_empty(),
            "CancelMapEntryGuard leaked entry on panic"
        );
    }

    /// Sibling assertion: the guard removes the entry on
    /// normal scope exit too (the success path).
    #[test]
    fn cancel_map_entry_guard_removes_on_normal_drop() {
        let map: CancelMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let key = (7_u64, FetchId::next());
        map.lock().unwrap().insert(key, crate::CancelHandle::new());
        {
            let _entry = CancelMapEntryGuard {
                map: Arc::clone(&map),
                key,
            };
            // No panic — guard drops at end of block.
        }
        assert!(map.lock().unwrap().is_empty());
    }

    // -------------------------------------------------------------------
    // drain_fetch_responses_only — partition-in-place partial drain
    // (slot #6.8).  Replaces the prior `drain_events + rebuffer_events`
    // pattern that the elidex-js VM's `tick_network` used.
    // -------------------------------------------------------------------

    /// Construct a `NetworkHandle` whose response channel is held
    /// open by the returned `Sender`, so a test can inject events
    /// through `response_tx.send(...)` and exercise the channel
    /// `try_recv` branch of `drain_fetch_responses_only`.
    /// `disconnected()` drops its `response_tx` immediately, so its
    /// `try_recv` returns `Disconnected` and skips the channel
    /// branch entirely — fine for buffered-only tests, useless for
    /// channel-arrival tests.
    fn handle_with_injectable_channel(
    ) -> (NetworkHandle, crossbeam_channel::Sender<NetworkToRenderer>) {
        let (request_tx, _request_rx) = crossbeam_channel::unbounded();
        let (control_tx, _control_rx) = crossbeam_channel::unbounded();
        let (response_tx, response_rx) = crossbeam_channel::unbounded();
        let handle = NetworkHandle {
            client_id: 0,
            request_tx,
            control_tx,
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            #[cfg(feature = "test-hooks")]
            mock_responses: None,
            #[cfg(feature = "test-hooks")]
            recorded_requests: None,
        };
        (handle, response_tx)
    }

    fn ok_response() -> Response {
        let url = url::Url::parse("http://example.com/r").unwrap();
        Response {
            status: 200,
            headers: Vec::new(),
            body: bytes::Bytes::from_static(b"ok"),
            url: url.clone(),
            version: crate::HttpVersion::H1,
            url_list: vec![url],
            is_redirect_tainted: false,
            credentialed_network: false,
        }
    }

    #[test]
    fn drain_fetch_responses_only_empty() {
        let renderer = NetworkHandle::disconnected();
        let fetches = renderer.drain_fetch_responses_only();
        assert!(fetches.is_empty());
        assert!(renderer.drain_events().is_empty());
    }

    #[test]
    fn drain_fetch_responses_only_partitions_buffered_events_in_place() {
        // Pre-populate `buffered` with [WS, Fetch, WS, Fetch, SSE].
        // After the partial drain, the two Fetch entries come back
        // (in arrival order), and the remaining buffer is
        // [WS, WS, SSE] — non-fetch events keep their original
        // relative order so a sibling consumer's later
        // `drain_events` sees the same sequence the broker
        // produced.
        let renderer = NetworkHandle::disconnected();
        let fetch_a = FetchId::next();
        let fetch_b = FetchId::next();
        renderer.rebuffer_events(vec![
            NetworkToRenderer::WebSocketEvent(1, crate::ws::WsEvent::TextMessage("a".into())),
            NetworkToRenderer::FetchResponse(fetch_a, Ok(ok_response())),
            NetworkToRenderer::WebSocketEvent(1, crate::ws::WsEvent::TextMessage("b".into())),
            NetworkToRenderer::FetchResponse(fetch_b, Err("boom".into())),
            NetworkToRenderer::EventSourceEvent(2, crate::sse::SseEvent::Connected),
        ]);

        let fetches = renderer.drain_fetch_responses_only();
        let ids: Vec<FetchId> = fetches.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![fetch_a, fetch_b]);
        assert!(fetches[0].1.is_ok());
        assert!(fetches[1].1.is_err());

        let leftover = renderer.drain_events();
        assert_eq!(leftover.len(), 3);
        match &leftover[0] {
            NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                assert_eq!(s, "a");
            }
            other => panic!("expected WS('a'), got {other:?}"),
        }
        match &leftover[1] {
            NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                assert_eq!(s, "b");
            }
            other => panic!("expected WS('b'), got {other:?}"),
        }
        assert!(matches!(
            leftover[2],
            NetworkToRenderer::EventSourceEvent(_, _)
        ));
    }

    #[test]
    fn drain_fetch_responses_only_partitions_channel_arrivals() {
        // Inject events through the live response channel so the
        // `try_recv` branch is exercised (buffered is empty).
        let (renderer, response_tx) = handle_with_injectable_channel();
        let fetch_a = FetchId::next();
        let fetch_b = FetchId::next();
        response_tx
            .send(NetworkToRenderer::WebSocketEvent(
                3,
                crate::ws::WsEvent::TextMessage("x".into()),
            ))
            .unwrap();
        response_tx
            .send(NetworkToRenderer::FetchResponse(fetch_a, Ok(ok_response())))
            .unwrap();
        response_tx
            .send(NetworkToRenderer::FetchResponse(fetch_b, Ok(ok_response())))
            .unwrap();
        response_tx
            .send(NetworkToRenderer::WebSocketEvent(
                3,
                crate::ws::WsEvent::TextMessage("y".into()),
            ))
            .unwrap();

        let fetches = renderer.drain_fetch_responses_only();
        let ids: Vec<FetchId> = fetches.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![fetch_a, fetch_b]);

        let leftover = renderer.drain_events();
        assert_eq!(leftover.len(), 2);
        for ev in &leftover {
            assert!(matches!(ev, NetworkToRenderer::WebSocketEvent(_, _)));
        }
    }

    #[test]
    fn drain_fetch_responses_only_buffered_precedes_channel_arrivals() {
        // The order-of-fetches contract: previously buffered
        // entries come first, then channel arrivals, mirroring
        // `drain_events`'s buffered-then-try_recv walk.  Required
        // so the partial-drain replacement preserves the same
        // arrival order the VM previously observed.
        let (renderer, response_tx) = handle_with_injectable_channel();
        let buffered_id = FetchId::next();
        let arrival_id = FetchId::next();
        renderer.rebuffer_events(vec![NetworkToRenderer::FetchResponse(
            buffered_id,
            Ok(ok_response()),
        )]);
        response_tx
            .send(NetworkToRenderer::FetchResponse(
                arrival_id,
                Ok(ok_response()),
            ))
            .unwrap();

        let fetches = renderer.drain_fetch_responses_only();
        let ids: Vec<FetchId> = fetches.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![buffered_id, arrival_id]);
        assert!(renderer.drain_events().is_empty());
    }

    #[test]
    fn drain_fetch_responses_only_keeps_buffered_non_fetch_before_channel_non_fetch() {
        // Sibling guarantee for the non-fetch side: the relative
        // order of *non-fetch* events across (prior buffer, channel
        // arrivals) is preserved in the new buffer, even when the
        // channel produces non-fetch events too.
        let (renderer, response_tx) = handle_with_injectable_channel();
        renderer.rebuffer_events(vec![NetworkToRenderer::WebSocketEvent(
            5,
            crate::ws::WsEvent::TextMessage("buffered".into()),
        )]);
        response_tx
            .send(NetworkToRenderer::FetchResponse(
                FetchId::next(),
                Ok(ok_response()),
            ))
            .unwrap();
        response_tx
            .send(NetworkToRenderer::WebSocketEvent(
                5,
                crate::ws::WsEvent::TextMessage("arrival".into()),
            ))
            .unwrap();

        let fetches = renderer.drain_fetch_responses_only();
        assert_eq!(fetches.len(), 1);
        let leftover = renderer.drain_events();
        assert_eq!(leftover.len(), 2);
        match &leftover[0] {
            NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                assert_eq!(s, "buffered");
            }
            other => panic!("expected WS('buffered'), got {other:?}"),
        }
        match &leftover[1] {
            NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                assert_eq!(s, "arrival");
            }
            other => panic!("expected WS('arrival'), got {other:?}"),
        }
    }
}
