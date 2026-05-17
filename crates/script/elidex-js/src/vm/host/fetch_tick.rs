//! Async-fetch settlement: broker reply → Promise resolution.
//!
//! Hosts the three [`VmInner`] methods that drive the M4-12
//! PR5-async-fetch lifecycle:
//!
//! - [`VmInner::tick_network`] — public-facing per-tick drain entry.
//!   Pulls only fetch replies via
//!   [`elidex_net::broker::NetworkHandle::drain_fetch_responses_only`]
//!   so WS/SSE events stay in the broker handle's internal buffer
//!   for a sibling consumer's later `drain_events`.  Always runs
//!   the post-tick microtask checkpoint, even when no handle is
//!   installed (R4.1).
//! - [`VmInner::settle_fetch`] — private helper that settles a single
//!   pending Promise from a `FetchResponse(id, result)` event,
//!   pruning the matching back-refs / abort-observer entries.
//! - [`VmInner::reject_pending_fetches_with_error`] — handle-swap
//!   teardown invoked by [`super::super::Vm::install_network_handle`]
//!   so in-flight fetches against the old handle don't strand
//!   un-settleable Promises (R3.3).
//!
//! Split out of [`super::fetch`] to keep that file under the
//! project's 1000-line convention (R4.2).  Companion to
//! [`super::fetch::create_response_from_net`] which is `pub(super)`
//! so this module can build the success-branch Response.

#![cfg(feature = "engine")]

use elidex_net::broker::FetchId;

use super::super::value::{JsValue, ObjectId, VmError};
use super::super::VmInner;
use super::blob::{reject_promise_sync, resolve_promise_sync};
use super::cors::{classify_response_type, CorsOutcome};
use super::fetch::create_response_from_net;

impl VmInner {
    /// Drain pending fetch replies from the installed
    /// [`NetworkHandle`](elidex_net::broker::NetworkHandle) and
    /// dispatch them to the JS side.  See
    /// [`super::super::Vm::tick_network`] for the public-API
    /// contract.
    ///
    /// Uses
    /// [`elidex_net::broker::NetworkHandle::drain_fetch_responses_only`]
    /// so any WS/SSE events on the handle stay in its internal
    /// buffer for a sibling consumer (e.g. the boa-side realtime
    /// bridge during the boa→VM cutover) to drain on its own
    /// schedule.  Non-fetch event ordering across the handle is
    /// preserved by the broker API — see that method's doc for
    /// the order guarantee.
    ///
    /// Always runs the microtask checkpoint at the end, even when
    /// no handle is installed — `tick_network` is also a generic
    /// "advance the event loop one beat" hook for embedders that
    /// only use the VM's microtask queue (R4.1).
    pub(in crate::vm) fn tick_network(&mut self) {
        if let Some(handle) = self.network_handle.clone() {
            for (fetch_id, result) in handle.drain_fetch_responses_only() {
                self.settle_fetch(fetch_id, result);
            }
            // D-12 `#11-net-ws-sse` (IMP-6): single Vm-API surface
            // for realtime — partition the broker handle's
            // non-fetch event drain into WebSocket vs
            // EventSource buckets and dispatch each to the
            // matching wrapper via the
            // `HostData::ws_conn_to_object` /
            // `sse_conn_to_object` reverse maps.  Fetch
            // settlements run BEFORE realtime per `buffered.rs`
            // ordering invariant so a `fetch().then(...)` that
            // happens to share a renderer tick with a stream of
            // WS frames lands its `.then` reactions in the same
            // microtask checkpoint, ahead of the WS frame
            // dispatch.
            //
            // Per-event variant dispatch lives in
            // `dispatch_realtime_event` below; this loop only
            // owns the drain + partition-by-reverse-map glue.
            for event in handle.drain_events() {
                self.dispatch_realtime_event(event);
            }
        }
        // Microtask checkpoint — `.then` reactions attached to a
        // settled Promise (or any other queued reaction) must run
        // before this call returns so the shell event loop's
        // per-tick observable order matches a real browser's
        // microtask drain at the end of every task.  Runs even
        // when no handle is installed (R4.1) so the public API's
        // unconditional contract holds for handle-less embedders.
        self.drain_microtasks();
    }

