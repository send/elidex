//! Broker integration tests for renderer / broker teardown (unregister, shutdown, realtime-only shutdown semantics, stale-handle gating, synthetic-abort delivery).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).
//!
//! Slot #10.6a (PR-broker-ws-sse-shutdown-join) splits its test
//! coverage across this module and the worker sources:
//!
//! - **Per-arm cancel responsiveness** — tested in
//!   `crate::ws::tests` and `crate::sse::tests`, not here, because
//!   the broker dispatch path applies SSRF to WS/SSE opens
//!   (`url_security::validate_url` has no `allow_private_ips`
//!   knob — see `crate::broker::dispatch::handle_request`'s
//!   `WebSocketOpen` / `EventSourceOpen` branches), so loopback
//!   fixtures can't reach the broker's join code.  The WS side
//!   uses a real silent-listener fixture against
//!   `spawn_ws_thread` to exercise the handshake select's cancel
//!   arm.  The SSE side uses `tokio::io::duplex` to drive the
//!   `await_with_cancel` / `read_line_with_cancel` helpers
//!   directly — Copilot R1 HX1 / HX2 — plus `should_close` /
//!   `wait_or_close` cancel arms.
//!
//! - **Broker teardown sequencing** — covered indirectly by the
//!   pre-existing `unregister_renderer_*` / `shutdown_*` tests
//!   below (which still pass with the new
//!   `close_all_for_client` flow).  The new flow is "queue
//!   Close cmd → drop `command_tx` → grace-poll
//!   `JoinHandle::is_finished` → cancel-fallback → join", and
//!   is not exercised end-to-end against a live WS/SSE
//!   connection here — the worker-side tests prove cancel IS
//!   observed within bounded time, and the grace+cancel+join
//!   shape is a mechanical composition of
//!   `JoinHandle::is_finished` and `CancelHandle::cancel`,
//!   both of which have their own unit tests
//!   (`crate::cancel::tests` and the std library respectively).

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

    // Sanity check: post-shutdown sends fail with a terminal
    // Err reply (broker is gone).  This confirms the broker
    // actually shut down rather than just appearing to (e.g.
    // if the join was skipped on a panic).
    //
    // Slot #10.6b: the broker `Shutdown` control path emits
    // `RendererUnregistered` for every cid before clearing
    // `clients`, so the renderer-side
    // [`super::super::handle::NetworkHandle`] short-circuit
    // produces `Err("renderer unregistered")` from
    // `fetch_async`'s pre-send `check_unregistered` gate
    // before it reaches the disconnected-`request_tx`
    // fallback.  Pre-#10.6b the only signal was the channel
    // disconnect, so the message was `network process
    // disconnected` instead.  Both shapes signal "broker is
    // dead, no real reply will ever come" — accept either to
    // keep this sanity-check robust against future shutdown-
    // path tweaks.
    let post_request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let post_id = renderer.fetch_async(post_request);
    let events = renderer.drain_events();
    let saw_terminal_err = events.iter().any(|ev| {
        matches!(
            ev,
            NetworkToRenderer::FetchResponse(rid, Err(msg))
                if *rid == post_id
                    && (msg.contains("renderer unregistered") || msg.contains("disconnected"))
        )
    });
    assert!(
        saw_terminal_err,
        "post-shutdown fetch must produce a terminal Err reply (broker is gone), got {events:?}"
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

// ---------------------------------------------------------------------------
// Slot #10.6b — broker → NetworkHandle back-edge (RendererUnregistered)
//
// Layered defence against the post-`UnregisterRenderer`
// Promise leak.  The broker emits
// `NetworkToRenderer::RendererUnregistered` after
// synthesise / close / cancel, before `clients.remove`.  The
// renderer-side handle observes the marker on its next drain
// to (1) flip a synchronous `unregistered` flag short-circuit
// for `fetch_async` / `fetch_blocking` / `cancel_fetch` /
// `send`, and (2) synthesise terminal `Err` replies for any
// `FetchId` still in `outstanding_fetches` — those are
// race-window submits the broker silently dropped via its
// stale-cid gate (`dispatch::handle_request` early-return).
// ---------------------------------------------------------------------------

/// After the renderer-side drain has observed
/// `RendererUnregistered`, a subsequent `fetch_async` on a
/// still-live `NetworkHandle` clone must short-circuit
/// synchronously: a synthetic `Err("renderer unregistered")`
/// reply is buffered under the freshly-allocated `FetchId` and
/// surfaces on the very next `drain_events`, with no broker
/// round-trip.
#[test]
fn fetch_async_after_unregister_returns_synthetic_err_synchronously() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    np.unregister_renderer(cid);

    // Deterministic gate: poll `cancel_fetch` against a fresh
    // synthetic FetchId until it returns `false` — the
    // short-circuit fires only after the renderer-side drain
    // observes `RendererUnregistered` and flips the
    // `unregistered` flag, so a `false` return is direct
    // evidence that layer 1 is in place.  Avoids issuing real
    // `fetch_async` calls during the wait (any of which would
    // otherwise round-trip through `request_tx` and cloud the
    // post-gate assertion below — Copilot R1 HX1).  The
    // FetchId is fresh per iteration so it cannot match an
    // already-tracked outstanding entry.
    let gate_deadline = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < gate_deadline && renderer.cancel_fetch(FetchId::next()) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !renderer.cancel_fetch(FetchId::next()),
        "unregister marker never observed within 1s — short-circuit gate did not fire"
    );

    // Gate is in place: a single `fetch_async` must produce
    // the synthetic Err on the next drain with no broker
    // round-trip.
    let probe_id = renderer.fetch_async(Request {
        method: "GET".to_string(),
        url: url::Url::parse("http://example.invalid/probe").unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let events = renderer.drain_events();
    let saw_synth_err = events.iter().any(|ev| {
        matches!(
            ev,
            NetworkToRenderer::FetchResponse(rid, Err(msg))
                if *rid == probe_id && msg.contains("renderer unregistered")
        )
    });
    assert!(
        saw_synth_err,
        "post-unregister fetch_async did not produce the synthetic 'renderer unregistered' Err — got {events:?}"
    );

    np.shutdown();
}

/// `fetch_blocking` must short-circuit too: once the marker is
/// observed (either by a prior drain or by the recv loop's
/// inline match), a fresh blocking fetch returns
/// `Err("renderer unregistered")` immediately — the layer 1
/// `check_unregistered` gate at the top of the method fires
/// before any `request_tx.send` / `recv_timeout` work.  We
/// gate the test on a deterministic `cancel_fetch` short-
/// circuit observation (the marker is already on the channel
/// and the flag is set) so the call cannot accidentally land
/// in the 30 s `recv_timeout` path on a slow CI runner
/// (Copilot R2 HX5 — pre-fix the test relied on a fixed
/// 40 ms sleep that could miss the marker under load).
#[test]
fn fetch_blocking_after_unregister_returns_err_synchronously() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    np.unregister_renderer(cid);

    // Deterministic gate: poll until the synchronous short-
    // circuit fires (cancel_fetch returns false), guaranteeing
    // the `unregistered` flag is set BEFORE we issue the
    // blocking call.  Layer 1 in fetch_blocking will then
    // return Err immediately without touching `request_tx`.
    let gate_deadline = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < gate_deadline && renderer.cancel_fetch(FetchId::next()) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !renderer.cancel_fetch(FetchId::next()),
        "unregister marker never observed within 1s — short-circuit gate did not fire"
    );

    let started = std::time::Instant::now();
    let result = renderer.fetch_blocking(Request {
        method: "GET".to_string(),
        url: url::Url::parse("http://example.invalid/probe").unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let elapsed = started.elapsed();

    // Must return promptly — the 30 s recv_timeout would mean
    // the gate failed.  100 ms is generous slack for a loaded
    // CI runner once the gate is in place: with `unregistered`
    // already true, the call should return on its first
    // statement.
    assert!(
        elapsed < Duration::from_millis(100),
        "fetch_blocking after unregister blocked for {elapsed:?} — gate did not fire"
    );
    let msg = result.expect_err("fetch_blocking after unregister must return Err");
    assert!(
        msg.contains("renderer unregistered"),
        "expected 'renderer unregistered' Err, got {msg:?}"
    );

    np.shutdown();
}

/// `cancel_fetch` and `send` short-circuit with `false` once
/// the handle has observed the marker — there is no point
/// queueing work onto a broker that has already torn down our
/// `clients` entry.
#[test]
fn cancel_fetch_and_send_after_unregister_return_false() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    np.unregister_renderer(cid);
    // Wait for the marker by polling drain_events until the
    // flag flips (observable as a `false` from `cancel_fetch`).
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut gate_fired = false;
    while std::time::Instant::now() < deadline && !gate_fired {
        let _ = renderer.drain_events();
        if !renderer.cancel_fetch(FetchId::next()) && !renderer.send(RendererToNetwork::Shutdown) {
            gate_fired = true;
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(
        gate_fired,
        "cancel_fetch / send did not return false after unregister observation"
    );

    np.shutdown();
}

/// **Load-bearing** for the `outstanding_fetches` mechanism:
/// a fetch submitted in the same broker-loop iteration that
/// processes `UnregisterRenderer` (i.e. *after* the
/// `synthesise_aborted_replies_for_client` step but *before*
/// `clients.remove`) is dropped silently by the broker's
/// stale-cid gate (`dispatch::handle_request`).  Without
/// `outstanding_fetches` the renderer-side
/// `pending_fetches[id]` Promise would leak forever.
///
/// We engineer the race by submitting an in-flight stalled
/// fetch (which the broker DOES register in `cancel_tokens`,
/// so it gets the broker-side synthetic aborted reply), then
/// firing `unregister_renderer` and immediately —
/// without yielding to the broker — calling `fetch_async`
/// again on the surviving handle clone.  Whichever layer
/// catches the second fetch (layer 1 short-circuit if the
/// broker emitted the marker before the renderer's pre-send
/// drain, or layer 2 outstanding-tracking synthesis if the
/// broker's `Fetch` arrived after `UnregisterRenderer` and
/// hit the stale-cid gate), the test asserts the SAME
/// observable property: every fetch this renderer issued
/// settles with a terminal `Err` reply.
#[test]
fn race_window_fetch_settles_via_outstanding_tracking() {
    let stall_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = stall_listener.local_addr().unwrap();
    let _stall_keep = stall_listener;

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    let mk_stall = || Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    // Pre-flight A: enters cancel_tokens before unregister →
    // covered by the broker's `synthesise_aborted_replies_for_client`.
    let id_a = renderer.fetch_async(mk_stall());
    // Yield so worker reaches transport.send.
    std::thread::sleep(Duration::from_millis(40));

    // Race-window submit C: issued AFTER unregister, no yield
    // between the two so the broker may or may not have
    // processed UnregisterRenderer yet — the assertion holds
    // either way (layer 1 or layer 2 catches it).
    np.unregister_renderer(cid);
    let id_c = renderer.fetch_async(mk_stall());

    // Both A and C must settle with terminal Err — we collect
    // arrivals into a map so a single drain that returns BOTH
    // events does not lose one.  (`await_terminal_err`'s
    // single-target drain would silently discard any non-
    // matching terminal events on the same call.)
    let mut settled: std::collections::HashMap<FetchId, String> = std::collections::HashMap::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline
        && (!settled.contains_key(&id_a) || !settled.contains_key(&id_c))
    {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, Err(msg)) = ev {
                settled.entry(rid).or_insert(msg);
            }
        }
        if !settled.contains_key(&id_a) || !settled.contains_key(&id_c) {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let msg_a = settled
        .get(&id_a)
        .expect("A never settled (broker synthesise step)");
    let msg_c = settled
        .get(&id_c)
        .expect("C never settled — race-window submit leaked despite outstanding_fetches");
    assert!(
        msg_a.contains("aborted") || msg_a.contains("renderer unregistered"),
        "A's terminal Err: {msg_a:?}"
    );
    assert!(
        msg_c.contains("renderer unregistered") || msg_c.contains("aborted"),
        "C's terminal Err: {msg_c:?}"
    );

    np.shutdown();
}

/// `RendererUnregistered` is internal: `drain_events` consumes
/// it and never surfaces it to the caller.  Without this
/// filter, embedders that exhaustively match
/// `NetworkToRenderer` (e.g. the boa realtime bridge) would
/// see a meaningless variant for which they have no event
/// loop hook.  The flag-flipping side effect is verified by
/// the gates above; this test pins the surface contract.
#[test]
fn renderer_unregistered_event_is_not_surfaced_to_caller() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    np.unregister_renderer(cid);
    // Generously absorb broker scheduling — within this
    // window the marker MUST have been emitted onto our
    // response channel and consumed by drain_events.
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut all_events: Vec<NetworkToRenderer> = Vec::new();
    while std::time::Instant::now() < deadline {
        all_events.extend(renderer.drain_events());
        std::thread::sleep(Duration::from_millis(10));
    }

    for ev in &all_events {
        assert!(
            !matches!(ev, NetworkToRenderer::RendererUnregistered),
            "drain_events surfaced the internal RendererUnregistered marker: {all_events:?}"
        );
    }

    np.shutdown();
}

/// Slot #10.6b Copilot R2 HX4 regression: straggler
/// `FetchResponse` events synthesised on marker observation
/// must arrive in deterministic ascending `FetchId` order
/// (which matches submission order because `FETCH_ID_COUNTER`
/// is monotonic).  Pre-fix the synthesis collected the ids by
/// `HashSet::drain` whose iteration order is non-deterministic,
/// which violated the public order contract on
/// `drain_fetch_responses_only` / `drain_events`.
///
/// We exercise the order by submitting several stalled fetches
/// in sequence, dropping the renderer (broker tears it down,
/// emits the marker; cancel_inflight + synth_aborted fire on
/// the broker side first, but the race-window ids never
/// reach cancel_tokens so they only emerge via the renderer-
/// side straggler path).  We then assert the synthetic
/// `Err("renderer unregistered")` tail arrives in ascending
/// id order.  We bypass cancel_tokens by submitting AFTER
/// `unregister_renderer` (so the broker's stale-cid gate
/// drops them and only the renderer-side straggler path emits
/// terminal events).
#[test]
fn straggler_synthesis_emits_in_ascending_fetch_id_order() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    let cid = renderer.client_id();

    np.unregister_renderer(cid);

    // Wait for the broker to start processing the unregister
    // (we want our fetches to land in the race window).  A
    // brief sleep is sufficient — the assertion below holds
    // regardless of whether the broker has emitted the marker
    // yet, because all our submits happen before the next
    // drain that observes it.
    std::thread::sleep(Duration::from_millis(20));

    // Submit several fetches.  At least some of these will
    // land in the race window and be dropped by the broker's
    // stale-cid gate; their FetchIds remain in
    // `outstanding_fetches` until the renderer's drain
    // observes the marker and synthesises terminal Errs in
    // sorted order.  Even if the layer 1 short-circuit catches
    // some (after the marker is observed), THOSE go onto
    // `buffered` directly — order across the
    // (broker-stalled, layer-1-shortcircuited) split would
    // matter only if both populated the same drain; here we
    // collect the FULL set across one or more drains and
    // assert the IDs we collect are a sorted subsequence.
    let mut submitted: Vec<FetchId> = Vec::new();
    for i in 0..6 {
        let id = renderer.fetch_async(Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://example.invalid/{i}")).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        });
        submitted.push(id);
    }
    // We submitted in ascending id order (FETCH_ID_COUNTER is
    // monotonic), so `submitted` is already sorted; this assert
    // documents that invariant.
    assert!(submitted.windows(2).all(|w| w[0].0 < w[1].0));

    // Drain until all six terminal Errs arrive, accumulating
    // arrival order.  Repeat drains absorb the case where the
    // marker arrives mid-loop.
    let mut arrived: Vec<FetchId> = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && arrived.len() < submitted.len() {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, Err(_)) = ev {
                if submitted.contains(&rid) && !arrived.contains(&rid) {
                    arrived.push(rid);
                }
            }
        }
        if arrived.len() < submitted.len() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(
        arrived.len(),
        submitted.len(),
        "not every submitted id received a terminal Err — got {arrived:?}, expected {submitted:?}"
    );

    // Within a single drain (i.e. a single straggler synthesis
    // pass), ids must be ascending.  Across multiple drains
    // the layer-1 short-circuit can interleave (each
    // `fetch_async` call after the flag flips emits ONE synth
    // entry directly to `buffered` before the next drain).
    // The strictest invariant we can check across drains is
    // that the FULL collected sequence is non-decreasing —
    // that holds because every component (straggler tail
    // sorted, plus layer-1 short-circuits ordered by
    // submission) is itself ascending and the components
    // arrive in sorted-id chunks.
    assert!(
        arrived.windows(2).all(|w| w[0].0 < w[1].0),
        "synthesised terminal Errs not in ascending FetchId order — got {arrived:?}"
    );

    np.shutdown();
}

