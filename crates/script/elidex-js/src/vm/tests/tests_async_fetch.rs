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
    // Default the document origin to `http://example.com/page` so
    // the lifecycle tests below (which fetch `http://example.com/...`
    // URLs) classify as same-origin → `Basic`.  Without this, the
    // default `about:blank` origin would become opaque after Copilot
    // R3 fix, making every fetch cross-origin and tripping the
    // `NetworkError` (no ACAO) path — these tests aren't about CORS,
    // they're about the Promise / abort lifecycle.
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page").expect("valid base URL");
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(responses)));
    vm
}

use super::drain_fetch_replies as drain;

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
    // Verify both halves of the abort wire against a real (non-mock)
    // NetworkHandle:
    //
    //   (a) JS-observable: Promise rejects with AbortError
    //       synchronously inside `controller.abort()` via the VM's
    //       abort fan-out.
    //   (b) Broker-observable: the broker receives the
    //       `RendererToNetwork::CancelFetch` and synthesises an
    //       `Err("aborted")` `FetchResponse` for the same FetchId,
    //       which arrives on the renderer's response channel.
    //       Without (b), `NetworkHandle::cancel_fetch` could be a
    //       no-op and the JS-side test would still pass — that is
    //       the gap R5.1 flagged.
    //
    // We pre-clone the handle Rc so the test can drain events
    // *before* `vm.tick_network()` consumes them; otherwise the
    // VM's settle-fetch path would silently absorb the aborted
    // reply (the Promise was already settled by abort fan-out) and
    // the broker side would be unobservable from the test.
    use elidex_net::broker::{spawn_network_process, NetworkHandle, NetworkToRenderer};
    use elidex_net::{NetClient, NetClientConfig, TransportConfig};

    let np = spawn_network_process(NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        },
        ..Default::default()
    }));
    let renderer_handle: Rc<NetworkHandle> = Rc::new(np.create_renderer_handle());
    let mut vm = Vm::new();
    vm.install_network_handle(Rc::clone(&renderer_handle));

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

    // (b) Drain handle events directly (without going through
    // `vm.tick_network`) and wait for the broker-synthesised
    // aborted reply to land.  Polling because the broker thread
    // is asynchronous; deadline guards against test hangs.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut got_aborted_reply = false;
    while std::time::Instant::now() < deadline && !got_aborted_reply {
        for ev in renderer_handle.drain_events() {
            if let NetworkToRenderer::FetchResponse(_, Err(msg)) = ev {
                if msg.contains("aborted") {
                    got_aborted_reply = true;
                }
            }
        }
        if !got_aborted_reply {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    assert!(
        got_aborted_reply,
        "broker must synthesise an Err(\"aborted\") FetchResponse on CancelFetch"
    );

    // (a) JS-observable: Promise was rejected synchronously inside
    // `c.abort()`; the eval microtask drain ran the `.catch`
    // reaction.  No `tick_network` needed for this assertion
    // because the broker reply was already drained out manually.
    match vm.get_global("r") {
        Some(JsValue::String(id)) => assert_eq!(vm.get_string(id), "AbortError"),
        other => panic!("expected r to be 'AbortError', got {other:?}"),
    }

    // Drop the VM (and its NetworkHandle Rc) before the broker —
    // unregisters the renderer cleanly.
    drop(vm);
    drop(renderer_handle);
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

// ---------------------------------------------------------------------------
// PR5-cors Stage 3: same-origin reject + Origin / redirect / credentials
// thread.  Verifies the broker-bound `elidex_net::Request` carries the
// values selected by `init.*` plus the source document's origin.
// ---------------------------------------------------------------------------

fn vm_with_origin_and_mock(
    document_url: &str,
    target: &str,
    response: Result<NetResponse, String>,
) -> (Vm, Rc<NetworkHandle>) {
    let mut vm = Vm::new();
    vm.inner.navigation.current_url = url::Url::parse(document_url).expect("valid document URL");
    let parsed = url::Url::parse(target).expect("valid target URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![(parsed, response)]));
    vm.install_network_handle(handle.clone());
    (vm, handle)
}

#[test]
fn fetch_threads_same_origin_credentials_redirect_to_broker() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "fetch('http://example.com/api', \
              {credentials: 'omit', redirect: 'manual'});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Omit);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Manual);
    assert_eq!(
        req.origin,
        Some(url::Url::parse("http://example.com/page").unwrap().origin())
    );
}

#[test]
fn fetch_threads_request_state_to_broker_when_input_is_request() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "var req = new Request('http://example.com/api', \
             {credentials: 'include', redirect: 'error'}); \
         fetch(req);",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Include);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Error);
}

