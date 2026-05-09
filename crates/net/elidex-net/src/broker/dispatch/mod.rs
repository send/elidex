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

mod teardown;

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
        // **Layered defence**.  Slot #10.6c's ack handshake
        // (factories block on broker `clients.insert`) makes the
        // legitimate Register-then-Fetch race impossible.  Slot
        // #10.6b's renderer-side `unregistered` flag short-
        // circuits `fetch_async` / `fetch_blocking` /
        // `cancel_fetch` / `send` after the marker is observed,
        // and `outstanding_fetches` synthesises terminal `Err`
        // replies for race-window fetches dropped HERE.  This
        // broker-side gate stays as the **defensive floor** for
        // (a) stale `NetworkHandle` state captured in background
        // threads before its renderer flag has flipped, (b) any
        // caller that posts directly to `request_tx` bypassing
        // the handle helpers, and (c) the
        // synthesise → cancel → `clients.remove` teardown
        // window.  Lesson #134 (slot #10.6b landing memo):
        // when a renderer-side gate overlaps a broker-side
        // gate, document the layering to pre-empt "delete
        // redundant code" reviews.
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
}

#[cfg(test)]
mod tests {
    use super::teardown::join_pending_with_grace_then_cancel;
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
