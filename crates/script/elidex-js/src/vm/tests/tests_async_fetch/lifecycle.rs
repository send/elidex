//! Basic Promise lifecycle (`tick_network`, microtask drain), WS/SSE
//! interaction with the same handle, and `install_network_handle`
//! semantics.
//!
//! Companion to [`super::abort`] (`controller.abort()` fan-out) and
//! [`super::cors`] (PR5-cors Stages 3 / 4 / 5).

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{drain, mock_vm, ok_response};

#[test]
fn promise_stays_pending_until_tick_network() {
    // Without an explicit `tick_network`, the broker reply sits in
    // the handle's buffer and the Promise stays Pending — observable
    // via `globalThis.r` never being assigned.  Tick once and the
    // `.then` reaction fires.
    let url = url::Url::parse("http://example.com/pending").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/pending", "x")),
    )]);
    vm.eval(
        "globalThis.r = 'untouched'; \
         fetch('http://example.com/pending').then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    // No tick_network yet — the reaction must not have fired.
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "untouched"),
        other => panic!("expected r untouched, got {other:?}"),
    }
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200 after tick_network, got {other:?}"),
    }
}

#[test]
fn tick_network_with_no_pending_is_noop() {
    // Idempotent / cheap when nothing is in flight.  Both with and
    // without a handle installed.
    let mut vm = Vm::new();
    vm.tick_network();
    let mut vm2 = mock_vm(vec![]);
    vm2.tick_network();
    vm2.tick_network();
}

#[test]
fn tick_network_drains_microtasks_without_handle() {
    // R4.1 regression: the public `Vm::tick_network` contract
    // promises a microtask checkpoint at the end "unconditionally";
    // earlier impl skipped the drain when no NetworkHandle was
    // installed, leaving queued reactions un-drained on
    // handle-less embedders.  Verify by enqueuing a Promise
    // reaction inside an immediate-fulfilled `.then` chain (which
    // the eval microtask drain already runs), then a *second*
    // `.then` whose reaction is queued for the next drain.  After
    // a handle-less `tick_network`, the deferred reaction must
    // have run.
    let mut vm = Vm::new();
    // No `install_network_handle` — handle is None.
    vm.eval(
        "globalThis.r = 0; \
         Promise.resolve(7).then(v => Promise.resolve(v).then(w => { globalThis.r = w; }));",
    )
    .unwrap();
    // Eval drains microtasks once at end-of-script, but the
    // chained .then queues a second reaction that runs at the
    // *next* drain.  Verify with a simple counter: r should be 7
    // after the second drain.
    vm.tick_network();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 7.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 7 after handle-less tick, got {other:?}"),
    }
}

#[test]
fn tick_network_leaves_unhandled_ws_sse_events_in_buffer() {
    // The VM's `tick_network` only consumes `FetchResponse`
    // events; any `WebSocketEvent` / `EventSourceEvent` that hits
    // the same handle must remain in the broker handle's internal
    // buffer so a sibling consumer (boa bridge during the boa→VM
    // cutover, or future VM-side WS module) still observes them
    // on its own `drain_events`.  Slot #6.8 implements this via
    // `NetworkHandle::drain_fetch_responses_only`'s partition-in-
    // place semantics.
    use elidex_net::broker::NetworkToRenderer;
    use elidex_net::ws::WsEvent;
    let mut vm = mock_vm(vec![]);
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    handle.rebuffer_events(vec![
        NetworkToRenderer::WebSocketEvent(7, WsEvent::TextMessage("hi".to_string())),
        NetworkToRenderer::WebSocketEvent(7, WsEvent::BytesSent(2)),
    ]);
    vm.tick_network();
    let leftover = handle.drain_events();
    assert_eq!(leftover.len(), 2, "WS events must survive tick_network");
    for ev in &leftover {
        assert!(matches!(ev, NetworkToRenderer::WebSocketEvent(_, _)));
    }
}

#[test]
fn tick_network_settles_fetch_and_keeps_surrounding_ws_in_order() {
    // Slot #6.8 contract: `tick_network` partitions fetch replies
    // out of the broker buffer in a single pass, settles them,
    // and leaves every non-fetch event in the buffer in its
    // original relative order.  Pre-#6.8 behaviour stopped at the
    // first non-fetch event and re-buffered the tail (including
    // any later fetch replies) — this test exercises the post-#6.8
    // contract on a [WS_before, FetchResponse, WS_after] stage.
    use elidex_net::broker::NetworkToRenderer;
    use elidex_net::ws::WsEvent;
    let url = url::Url::parse("http://example.com/ordered").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ordered", "ok")),
    )]);
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    // Stage: dispatch the fetch (mock seeds FetchResponse into
    // `buffered`), then drain + re-stage as [WS_before, FetchResponse,
    // WS_after] via a single rebuffer call.
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/ordered').then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    let drained = handle.drain_events();
    assert_eq!(drained.len(), 1, "exactly one buffered FetchResponse");
    let mut staged: Vec<NetworkToRenderer> = vec![NetworkToRenderer::WebSocketEvent(
        1,
        WsEvent::TextMessage("before".to_string()),
    )];
    staged.extend(drained);
    staged.push(NetworkToRenderer::WebSocketEvent(
        1,
        WsEvent::TextMessage("after".to_string()),
    ));
    handle.rebuffer_events(staged);

    vm.tick_network();

    // FetchResponse extracted and settled — promise reaction ran.
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("fetch must have settled to 200, got {other:?}"),
    }
    // Both WS events remain in original order; no FetchResponse
    // left over.
    let leftover = handle.drain_events();
    assert_eq!(leftover.len(), 2, "both WS events must remain");
    match &leftover[0] {
        NetworkToRenderer::WebSocketEvent(_, WsEvent::TextMessage(s)) => {
            assert_eq!(s, "before");
        }
        other => panic!("expected WS('before'), got {other:?}"),
    }
    match &leftover[1] {
        NetworkToRenderer::WebSocketEvent(_, WsEvent::TextMessage(s)) => {
            assert_eq!(s, "after");
        }
        other => panic!("expected WS('after'), got {other:?}"),
    }
}

