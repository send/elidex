//! Handle types — [`NetworkProcessHandle`] (browser side) and
//! [`NetworkHandle`] (renderer / content thread side) — plus the
//! [`spawn_network_process`] entry point that boots the dedicated
//! Network Process thread.
//!
//! `NetworkHandle` is the only API surface a content thread sees;
//! all network access is mediated through one of its methods so
//! the renderer can run in an OS-level sandbox without direct IO
//! capability.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::{NetClient, Request, Response};

use super::dispatch::network_process_main;
use super::register::register_with_ack;
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
    ///
    /// **Blocks** until the broker acknowledges the registration
    /// (slot #10.6c).  This closes the cross-channel race where
    /// a Fetch on `request_tx` could be observed by the broker
    /// BEFORE the matching `RegisterRenderer` on `control_tx`
    /// — the broker's stale-cid gate would have silently dropped
    /// the Fetch and the renderer-side `pending_fetches[id]`
    /// Promise would have stayed pending forever.  The wait is
    /// bounded by an internal 500 ms `REGISTER_ACK_TIMEOUT`;
    /// on timeout or channel disconnect the call falls through
    /// to a **pre-unregistered** [`NetworkHandle`].  The
    /// pre-unregistered fallback uses slot #10.6b's same
    /// short-circuit machinery, which differs by method:
    /// `fetch_async` and `fetch_blocking` emit the synthetic
    /// terminal `Err("renderer unregistered")` reply (buffered
    /// for `fetch_async`, returned directly for
    /// `fetch_blocking`) so the renderer-side
    /// `pending_fetches[id]` Promise settles; `cancel_fetch`
    /// and `send` short-circuit by returning `false` (no
    /// synthetic event — the caller's bool return is the
    /// signal).  Healthy registration latency is sub-
    /// millisecond; the 500 ms ceiling is tight by design
    /// because this method is called from browser-thread paths
    /// (`App::open_new_tab`, `sw_coordinator::register`) that
    /// do not tolerate multi-second event-loop freezes —
    /// Copilot R1.  On timeout the helper also emits a follow-
    /// up `UnregisterRenderer` so a stalled-but-alive broker
    /// that resumes draining later cleans up the orphan entry
    /// itself (no reliance on the caller's eventual handle
    /// `Drop`).
    #[must_use]
    pub fn create_renderer_handle(&self) -> NetworkHandle {
        let client_id = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = crossbeam_channel::unbounded();
        // Slot #10.6c R9: the broker stores a clone of this
        // atomic in its `clients` map and flips it to `true`
        // BEFORE emitting `RendererUnregistered`, so a
        // concurrent `create_sibling_handle` against this cid
        // can detect the unregister with an O(1) load instead
        // of having to drain the renderer's response channel.
        let unregistered = Arc::new(AtomicBool::new(false));
        let pre_unregistered = register_with_ack(
            &self.control_tx,
            client_id,
            response_tx,
            Arc::clone(&unregistered),
            "create_renderer_handle",
        );
        if pre_unregistered {
            // Slot #10.6c: the ack was lost (timeout /
            // disconnect).  Set the flag locally so this
            // handle's own short-circuit machinery fires from
            // the first call.  In the timeout case the broker
            // may eventually receive Register + the follow-up
            // UnregisterRenderer and would also flip the flag
            // through its `clients` clone (idempotent on
            // re-set), but we cannot rely on that — broker may
            // be hung indefinitely or already gone.
            unregistered.store(true, Ordering::Release);
        }

        NetworkHandle {
            client_id,
            request_tx: self.request_tx.clone(),
            control_tx: self.control_tx.clone(),
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            unregistered,
            outstanding_fetches: std::cell::RefCell::new(HashSet::new()),
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
    /// Slot #10.6b: set to `true` once this handle's drain has
    /// observed the [`NetworkToRenderer::RendererUnregistered`]
    /// back-edge.  Sender on the broker side is the
    /// `UnregisterRenderer` / broker `Shutdown` control branch
    /// in `dispatch::handle_control`; reader is every method
    /// that would otherwise enqueue work onto the now-stale
    /// `request_tx` (`fetch_async`, `fetch_blocking`,
    /// `cancel_fetch`, `send`).  Wrapped in `Arc<AtomicBool>`
    /// for forward compatibility: `NetworkHandle` is single-
    /// owner today (no `Clone` impl), but the broker hands the
    /// inner atomic to anything that needs to observe the flag
    /// from another owner — for example a future
    /// `Arc<NetworkHandle>` shared with a Web Worker thread or
    /// a side-table on the broker that cross-references the
    /// flag for diagnostics.  The `Arc` keeps the field correct
    /// under that future even if `NetworkHandle` itself stays
    /// `!Clone`.  Sibling handles
    /// ([`Self::create_sibling_handle`]) and disconnected
    /// handles get their own independent flag because each has
    /// its own response channel and only sees its own cid's
    /// marker.  Ordering: drain stores with `Release`, the
    /// short-circuit reads with `Acquire` — pre-empts the
    /// `SeqCst` review nit (no other shared state needs to be
    /// observed in any particular order relative to this flag).
    pub(super) unregistered: Arc<AtomicBool>,
    /// Slot #10.6b: in-flight [`FetchId`]s that the renderer has
    /// submitted via `fetch_async` / `fetch_blocking` and for
    /// which no terminal `FetchResponse` has yet been observed
    /// on the response channel.  Used to close the
    /// `synthesise_aborted_replies_for_client → cancel →
    /// clients.remove` race window in the broker's
    /// `UnregisterRenderer` path: a fetch submitted *between*
    /// the synthesise step (1) and the `clients.remove` step
    /// (4) lands in `request_rx` and hits the broker's stale-
    /// cid gate (`handle_request` early-return) — the broker
    /// emits no terminal event for it.  When the renderer's
    /// drain observes [`NetworkToRenderer::RendererUnregistered`],
    /// every id still in this set gets a synthetic `Err` reply
    /// pushed onto `buffered` so the renderer-side
    /// `pending_fetches[id]` Promise settles on the very next
    /// drain.  `drain_events` /
    /// `drain_fetch_responses_only` route every observed
    /// `FetchResponse(id, _)` through `process_response` which
    /// removes `id` from this set before forwarding the event.
    /// `RefCell` (not `Mutex`) because the renderer side is
    /// single-threaded today; the borrow regions are short and
    /// non-nested by construction (see `process_response` /
    /// `fetch_async` — each call site borrows once for one
    /// `insert` / `remove` / `drain` and drops the guard
    /// immediately).
    pub(super) outstanding_fetches: std::cell::RefCell<HashSet<FetchId>>,
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
            // Disconnected handles never see a broker-side
            // marker (the response channel is closed at
            // construction), so the flag stays `false`
            // forever — `send` already fails silently via the
            // dropped `request_rx`, so the short-circuit is a
            // no-op for this path.
            unregistered: Arc::new(AtomicBool::new(false)),
            outstanding_fetches: std::cell::RefCell::new(HashSet::new()),
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
    ///
    /// **Blocks** on the broker's `RegisterRenderer` ack with the
    /// same 500 ms timeout semantics as
    /// [`NetworkProcessHandle::create_renderer_handle`] (slot
    /// #10.6c).  The tight ceiling matters here too: this helper
    /// is called from the JS `new Worker()` constructor (via
    /// `HostBridge::create_sibling_network_handle` →
    /// `globals::worker_constructor`), which the spec expects
    /// to return effectively-immediately — Copilot R1.  If the
    /// parent's `NetworkProcessHandle` is mid-shutdown when this
    /// is called, the ack times out / the channel disconnects
    /// and the sibling is returned in the **pre-unregistered**
    /// state with the same dual-form short-circuit as
    /// [`NetworkProcessHandle::create_renderer_handle`]:
    /// `fetch_async` / `fetch_blocking` synthesise the terminal
    /// `Err("renderer unregistered")`, while `cancel_fetch` /
    /// `send` simply return `false`.  This is the correct
    /// behaviour: panicking would destabilise renderer threads
    /// that legitimately race shutdown.  The helper also emits
    /// a follow-up `UnregisterRenderer` on timeout so a stalled-
    /// but-alive broker cleans up the orphan registration
    /// itself once it resumes draining.
    ///
    /// **R6/R7 inheritance**: when this method is called against
    /// a parent that is already unregistered (its
    /// `unregistered` flag is set, OR its response channel
    /// has the [`NetworkToRenderer::RendererUnregistered`]
    /// marker queued but not yet drained), the call short-
    /// circuits without ever talking to the broker — the
    /// returned sibling enters the world in the same pre-
    /// unregistered state with the same dual-form contract
    /// described above.  This avoids the embedder-visible
    /// inconsistency of a broken parent spawning a working
    /// child against a broker that may have recovered between
    /// the parent's teardown and this call.
    #[must_use]
    pub fn create_sibling_handle(&self) -> Self {
        let client_id = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = crossbeam_channel::unbounded();
        let unregistered = Arc::new(AtomicBool::new(false));

        // Slot #10.6c (Copilot R6/R7/R9): if the parent has
        // already been marked unregistered, short-circuit
        // sibling creation here without touching the broker.
        // Without this gate, routing through `register_with_ack`
        // would either (a) fail at send time when the broker is
        // gone (yielding the same pre-unregistered result via
        // the SendError path, just with a wasted Register
        // attempt and an orphan ack channel) OR (b) succeed
        // against a broker that has recovered between the
        // parent's unregister and now, leaving the embedder
        // with a live sibling whose parent's every operation
        // short-circuits as unregistered — an inconsistent
        // contract for callers who treat sibling spawning as
        // "child of parent".
        //
        // **R9 architectural fix**: detection is an O(1) atomic
        // load, not a channel drain.  The broker holds a clone
        // of [`Self::unregistered`] in its `clients` map (slot
        // #10.6c R9 `ClientEntry`) and stores `true` (Release)
        // BEFORE emitting the
        // [`NetworkToRenderer::RendererUnregistered`] marker on
        // the parent's response channel — so this `Acquire`
        // load synchronises with the broker's flip without ever
        // touching the channel queue.  This replaces the earlier
        // R7/R8 bounded-drain probe (which was unbounded in
        // WS/SSE backlog size and could trigger an
        // `outstanding_fetches` synthesis pass on hitting the
        // marker — Copilot R9 F1/F2).  Renderer-side
        // `process_response` still stores Release on the marker
        // for defence-in-depth, but the broker-side store is
        // the load-bearing one for cross-handle observation.
        let pre_unregistered = if self.unregistered.load(Ordering::Acquire) {
            // Drop response_tx implicitly at end of scope; no
            // one ever sends on it because we don't hand it to
            // the broker.
            drop(response_tx);
            true
        } else {
            register_with_ack(
                &self.control_tx,
                client_id,
                response_tx,
                Arc::clone(&unregistered),
                "create_sibling_handle",
            )
        };
        if pre_unregistered {
            unregistered.store(true, Ordering::Release);
        }

        Self {
            client_id,
            request_tx: self.request_tx.clone(),
            control_tx: self.control_tx.clone(),
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
            // Slot #10.6b/c: siblings get fresh flag state.
            // Each has its own response channel + cid, so the
            // parent's unregister marker is delivered only on
            // the parent's response channel, not here.  Sharing
            // the flag wholesale would incorrectly disable the
            // sibling forever on the parent's teardown.  The
            // flag is pre-set to `true` only at construction
            // time, in three cases: (i) the slot #10.6c ack
            // handshake failed (broker hung / gone), (ii) the
            // parent itself was already unregistered when this
            // sibling was constructed (R6 inheritance — the
            // sibling enters the world in the same broken state
            // its parent was in), or (iii) the broker stored
            // `true` into our cloned atomic via the slot #10.6c
            // R9 `ClientEntry` path before clients.remove (e.g.
            // racing UnregisterRenderer arriving on control_tx
            // shortly after our register).  After construction
            // the sibling's flag evolves independently via its
            // own response channel.
            unregistered,
            outstanding_fetches: std::cell::RefCell::new(HashSet::new()),
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
    ///
    /// Slot #10.6b: when this handle has already observed the
    /// broker's [`NetworkToRenderer::RendererUnregistered`]
    /// back-edge, the call short-circuits — a synthetic
    /// `Err("renderer unregistered")` reply is buffered under
    /// the new id so the renderer-side
    /// `pending_fetches[id]` Promise settles on the next drain
    /// without round-tripping through a broker that no longer
    /// has a `clients` entry for us.  The race window where
    /// the broker has started teardown but the marker has not
    /// yet been observed locally is closed by the private
    /// `outstanding_fetches` tracking — a `Fetch` sent in
    /// that window is dropped by the broker's stale-cid gate,
    /// but the id stays in `outstanding_fetches` until the
    /// marker arrives, at which point the private
    /// `process_response` helper synthesises a terminal
    /// `Err` for it.
    pub fn fetch_async(&self, request: Request) -> FetchId {
        let fetch_id = FetchId::next();

        #[cfg(feature = "test-hooks")]
        if let Some(ref map) = self.mock_responses {
            // Mock short-circuit runs BEFORE the unregister
            // gate so the test-hooks path stays oblivious to
            // the broker-lifecycle layer — mock handles are
            // disconnected() copies whose `unregistered` flag
            // never flips, but skipping the gate altogether
            // keeps this branch a pure deterministic answer.
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

        // Slot #10.6b layer 1: handles whose drain has already
        // observed `RendererUnregistered` short-circuit here.
        if self.check_unregistered() {
            self.buffered
                .borrow_mut()
                .push(NetworkToRenderer::FetchResponse(
                    fetch_id,
                    Err("renderer unregistered".into()),
                ));
            return fetch_id;
        }

        // Slot #10.6b layer 2: track BEFORE the send so a
        // marker that arrives between this insert and the next
        // drain triggers the straggler-synthesis path for
        // `fetch_id`.  Without the pre-send insert, a fetch
        // dropped by the broker's stale-cid gate would have no
        // outstanding entry to recover on marker arrival,
        // leaving the Promise stranded.
        self.outstanding_fetches.borrow_mut().insert(fetch_id);

        // Send may fail when the broker has shut down or the handle
        // was created via `disconnected()`; in that case buffer a
        // synthetic `Err` reply so the renderer's `pending_fetches`
        // table can settle on the next `drain_events()` instead of
        // leaking the entry forever (R1.1).
        if self
            .request_tx
            .send((self.client_id, RendererToNetwork::Fetch(fetch_id, request)))
            .is_err()
        {
            self.outstanding_fetches.borrow_mut().remove(&fetch_id);
            self.buffered
                .borrow_mut()
                .push(NetworkToRenderer::FetchResponse(
                    fetch_id,
                    Err("network process disconnected".into()),
                ));
        }
        fetch_id
    }

    /// Cancel an in-flight fetch.  Idempotent: calling on a
    /// completed / unknown id is harmless because the broker
    /// merely posts a synthesised `Err("aborted")` reply for that
    /// id and the renderer's pending-fetch table dedupes late
    /// arrivals (the second `remove` returns `None`).
    ///
    /// Cross-renderer isolation: the broker's cancel-token map is
    /// keyed by `(client_id, FetchId)`, so a renderer cannot
    /// cancel another renderer's in-flight fetch by guessing its
    /// id — a non-owner cancel becomes a no-op against the actual
    /// worker, and the synthetic aborted reply is delivered to
    /// the cancelling renderer where the absent pending-fetch
    /// entry causes it to be silently dropped.
    ///
    /// On the worker side, the broker drives true cancellation:
    /// the per-fetch `CancelHandle` aborts the in-flight hyper
    /// future immediately, the `FetchInflightGuard` releases the
    /// `MAX_CONCURRENT_FETCHES` slot promptly, and the worker
    /// suppresses its own duplicate reply on observing
    /// `NetErrorKind::Cancelled` so the renderer sees exactly one
    /// terminal reply per fetch.
    ///
    /// Returns `true` if the cancel was queued, `false` if the
    /// broker is disconnected, or `false` once this handle has
    /// observed the broker's
    /// [`NetworkToRenderer::RendererUnregistered`] back-edge —
    /// in that case the renderer-side `pending_fetches[id]`
    /// Promise is already being settled by the synthesised
    /// `Err("renderer unregistered")` reply (slot #10.6b's
    /// straggler synthesis or the broker's
    /// `synthesise_aborted_replies_for_client` step), so a
    /// queued cancel would race the dead broker for no benefit.
    /// The short-circuit is inherited through [`Self::send`] —
    /// duplicating the check here would just double-drain the
    /// response channel on the cancel hot path (Copilot R1
    /// HX2).
    pub fn cancel_fetch(&self, id: FetchId) -> bool {
        self.send(RendererToNetwork::CancelFetch(id))
    }

    /// Send a blocking fetch request.
    ///
    /// The content thread blocks until the fetch completes (or times out
    /// at 30 seconds). Any WS/SSE events received while waiting are
    /// buffered internally and returned by the next [`drain_events`](Self::drain_events)
    /// call.
    ///
    /// Slot #10.6b: short-circuits with
    /// `Err("renderer unregistered")` if the handle has already
    /// observed the broker's
    /// [`NetworkToRenderer::RendererUnregistered`] back-edge.
    /// If the marker arrives mid-blocking-call (the broker tore
    /// us down between `request_tx.send` and the recv loop), the
    /// loop observes it inline, flips the flag, drains stragglers
    /// (every other tracked fetch) into `buffered`, and returns
    /// the unregistered Err for this call's `fetch_id`.  The
    /// buffered stragglers settle on the next `drain_events`,
    /// so no Promise is leaked across the teardown.
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

        // Slot #10.6b layer 1: short-circuit a handle whose
        // drain has already observed RendererUnregistered.
        if self.check_unregistered() {
            return Err("renderer unregistered".into());
        }

        let fetch_id = FetchId::next();
        // Track BEFORE send for the same race-window reason as
        // `fetch_async`.
        self.outstanding_fetches.borrow_mut().insert(fetch_id);
        let _ = self
            .request_tx
            .send((self.client_id, RendererToNetwork::Fetch(fetch_id, request)));

        let mut buf = self.buffered.borrow_mut();
        loop {
            match self.response_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(NetworkToRenderer::FetchResponse(id, result)) if id == fetch_id => {
                    self.outstanding_fetches.borrow_mut().remove(&id);
                    return result;
                }
                Ok(NetworkToRenderer::FetchResponse(id, result)) => {
                    // Some other fetch's reply arrived while we
                    // were blocked on ours; route it through
                    // the bookkeeping (drops `id` from
                    // `outstanding_fetches`) and buffer for the
                    // next drain.
                    self.outstanding_fetches.borrow_mut().remove(&id);
                    buf.push(NetworkToRenderer::FetchResponse(id, result));
                }
                Ok(NetworkToRenderer::RendererUnregistered) => {
                    // Broker tore us down mid-call.  Flip the
                    // flag + synthesise terminal Err for every
                    // OTHER outstanding fetch (this call's
                    // `fetch_id` is returned directly below; we
                    // skip it in the straggler synthesis to
                    // avoid pushing a duplicate event into
                    // `buffered` when the caller's Promise is
                    // already being settled by our return).
                    self.unregistered.store(true, Ordering::Release);
                    // Sort by ascending `FetchId` (monotonic
                    // submission counter) so the synthetic
                    // straggler tail is deterministic — same
                    // contract as `process_response` (Copilot
                    // R2 HX4).
                    let mut stragglers: Vec<FetchId> =
                        self.outstanding_fetches.borrow_mut().drain().collect();
                    stragglers.sort_unstable_by_key(|id| id.0);
                    for sid in stragglers {
                        if sid == fetch_id {
                            continue;
                        }
                        buf.push(NetworkToRenderer::FetchResponse(
                            sid,
                            Err("renderer unregistered".into()),
                        ));
                    }
                    return Err("renderer unregistered".into());
                }
                Ok(other) => buf.push(other),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    self.outstanding_fetches.borrow_mut().remove(&fetch_id);
                    return Err("fetch timeout (30s)".into());
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    self.outstanding_fetches.borrow_mut().remove(&fetch_id);
                    return Err("network process disconnected".into());
                }
            }
        }
    }

    /// Send a message to the Network Process without waiting for a response.
    ///
    /// Returns `true` if the message was queued, `false` if the broker is disconnected.
    ///
    /// Slot #10.6b: short-circuits with `false` once this
    /// handle's drain has observed the broker's
    /// [`NetworkToRenderer::RendererUnregistered`] back-edge,
    /// so callers don't enqueue work onto a broker that has
    /// already torn down our `clients` entry.
    pub fn send(&self, msg: RendererToNetwork) -> bool {
        if self.check_unregistered() {
            return false;
        }
        self.request_tx.send((self.client_id, msg)).is_ok()
    }

    /// Slot #10.6b helper: drain pending events on the response
    /// channel through [`Self::process_response`] and return
    /// whether the renderer-side `unregistered` flag is set.
    /// Exit point for the synchronous short-circuit in
    /// [`Self::fetch_async`] / [`Self::fetch_blocking`] /
    /// [`Self::cancel_fetch`] / [`Self::send`].
    ///
    /// Fast path: if the flag is already `true`, return
    /// immediately without touching the channel — the broker
    /// emits the marker exactly once per cid, so once observed
    /// re-polling can never flip the flag back.
    fn check_unregistered(&self) -> bool {
        if self.unregistered.load(Ordering::Acquire) {
            return true;
        }
        // Drain the channel into `buffered` so a marker that
        // arrived before this call has a chance to flip the
        // flag.  Must hold one borrow of `buffered` for the
        // whole loop because otherwise an interleaved
        // `process_response` straggler-synthesis push would
        // need a fresh borrow per call (RefCell would still
        // allow it — they're sequential — but the single
        // borrow is cheaper).
        let mut buf = self.buffered.borrow_mut();
        while let Ok(evt) = self.response_rx.try_recv() {
            self.process_response(evt, &mut |e| buf.push(e));
        }
        drop(buf);
        self.unregistered.load(Ordering::Acquire)
    }

    /// Slot #10.6b helper: route an event through the
    /// `outstanding_fetches` bookkeeping + the
    /// [`NetworkToRenderer::RendererUnregistered`] back-edge.
    /// `emit` receives every event the caller should forward
    /// (i.e. fetch responses, WS events, SSE events); the
    /// marker itself is consumed internally and replaced by
    /// synthetic `FetchResponse(id, Err("renderer unregistered"))`
    /// events for every still-tracked race-window fetch.
    ///
    /// Borrow scope: only `outstanding_fetches` is borrowed
    /// (briefly, for `remove` / `drain`); the caller is free
    /// to hold `buffered` borrowed across the call.
    pub(super) fn process_response<F>(&self, evt: NetworkToRenderer, emit: &mut F)
    where
        F: FnMut(NetworkToRenderer),
    {
        match evt {
            NetworkToRenderer::FetchResponse(id, result) => {
                self.outstanding_fetches.borrow_mut().remove(&id);
                emit(NetworkToRenderer::FetchResponse(id, result));
            }
            NetworkToRenderer::RendererUnregistered => {
                // Release: pairs with the Acquire load in
                // `check_unregistered`.  No other shared state
                // needs to be observed in any particular order
                // relative to the flag, so SeqCst is unnecessary.
                self.unregistered.store(true, Ordering::Release);
                // Drain ids the broker silently dropped via
                // its stale-cid gate (race-window submits
                // between the broker's
                // `synthesise_aborted_replies_for_client` step
                // and `clients.remove`), and emit a terminal
                // `Err` for each so the renderer-side
                // `pending_fetches[id]` Promise settles.
                //
                // Sort the drained ids by ascending `FetchId`
                // before emit so the synthetic tail is
                // deterministic.  `HashSet::drain` order is
                // non-deterministic; without the sort the
                // `drain_events` /
                // `drain_fetch_responses_only` "fetch replies
                // in arrival order" doc contract would be
                // violated for race-window stragglers
                // (Copilot R2 HX4).  `FetchId` is a monotonic
                // counter (`FETCH_ID_COUNTER` in `mod.rs`), so
                // ascending id order matches submission
                // order — the closest deterministic analogue
                // to "arrival order" for events the broker
                // never delivered.
                let mut stragglers: Vec<FetchId> =
                    self.outstanding_fetches.borrow_mut().drain().collect();
                stragglers.sort_unstable_by_key(|id| id.0);
                for id in stragglers {
                    emit(NetworkToRenderer::FetchResponse(
                        id,
                        Err("renderer unregistered".into()),
                    ));
                }
                // The marker itself is internal — never
                // surfaced to JS / embedder code.
            }
            other => emit(other),
        }
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