    /// Dispatch a single non-fetch broker event (WS or SSE) to the
    /// matching VM-side wrapper.
    ///
    /// Looks the receiver up through the reverse map and silently
    /// drops if the wrapper has been GC-swept (the sweep tail
    /// already emitted the matching `WebSocketClose` /
    /// `EventSourceClose` so the broker should not send further
    /// events for that `conn_id`; a late arrival between sweep and
    /// broker observe is benign).
    ///
    /// Implements every WebSocket arm (Phase 1+2) and every
    /// EventSource arm (Phase 3); delegates the actual event
    /// allocation + handler fire to
    /// [`super::websocket_dispatch`] / [`super::event_source_dispatch`].
    ///
    /// Borrow discipline: the reverse-map lookup snapshots the
    /// instance `ObjectId` into a local before the per-variant
    /// dispatch helpers are called, dropping the `host_data` borrow
    /// up front.  Each helper takes `&mut self` and re-borrows
    /// `host_data` for its mutation (state transition, field
    /// populate) before the handler call.
    fn dispatch_realtime_event(&mut self, event: elidex_net::broker::NetworkToRenderer) {
        use elidex_net::broker::NetworkToRenderer;
        use elidex_net::sse::SseEvent;
        use elidex_net::ws::WsEvent;
        match event {
            NetworkToRenderer::WebSocketEvent(conn_id, ws_event) => {
                let Some(instance) = self
                    .host_data
                    .as_deref()
                    .and_then(|hd| hd.ws_conn_to_object.get(&conn_id).copied())
                else {
                    return;
                };
                match ws_event {
                    WsEvent::Connected {
                        protocol,
                        extensions,
                    } => {
                        super::websocket_dispatch::dispatch_ws_connected(
                            self, instance, protocol, extensions,
                        );
                    }
                    WsEvent::Closed {
                        code,
                        reason,
                        was_clean,
                    } => {
                        super::websocket_dispatch::dispatch_ws_closed(
                            self, instance, code, &reason, was_clean,
                        );
                    }
                    WsEvent::TextMessage(s) => {
                        super::websocket_dispatch::dispatch_ws_text_message(self, instance, &s);
                    }
                    WsEvent::BinaryMessage(bytes) => {
                        super::websocket_dispatch::dispatch_ws_binary_message(
                            self, instance, bytes,
                        );
                    }
                    WsEvent::Error(_msg) => {
                        // Per WHATWG §9.3.7 the script-visible "error"
                        // is a plain Event with no detail — the broker
                        // message is discarded intentionally to avoid
                        // leaking server-internals through the
                        // unsandboxed handler.
                        super::websocket_dispatch::dispatch_ws_error(self, instance);
                    }
                    WsEvent::BytesSent(n) => {
                        super::websocket_dispatch::dispatch_ws_bytes_sent(self, instance, n);
                    }
                }
            }
            NetworkToRenderer::EventSourceEvent(conn_id, sse_event) => {
                let Some(instance) = self
                    .host_data
                    .as_deref()
                    .and_then(|hd| hd.sse_conn_to_object.get(&conn_id).copied())
                else {
                    return;
                };
                match sse_event {
                    SseEvent::Connected => {
                        super::event_source_dispatch::dispatch_sse_connected(self, instance);
                    }
                    SseEvent::Event {
                        event_type,
                        data,
                        last_event_id,
                    } => {
                        super::event_source_dispatch::dispatch_sse_event(
                            self,
                            instance,
                            &event_type,
                            &data,
                            last_event_id,
                        );
                    }
                    SseEvent::Error(_msg) => {
                        // Per WHATWG HTML §9.2.5 the script-visible
                        // "error" is a plain Event with no detail —
                        // the broker message is discarded
                        // intentionally to avoid leaking server-
                        // internals through the unsandboxed handler.
                        super::event_source_dispatch::dispatch_sse_error(self, instance);
                    }
                    SseEvent::FatalError(_msg) => {
                        super::event_source_dispatch::dispatch_sse_fatal_error(self, instance);
                    }
                }
            }
            // FetchResponse already drained by
            // `drain_fetch_responses_only` above — should never
            // appear in `drain_events`'s residual stream, but the
            // arm is exhaustive so the broker's existing
            // ordering invariant is the only contract this code
            // relies on.
            NetworkToRenderer::FetchResponse(_, _) | NetworkToRenderer::RendererUnregistered => {}
        }
    }

