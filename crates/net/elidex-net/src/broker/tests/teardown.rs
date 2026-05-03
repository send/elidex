//! Broker integration tests for renderer / broker teardown (unregister, shutdown, realtime-only shutdown semantics, stale-handle gating, synthetic-abort delivery).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).
//!
//! Slot #10.6a (PR-broker-ws-sse-shutdown-join) tests for the
//! per-handle [`crate::CancelHandle`] wiring + `thread.join()` step
//! in [`super::super::dispatch::NetworkProcessState::close_all_for_client`]
//! live alongside the worker source (`crate::ws::tests` /
//! `crate::sse::tests`) rather than here, because broker-level
//! SSRF rejects loopback addresses (`url_security::validate_url`
//! has no `allow_private_ips` knob for the WS/SSE dispatch path,
//! by design — see `crate::broker::dispatch::handle_request`'s
//! `WebSocketOpen` / `EventSourceOpen` branches).  Testing the
//! worker's cancel-responsiveness directly via `spawn_ws_thread`
//! / `spawn_sse_thread` exercises exactly the same `tokio::select!`
//! arms the broker relies on, with no observability gap — the
//! broker's join sequence is `cancel.cancel()` → `drop(command_tx)`
//! → `thread.join()`, which the unit tests reproduce verbatim
//! against a real local TCP fixture.

use std::time::Duration;

use super::super::*;
use super::test_client;
use crate::Request;
/// PR-file-split-a Copilot R4: when a renderer is unregistered
/// (tab/worker drop) while it has in-flight fetches stalled on
/// network IO, those workers must be cancelled so the
/// `MAX_CONCURRENT_FETCHES` slots release promptly.  Pre-fix
/// the workers kept holding their slots until network timeout
/// (~30s), starving subsequent fetches issued by other
/// renderers.
#[test]
fn unregister_renderer_cancels_inflight_fetches_promptly() {
    // Stalling server: hold connection open forever.
    let stall_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = stall_listener.local_addr().unwrap();
    let _stall_keep = stall_listener;

    // Probe server: replies promptly so the post-unregister
    // fetch confirms the inflight slot freed up.
    let probe_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let probe_addr = probe_listener.local_addr().unwrap();
    let probe_thread = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let Ok((mut stream, _)) = probe_listener.accept() else {
            return;
        };
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    });

    let np = spawn_network_process(test_client());
    let droppee = np.create_renderer_handle();
    let droppee_cid = droppee.client_id();
    let observer = np.create_renderer_handle();

    // Saturate the dropping renderer's fetches against the
    // stalling server.  Even one fetch is enough to expose
    // the bug if it holds a slot past unregister, but several
    // make the test less timing-sensitive.
    let mk_stall = || Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let mut stall_ids = Vec::new();
    for _ in 0..10 {
        stall_ids.push(droppee.fetch_async(mk_stall()));
    }
    // Yield so each worker reaches `transport.send`'s await
    // point (otherwise cancel triggers the pre-await
    // `is_cancelled` fast path which exits before holding the
    // slot — true positive but doesn't exercise the
    // mid-flight cancel path the fix targets).
    std::thread::sleep(Duration::from_millis(40));

    // Drop the renderer: the unregister handler must cancel
    // every in-flight fetch keyed by `(droppee_cid, _)` and
    // remove its `cancel_tokens` entries so the workers
    // release their `MAX_CONCURRENT_FETCHES` slots.
    np.unregister_renderer(droppee_cid);
    // Drop the handle too so the broker doesn't see us as a
    // live renderer when the probe goes out.
    drop(droppee);

    // The observer's probe fetch must complete promptly.  If
    // the dropped renderer's slots leaked, this would wait
    // for the underlying network timeout (~30s) and fail the
    // 5-second deadline below.
    let probe_id = observer.fetch_async(Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let probe_deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut probe_got = None;
    while std::time::Instant::now() < probe_deadline && probe_got.is_none() {
        for ev in observer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == probe_id {
                    probe_got = Some(result);
                }
            }
        }
        if probe_got.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let probe_resp = probe_got
        .expect("probe fetch starved — UnregisterRenderer left dropped renderer's fetches holding inflight slots")
        .unwrap();
    assert_eq!(probe_resp.status, 200);

    np.shutdown();
    probe_thread.join().unwrap();
}

