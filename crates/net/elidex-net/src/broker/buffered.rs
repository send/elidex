//! Buffered-event drain helpers — the partial-drain machinery
//! that lets the elidex-js VM's `tick_network` settle fetch
//! replies without disturbing WS / SSE arrival order for a
//! sibling consumer's later [`NetworkHandle::drain_events`].
//!
//! Two API shapes coexist:
//! - [`NetworkHandle::drain_events`] — drain everything; primary
//!   path for embedders that handle every event in one place.
//! - [`NetworkHandle::drain_fetch_responses_only`] — partition
//!   in place: returns fetch replies, leaves WS/SSE in the
//!   internal buffer for a sibling consumer's later drain.  Used
//!   by the elidex-js VM's `tick_network`.
//!
//! [`NetworkHandle::rebuffer_events`] is a legacy helper from the
//! pre-`drain_fetch_responses_only` pattern; retained for direct
//! embedders who push events back themselves.

use super::{FetchId, NetworkHandle, NetworkToRenderer, Response};

impl NetworkHandle {
    /// Non-blocking drain of all pending events (WS/SSE/fetch responses).
    ///
    /// Includes any events buffered during a prior [`fetch_blocking`](Self::fetch_blocking) call.
    ///
    /// Slot #10.6b: every event is routed through the private
    /// `process_response` helper so
    /// [`NetworkToRenderer::FetchResponse`] arrivals are
    /// removed from the handle's `outstanding_fetches` set
    /// AND the internal
    /// [`NetworkToRenderer::RendererUnregistered`] back-edge
    /// is consumed (it never appears in the returned `Vec`).
    /// On marker observation, every still-tracked id is
    /// folded into the returned `Vec` as a synthetic
    /// `FetchResponse(id, Err("renderer unregistered"))` —
    /// those are race-window fetches the broker silently
    /// dropped via its stale-cid gate
    /// (`dispatch::handle_request`).  Without this synthesis
    /// the renderer-side `pending_fetches[id]` Promises
    /// would stay pending forever.
    ///
    /// Order: broker-originated events appear in arrival
    /// order across `(prior buffer, channel try_recv drain)`.
    /// Synthetic straggler `FetchResponse`s appear after the
    /// arrival-order tail in ascending `FetchId` (which is
    /// also submission order — `FETCH_ID_COUNTER` is
    /// monotonic).
    pub fn drain_events(&self) -> Vec<NetworkToRenderer> {
        let prior: Vec<_> = self.buffered.borrow_mut().drain(..).collect();
        let mut events: Vec<NetworkToRenderer> = Vec::with_capacity(prior.len());
        for evt in prior {
            self.process_response(evt, &mut |e| events.push(e));
        }
        while let Ok(evt) = self.response_rx.try_recv() {
            self.process_response(evt, &mut |e| events.push(e));
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
    /// Order guarantee:
    /// - **Broker-originated fetch replies** appear in the
    ///   returned `Vec` in arrival order across
    ///   `(prior buffer, channel try_recv drain)`.
    /// - **Synthetic stragglers** (slot #10.6b — see below)
    ///   appear after every broker-originated reply observed
    ///   on the same tick, in ascending `FetchId` order
    ///   (which matches submission order because `FetchId` is
    ///   a monotonic counter — `FETCH_ID_COUNTER` in `mod.rs`).
    /// - **Non-fetch events** stay in `self.buffered` in the
    ///   same arrival order.
    ///
    /// Slot #10.6b: every event is routed through the private
    /// `process_response` helper before partition.  The
    /// internal [`NetworkToRenderer::RendererUnregistered`]
    /// back-edge is consumed (never appears in `kept`); on
    /// marker observation, every still-tracked id in
    /// `outstanding_fetches` is converted to a synthetic
    /// `FetchResponse(id, Err("renderer unregistered"))` and
    /// flows into the returned fetch partition (sorted, per
    /// the ordering rule above).  Race-window fetches that the
    /// broker dropped via its stale-cid gate
    /// (`dispatch::handle_request`) therefore reach the
    /// elidex-js VM's `tick_network` on the same tick the
    /// marker arrives, settling their Promises without a leak.
    pub fn drain_fetch_responses_only(&self) -> Vec<(FetchId, Result<Response, String>)> {
        // Pre-size both partitions to `prior.len()`: the steady-
        // state per-tick caller (the elidex-js VM's `tick_network`)
        // typically sees a single-digit buffer + a single-digit
        // arrival batch, and we don't know the split a priori, so
        // sizing each bucket to the prior length avoids the early
        // `Vec::push` reallocations on the buffered branch.  The
        // channel branch may push past the reserve when arrivals
        // exceed `prior.len()`; that's still amortised O(1) per
        // push and is bounded by the broker's per-tick fan-in.
        //
        // We can't hold `self.buffered.borrow_mut()` across the
        // routing step because `process_response`'s straggler-
        // synthesis path runs `outstanding_fetches.borrow_mut()`
        // (separate RefCell — would be fine) but the helper's
        // contract assumes the caller may freely take fresh
        // borrows on either; routing through a temporary `prior`
        // Vec keeps the contract clean and removes any chance
        // of a future re-entrance from getting tangled with
        // this drain's own buffer borrow.
        let prior: Vec<NetworkToRenderer> = self.buffered.borrow_mut().drain(..).collect();
        let mut fetches: Vec<(FetchId, Result<Response, String>)> = Vec::with_capacity(prior.len());
        let mut kept: Vec<NetworkToRenderer> = Vec::with_capacity(prior.len());
        // Scope the partition closure so its mutable borrows on
        // `fetches` / `kept` are released before we re-acquire
        // `self.buffered` below — without the scope, those
        // borrows would still be live at the assignment.
        {
            let mut emit = |evt: NetworkToRenderer| match evt {
                NetworkToRenderer::FetchResponse(id, result) => fetches.push((id, result)),
                other => kept.push(other),
            };
            for evt in prior {
                self.process_response(evt, &mut emit);
            }
            while let Ok(evt) = self.response_rx.try_recv() {
                self.process_response(evt, &mut emit);
            }
        }
        *self.buffered.borrow_mut() = kept;
        fetches
    }

    /// Drain the response channel into `buffered`, routing each event
    /// through `process_response` (marker consume / straggler
    /// synthesis).  Buffered events already precede channel arrivals,
    /// so appending settled channel events keeps them exactly where an
    /// unfiltered drain would place them — arrival order is preserved.
    ///
    /// Shared prologue of [`Self::has_pending_event_for_conn`] (the
    /// count-bounded batch pop [`Self::pop_buffered_front`] is
    /// buffered-only and does NOT settle): the peek must see the FULL
    /// inbound pipeline (channel + buffer), not just already-buffered
    /// events — else a GC / peek between an arrival and the first drain
    /// misses a channel-pending event (Codex R2a).  The drain methods
    /// ([`Self::drain_events`] / [`Self::drain_fetch_responses_only`])
    /// keep their own inline settle: they interleave the channel drain
    /// with a partition/collect step and have a different shape.
    fn settle_channel_into_buffer(&self) {
        let mut newly: Vec<NetworkToRenderer> = Vec::new();
        while let Ok(evt) = self.response_rx.try_recv() {
            self.process_response(evt, &mut |e| newly.push(e));
        }
        if !newly.is_empty() {
            self.buffered.borrow_mut().extend(newly);
        }
    }

    /// Number of events currently in the internal buffer — the finite
    /// realtime batch size for the elidex-js VM's `tick_network`
    /// count-bounded drain.  Snapshotted AFTER
    /// [`Self::drain_fetch_responses_only`] (which drains `response_rx`
    /// into `buffered`, fetch replies partitioned out) so it counts
    /// exactly the WS/SSE events settled for THIS tick; the VM then pops
    /// that many via [`Self::pop_buffered_front`], leaving any mid-loop
    /// arrival (channel or a GC-time `has_pending_event_for_conn` settle,
    /// both of which land at the BACK) for the next tick.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.buffered.borrow().len()
    }

    /// Pop the FRONT buffered event WITHOUT settling the channel — the
    /// buffered-only counterpart to the drain methods'
    /// settle-then-partition (this does NOT re-drain `response_rx`).
    ///
    /// The elidex-js VM's `tick_network` drives the realtime dispatch
    /// loop as a COUNT-BOUNDED batch: it takes `batch =`
    /// [`buffered_len`](Self::buffered_len) once (after
    /// [`Self::drain_fetch_responses_only`] settled the channel) and calls
    /// this exactly `batch` times.  Because this does NOT re-drain
    /// `response_rx`, an event arriving mid-loop — whether pushed to the
    /// channel by the network thread or moved into `buffered` by a
    /// GC-time [`Self::has_pending_event_for_conn`] settle (which
    /// `.extend`s the BACK) — lands past the batch boundary and is NOT
    /// dispatched this tick; the next tick's `drain_fetch_responses_only`
    /// handles it.  That is what hard-bounds the tick (Codex R4 F1:
    /// the old whole-buffer `pop`'s per-iteration settle livelocked under
    /// a busy stream) and keeps a mid-callback `FetchResponse` recoverable
    /// instead of dropped (Codex R4 F2: the per-iteration settle pulled
    /// it into `buffered`, cleared its `outstanding_fetches` id, then the
    /// realtime `FetchResponse => {}` arm dropped it → Promise leak).
    ///
    /// The un-popped tail stays in `buffered`, so a sibling
    /// [`Self::has_pending_event_for_conn`] scan during the dispatch of an
    /// earlier batch event still sees the not-yet-popped events for every
    /// conn (Codex R3 finding 3 sibling-conn keepalive).
    ///
    /// Routes the popped event through `process_response` for the
    /// raw-buffered-`FetchResponse` `outstanding_fetches` bookkeeping and
    /// asserts the marker-free `buffered` invariant, identical to the
    /// old whole-buffer pop's post-pop step — only the settle prologue is
    /// dropped.
    /// Returns `None` when `buffered` is empty (regardless of any
    /// channel-pending events — this is buffered-only by design).
    /// Single-threaded content-thread borrows only.
    #[must_use]
    pub fn pop_buffered_front(&self) -> Option<NetworkToRenderer> {
        let front = {
            let mut buf = self.buffered.borrow_mut();
            if buf.is_empty() {
                return None;
            }
            buf.remove(0)
        };
        let mut emitted: Vec<NetworkToRenderer> = Vec::new();
        self.process_response(front, &mut |e| emitted.push(e));
        debug_assert_eq!(
            emitted.len(),
            1,
            "pop_buffered_front: front-of-buffer event must emit exactly one event — \
             `buffered` must never hold a RendererUnregistered marker \
             (markers are consumed at settle time)",
        );
        emitted.into_iter().next()
    }

    /// Peek: is there an [`NetworkToRenderer::EventSourceEvent`] pending for
    /// `conn_id` anywhere in the inbound pipeline (channel + buffer), awaiting a
    /// later [`Self::drain_events`]?
    ///
    /// The elidex-js VM's GC keepalive seam calls this to derive the HTML §9.2.9
    /// "task queued on the remote event task source" clause: an inbound SSE event
    /// buffers here **between** [`Self::drain_fetch_responses_only`] and
    /// [`Self::drain_events`], and an allocation-triggered GC in that window must
    /// keep the target `EventSource` wrapper alive so the buffered event can still
    /// be routed (else it is silently dropped via a reverse-map miss, Codex F3).
    ///
    /// This **settles the channel into the buffer** (same `process_response`
    /// routing a drain uses) then scans `buffered` — it is NOT non-draining. An
    /// inbound `EventSourceEvent` can still sit in `response_rx` (not yet moved to
    /// `buffered`) at GC time — e.g. a GC firing between arrival and the very
    /// first drain — so a scan of `buffered` alone would miss it and the peek
    /// would report "no queued task" for a conn that in fact has one (Codex R2a,
    /// F3 silent-drop one layer up). Draining the channel into `buffered` here is
    /// safe for arrival order and fetch settlement because the drain methods
    /// ([`Self::drain_events`] / [`Self::drain_fetch_responses_only`]) process the
    /// buffer *before* the channel: moving channel-pending events to the buffer
    /// early keeps them ahead of any later channel arrivals, exactly where an
    /// unfiltered drain would have placed them. Markers / stragglers are handled
    /// identically to a drain because the same `process_response` routing runs.
    /// Single-threaded content-thread borrows only.
    #[must_use]
    pub fn has_pending_event_for_conn(&self, conn_id: u64) -> bool {
        // Settle any channel-pending events into `buffered` (via `process_response`,
        // identical to the drain methods' channel handling) so the scan sees the FULL
        // inbound pipeline (channel + buffer), not just already-buffered events — else a
        // GC between arrival and the first drain misses a channel-pending event (Codex
        // R2a). A later `drain_events` / `drain_fetch_responses_only` processes `prior`
        // (buffered) first, so this early move preserves arrival order and fetch settlement.
        self.settle_channel_into_buffer();
        self.buffered
            .borrow()
            .iter()
            .any(|evt| matches!(evt, NetworkToRenderer::EventSourceEvent(id, _) if *id == conn_id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpVersion;

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
            unregistered: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            outstanding_fetches: std::cell::RefCell::new(std::collections::HashSet::new()),
            #[cfg(feature = "test-hooks")]
            mock_responses: None,
            #[cfg(feature = "test-hooks")]
            recorded_requests: None,
            #[cfg(feature = "test-hooks")]
            recorded_outgoing: None,
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
            version: HttpVersion::H1,
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
            NetworkToRenderer::EventSourceEvent(
                2,
                crate::sse::SseEvent::Connected {
                    final_url: url::Url::parse("https://example.com/stream").unwrap(),
                },
            ),
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
    fn has_pending_event_for_conn_matches_only_buffered_sse_for_that_conn() {
        let renderer = NetworkHandle::disconnected();
        // No buffered events → no pending event for any conn.
        assert!(!renderer.has_pending_event_for_conn(7));

        renderer.rebuffer_events(vec![
            NetworkToRenderer::WebSocketEvent(7, crate::ws::WsEvent::TextMessage("ws".into())),
            NetworkToRenderer::EventSourceEvent(
                7,
                crate::sse::SseEvent::Event {
                    event_type: "foo".into(),
                    data: "hi".into(),
                    last_event_id: String::new(),
                },
            ),
        ]);
        // Matches the SSE event for conn 7 (NOT the WS event, NOT another conn).
        assert!(renderer.has_pending_event_for_conn(7));
        assert!(!renderer.has_pending_event_for_conn(9));

        // The peek settles the channel into the buffer but leaves the already-
        // buffered events intact (no channel here → nothing moves), so a repeat
        // peek still reports the same and the later drain still returns both.
        assert!(renderer.has_pending_event_for_conn(7));
        let drained = renderer.drain_events();
        assert_eq!(drained.len(), 2);
        // Drained → no longer pending.
        assert!(!renderer.has_pending_event_for_conn(7));
    }

    #[test]
    fn has_pending_event_for_conn_sees_channel_pending_sse_before_first_drain() {
        // Codex R2a: an inbound `EventSourceEvent` sitting in `response_rx` (not
        // yet moved to `buffered`) — e.g. a GC firing between arrival and the very
        // first drain — MUST be visible to the peek, else the §9.2.9 task-queued
        // clause reports "no queued task" for a conn that has one → F3 silent
        // drop. The peek settles the channel into the buffer (same
        // `process_response` routing a drain uses), so it covers the full inbound
        // pipeline (channel + buffer), not just already-buffered events.
        let (renderer, response_tx) = handle_with_injectable_channel();
        // Nothing buffered yet; the event lives only on the channel.
        response_tx
            .send(NetworkToRenderer::EventSourceEvent(
                11,
                crate::sse::SseEvent::Event {
                    event_type: "foo".into(),
                    data: "hi".into(),
                    last_event_id: String::new(),
                },
            ))
            .unwrap();
        // A buffered-only scan would miss this; the peek settles the channel first.
        assert!(
            renderer.has_pending_event_for_conn(11),
            "a channel-pending SSE event must be seen by the peek (R2a)",
        );
        // Wrong conn still doesn't match, and the settle didn't drop the event:
        // a subsequent `drain_events` still delivers it.
        assert!(!renderer.has_pending_event_for_conn(9));
        let drained = renderer.drain_events();
        assert_eq!(drained.len(), 1);
        assert!(matches!(
            drained[0],
            NetworkToRenderer::EventSourceEvent(11, _)
        ));
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

    fn sse_event(conn: u64, data: &str) -> NetworkToRenderer {
        NetworkToRenderer::EventSourceEvent(
            conn,
            crate::sse::SseEvent::Event {
                event_type: "message".into(),
                data: data.into(),
                last_event_id: String::new(),
            },
        )
    }

    fn ws_text(conn: u64, txt: &str) -> NetworkToRenderer {
        NetworkToRenderer::WebSocketEvent(conn, crate::ws::WsEvent::TextMessage(txt.into()))
    }

    #[test]
    fn pop_buffered_front_returns_front_leaving_tail_buffered() {
        // The count-bounded batch drain's per-event step (the elidex-js VM's
        // `tick_network` loop, Codex R3 finding 3): each `pop_buffered_front`
        // returns the FRONT event and leaves the rest in the buffer, so a
        // `has_pending_event_for_conn` scan between pops still sees the
        // not-yet-pulled events for every conn.
        let renderer = NetworkHandle::disconnected();
        renderer.rebuffer_events(vec![
            sse_event(0, "a"),
            sse_event(1, "b"),
            sse_event(0, "c"),
        ]);
        assert_eq!(renderer.buffered_len(), 3);

        // Before pulling: both conns show pending.
        assert!(renderer.has_pending_event_for_conn(0));
        assert!(renderer.has_pending_event_for_conn(1));

        // Pop #1 (conn 0 "a"); conn 1's event is still buffered.
        let e0 = renderer.pop_buffered_front().expect("event 1");
        assert!(matches!(e0, NetworkToRenderer::EventSourceEvent(0, _)));
        assert!(
            renderer.has_pending_event_for_conn(1),
            "sibling conn 1's event must remain buffered while conn 0's dispatches (R3)",
        );

        // Pop #2 (conn 1 "b").
        let e1 = renderer.pop_buffered_front().expect("event 2");
        assert!(matches!(e1, NetworkToRenderer::EventSourceEvent(1, _)));

        // Pop #3 (conn 0 "c").
        let e2 = renderer.pop_buffered_front().expect("event 3");
        assert!(matches!(e2, NetworkToRenderer::EventSourceEvent(0, _)));

        // Drained.
        assert!(renderer.pop_buffered_front().is_none());
        assert!(!renderer.has_pending_event_for_conn(0));
        assert!(!renderer.has_pending_event_for_conn(1));
    }

    #[test]
    fn pop_buffered_front_does_not_settle_channel() {
        // Buffered-only guard (Codex R4): a channel-pending event (not yet in
        // `buffered`) is NOT visible to `pop_buffered_front` — it does NOT settle
        // the channel, so the front pop of an empty buffer returns `None` even
        // though the channel holds an event.  That event is still recoverable via
        // a subsequent drain (nothing was dropped).
        let (renderer, response_tx) = handle_with_injectable_channel();
        response_tx.send(sse_event(3, "x")).unwrap();
        // Buffer empty, channel NOT settled — the no-settle guard.
        assert!(
            renderer.pop_buffered_front().is_none(),
            "pop_buffered_front must NOT settle the channel (buffered-only)",
        );
        // The event is still recoverable: a real drain settles and returns it.
        let drained = renderer.drain_events();
        assert_eq!(drained.len(), 1);
        assert!(matches!(
            drained[0],
            NetworkToRenderer::EventSourceEvent(3, _)
        ));
    }

    #[test]
    fn pop_buffered_front_bookkeeps_raw_buffered_fetchresponse() {
        // A `FetchResponse` sitting directly in `buffered` (raw-buffered, not
        // arriving via the channel) still needs its `outstanding_fetches` id
        // removed when popped — `pop_buffered_front` routes the popped event
        // through `process_response` for exactly that bookkeeping (mirrors the
        // old marker test's bookkeeping intent, minus the marker — markers are
        // never buffered).
        let renderer = NetworkHandle::disconnected();
        let fid = FetchId::next();
        renderer.outstanding_fetches.borrow_mut().insert(fid);
        renderer.rebuffer_events(vec![NetworkToRenderer::FetchResponse(
            fid,
            Ok(ok_response()),
        )]);

        let ev = renderer
            .pop_buffered_front()
            .expect("raw-buffered FetchResponse popped");
        match ev {
            NetworkToRenderer::FetchResponse(id, Ok(_)) => assert_eq!(id, fid),
            other => panic!("expected FetchResponse, got {other:?}"),
        }
        assert!(
            !renderer.outstanding_fetches.borrow().contains(&fid),
            "popping a raw-buffered FetchResponse must remove its outstanding_fetches id",
        );
    }

    #[test]
    fn count_bounded_batch_leaves_mid_loop_channel_arrival_for_next_tick() {
        // Codex R4 F1 (hard-bound the tick) at broker level: the VM's
        // `tick_network` runs a COUNT-BOUNDED batch — snapshot `batch =
        // buffered_len()` after the fetch drain, then pop exactly `batch` events
        // buffered-only.  A mid-loop channel arrival lands PAST the batch
        // boundary and is NOT consumed this tick (the old whole-buffer pop
        // re-settled the channel every iteration and livelocked under a busy
        // stream).  The arrival is recovered by the next tick's drain.
        let (renderer, response_tx) = handle_with_injectable_channel();
        renderer.rebuffer_events(vec![ws_text(1, "w1"), ws_text(1, "w2")]);
        let batch = renderer.buffered_len();
        assert_eq!(batch, 2);

        // Simulate a mid-loop network-thread arrival on the channel.
        response_tx.send(ws_text(1, "w3")).unwrap();

        // The count-bounded loop: exactly `batch` pops, buffered-only.
        let mut dispatched: Vec<NetworkToRenderer> = Vec::new();
        let mut iters = 0;
        for _ in 0..batch {
            iters += 1;
            let Some(ev) = renderer.pop_buffered_front() else {
                break;
            };
            dispatched.push(ev);
        }
        assert_eq!(iters, batch, "the loop ran exactly `batch` times");
        assert_eq!(dispatched.len(), 2);
        let texts: Vec<String> = dispatched
            .iter()
            .map(|e| match e {
                NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                    s.clone()
                }
                other => panic!("expected WS text, got {other:?}"),
            })
            .collect();
        assert_eq!(texts, vec!["w1".to_string(), "w2".to_string()]);
        assert!(
            !texts.iter().any(|t| t == "w3"),
            "the mid-loop arrival must NOT be consumed by this batch (hard bound)",
        );

        // Next tick: the drain settles the channel and recovers "w3".
        let next = renderer.drain_events();
        assert_eq!(next.len(), 1);
        match &next[0] {
            NetworkToRenderer::WebSocketEvent(_, crate::ws::WsEvent::TextMessage(s)) => {
                assert_eq!(s, "w3");
            }
            other => panic!("expected WS('w3') next tick, got {other:?}"),
        }
    }

    #[test]
    fn count_bounded_batch_does_not_drop_mid_loop_fetch_reply() {
        // Codex R4 F2 (fetch reply not dropped) at broker level: a
        // `FetchResponse` arriving mid-callback (on the channel, after the fetch
        // drain ran this tick) must NOT be pulled into the realtime batch — the
        // old per-iteration settle pulled it into `buffered`, cleared its
        // `outstanding_fetches` id, then the realtime `FetchResponse => {}` arm
        // DROPPED it → Promise leak.  Count-bounded buffered-only pops leave it
        // in `response_rx` for the next tick's `drain_fetch_responses_only`.
        let (renderer, response_tx) = handle_with_injectable_channel();
        renderer.rebuffer_events(vec![ws_text(1, "w1")]);
        let batch = renderer.buffered_len();
        assert_eq!(batch, 1);

        // Simulate a mid-callback fetch reply on the channel.
        let fid = FetchId::next();
        response_tx
            .send(NetworkToRenderer::FetchResponse(fid, Ok(ok_response())))
            .unwrap();

        // The count-bounded loop dispatches only the buffered WS event.
        let mut dispatched: Vec<NetworkToRenderer> = Vec::new();
        for _ in 0..batch {
            let Some(ev) = renderer.pop_buffered_front() else {
                break;
            };
            dispatched.push(ev);
        }
        assert_eq!(dispatched.len(), 1);
        assert!(
            matches!(
                dispatched[0],
                NetworkToRenderer::WebSocketEvent(1, crate::ws::WsEvent::TextMessage(_))
            ),
            "only the buffered WS event is dispatched this batch",
        );

        // The fetch reply is recovered — settle-able next tick, NOT dropped.
        let fetches = renderer.drain_fetch_responses_only();
        let ids: Vec<FetchId> = fetches.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![fid]);
        assert!(fetches[0].1.is_ok());
    }

    #[test]
    fn count_bounded_batch_does_not_drop_gc_settled_fetch_reply() {
        // Codex R4 F2 GC-settle residual — the path the count bound closes but the
        // pure-channel F2 test (`count_bounded_batch_does_not_drop_mid_loop_fetch_reply`)
        // does NOT exercise. During batch dispatch a GC fires and
        // `has_pending_event_for_conn` runs `settle_channel_into_buffer`, appending a
        // mid-callback FetchResponse to the BACK of `buffered` (past the batch
        // boundary snapshotted before the loop). The count bound must NOT pop it this
        // tick; the next tick's `drain_fetch_responses_only` recovers + settles it.
        let (renderer, response_tx) = handle_with_injectable_channel();
        renderer.rebuffer_events(vec![ws_text(1, "w1"), ws_text(1, "w2")]);
        let batch = renderer.buffered_len();
        assert_eq!(batch, 2);

        let mut dispatched: Vec<NetworkToRenderer> = Vec::new();
        for i in 0..batch {
            let Some(ev) = renderer.pop_buffered_front() else {
                break;
            };
            dispatched.push(ev);
            if i == 0 {
                // Simulate a GC firing during dispatch of event 0: a mid-callback
                // FetchResponse arrives on the channel and a keepalive peek
                // (`has_pending_event_for_conn`) settles it into `buffered`'s BACK.
                let fid = FetchId::next();
                response_tx
                    .send(NetworkToRenderer::FetchResponse(fid, Ok(ok_response())))
                    .unwrap();
                let _ = renderer.has_pending_event_for_conn(999);
                // `buffered` is now [w2, FetchResponse(fid)] — the FetchResponse is
                // past the batch boundary (index 1 >= remaining batch slots).
            }
        }
        // Only the two batched WS events were dispatched — the GC-settled
        // FetchResponse was NOT popped by the count-bounded loop.
        assert_eq!(dispatched.len(), 2);
        assert!(dispatched
            .iter()
            .all(|e| matches!(e, NetworkToRenderer::WebSocketEvent(..))));

        // The GC-settled FetchResponse is recovered next tick, NOT dropped.
        let fetches = renderer.drain_fetch_responses_only();
        let ids: Vec<FetchId> = fetches.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 1);
        assert!(fetches[0].1.is_ok());
    }
}
