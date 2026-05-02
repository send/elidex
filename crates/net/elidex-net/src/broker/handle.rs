//! Handle types — [`NetworkProcessHandle`] (browser side) and
//! [`NetworkHandle`] (renderer / content thread side) — plus the
//! [`spawn_network_process`] entry point that boots the dedicated
//! Network Process thread.
//!
//! `NetworkHandle` is the only API surface a content thread sees;
//! all network access is mediated through one of its methods so
//! the renderer can run in an OS-level sandbox without direct IO
//! capability.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::{NetClient, Request, Response};

use super::dispatch::network_process_main;
use super::{
    FetchId, NetworkProcessControl, NetworkToRenderer, RendererToNetwork, CLIENT_ID_COUNTER,
};

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
    pub(super) client_id: u64,
    /// Shared request channel (all renderers → Network Process).
    pub(super) request_tx: crossbeam_channel::Sender<(u64, RendererToNetwork)>,
    /// Control channel for registering sibling handles (e.g., for workers).
    pub(super) control_tx: crossbeam_channel::Sender<NetworkProcessControl>,
    /// Dedicated response channel (Network Process → this renderer).
    pub(super) response_rx: crossbeam_channel::Receiver<NetworkToRenderer>,
    /// Events buffered during blocking fetch (drained by `drain_events()`).
    /// Uses `RefCell` for interior mutability (content thread is single-threaded).
    pub(super) buffered: std::cell::RefCell<Vec<NetworkToRenderer>>,
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
    pub(super) mock_responses:
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
    pub(super) recorded_requests: Option<std::cell::RefCell<Vec<Request>>>,
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

    /// Send a message to the Network Process without waiting for a response.
    ///
    /// Returns `true` if the message was queued, `false` if the broker is disconnected.
    pub fn send(&self, msg: RendererToNetwork) -> bool {
        self.request_tx.send((self.client_id, msg)).is_ok()
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
// Network Process thread spawn
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
