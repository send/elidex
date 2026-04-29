//! M4-12 PR5-async-fetch: Promise/abort lifecycle tests for the
//! async `fetch()` path (WHATWG Fetch §5.1).
//!
//! These tests focus on behaviours that *depend* on the
//! broker-reply-via-`tick_network` indirection: the Promise stays
//! pending across the fetch dispatch, an explicit `tick_network`
//! call is required to settle it, and abort fan-out can interpose
//! between dispatch and reply.
//!
//! Round-trip happy-path coverage continues to live in
//! `tests_fetch.rs`; this file complements it with the
//! abort + dedup edges that the synchronous-blocking variant
//! could not exercise.

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};

use super::super::value::JsValue;
use super::super::Vm;

fn ok_response(url: &str, body: &'static str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 200,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

fn mock_vm(responses: Vec<(url::Url, Result<NetResponse, String>)>) -> Vm {
    let mut vm = Vm::new();
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(responses)));
    vm
}

fn drain(vm: &mut Vm) {
    for _ in 0..16 {
        if vm.inner.pending_fetches.is_empty() {
            break;
        }
        vm.tick_network();
    }
    vm.tick_network();
}

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
fn inflight_abort_rejects_with_signal_reason_synchronously() {
    // Mid-flight `controller.abort()` (between dispatch and tick)
    // settles the Promise with the signal's reason synchronously.
    let url = url::Url::parse("http://example.com/inflight").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/inflight", "never")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         var c = new AbortController(); \
         fetch('http://example.com/inflight', {signal: c.signal}) \
             .catch(e => { globalThis.r = e instanceof DOMException && e.name; }); \
         c.abort();",
    )
    .unwrap();
    // The Promise rejected synchronously inside `c.abort()` — the
    // microtask checkpoint at the end of `eval` ran the `.catch`
    // reaction, so `r` is already set without a tick_network.
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("expected r to be 'AbortError', got {other:?}"),
    }
    // Tick now: the broker's eventual reply (mock pre-buffered the
    // 200 OK response into the handle on the original `fetch_async`
    // call) must NOT settle the Promise a second time — the
    // `.then`/`.catch` reactions only fire once.
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("late broker reply must not overwrite the abort-rejection, got {other:?}"),
    }
}

#[test]
fn inflight_abort_propagates_custom_reason() {
    let url = url::Url::parse("http://example.com/inflight").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/inflight", "x")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         var c = new AbortController(); \
         fetch('http://example.com/inflight', {signal: c.signal}) \
             .catch(e => { globalThis.r = e; }); \
         c.abort('user-cancel');",
    )
    .unwrap();
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "user-cancel"),
        other => panic!("expected r to be 'user-cancel', got {other:?}"),
    }
}

#[test]
fn shared_signal_aborts_multiple_inflight_fetches_atomically() {
    // Two parallel fetches share a single signal; one `controller.
    // abort()` must reject both Promises with the same reason.
    let url1 = url::Url::parse("http://example.com/a").expect("valid");
    let url2 = url::Url::parse("http://example.com/b").expect("valid");
    let mut vm = mock_vm(vec![
        (url1, Ok(ok_response("http://example.com/a", "a"))),
        (url2, Ok(ok_response("http://example.com/b", "b"))),
    ]);
    vm.eval(
        "globalThis.ra = ''; \
         globalThis.rb = ''; \
         var c = new AbortController(); \
         fetch('http://example.com/a', {signal: c.signal}) \
             .catch(e => { globalThis.ra = e instanceof DOMException && e.name; }); \
         fetch('http://example.com/b', {signal: c.signal}) \
             .catch(e => { globalThis.rb = e instanceof DOMException && e.name; }); \
         c.abort();",
    )
    .unwrap();
    for (key, expect) in [("ra", "AbortError"), ("rb", "AbortError")] {
        match vm.get_global(key) {
            Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), expect),
            other => panic!("expected {key} to be '{expect}', got {other:?}"),
        }
    }
    // Subsequent tick must not surface the broker's late replies.
    drain(&mut vm);
    for (key, expect) in [("ra", "AbortError"), ("rb", "AbortError")] {
        match vm.get_global(key) {
            Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), expect),
            other => panic!("late reply must not overwrite {key}: {other:?}"),
        }
    }
}

