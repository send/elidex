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
