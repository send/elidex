//! Async-fetch settlement: broker reply → Promise resolution.
//!
//! Hosts the three [`VmInner`] methods that drive the M4-12
//! PR5-async-fetch lifecycle:
//!
//! - [`VmInner::tick_network`] — public-facing per-tick drain entry,
//!   re-buffers WS/SSE events for sibling consumers (R3.2 ordering
//!   fix), runs the post-tick microtask checkpoint unconditionally
//!   (R4.1).
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

use elidex_net::broker::{FetchId, NetworkToRenderer};

use super::super::value::{JsValue, ObjectId, VmError};
use super::super::VmInner;
use super::blob::{reject_promise_sync, resolve_promise_sync};
use super::fetch::create_response_from_net;

impl VmInner {
    /// Drain pending [`elidex_net::broker::NetworkToRenderer`] events
    /// from the installed [`NetworkHandle`](elidex_net::broker::NetworkHandle)
    /// and dispatch them to the JS side.  See
    /// [`super::super::Vm::tick_network`] for the public-API
    /// contract.
    ///
    /// Always runs the microtask checkpoint at the end, even when
    /// no handle is installed — `tick_network` is also a generic
    /// "advance the event loop one beat" hook for embedders that
    /// only use the VM's microtask queue (R4.1).
    pub(in crate::vm) fn tick_network(&mut self) {
        if let Some(handle) = self.network_handle.clone() {
            // `drain_events` is the only consumer of the broker's
            // response channel for this handle, so any non-fetch
            // event we drain here cannot be re-fetched by another
            // consumer.  To preserve the original arrival order
            // between fetch replies and WS/SSE events (matters when
            // the handle is shared with a sibling consumer during
            // the boa→VM cutover), we settle fetch replies *only up
            // to the first non-fetch event*, then re-buffer that
            // event AND every event after it — including any
            // subsequent fetch replies — onto the handle's
            // `buffered` queue (R1.2 + R3.2).  A sibling consumer's
            // next `drain_events` then sees the original sequence;
            // later VM `tick_network` calls pick up the trailing
            // fetch replies once the sibling has consumed the
            // intervening WS/SSE events.
            let events = handle.drain_events();
            let mut iter = events.into_iter();
            let mut tail: Vec<NetworkToRenderer> = Vec::new();
            for event in iter.by_ref() {
                match event {
                    NetworkToRenderer::FetchResponse(fetch_id, result) => {
                        self.settle_fetch(fetch_id, result);
                    }
                    other @ (NetworkToRenderer::WebSocketEvent(_, _)
                    | NetworkToRenderer::EventSourceEvent(_, _)) => {
                        tail.push(other);
                        break;
                    }
                }
            }
            tail.extend(iter);
            if !tail.is_empty() {
                handle.rebuffer_events(tail);
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
                let resp_id = create_response_from_net(&mut g, response);
                resolve_promise_sync(&mut g, promise, JsValue::Object(resp_id));
            }
            Err(msg) => {
                let err = VmError::type_error(format!("Failed to fetch: {msg}"));
                let reason = g.vm_error_to_thrown(&err);
                reject_promise_sync(&mut g, promise, reason);
            }
        }
    }
}