    /// Reject every entry in [`Self::pending_fetches`] with a
    /// `TypeError` carrying `msg`, tearing down the matching
    /// `fetch_signal_back_refs` / `fetch_abort_observers` entries.
    /// Used by [`super::super::Vm::install_network_handle`] before
    /// a handle swap (R3.3): the old handle's broker-reply channel
    /// becomes unreachable, so otherwise-pending Promises would be
    /// permanently un-settleable from the user's perspective.
    /// No-op when `pending_fetches` is empty (the common case —
    /// production embedders install the handle once at VM
    /// construction).
    pub(in crate::vm) fn reject_pending_fetches_with_error(&mut self, msg: &str) {
        if self.pending_fetches.is_empty() {
            return;
        }
        // Drain CORS meta alongside `pending_fetches` so the
        // entries don't outlive their owning Promises.
        self.pending_fetch_cors.clear();
        // Snapshot the *current* handle (if any) for the cancel
        // wave below — we want each pending fetch's broker-side
        // work to halt promptly so the network thread doesn't
        // keep running IO whose result will never be observed
        // (R6.3).  The handle becomes unreachable from `self`
        // immediately after the caller's `network_handle =
        // Some(...)` swap, so the Rc clone here is the last
        // chance to drive cancels through the old handle.
        let outgoing_handle = self.network_handle.as_ref().map(std::rc::Rc::clone);
        let stale: Vec<(FetchId, ObjectId)> = self.pending_fetches.drain().collect();
        for (fetch_id, promise) in stale {
            // Tear down the back-refs so a subsequent
            // `controller.abort()` does not chase a stale FetchId
            // through the old handle.
            if let Some(signal_id) = self.fetch_signal_back_refs.remove(&fetch_id) {
                if let Some(observers) = self.fetch_abort_observers.get_mut(&signal_id) {
                    observers.retain(|&id| id != fetch_id);
                    if observers.is_empty() {
                        self.fetch_abort_observers.remove(&signal_id);
                    }
                }
            }
            // Best-effort cancel through the outgoing handle
            // (R6.3).  Disconnected handles silently no-op via
            // `send`'s bool return.  A successful send routes the
            // broker through the same synthesised `Err("aborted")`
            // path as a normal `controller.abort()`, but the
            // renderer-side reply is unobservable from this point
            // (the Promise was just rejected and `pending_fetches`
            // drained).
            if let Some(ref h) = outgoing_handle {
                let _ = h.cancel_fetch(fetch_id);
            }
            // Defensive root mirroring `settle_fetch` /
            // `abort_signal` (R2 fixes): `vm_error_to_thrown`
            // allocates an Error object before settlement.
            let mut g = self.push_temp_root(JsValue::Object(promise));
            let err = VmError::type_error(format!("Failed to fetch: {msg}"));
            let reason = g.vm_error_to_thrown(&err);
            reject_promise_sync(&mut g, promise, reason);
            drop(g);
        }
    }

