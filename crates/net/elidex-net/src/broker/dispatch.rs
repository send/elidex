//! Network Process worker thread: main loop, per-renderer state
//! machine, fetch dispatcher, WS/SSE forwarders.
//!
//! [`network_process_main`] runs on the dedicated `elidex-network`
//! OS thread spawned by [`super::handle::spawn_network_process`]
//! and owns the [`NetworkProcessState`] for the broker's lifetime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{self, TryRecvError};

use crate::sse::{SseCommand, SseEvent, SseHandle};
use crate::ws::{WsCommand, WsEvent, WsHandle};
use crate::NetClient;

use super::cancel::{CancelMap, CancelMapEntryGuard, FetchInflightGuard};
use super::{FetchId, NetworkProcessControl, NetworkToRenderer, RendererToNetwork, Request};

// ---------------------------------------------------------------------------
// Worker entry point
// ---------------------------------------------------------------------------

/// Main loop of the Network Process thread.
#[allow(clippy::needless_pass_by_value)] // Owned channels consumed by the thread.
pub(super) fn network_process_main(
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

/// Maximum time [`NetworkProcessState::close_all_for_client`] waits
/// for a WS/SSE worker to exit gracefully (after sending the
/// protocol-clean close command and dropping the command sender)
/// before falling back to [`crate::CancelHandle::cancel`].
///
/// 100 ms is a generous loopback/well-connected ceiling: a
/// graceful WS close is one frame round-trip + a select tick to
/// observe the dropped command channel; SSE just needs the
/// worker's next `should_close` / `wait_or_close` poll.  Slow
/// peers fall back to cancel — the right tradeoff because the
/// renderer is being torn down regardless and the alternative
/// (unbounded join) would block `NetworkProcessHandle::shutdown`
/// for the TCP timeout window (slot #10.6a HX4).
const GRACEFUL_CLOSE_GRACE: Duration = Duration::from_millis(100);

/// Step between [`std::thread::JoinHandle::is_finished`] polls
/// during the grace window.  5 ms strikes a balance between
/// CPU cost and worst-case waste at the boundary (5 ms
/// quantisation).
const GRACEFUL_CLOSE_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Wait up to [`GRACEFUL_CLOSE_GRACE`] for ALL `pending` threads
/// to exit on their own (single shared grace window across the
/// batch); if any are still running after the grace period,
/// trigger their cancel handles as a hard fallback and then
/// `join()` each thread.  Used by
/// [`NetworkProcessState::close_all_for_client`] (slot #10.6a
/// HX10).
///
/// **Why a single shared grace window**: the broker thread
/// drives this teardown serially per renderer, so a per-handle
/// grace would compose to `GRACEFUL_CLOSE_GRACE × handle_count`
/// (up to ~25 s with the 256-connection per-document cap), during
/// which fetch dispatch / WS+SSE event forwarding for OTHER
/// renderers is blocked on the broker's main loop.  Sharing the
/// grace window across the batch keeps total broker stall to
/// `GRACEFUL_CLOSE_GRACE + max-individual-cancel-propagation`
/// regardless of handle count.
///
/// The `pending` vector pairs each thread with its
/// [`crate::CancelHandle`] so the cancel-fallback is keyed to the
/// correct worker (different workers don't share cancels).  The
/// final `join()` is unbounded by design — workers observe cancel
/// within a select tick (see `ws::io_loop::ws_io_loop` /
/// [`crate::sse::sse_io_loop`] cancel arms), so the unbounded
/// wait is bounded in practice by the cancel-propagation latency.
/// The WS reference is rendered as inline code rather than an
/// intra-doc link because `io_loop` is a `pub(super)` submodule
/// of `ws` (slot #10.6a HX26 split — Copilot R8 HX30 caught the
/// stale link).
fn join_pending_with_grace_then_cancel(
    pending: Vec<(std::thread::JoinHandle<()>, crate::CancelHandle)>,
) {
    if pending.is_empty() {
        return;
    }
    let deadline = std::time::Instant::now() + GRACEFUL_CLOSE_GRACE;
    while !pending.iter().all(|(t, _)| t.is_finished()) && std::time::Instant::now() < deadline {
        std::thread::sleep(GRACEFUL_CLOSE_POLL_INTERVAL);
    }
    for (thread, cancel) in &pending {
        if !thread.is_finished() {
            cancel.cancel();
        }
    }
    for (thread, _) in pending {
        let _ = thread.join();
    }
}

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
    /// Background teardown threads spawned by
    /// [`Self::close_all_for_client`].  Each owns a batch of
    /// `(JoinHandle<()>, CancelHandle)` pairs whose grace +
    /// cancel + join sequence runs OFF the broker thread, so a
    /// renderer with slow-to-close realtime sockets cannot
    /// inject the [`GRACEFUL_CLOSE_GRACE`] window's worth of
    /// latency into fetch dispatch / event forwarding for
    /// other renderers (slot #10.6a Copilot R3 HX14).
    /// `cleanup_finished` reaps finished entries; the broker
    /// `Shutdown` path joins remaining threads before exit so
    /// `NetworkProcessHandle::shutdown` only returns once every
    /// realtime worker is gone.
    pending_teardowns: Vec<std::thread::JoinHandle<()>>,
}