#[test]
fn fetch_threads_request_mode_to_broker() {
    // PR5-cors-preflight: `init.mode` is threaded into the broker
    // `Request.mode` so the NetClient::send preflight stage can
    // distinguish Cors / NoCors / SameOrigin without round-trip
    // conversion.  Default for `fetch()` is `Cors`.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {mode: 'no-cors'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].mode, elidex_net::RequestMode::NoCors);
}

#[test]
fn fetch_default_mode_is_cors() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    // Spec default for the fetch() URL-string input path is
    // `Cors` (see `build_net_request` in fetch.rs line 410).
    assert_eq!(logged[0].mode, elidex_net::RequestMode::Cors);
}

#[test]
fn fetch_init_overrides_request_state_for_redirect_credentials() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "var req = new Request('http://example.com/api', \
             {credentials: 'include', redirect: 'follow'}); \
         fetch(req, {credentials: 'omit', redirect: 'manual'});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let req = &logged[0];
    assert_eq!(req.credentials, elidex_net::CredentialsMode::Omit);
    assert_eq!(req.redirect, elidex_net::RedirectMode::Manual);
}

#[test]
fn fetch_same_origin_mode_rejects_cross_origin_url_with_typeerror() {
    // mode='same-origin' + cross-origin URL → synchronous rejection
    // before the broker is even contacted.  The mock has no entry
    // for the target, so a successful broker dispatch would return
    // a "no response for ..." error — different from the
    // TypeError we expect.
    let (mut vm, _handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(ok_response("http://other.com/api", "should-not-reach")),
    );
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://other.com/api', {mode: 'same-origin'}) \
             .catch(e => { globalThis.r = e.message; });",
    )
    .unwrap();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let msg = vm.get_string(id);
            assert!(
                msg.contains("cross-origin") || msg.contains("same-origin"),
                "expected same-origin rejection, got {msg:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn fetch_same_origin_mode_passes_same_origin_url() {
    let (mut vm, _handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "globalThis.r = 0; \
         fetch('http://example.com/api', {mode: 'same-origin'}) \
             .then(resp => { globalThis.r = resp.status; });",
    )
    .unwrap();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::Number(n)) => assert!((n - 200.0).abs() < f64::EPSILON),
        other => panic!("expected 200, got {other:?}"),
    }
}

#[test]
fn opaque_origin_initiator_emits_origin_null_header() {
    // Copilot R4 regression: an opaque-origin script (data: /
    // about:blank) doing a CORS-mode cross-origin fetch must
    // send `Origin: null` so the server can satisfy the CORS
    // check against the spec-mandated serialisation of opaque
    // origins.  Pre-R4, `attach_default_origin` skipped any
    // non-HTTP(S) source, so opaque-origin fetches went out
    // without an Origin header and CORS gates that key on its
    // presence would silently fail.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "about:blank",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let origin_header = logged[0]
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("origin"))
        .map(|(_, v)| v.as_str());
    assert_eq!(
        origin_header,
        Some("null"),
        "opaque initiator must send `Origin: null` for cross-origin HTTP target"
    );
}

#[test]
fn fetch_threads_opaque_origin_for_about_blank_initiator() {
    // Copilot R3 fix: `about:blank` script-initiated fetches
    // produce an opaque origin (Origin::Opaque, ascii_serialization
    // = "null") rather than `None`.  The previous behaviour —
    // returning `None` for non-HTTP(S) — caused the classifier to
    // short-circuit to `Basic`, which was a CORS bypass.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "about:blank",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api');").unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    let origin = logged[0]
        .origin
        .as_ref()
        .expect("script-initiated fetch always carries Some(origin)");
    // Opaque origins serialise as "null" per HTML §3.2.1.2.
    assert_eq!(origin.ascii_serialization(), "null");
    assert!(!origin.is_tuple());
}