    /// Settle a single in-flight fetch's Promise.  Removes the
    /// `pending_fetches` entry; if absent (already settled by
    /// abort), the late reply is silently dropped.  Prunes the
    /// reverse signal-back-refs entry and the abort fan-out list
    /// so a subsequent `controller.abort()` does not send a
    /// redundant CancelFetch for an already-completed fetch.
    fn settle_fetch(&mut self, fetch_id: FetchId, result: Result<elidex_net::Response, String>) {
        let Some(promise) = self.pending_fetches.remove(&fetch_id) else {
            // Late reply for a fetch already settled by abort —
            // the meta entry was already drained alongside the
            // Promise, but a defensive `remove` keeps the map
            // sized when an out-of-order arrival occurs.
            self.pending_fetch_cors.remove(&fetch_id);
            return;
        };
        // Prune the reverse index — drops a stale entry, harmless
        // if the fetch had no signal (entry never existed).
        if let Some(signal_id) = self.fetch_signal_back_refs.remove(&fetch_id) {
            if let Some(observers) = self.fetch_abort_observers.get_mut(&signal_id) {
                observers.retain(|&id| id != fetch_id);
                if observers.is_empty() {
                    self.fetch_abort_observers.remove(&signal_id);
                }
            }
        }
        let cors_meta = self.pending_fetch_cors.remove(&fetch_id);
        // GC root the Promise across the settlement work (R2.2):
        // `pending_fetches` was its only root for the
        // user-discarded case
        // (`promise_survives_user_dropping_reference`), and the
        // success branch's `create_response_from_net` allocates an
        // Object + Headers + body inserts that could trigger GC
        // under a future runtime that relaxes the
        // `gc_enabled = false` invariant inside native calls.  The
        // Err branch's `vm_error_to_thrown` also allocates an
        // Error object.  Match the rest of the codebase
        // (`wrap_in_array_iterator` / `native_fetch` / etc.) by
        // routing the post-remove work through a `push_temp_root`
        // guard.
        let mut g = self.push_temp_root(JsValue::Object(promise));
        match result {
            Ok(response) => {
                // PR5-cors Stage 4: classify the response per
                // request mode + redirect mode.  Returns either a
                // `CorsClassification` (response_type + opaque-
                // shape flag) or `NetworkError` (cors-mode failure
                // → reject with TypeError).
                //
                // **Fail closed on missing meta** (Copilot R2):
                // every successful broker reply for an in-flight
                // fetch must have a `pending_fetch_cors` entry
                // because `native_fetch` inserts both maps
                // atomically.  An absent entry signals an
                // internal bookkeeping bug — fall through to a
                // permissive `Basic` default would silently
                // disable CORS enforcement, so reject the Promise
                // instead.  The success path therefore demands
                // `cors_meta = Some(...)`; the abort/handle-swap
                // paths drain both maps together, so this branch
                // never fires for those.
                let Some(cors_meta) = cors_meta.as_ref() else {
                    let err = VmError::type_error(
                        "Failed to fetch: missing CORS metadata for pending fetch (internal invariant)"
                            .to_string(),
                    );
                    let reason = g.vm_error_to_thrown(&err);
                    reject_promise_sync(&mut g, promise, reason);
                    return;
                };
                let outcome = classify_response_type(
                    cors_meta.request_origin.as_ref(),
                    &cors_meta.request_url,
                    cors_meta.request_mode,
                    cors_meta.redirect_mode,
                    &response.url,
                    response.status,
                    &response.headers,
                    response.is_redirect_tainted,
                    response.credentialed_network,
                );
                match outcome {
                    CorsOutcome::Ok(classification) => {
                        let resp_id = create_response_from_net(&mut g, response, classification);
                        resolve_promise_sync(&mut g, promise, JsValue::Object(resp_id));
                    }
                    CorsOutcome::NetworkError => {
                        let err = VmError::type_error(
                            "Failed to fetch: CORS check failed (no matching Access-Control-Allow-Origin)"
                                .to_string(),
                        );
                        let reason = g.vm_error_to_thrown(&err);
                        reject_promise_sync(&mut g, promise, reason);
                    }
                }
            }
            Err(msg) => {
                let err = VmError::type_error(format!("Failed to fetch: {msg}"));
                let reason = g.vm_error_to_thrown(&err);
                reject_promise_sync(&mut g, promise, reason);
            }
        }
    }
}