impl NetworkProcessState {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            ws_handles: HashMap::new(),
            sse_handles: HashMap::new(),
            inflight_fetches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            cancel_tokens: Arc::new(std::sync::Mutex::new(HashMap::new())),
            pending_teardowns: Vec::new(),
        }
    }

    fn handle_request(&mut self, cid: u64, msg: RendererToNetwork, client: &Arc<NetClient>) {
        // Drop messages from clients that have already been
        // unregistered.  A stale `NetworkHandle` clone (e.g. one
        // captured by a worker thread before its renderer was
        // unregistered) could otherwise spawn fetch workers
        // that consume `MAX_CONCURRENT_FETCHES` slots with no
        // response sender to deliver the reply, or open WS/SSE
        // I/O threads whose events would never reach a renderer
        // (Copilot R10 finding, pre-existing).  Cancel /
        // Shutdown for an unregistered cid are likewise no-ops
        // — the per-client tables (`ws_handles` / `sse_handles`
        // / `cancel_tokens`) are already empty for that cid
        // because `UnregisterRenderer` ran `close_all_for_client`
        // + `cancel_inflight_fetches_for` on the way out.
        //
        // **Layered defence**.  Three independent layers cover
        // distinct windows of the renderer-lifecycle race:
        //
        // 1. **Pre-register** (slot #10.6c): the
        //    [`super::handle::NetworkProcessHandle::create_renderer_handle`]
        //    /
        //    [`super::handle::NetworkHandle::create_sibling_handle`]
        //    factories now block on a one-shot ack from
        //    [`Self::handle_control`]'s `RegisterRenderer` arm
        //    before returning a usable [`super::handle::NetworkHandle`].
        //    That makes the legitimate Register-then-Fetch race
        //    impossible: any send on `request_tx` from a
        //    successfully-registered handle is happens-after the
        //    broker's `clients.insert`, so the cid is guaranteed
        //    present by the time this gate runs.
        // 2. **Post-unregister** (slot #10.6b): the renderer-side
        //    [`super::handle::NetworkHandle`] short-circuits
        //    `fetch_async` / `fetch_blocking` / `cancel_fetch` /
        //    `send` once it observes the
        //    [`NetworkToRenderer::RendererUnregistered`]
        //    back-edge, and tracks issued [`FetchId`]s in
        //    `outstanding_fetches` so any race-window fetch
        //    dropped HERE still gets a synthetic terminal `Err`
        //    reply injected via the renderer's local `buffered`
        //    queue.  Together with the order contract in
        //    [`Self::emit_renderer_unregistered`] /
        //    [`Self::synthesise_aborted_replies_for_client`]
        //    this closes the synthesise → cancel →
        //    clients.remove window.
        // 3. **This gate (defensive floor)**: with #10.6c the
        //    legitimate cross-channel race is gone, but the gate
        //    still earns its keep against (a) stale
        //    [`super::handle::NetworkHandle`] state captured in
        //    background threads (e.g. a worker that sent a Fetch
        //    before its parent was unregistered, where #10.6b's
        //    renderer-side flag hasn't been observed yet), (b)
        //    any future caller that bypasses the handle helpers
        //    and posts a `(cid, msg)` tuple directly onto
        //    `request_tx`, and (c) Fetches dispatched between
        //    [`Self::synthesise_aborted_replies_for_client`] and
        //    `clients.remove` in the broker-side teardown
        //    sequence (also covered by layer 2 stragglers, but
        //    the broker-side drop is cheaper and avoids spawning
        //    a fetch worker that would have no `tx` to deliver
        //    on).  See lesson #134 in the slot #10.6b landing
        //    memo: when adding a renderer-side gate that
        //    overlaps a broker-side gate, document the layered
        //    defence explicitly to pre-empt "delete redundant
        //    code" reviews.
        if !self.clients.contains_key(&cid) {
            return;
        }
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
                    // Send the protocol-clean Close + trigger
                    // the per-handle cancel.  The Close cmd
                    // alone is only observed via `try_recv`
                    // between `read_line` chunks, so a silent
                    // server can keep the worker parked inside
                    // `reader.read_line(&mut line).await` for
                    // up to the per-attempt 60-second timeout —
                    // even regular `eventSource.close()` from
                    // JS would not see the worker exit
                    // promptly.  The cancel arm of
                    // `read_line_with_cancel` aborts the read
                    // future immediately, so the worker exits
                    // within bounded time regardless of peer
                    // responsiveness (slot #10.6a Copilot R4
                    // HX20).
                    let _ = handle.command_tx.send(SseCommand::Close);
                    handle.cancel.cancel();
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
                ack_tx,
            } => {
                self.clients.insert(client_id, response_tx);
                // Slot #10.6c: ack AFTER insert so a renderer
                // waking from `recv_timeout` observes the cid as
                // registered.  Send is best-effort: a dropped
                // receiver (renderer abandoned the handshake on
                // its own timeout / disconnect path) is not an
                // error from the broker's perspective.  The
                // handshake closes the cross-channel race where
                // a Fetch on `request_tx` could be observed
                // before the matching Register on `control_tx`
                // (the stale-cid gate would silently drop it);
                // see [`super::NetworkProcessControl::RegisterRenderer`]
                // for the full rationale.
                let _ = ack_tx.send(());
                true
            }
            NetworkProcessControl::UnregisterRenderer { client_id } => {
                // Renderer is gone — tear down realtime channels
                // AND cancel in-flight fetches.  The fetch cancel
                // is intentional here (unlike the
                // `RendererToNetwork::Shutdown` path which is used
                // for realtime-only teardown, see Copilot R8).
                //
                // Synthesise `aborted` replies first so a still-live
                // `NetworkHandle` clone (whose owner is being
                // unregistered, but whose Promise queue may still
                // be observed) sees a terminal event for each
                // in-flight fetch — Copilot R11 HJTc, pairs with
                // R9 HEld for the broker `Shutdown` path.
                self.synthesise_aborted_replies_for_client(client_id);
                self.close_all_for_client(client_id);
                self.cancel_inflight_fetches_for(client_id);
                self.emit_renderer_unregistered(client_id);
                self.clients.remove(&client_id);
                true
            }
            NetworkProcessControl::Shutdown => {
                // Close every renderer's connections + cancel its
                // in-flight fetches before exiting the loop.  Without
                // this, fetch worker threads keep their tokio futures
                // running past `NetworkProcessHandle::shutdown()` and
                // can deliver replies into channel clones the workers
                // captured at dispatch time, while WS/SSE I/O threads
                // continue holding their handles (Copilot R7 finding,
                // pre-existing — surfaced by the file-split review).
                //
                // Order matters (Copilot R9 HEld): synthesise the
                // `aborted` reply for each in-flight fetch BEFORE
                // we cancel the worker, because cancelled workers
                // suppress their own `FetchResponse` on
                // `NetErrorKind::Cancelled`.  Without the synthetic
                // reply the renderer-side Promise stays pending
                // forever — the worker won't deliver one and the
                // broker is about to disappear.  Spec mirror of
                // `handle_cancel_fetch`'s synthetic-reply step.
                let client_ids: Vec<u64> = self.clients.keys().copied().collect();
                for cid in client_ids {
                    self.synthesise_aborted_replies_for_client(cid);
                    self.close_all_for_client(cid);
                    self.cancel_inflight_fetches_for(cid);
                    self.emit_renderer_unregistered(cid);
                }
                self.clients.clear();
                // `close_all_for_client` spawns a background
                // teardown thread per renderer so the grace
                // window doesn't block the broker thread (slot
                // #10.6a Copilot R3 HX14).  Before the broker
                // returns from its main loop we join all of
                // those threads so `NetworkProcessHandle::shutdown`
                // only resolves once every realtime worker has
                // fully exited — without this, workers spawned
                // by the renderers above could outlive the
                // broker by `GRACEFUL_CLOSE_GRACE` + cancel
                // propagation.
                for thread in std::mem::take(&mut self.pending_teardowns) {
                    let _ = thread.join();
                }
                false
            }
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

    /// Reap finished I/O thread handles + finished background
    /// teardown threads.  Called once per broker-loop iteration
    /// so the bookkeeping vectors stay bounded by the live
    /// per-renderer connection count, not the total connections
    /// ever opened.  Dropping a finished `JoinHandle` is
    /// equivalent to joining (the OS thread state is already
    /// reaped — see std lib docs), so we don't need to call
    /// `.join()` here.
    fn cleanup_finished(&mut self) {
        self.ws_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
        self.sse_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
        self.pending_teardowns.retain(|t| !t.is_finished());
    }

    /// Close every WS / SSE connection registered against this
    /// `client_id` and **fan out** their teardown onto a
    /// background thread.
    ///
    /// **Returns synchronously** after Phase 1 (channel-send
    /// fan-out, microsecond-scale on the broker thread) — the
    /// worker threads are NOT necessarily joined yet when this
    /// function returns.  Callers that need worker-exit
    /// synchronisation must drain
    /// [`Self::pending_teardowns`] or wait for the broker
    /// `Shutdown` control path (which joins everything before
    /// the broker exits).  `cleanup_finished` reaps finished
    /// teardown threads on every broker iteration so the
    /// vector stays bounded (slot #10.6a Copilot R4 HX17).
    ///
    /// Used by three call paths:
    /// - [`Self::handle_request`]'s `RendererToNetwork::Shutdown`
    ///   branch — used by `HostBridge::shutdown_all_realtime` to
    ///   tear down only realtime connections (in-flight fetches
    ///   keep running by design; the renderer is still alive).
    /// - [`Self::handle_control`]'s `UnregisterRenderer` branch —
    ///   pairs with [`Self::cancel_inflight_fetches_for`] to also
    ///   cancel the renderer's fetches because the client is gone.
    /// - [`Self::handle_control`]'s `Shutdown` branch — same as
    ///   `UnregisterRenderer` but for every client.
    ///
    /// **Teardown sequence** (slot #10.6a, fixes Copilot R8
    /// HCau / HCv / HJTV / HKhZ + R1 HX4 + R2 HX10):
    /// 1. **Phase 1 — fan-out close**: for every WS/SSE handle
    ///    bound to `client_id`, queue the protocol-clean close
    ///    command and drop the command sender.  Each handle's
    ///    cancel + thread is moved into a shared `pending`
    ///    vector for the next phase.  This phase runs as a
    ///    tight loop over channel sends only — no awaits, no
    ///    sleeps.
    /// 2. **Phase 2 — shared grace window**: a single
    ///    [`GRACEFUL_CLOSE_GRACE`] window for the entire batch
    ///    (handled by [`join_pending_with_grace_then_cancel`]).
    ///    A responsive worker observes the queued Close on its
    ///    next `cmd_rx.recv()` poll, sends the close frame, and
    ///    exits within milliseconds.  Sharing the grace window
    ///    across the batch keeps total broker stall to
    ///    `GRACEFUL_CLOSE_GRACE + max-cancel-propagation`
    ///    regardless of handle count — without this, the
    ///    serialised per-handle grace at 256 connections would
    ///    block fetch dispatch and event forwarding for ~25 s
    ///    (slot #10.6a HX10).
    /// 3. **Phase 3 — cancel fallback**: handles still running
    ///    after the grace window get their per-handle
    ///    [`crate::CancelHandle`] triggered.  The cancel arm of
    ///    the worker's `tokio::select!` aborts both the read
    ///    future AND any in-flight `write.send().await`
    ///    (cancel-aware via `ws::io_loop::send_frame` /
    ///    `ws::io_loop::send_close_frame` — slot #10.6a HX5;
    ///    rendered as inline code because the `io_loop`
    ///    submodule is `pub(super)`, Copilot R8 HX31).
    /// 4. **Phase 4 — join all**: every worker thread is
    ///    joined so `close_all_for_client` only returns once
    ///    every worker has fully exited.
    ///
    /// The grace window is intentionally short
    /// ([`GRACEFUL_CLOSE_GRACE`]).  Slow peers that need more
    /// time hit cancel and get a non-graceful close — that is
    /// the right tradeoff because the broker is only on this
    /// path when the renderer (or the entire network process)
    /// is being torn down anyway, and the alternative
    /// (unbounded join) would block `NetworkProcessHandle::shutdown`
    /// for the full TCP timeout window.
    ///
    /// Joining inside the broker thread is safe because each
    /// worker observes either the cancel signal or the dropped
    /// command channel within bounded time — see
    /// `ws::io_loop::ws_io_loop` and [`crate::sse::sse_io_loop`]
    /// for the cancel-injection surface.  Without the join, a
    /// stale renderer's `WsHandle` / `SseHandle` would be
    /// detached and the worker thread could outlive
    /// `NetworkProcessHandle::shutdown`, continuing to consume
    /// socket / TLS resources past the caller's expected
    /// lifetime (pre-existing leak surfaced by PR #142's
    /// structural review).
    fn close_all_for_client(&mut self, client_id: u64) {
        let mut pending: Vec<(std::thread::JoinHandle<()>, crate::CancelHandle)> = Vec::new();

        // Phase 1 (WS): fan-out close commands + drop senders.
        let ws_keys: Vec<_> = self
            .ws_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in ws_keys {
            if let Some(mut handle) = self.ws_handles.remove(&key) {
                let _ = handle
                    .command_tx
                    .send(WsCommand::Close(1001, "navigated away".into()));
                drop(handle.command_tx);
                if let Some(thread) = handle.thread.take() {
                    pending.push((thread, handle.cancel));
                }
            }
        }

        // Phase 1 (SSE): fan-out close commands + drop senders.
        let sse_keys: Vec<_> = self
            .sse_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in sse_keys {
            if let Some(mut handle) = self.sse_handles.remove(&key) {
                let _ = handle.command_tx.send(SseCommand::Close);
                drop(handle.command_tx);
                if let Some(thread) = handle.thread.take() {
                    pending.push((thread, handle.cancel));
                }
            }
        }

        // Phases 2-4 run on a background thread so the grace
        // window (+ cancel-propagation) does not block the
        // broker thread's main loop — without this, tearing
        // down one renderer with slow-to-close realtime sockets
        // would inject [`GRACEFUL_CLOSE_GRACE`] of cross-tab
        // latency into fetch dispatch / event forwarding for
        // every other renderer (slot #10.6a Copilot R3 HX14).
        // The broker tracks the spawned thread in
        // [`Self::pending_teardowns`] so the `Shutdown` control
        // path can join all in-flight teardowns before exiting,
        // and `cleanup_finished` reaps finished entries to
        // bound the vector's growth.
        if pending.is_empty() {
            return;
        }
        // Spawn the background teardown thread, with a true
        // in-thread fallback if the OS rejects the spawn
        // (typically EAGAIN under a process / user thread-limit
        // ulimit).  Without the fallback the broker would
        // panic on routine renderer shutdown, turning a single
        // connection cleanup into a network-process crash that
        // takes every other renderer down with it (slot #10.6a
        // Copilot R5 HX21).
        //
        // The tricky part is ownership: `std::thread::Builder::spawn`
        // consumes the closure on the call, so a failed spawn
        // would normally drop `pending` along with the closure
        // — leaving the JoinHandles unjoined and the
        // CancelHandles unfired (R6 HX28 found this gap in the
        // R5 attempt).  We park `pending` behind
        // `Arc<Mutex<Option<_>>>` so the closure can take it
        // lazily on its first poll AND the broker can reclaim
        // it on spawn-error to run the teardown in-thread —
        // matching the contract documented in
        // `close_all_for_client`.
        let pending_slot: std::sync::Arc<std::sync::Mutex<Option<Vec<_>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Some(pending)));
        let slot_for_worker = std::sync::Arc::clone(&pending_slot);
        match std::thread::Builder::new()
            .name("elidex-network-teardown".into())
            .spawn(move || {
                if let Some(pending) = slot_for_worker
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    join_pending_with_grace_then_cancel(pending);
                }
            }) {
            Ok(thread) => self.pending_teardowns.push(thread),
            Err(spawn_err) => {
                // Spawn failed.  The closure that the failed
                // spawn would have called is dropped, but its
                // capture is just an `Arc` clone — the data
                // itself is still in `pending_slot`.  Reclaim
                // it and run the teardown synchronously on the
                // broker thread so workers are still joined +
                // cancelled.  Falls back to the pre-HX14
                // exposure (broker stalls for
                // [`GRACEFUL_CLOSE_GRACE`] + cancel-propagation)
                // which is the right tradeoff under thread
                // pressure: bounded latency beats indeterminate
                // worker leak.
                tracing::warn!(
                    error = %spawn_err,
                    "failed to spawn teardown thread; running join+cancel in-thread on the broker"
                );
                if let Some(pending) = pending_slot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    join_pending_with_grace_then_cancel(pending);
                }
            }
        }
    }

    /// Push a synthetic `FetchResponse(id, Err("aborted"))` to
    /// `client_id` for every fetch currently in `cancel_tokens`
    /// keyed by this client.  Mirrors `handle_cancel_fetch`'s
    /// synthetic-reply step, but for every owned fetch in one
    /// pass.
    ///
    /// **Order contract**: callers must invoke this BEFORE
    /// triggering the cancel tokens (and before removing the
    /// client from `clients`).  Cancelled workers suppress
    /// their own `FetchResponse` on `NetErrorKind::Cancelled`,
    /// so the synthetic reply is the *only* terminal event the
    /// renderer-side Promise will ever see.  Doing this after
    /// cancel — or after `clients.remove` — leaves the Promise
    /// pending forever (Copilot R9 HEld for broker `Shutdown`,
    /// R11 HJTc for `UnregisterRenderer`).
    ///
    /// No-op if `client_id` is not in `self.clients` (the
    /// reply has nowhere to go).
    fn synthesise_aborted_replies_for_client(&self, client_id: u64) {
        let Some(tx) = self.clients.get(&client_id) else {
            return;
        };
        let inflight: Vec<FetchId> = self
            .cancel_tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .filter(|(c, _)| *c == client_id)
            .map(|(_, fid)| *fid)
            .collect();
        for fid in inflight {
            let _ = tx.send(NetworkToRenderer::FetchResponse(fid, Err("aborted".into())));
        }
    }

    /// Send the [`NetworkToRenderer::RendererUnregistered`]
    /// back-edge so the renderer-side `NetworkHandle` flips its
    /// `unregistered` flag and synthesises terminal `Err` replies
    /// for any fetches still tracked in `outstanding_fetches`
    /// (slot #10.6b).
    ///
    /// **Order contract**: callers must invoke this AFTER
    /// `synthesise_aborted_replies_for_client` /
    /// `close_all_for_client` / `cancel_inflight_fetches_for`
    /// and BEFORE `clients.remove(&client_id)`.  Putting it last
    /// in the broker-thread sequence ensures the renderer's
    /// drain processes the synthesised aborted replies (which
    /// remove ids from `outstanding_fetches`) before the marker
    /// runs the residual-synthesis pass — without that order
    /// the marker's pass would re-synthesise replies for ids
    /// the broker already covered, which the renderer would
    /// dedupe but would still cost a wasted Promise resolution
    /// path.
    ///
    /// No-op if `client_id` is not in `self.clients` (the marker
    /// has nowhere to go) — matches the same defensive check
    /// in `synthesise_aborted_replies_for_client`.
    fn emit_renderer_unregistered(&self, client_id: u64) {
        if let Some(tx) = self.clients.get(&client_id) {
            let _ = tx.send(NetworkToRenderer::RendererUnregistered);
        }
    }

    /// Cancel every in-flight fetch keyed by this `client_id`.
    /// Without this, a tab/worker that drops while its fetches are
    /// stalled would leave the worker threads holding their
    /// `MAX_CONCURRENT_FETCHES` slots until network IO completes
    /// (request_timeout ~30s for HTTP requests that connect but
    /// never respond).  Other renderers' fetches would be starved
    /// for that whole window (Copilot R4, file-split-a).
    ///
    /// Mirrors `handle_cancel_fetch`'s poison-tolerant remove +
    /// `cancel()` step but iterates every key matching this
    /// `client_id`.  No synthetic aborted reply is emitted — every
    /// caller pairs this with a sender-side teardown
    /// (`UnregisterRenderer` removes the `clients` entry; broker
    /// `Shutdown` clears the whole map), so any reply would be
    /// dropped on send anyway, and the worker observes
    /// `NetErrorKind::Cancelled` and silently exits via its
    /// `FetchInflightGuard` + `CancelMapEntryGuard`.
    ///
    /// **Not** called from `RendererToNetwork::Shutdown` because
    /// that path (used by `HostBridge::shutdown_all_realtime`)
    /// only intends to tear down WS/SSE — the renderer is still
    /// alive and its in-flight fetches must continue (Copilot R8).
    fn cancel_inflight_fetches_for(&self, client_id: u64) {
        let mut cancel_map = self
            .cancel_tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cancelled_keys: Vec<(u64, FetchId)> = cancel_map
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in cancelled_keys {
            if let Some(token) = cancel_map.remove(&key) {
                token.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slot #10.6a (Copilot R2 HX10) regression: a single
    /// teardown of N silent realtime handles must NOT compose
    /// to `GRACEFUL_CLOSE_GRACE × N` of broker-thread blocking
    /// — at the 256-connection per-document cap and the
    /// 100 ms grace, that would be ~25 s during which fetch
    /// dispatch and event forwarding for unrelated renderers
    /// are frozen.  [`join_pending_with_grace_then_cancel`]
    /// shares one grace window across the batch + then issues
    /// cancel to ALL still-running threads in parallel, so
    /// the total blocking time scales with the worst-case
    /// individual cancel-propagation latency, not the count.
    ///
    /// We model this by spawning N worker threads that each
    /// poll a [`crate::CancelHandle`] with a 5 ms cadence.
    /// Pre-fix the broker would have spent at least
    /// `GRACEFUL_CLOSE_GRACE × N` (~2 s for N=20) inside the
    /// per-handle grace loops before triggering any cancel.
    /// Post-fix the entire teardown completes well under
    /// `GRACEFUL_CLOSE_GRACE + a few ms` of cancel propagation.
    #[test]
    fn join_pending_bounded_independent_of_count() {
        const N: usize = 20;
        let mut pending: Vec<(std::thread::JoinHandle<()>, crate::CancelHandle)> =
            Vec::with_capacity(N);
        for _ in 0..N {
            let cancel = crate::CancelHandle::new();
            let cancel_for_worker = cancel.clone();
            let thread = std::thread::spawn(move || {
                while !cancel_for_worker.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            });
            pending.push((thread, cancel));
        }
        let started = std::time::Instant::now();
        join_pending_with_grace_then_cancel(pending);
        let elapsed = started.elapsed();
        // Pre-fix bound: `N × GRACEFUL_CLOSE_GRACE` = 20 × 100 ms = 2 s.
        // Post-fix bound: ~`GRACEFUL_CLOSE_GRACE` + propagation ≪ 500 ms.
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "join_pending_with_grace_then_cancel blocked for {elapsed:?} — \
             grace window not shared across the {N}-handle batch (would scale linearly)"
        );
    }

    /// Slot #10.6a sanity: an empty `pending` vector is a no-op
    /// — must not enter the grace loop / cancel pass / join
    /// pass with no work to do.  Keeps
    /// [`NetworkProcessState::close_all_for_client`] cheap when
    /// it's invoked against a renderer with no realtime
    /// handles open (the common case for fetch-only workloads).
    ///
    /// The 50 ms ceiling absorbs scheduler jitter on loaded CI
    /// runners (slot #10.6a Copilot R3 HX13) — the function
    /// itself only does an `is_empty` check + early return, so
    /// even with a fully-loaded test process the wall-clock is
    /// dominated by tokio test harness overhead, not the helper.
    #[test]
    fn join_pending_no_op_on_empty() {
        let started = std::time::Instant::now();
        join_pending_with_grace_then_cancel(Vec::new());
        assert!(
            started.elapsed() < std::time::Duration::from_millis(50),
            "empty-pending teardown took {:?} — should short-circuit",
            started.elapsed()
        );
    }

    /// Slot #10.6a (Copilot R4 HX20) regression: the broker's
    /// [`RendererToNetwork::EventSourceClose`] dispatch must
    /// trigger the per-handle [`crate::CancelHandle`] in
    /// addition to queueing `SseCommand::Close`.  Without the
    /// cancel trigger, a regular JS `eventSource.close()` is
    /// only bounded by the SSE worker's per-attempt 60-second
    /// `read_line` timeout — a silent server keeps the socket
    /// alive for up to that full window because `cmd_rx` is
    /// only polled via `try_recv` between read chunks.
    ///
    /// We exercise the dispatch path directly by constructing
    /// a `NetworkProcessState`, registering a stub renderer
    /// client, inserting a real `SseHandle` (the SSRF check
    /// inside `connect_sse_stream` will reject the loopback
    /// URL and the worker will exit on its own — what we care
    /// about is that the cancel signal fires synchronously
    /// inside `handle_request`).  The post-cancel join then
    /// confirms the worker observed cancel; the `is_cancelled`
    /// probe confirms the dispatch site itself fired the
    /// signal.
    #[test]
    fn event_source_close_triggers_cancel() {
        // Spawn an SseHandle pointed at an unreachable port —
        // `connect_sse_stream`'s SSRF check will return Fatal
        // for 127.0.0.1 and the worker exits early, but the
        // cancel field on the handle still observes the
        // dispatch-site `cancel.cancel()` call we're testing.
        let url = url::Url::parse("http://127.0.0.1:1/stream").unwrap();
        let handle = crate::sse::spawn_sse_thread(url, None, None, None, false);
        let observer = handle.cancel.clone();
        assert!(
            !observer.is_cancelled(),
            "cancel handle must start in the un-cancelled state"
        );

        let mut state = NetworkProcessState::new();
        let (resp_tx, _resp_rx) = crossbeam_channel::unbounded::<NetworkToRenderer>();
        let cid = 7_u64;
        let conn_id = 99_u64;
        state.clients.insert(cid, resp_tx);
        state.sse_handles.insert((cid, conn_id), handle);

        let client = std::sync::Arc::new(crate::NetClient::default());
        state.handle_request(cid, RendererToNetwork::EventSourceClose(conn_id), &client);

        assert!(
            observer.is_cancelled(),
            "EventSourceClose dispatch must trigger cancel on the handle"
        );

        // Drain the worker thread so the test doesn't leak it
        // into subsequent tests' shared process state.  The
        // worker observes cancel and exits; the SSRF Fatal
        // path may also have already returned — either way the
        // join is bounded.
        if let Some(mut handle) = state.sse_handles.remove(&(cid, conn_id)) {
            if let Some(thread) = handle.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
