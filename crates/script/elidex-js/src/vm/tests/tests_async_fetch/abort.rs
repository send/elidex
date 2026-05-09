//! `controller.abort()` fan-out for in-flight `fetch()` Promises:
//! synchronous rejection, custom reason propagation, shared signal
//! atomicity, GC rooting across abort rejection, and the broker-side
//! `CancelFetch` wire (verifying both JS-observable and broker-
//! observable halves of abort).
//!
//! Companion to [`super::lifecycle`] (basic Promise lifecycle +
//! `install_network_handle`) and [`super::cors`] (PR5-cors Stages
//! 3 / 4 / 5).

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{drain, mock_vm, ok_response};

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
    use std::rc::Rc;

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