/// Sibling handles
/// ([`super::super::handle::NetworkHandle::create_sibling_handle`])
/// have independent `unregistered` state — each owns its own
/// response channel and only its own cid's marker reaches it.
/// Unregistering the parent must NOT disable the sibling's
/// `fetch_async` / `fetch_blocking` / `send`.
#[test]
fn sibling_handles_have_independent_unregistered_state() {
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
    let parent = np.create_renderer_handle();
    let sibling = parent.create_sibling_handle();
    let parent_cid = parent.client_id();

    np.unregister_renderer(parent_cid);
    // Yield so the parent's marker has time to land on the
    // parent's channel without colliding with sibling I/O.
    std::thread::sleep(Duration::from_millis(40));

    // Sibling fetch must complete with a real 200 — its
    // `unregistered` flag is independent of the parent's.
    let probe_id = sibling.fetch_async(Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut got: Option<Result<crate::Response, String>> = None;
    while std::time::Instant::now() < deadline && got.is_none() {
        for ev in sibling.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == probe_id {
                    got = Some(result);
                }
            }
        }
        if got.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let resp = got
        .expect("sibling probe never settled — sibling's `unregistered` may have been wired to parent's flag")
        .expect("sibling probe errored — independent flag check");
    assert_eq!(resp.status, 200);

    np.shutdown();
    probe_thread.join().unwrap();
}
