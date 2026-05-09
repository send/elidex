//! Renderer-teardown methods for [`super::NetworkProcessState`] —
//! the WS/SSE close-with-grace fan-out (`close_all_for_client`),
//! the synthesised `Err("aborted")` reply path
//! (`synthesise_aborted_replies_for_client`), the post-teardown
//! `RendererUnregistered` ack (`emit_renderer_unregistered`), and
//! the in-flight fetch cancel sweep (`cancel_inflight_fetches_for`).
//!
//! Extracted from `dispatch.rs` to keep the dispatch entry / per-
//! message handlers in `mod.rs` focused on the broker main loop;
//! teardown was the largest single category at ~300 LoC and pulls
//! its own grace-window helper [`join_pending_with_grace_then_cancel`].

use std::time::Duration;

use crate::sse::SseCommand;
use crate::ws::WsCommand;

use super::super::{FetchId, NetworkToRenderer};
use super::NetworkProcessState;

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
/// `crate::sse::sse_io_loop` cancel arms), so the unbounded
/// wait is bounded in practice by the cancel-propagation latency.
/// Both references are rendered as inline code rather than intra-
/// doc links because the underlying `io_loop` / `sse_io_loop`
/// items are private to their parent modules (slot #10.6a HX26
/// split — Copilot R8 HX30 caught the WS stale link, PR-7 R1
/// caught the sibling SSE one).
pub(super) fn join_pending_with_grace_then_cancel(
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

impl NetworkProcessState {
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
    /// 4. **Phase 4 — join all (off-thread)**: phases 2-4 run
    ///    inside a spawned teardown thread tracked in
    ///    [`Self::pending_teardowns`].  `close_all_for_client`
    ///    itself returns once Phase 1 has fanned out the close
    ///    commands; the per-handle joins (and the grace window /
    ///    cancel fallback above) wait inside that background
    ///    thread so the broker main loop is free to keep
    ///    dispatching for OTHER renderers.  The broker `Shutdown`
    ///    control path drains `pending_teardowns` before exit, so
    ///    `NetworkProcessHandle::shutdown` only returns once
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
    /// `ws::io_loop::ws_io_loop` and `crate::sse::sse_io_loop`
    /// for the cancel-injection surface (both rendered as inline
    /// code rather than intra-doc links because the underlying
    /// items are private to their parent modules).  Without the join, a
    /// stale renderer's `WsHandle` / `SseHandle` would be
    /// detached and the worker thread could outlive
    /// `NetworkProcessHandle::shutdown`, continuing to consume
    /// socket / TLS resources past the caller's expected
    /// lifetime (pre-existing leak surfaced by PR #142's
    /// structural review).
    pub(super) fn close_all_for_client(&mut self, client_id: u64) {
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
    pub(super) fn synthesise_aborted_replies_for_client(&self, client_id: u64) {
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
    pub(super) fn emit_renderer_unregistered(&self, client_id: u64) {
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
    pub(super) fn cancel_inflight_fetches_for(&self, client_id: u64) {
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