// ---------------------------------------------------------------------------
// PR5-cors Stage 4: response_type CORS classification matrix.  These
// tests verify the JS-observable `Response.type` value (and the
// associated header / body / status / url filtering) for each fetch
// scenario.
// ---------------------------------------------------------------------------

fn cors_response(url: &str, allow_origin: Option<&str>) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("x-custom".to_string(), "secret".to_string()),
    ];
    if let Some(origin) = allow_origin {
        headers.push((
            "Access-Control-Allow-Origin".to_string(),
            origin.to_string(),
        ));
    }
    NetResponse {
        status: 200,
        headers,
        body: bytes::Bytes::from_static(b"ok"),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

fn redirect_302_response(url: &str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 302,
        headers: vec![("location".to_string(), "/elsewhere".to_string())],
        body: bytes::Bytes::new(),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
    }
}

fn read_string(vm: &Vm, key: &str) -> String {
    match vm.get_global(key) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected {key} to be a string, got {other:?}"),
    }
}

#[test]
fn response_type_basic_for_same_origin() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "globalThis.t = ''; \
         fetch('http://example.com/api').then(r => { globalThis.t = r.type; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "basic");
}

#[test]
fn response_type_cors_for_cross_origin_with_acao() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response(
            "http://other.com/api",
            Some("http://example.com"),
        )),
    );
    vm.eval(
        "globalThis.t = ''; \
         fetch('http://other.com/api').then(r => { globalThis.t = r.type; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "cors");
}

#[test]
fn response_type_opaque_for_no_cors_cross_origin() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", None)),
    );
    vm.eval(
        "globalThis.t = ''; \
         globalThis.s = 0; \
         fetch('http://other.com/api', {mode: 'no-cors'}) \
             .then(r => { globalThis.t = r.type; globalThis.s = r.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "opaque");
    // Opaque responses report status 0.
    match vm.get_global("s") {
        Some(JsValue::Number(n)) => assert!((n - 0.0).abs() < f64::EPSILON),
        other => panic!("expected status 0, got {other:?}"),
    }
}

#[test]
fn cors_check_failure_rejects_with_typeerror() {
    // Cross-origin cors mode without an `Access-Control-Allow-Origin`
    // header → spec says this becomes a network error and the
    // Promise rejects with TypeError.  The mock returns a 200 OK
    // without ACAO; the classifier treats it as `NetworkError`.
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", None)),
    );
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://other.com/api') \
             .catch(e => { globalThis.r = e.message; });",
    )
    .unwrap();
    drain(&mut vm);
    let msg = read_string(&vm, "r");
    assert!(
        msg.to_lowercase().contains("cors") || msg.contains("Access-Control"),
        "expected CORS rejection, got: {msg}"
    );
}

#[test]
fn cors_filter_drops_non_safelisted_headers() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://other.com/api",
        Ok(cors_response("http://other.com/api", Some("*"))),
    );
    vm.eval(
        "globalThis.ct = ''; \
         globalThis.cust = 'unset'; \
         fetch('http://other.com/api').then(r => { \
             globalThis.ct = r.headers.get('content-type'); \
             globalThis.cust = r.headers.get('x-custom'); \
         });",
    )
    .unwrap();
    drain(&mut vm);
    // CORS-safelisted (`content-type`) is exposed.
    assert_eq!(read_string(&vm, "ct"), "application/json");
    // Custom header that is not in the safelist and not in
    // `Access-Control-Expose-Headers` — `headers.get` returns
    // null (== JS undefined for the test global slot? No — null
    // string-coerces to `null`).  Spec: when name not present,
    // `Headers.prototype.get` returns null.
    match vm.get_global("cust") {
        Some(JsValue::Null) => {}
        other => panic!("expected null for filtered header, got {other:?}"),
    }
}