#[test]
fn settle_fetch_roots_promise_across_response_alloc() {
    // R2.2 regression: `settle_fetch` removes the Promise from
    // `pending_fetches` (its sole root for user-discarded promises)
    // before allocating the `Response` + companion `Headers` + body
    // bytes via `create_response_from_net`.  The defensive
    // `push_temp_root` introduced in R2 must keep the Promise alive
    // across that allocation.  Today `gc_enabled = false` inside
    // native calls keeps the bare-remove path "safe" by accident; a
    // forced GC immediately *after* tick_network confirms no stale
    // Promise slot was recycled (the `.then` reaction would reach a
    // collected slot and panic / observe garbage).
    let url = url::Url::parse("http://example.com/gc-settle").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/gc-settle", "ok")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         (function () { \
              fetch('http://example.com/gc-settle') \
                  .then(resp => resp.text()) \
                  .then(body => { globalThis.r = body.length; }); \
         })();",
    )
    .unwrap();
    drain(&mut vm);
    vm.inner.collect_garbage();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 2.0).abs() < f64::EPSILON),
        other => panic!("expected r to be body.length=2, got {other:?}"),
    }
}

#[test]
fn abort_fan_out_roots_promise_across_rejection() {
    // R2.1 regression: same root-before-settle pattern in
    // `abort_signal`'s fan-out.  Each pending Promise is removed
    // from `pending_fetches` and rejected; the temp root must hold
    // it across `reject_promise_sync`.  Forced GC after the abort
    // confirms no slot was recycled.
    let url = url::Url::parse("http://example.com/gc-abort").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/gc-abort", "x")),
    )]);
    vm.eval(
        "globalThis.r = ''; \
         (function () { \
              var c = new AbortController(); \
              fetch('http://example.com/gc-abort', {signal: c.signal}) \
                  .catch(e => { globalThis.r = e instanceof DOMException && e.name; }); \
              c.abort(); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("expected r to be 'AbortError', got {other:?}"),
    }
}

#[test]
fn promise_survives_user_dropping_reference() {
    // The user dropped every JS-side reference to the returned
    // Promise; the only path keeping it alive is
    // `VmInner::pending_fetches`.  GC must keep the Promise rooted
    // long enough for the broker reply to settle the chained
    // reaction.
    let url = url::Url::parse("http://example.com/discarded").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/discarded", "ok")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         (function () { \
              fetch('http://example.com/discarded') \
                  .then(resp => { globalThis.r = resp.status; }); \
         })(); \
         /* IIFE returned; the Promise is unreachable from script. */",
    )
    .unwrap();
    // Force a GC cycle before settlement to verify the root holds.
    vm.inner.collect_garbage();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected r to be 200, got {other:?}"),
    }
}

#[test]
fn cancel_fetch_reaches_broker_on_abort() {
    // Verify the broker side: aborting an in-flight fetch with a
    // real (non-mock) NetworkHandle would call
    // `handle.cancel_fetch(id)` which sends a CancelFetch over the
    // request channel.  The mock handle does not stage this wire
    // (its `send` is a no-op on disconnected channels), but the
    // observable JS-side effect — Promise rejected — is asserted in
    // `inflight_abort_rejects_with_signal_reason_synchronously`.
    // This test exercises the integration via the real broker
    // handle.
    use elidex_net::broker::spawn_network_process;
    use elidex_net::{NetClient, NetClientConfig, TransportConfig};

    let np = spawn_network_process(NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        },
        ..Default::default()
    }));
    let renderer_handle = np.create_renderer_handle();
    let mut vm = Vm::new();
    vm.install_network_handle(Rc::new(renderer_handle));

    // Bind a sync server that never replies.  The test relies on
    // the broker's CancelFetch handler synthesising an Err("aborted")
    // reply within the cancel path.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let _hold = listener; // keep the socket open

    let script = format!(
        "globalThis.r = ''; \
         var c = new AbortController(); \
         fetch('http://127.0.0.1:{}/', {{signal: c.signal}}) \
             .catch(e => {{ globalThis.r = e instanceof DOMException && e.name; }}); \
         c.abort();",
        addr.port()
    );
    vm.eval(&script).unwrap();
    // Synchronous abort fires the rejection inline; tick to sweep
    // the broker's eventual aborted-reply (silently dropped).
    for _ in 0..32 {
        vm.tick_network();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("expected r to be 'AbortError', got {other:?}"),
    }

    // Drop the VM (and its NetworkHandle) before the broker —
    // unregisters the renderer cleanly.  The reach of this test is
    // the JS-observable Promise rejection above; deeper assertions
    // about broker-side state (e.g. CancelFetch arrival counts) are
    // covered by `cancel_fetch_delivers_aborted_reply` in
    // `crates/net/elidex-net/src/broker.rs`'s test module.
    drop(vm);
    np.shutdown();
}

