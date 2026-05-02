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
                }
                self.clients.clear();
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

    fn cleanup_finished(&mut self) {
        self.ws_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
        self.sse_handles
            .retain(|_, handle| handle.thread.as_ref().is_none_or(|t| !t.is_finished()));
    }

    /// Close every WS / SSE connection registered against this
    /// `client_id`.  Used by three call paths:
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
    /// **Lifecycle gap (Copilot R8 follow-up)**: the WS/SSE
    /// `JoinHandle`s are dropped after sending their close commands
    /// but never `join()`ed.  The worker threads do exit on their
    /// own once they observe the closed command channel, so this
    /// is bounded leakage of detached threads (eventual cleanup,
    /// not memory growth) — but a `shutdown()` call should
    /// arguably block until all I/O threads finish.  Adding a
    /// join step requires deadlock analysis (a worker stuck on a
    /// never-completing socket await would hang the broker
    /// shutdown), so it lands in a follow-up rather than this
    /// mechanical-split PR.  Tracked in
    /// `m4-12-broker-shutdown-join-followup.md` and the M4-12
    /// post-cutover roadmap.
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