#[test]
fn install_network_handle_rejects_pending_fetches_against_old_handle() {
    // R3.3 regression: replacing the NetworkHandle while
    // `pending_fetches` is non-empty would otherwise leave those
    // Promises permanently un-settleable (the old handle's
    // response channel is no longer drained).  Install must
    // proactively reject every pending Promise with TypeError
    // before swapping.
    let url = url::Url::parse("http://example.com/replaced").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/replaced", "ok")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         fetch('http://example.com/replaced') \
             .catch(e => { globalThis.r = e instanceof TypeError && e.message; });",
    )
    .unwrap();
    // Promise is pending — fetch dispatched, no tick_network yet.
    assert_eq!(vm.inner.pending_fetches.len(), 1);
    // Replace the handle with a fresh disconnected one.
    vm.install_network_handle(Rc::new(NetworkHandle::disconnected()));
    assert_eq!(
        vm.inner.pending_fetches.len(),
        0,
        "install_network_handle must drain pending_fetches"
    );
    assert_eq!(
        vm.inner.fetch_signal_back_refs.len(),
        0,
        "back-refs must also be cleared"
    );
    // The reject reaction's microtask still needs to drain.  Any
    // subsequent eval / tick_network triggers it.
    vm.tick_network();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let s = vm.get_string(id);
            assert!(s.contains("Failed to fetch"), "got: {s}");
            assert!(s.contains("NetworkHandle replaced"), "got: {s}");
        }
        other => panic!("expected TypeError message, got {other:?}"),
    }
}

#[test]
fn install_network_handle_same_rc_is_noop_for_pending_fetches() {
    // R6.1 regression: re-installing the same Rc<NetworkHandle>
    // (pointer-equal) must NOT spuriously reject in-flight
    // requests.  This protects benign re-install patterns where
    // an embedder threads the handle through a shared accessor.
    let url = url::Url::parse("http://example.com/same-rc").expect("valid");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![(
        url,
        Ok(ok_response("http://example.com/same-rc", "ok")),
    )]));
    let mut vm = Vm::new();
    // Same-origin context so the fetch classifies as Basic
    // (Copilot R3 — without explicit document origin the default
    // about:blank → opaque origin would force the cors path).
    vm.inner.navigation.current_url = url::Url::parse("http://example.com/page").expect("valid");
    vm.install_network_handle(Rc::clone(&handle));
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/same-rc').then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    assert_eq!(vm.inner.pending_fetches.len(), 1, "fetch dispatched");
    // Re-install the SAME Rc — must preserve pending_fetches.
    vm.install_network_handle(Rc::clone(&handle));
    assert_eq!(
        vm.inner.pending_fetches.len(),
        1,
        "same-Rc re-install must not drain pending_fetches"
    );
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn install_network_handle_rejects_signal_bound_pending_fetch() {
    // Variant of the above where the pending fetch carried a
    // signal — verify the back-refs + abort observers maps are
    // also cleared so a subsequent `controller.abort()` becomes a
    // pure no-op (no panic, no orphan CancelFetch send).
    let url = url::Url::parse("http://example.com/sig-replaced").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/sig-replaced", "ok")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         globalThis.c = new AbortController(); \
         fetch('http://example.com/sig-replaced', {signal: c.signal}) \
             .catch(e => { globalThis.r = e && e.message; });",
    )
    .unwrap();
    assert_eq!(vm.inner.pending_fetches.len(), 1);
    assert_eq!(vm.inner.fetch_signal_back_refs.len(), 1);
    assert_eq!(vm.inner.fetch_abort_observers.len(), 1);
    vm.install_network_handle(Rc::new(NetworkHandle::disconnected()));
    assert_eq!(vm.inner.pending_fetches.len(), 0);
    assert_eq!(vm.inner.fetch_signal_back_refs.len(), 0);
    assert_eq!(
        vm.inner.fetch_abort_observers.len(),
        0,
        "observer entry must be removed when its only fetch_id was rejected"
    );
    // Aborting now is a no-op.
    vm.eval("c.abort();").unwrap();
    vm.tick_network();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            assert!(vm.get_string(id).contains("NetworkHandle replaced"));
        }
        other => panic!("expected reject message, got {other:?}"),
    }
}
