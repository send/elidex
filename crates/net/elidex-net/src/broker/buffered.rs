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