#[test]
fn opaque_redirect_response_for_manual_redirect_3xx() {
    let (mut vm, _) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(redirect_302_response("http://example.com/api")),
    );
    vm.eval(
        "globalThis.t = ''; \
         globalThis.s = -1; \
         fetch('http://example.com/api', {redirect: 'manual'}) \
             .then(r => { globalThis.t = r.type; globalThis.s = r.status; });",
    )
    .unwrap();
    drain(&mut vm);
    assert_eq!(read_string(&vm, "t"), "opaqueredirect");
    match vm.get_global("s") {
        Some(JsValue::Number(n)) => assert!((n - 0.0).abs() < f64::EPSILON),
        other => panic!("expected status 0, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// PR5-cors Stage 5: cache-mode header injection (WHATWG Fetch §5.3 step 30).
// `force-cache` / `only-if-cached` are documented no-ops because elidex-net
// does not yet implement an HTTP cache layer.
// ---------------------------------------------------------------------------

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

#[test]
fn cache_no_store_appends_cache_control_no_store() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'no-store'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("no-store")
    );
}

#[test]
fn cache_reload_appends_cache_control_no_cache_and_pragma() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'reload'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("no-cache")
    );
    assert_eq!(header_value(&logged[0].headers, "pragma"), Some("no-cache"));
}

#[test]
fn cache_no_cache_appends_max_age_zero() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'no-cache'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("max-age=0")
    );
}

#[test]
fn cache_default_does_not_inject_headers() {
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval("fetch('http://example.com/api', {cache: 'default'});")
        .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert!(header_value(&logged[0].headers, "cache-control").is_none());
    assert!(header_value(&logged[0].headers, "pragma").is_none());
}

#[test]
fn user_set_cache_control_is_preserved() {
    // PR5-cors's cache-mode injection only fires when the same
    // header isn't already present — user-set headers win.
    let (mut vm, handle) = vm_with_origin_and_mock(
        "http://example.com/page",
        "http://example.com/api",
        Ok(ok_response("http://example.com/api", "ok")),
    );
    vm.eval(
        "fetch('http://example.com/api', \
             {cache: 'no-store', headers: {'Cache-Control': 'public, max-age=60'}});",
    )
    .unwrap();
    let logged = handle.drain_recorded_requests();
    assert_eq!(logged.len(), 1);
    assert_eq!(
        header_value(&logged[0].headers, "cache-control"),
        Some("public, max-age=60")
    );
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

/// Copilot R2 regression: when `settle_fetch` lands on a
/// `FetchId` whose `pending_fetch_cors` entry is missing (an
/// internal bookkeeping bug, not a user-visible state), the
/// Promise must be **rejected** with a `TypeError` rather than
/// silently fall through to a permissive `Basic` classification
/// (which would disable CORS enforcement for that fetch).
///
/// We can't easily reproduce a "real" bookkeeping bug from the
/// public API, so this test reaches into `vm.inner` to drop the
/// CORS meta entry between dispatch and `tick_network` — the
/// `pending_fetches` Promise survives but its meta is gone.
#[test]
fn settle_fetch_rejects_when_cors_meta_missing() {
    let url = url::Url::parse("http://example.com/api").unwrap();
    let mut vm = mock_vm(vec![(url, Ok(ok_response("http://example.com/api", "ok")))]);
    vm.inner.navigation.current_url = url::Url::parse("http://example.com/page").unwrap();
    vm.eval(
        "globalThis.r = 'unset'; \
         fetch('http://example.com/api') \
             .then(resp => { globalThis.r = 'resolved-' + resp.status; }) \
             .catch(e => { globalThis.r = 'rejected-' + e.message; });",
    )
    .unwrap();
    // Sanity: dispatch installed both maps.
    assert_eq!(vm.inner.pending_fetches.len(), 1);
    assert_eq!(vm.inner.pending_fetch_cors.len(), 1);
    // Drop the CORS meta entry only — this simulates the
    // bookkeeping bug Copilot R2 flagged.
    vm.inner.pending_fetch_cors.clear();
    drain(&mut vm);
    match vm.get_global("r") {
        Some(JsValue::String(id)) => {
            let msg = vm.get_string(id);
            assert!(
                msg.starts_with("rejected-") && msg.contains("missing CORS metadata"),
                "expected fail-closed rejection, got: {msg}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}