/// PR-file-split-a Copilot R7 regression: `Shutdown` must
/// cancel in-flight fetches across every registered client
/// before the broker thread exits.  Pre-fix the loop returned
/// `false` immediately, leaving fetch worker threads running
/// — they would still complete I/O and try to send replies
/// against `tx` clones captured at dispatch time, while
/// `NetworkProcessHandle::shutdown()` had already returned.
///
/// Strengthened R8 (HCa2): asserts the cancel actually
/// happened by observing the worker thread's release of its
/// inflight slot.  We verify this directly by reading the
/// `inflight_fetches` counter through a fresh probe fetch
/// after shutdown — but since shutdown destroys the broker
/// thread, we instead use a wall-clock proxy: the `shutdown()`
/// elapsed time has to be far less than the underlying
/// `request_timeout` (~30s).  More importantly, we time the
/// abort-to-thread-exit chain by observing the stalling
/// server's connection: pre-cancellation the broker thread
/// would block on the worker's `block_on` for the full
/// transport timeout, and shutdown would not return until
/// then.  A `Duration::from_secs(2)` deadline is more than
/// enough for a successful cancel + join while still being
/// orders of magnitude below the 30s un-cancelled latency.
#[test]
fn shutdown_cancels_inflight_fetches_across_clients() {
    let stall_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = stall_listener.local_addr().unwrap();
    let _stall_keep = stall_listener;

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let stall_id = renderer.fetch_async(request);
    // Yield so the worker reaches `transport.send`'s real
    // await point — without this the cancel might be observed
    // by the pre-await `is_cancelled` fast-path, which exits
    // before holding the inflight slot and would mask any
    // bug in the post-await cancel chain.
    std::thread::sleep(Duration::from_millis(40));

    // Shutdown — broker must cancel the inflight fetch and
    // join cleanly within the 2-second deadline.  Pre-fix
    // (R7) it would block ~30s waiting on the worker's
    // un-cancelled `block_on` to hit the transport timeout.
    let started = std::time::Instant::now();
    np.shutdown();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "shutdown() blocked for {elapsed:?} — likely waiting for un-cancelled fetch's transport timeout (regression in cancel-on-shutdown path)"
    );

    // Copilot R9 HEld: the in-flight fetch must observe a
    // synthetic `aborted` reply.  Pre-fix the broker cancelled
    // the worker (which then suppressed its `Cancelled` reply)
    // and cleared the clients map — leaving the renderer-side
    // Promise pending forever with no terminal event.  Post-fix
    // the broker pushes the synthetic reply BEFORE clearing the
    // client sender, so this drain finds it.
    let mut saw_aborted = false;
    for ev in renderer.drain_events() {
        if let NetworkToRenderer::FetchResponse(rid, Err(msg)) = ev {
            if rid == stall_id && msg.contains("aborted") {
                saw_aborted = true;
            }
        }
    }
    assert!(
        saw_aborted,
        "shutdown left the in-flight fetch without a terminal reply — pending Promise leak"
    );

    // Sanity check: post-shutdown sends fail with a disconnect
    // reply (broker is gone).  This confirms the broker actually
    // shut down rather than just appearing to (e.g. if the
    // join was skipped on a panic).
    let post_request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let post_id = renderer.fetch_async(post_request);
    let events = renderer.drain_events();
    let saw_disconnect = events.iter().any(|ev| {
        matches!(
            ev,
            NetworkToRenderer::FetchResponse(rid, Err(msg))
                if *rid == post_id && msg.contains("disconnected")
        )
    });
    assert!(
        saw_disconnect,
        "post-shutdown fetch must produce a disconnect reply, got {events:?}"
    );
}

/// Companion to [`Self::shutdown_cancels_inflight_fetches_across_clients`]
/// (Copilot R8 HBDH): `RendererToNetwork::Shutdown`
/// (used by `HostBridge::shutdown_all_realtime` to drop only
/// realtime channels) must NOT cancel the renderer's
/// in-flight fetches.  R4's cancel-fetch hook was originally
/// inside `close_all_for_client`, which made every caller of
/// `close_all_for_client` over-cancel — including this
/// realtime-only shutdown path.  Post-R8 the cancel-fetch
/// step lives in a separate `cancel_inflight_fetches_for`
/// helper invoked only from `UnregisterRenderer` and broker
/// `NetworkProcessControl::Shutdown`.
#[test]
fn renderer_shutdown_message_does_not_cancel_inflight_fetches() {
    // Simple HTTP server that delays its reply long enough for
    // us to observe the post-shutdown-message survival of the
    // in-flight fetch.  Use a short delay (~80ms) so the test
    // wall-clock stays low.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        std::thread::sleep(Duration::from_millis(80));
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    });

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", server_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let fetch_id = renderer.fetch_async(request);
    // Yield so the worker is mid-flight when the realtime
    // shutdown arrives.
    std::thread::sleep(Duration::from_millis(20));

    // Send the realtime-only shutdown.  Pre-R8 this would
    // route through `close_all_for_client` and cancel the
    // fetch.  Post-R8 it just tears down WS/SSE (a no-op
    // here since we have none) and the fetch survives.
    assert!(renderer.send(RendererToNetwork::Shutdown));

    // The fetch must still complete with the real 200 reply.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut got = None;
    while std::time::Instant::now() < deadline && got.is_none() {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == fetch_id {
                    got = Some(result);
                }
            }
        }
        if got.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let resp = got
        .expect("fetch reply not delivered — RendererToNetwork::Shutdown wrongly cancelled it")
        .expect("fetch errored — RendererToNetwork::Shutdown wrongly cancelled it");
    assert_eq!(resp.status, 200);

    np.shutdown();
    server_thread.join().unwrap();
}