#[test]
fn tick_network_re_buffers_unhandled_ws_sse_events() {
    // R1.2: the VM's `tick_network` only consumes `FetchResponse`
    // events; any `WebSocketEvent` / `EventSourceEvent` that hits
    // the same handle must be re-buffered so a sibling consumer
    // (boa bridge during the boa→VM cutover, or future VM-side
    // WS module) still observes them on its own `drain_events`.
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
fn tick_network_preserves_event_order_across_fetch_and_ws() {
    // R3.2 regression: when WS / SSE events are interleaved with
    // fetch replies in the broker buffer, `tick_network` must NOT
    // reorder fetch settlements ahead of preceding WS/SSE events.
    // The fix is to settle fetch replies only up to the first
    // non-fetch event, then re-buffer that event AND every event
    // after it.  Sibling consumers see the original sequence; a
    // later VM tick picks up trailing fetch replies once the WS
    // events have been drained externally.
    use elidex_net::broker::NetworkToRenderer;
    use elidex_net::ws::WsEvent;
    let url = url::Url::parse("http://example.com/ordered").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/ordered", "ok")),
    )]);
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    // Stage: [WS_a, Fetch_b (already buffered by mock fetch_async),
    // WS_c].  We dispatch the fetch first to seed the FetchResponse
    // into `buffered`, then prepend a WS event before it via
    // rebuffer + append a trailing WS event.
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/ordered').then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    // After eval, `buffered` contains exactly [FetchResponse(...)].
    // Insert a WS event before it (rebuffer is splice-front), and
    // a WS event after it.  Use rebuffer for the front-prepend.
    handle.rebuffer_events(vec![NetworkToRenderer::WebSocketEvent(
        1,
        WsEvent::TextMessage("before".to_string()),
    )]);
    // Append the trailing WS event by re-buffering after the
    // fetch reply: we drain everything, then re-buffer in the
    // desired order [WS_a, FetchResponse, WS_c].
    let drained = handle.drain_events();
    let mut staged: Vec<NetworkToRenderer> = drained;
    staged.push(NetworkToRenderer::WebSocketEvent(
        1,
        WsEvent::TextMessage("after".to_string()),
    ));
    handle.rebuffer_events(staged);

    vm.tick_network();
    // The first event was a WS, so tick_network must NOT have
    // settled the fetch — the entire sequence must be re-buffered
    // verbatim.
    let leftover = handle.drain_events();
    assert_eq!(
        leftover.len(),
        3,
        "all events must remain when WS comes first"
    );
    assert!(matches!(
        leftover[0],
        NetworkToRenderer::WebSocketEvent(_, _)
    ));
    assert!(matches!(
        leftover[1],
        NetworkToRenderer::FetchResponse(_, _)
    ));
    assert!(matches!(
        leftover[2],
        NetworkToRenderer::WebSocketEvent(_, _)
    ));
    // Promise still pending — fetch reply not consumed yet.
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 0.0).abs() < f64::EPSILON),
        other => panic!("fetch must NOT have settled, got {other:?}"),
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

#[test]
fn signal_back_refs_pruned_on_settlement() {
    // After a successful `tick_network` settle, the back-refs
    // table must be empty — otherwise a subsequent `controller.
    // abort()` would chase a stale FetchId and try to send a
    // redundant CancelFetch.
    let url = url::Url::parse("http://example.com/sig-prune").expect("valid");
    let mut vm = mock_vm(vec![(
        url,
        Ok(ok_response("http://example.com/sig-prune", "ok")),
    )]);
    vm.eval(
        "globalThis.r = 0; \
         globalThis.c = new AbortController(); \
         fetch('http://example.com/sig-prune', {signal: c.signal}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(
        vm.inner.pending_fetches.len(),
        0,
        "pending_fetches must be empty after settle"
    );
    assert_eq!(
        vm.inner.fetch_signal_back_refs.len(),
        0,
        "fetch_signal_back_refs must be empty after settle"
    );
    // Aborting now is a no-op for the already-settled fetch — and
    // must not double-fire any Promise reaction.
    vm.eval("c.abort();").unwrap();
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("late abort must not retro-reject: {other:?}"),
    }
}