/// PR-file-split-a Copilot R10 (HG4e) regression: a stale
/// `NetworkHandle` clone whose renderer was already unregistered
/// must NOT be able to spawn new fetch workers (which would
/// consume `MAX_CONCURRENT_FETCHES` slots with no response
/// destination) or open new WS/SSE connections.  Pre-fix
/// `handle_request` ran every dispatch unconditionally and
/// only checked `self.clients.get(&cid)` per-branch for the
/// reply path; the resource consumption was already paid by
/// the time we discovered the client was gone.  Post-fix the
/// top-of-`handle_request` early-return drops every message
/// from an unknown cid silently.
#[test]
fn unregistered_renderer_cannot_consume_inflight_slots() {
    let stall_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = stall_listener.local_addr().unwrap();
    let _stall_keep = stall_listener;

    let np = spawn_network_process(test_client());
    let stale = np.create_renderer_handle();
    let cid = stale.client_id();
    let observer = np.create_renderer_handle();

    // Unregister the soon-to-be-stale renderer.  The handle
    // clone (`stale`) survives — the Drop on `unregister_renderer`
    // does NOT consume it, so a bug-prone caller can still call
    // `fetch_async` on it.
    np.unregister_renderer(cid);
    // Yield so the unregister is processed before the fetch.
    std::thread::sleep(Duration::from_millis(20));

    // Saturate via the stale handle.  Pre-fix every fetch_async
    // would still spawn a worker thread + grab a slot.
    for _ in 0..80 {
        // 80 > MAX_CONCURRENT_FETCHES (64) — pre-fix this would
        // exhaust the pool and the observer's probe below would
        // see "too many concurrent fetches".
        let _ = stale.fetch_async(Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        });
    }
    // Yield so the broker has a chance to process the burst.
    std::thread::sleep(Duration::from_millis(60));

    // The observer's probe must still get an inflight slot.
    // Bind a probe server that replies promptly.
    let probe_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let probe_addr = probe_listener.local_addr().unwrap();
    let probe_thread = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let Ok((mut stream, _)) = probe_listener.accept() else {
            return;
        };
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    });

    let probe_id = observer.fetch_async(Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut probe_got = None;
    while std::time::Instant::now() < deadline && probe_got.is_none() {
        for ev in observer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == probe_id {
                    probe_got = Some(result);
                }
            }
        }
        if probe_got.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let probe_resp = probe_got
        .expect("probe fetch starved — stale renderer's fetch_async consumed inflight slots")
        .unwrap();
    assert_eq!(probe_resp.status, 200);

    np.shutdown();
    probe_thread.join().unwrap();
}

/// PR-file-split-a Copilot R11 (HJTc) regression: a still-live
/// `NetworkHandle` clone whose owner was unregistered must observe
/// a synthetic `aborted` reply for every in-flight fetch — without
/// it, cancelled workers suppress their own `FetchResponse` on
/// `NetErrorKind::Cancelled`, leaving the renderer-side Promises
/// pending forever.  Symmetric to R9 HEld which fixed the same
/// gap on the broker `Shutdown` path.
#[test]
fn unregister_renderer_delivers_synthetic_aborted_for_inflight_fetches() {
    let stall_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = stall_listener.local_addr().unwrap();
    let _stall_keep = stall_listener;

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let stall_id = renderer.fetch_async(request);
    // Yield so the worker reaches `transport.send`.
    std::thread::sleep(Duration::from_millis(40));

    np.unregister_renderer(cid);

    // Drain until the synthetic aborted reply arrives, or
    // timeout.  Pre-fix the renderer would never observe a
    // FetchResponse for this id.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut saw_aborted = false;
    while std::time::Instant::now() < deadline && !saw_aborted {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, Err(msg)) = ev {
                if rid == stall_id && msg.contains("aborted") {
                    saw_aborted = true;
                }
            }
        }
        if !saw_aborted {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(
        saw_aborted,
        "unregister_renderer left the in-flight fetch without a terminal reply — pending Promise leak"
    );

    np.shutdown();
}
