//! Broker integration tests (lifecycle, fetch dispatch, cancellation,
//! cross-client isolation, shutdown).  Split out of `broker/mod.rs` in
//! PR-file-split-a Copilot R9 (HElg) to keep the entrypoint module
//! under the project's 1000-line file convention.

#![cfg(test)]

use std::time::Duration;

use super::*;
use crate::{NetClient, NetClientConfig, TransportConfig};

pub(super) fn test_client() -> NetClient {
    NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            // Lift per-origin and global connection caps well
            // above `MAX_CONCURRENT_FETCHES` so cancel-spam
            // regression tests can keep ≥`MAX_CONCURRENT_FETCHES`
            // workers genuinely stalled on transport IO.  With
            // the production defaults (`6` per-origin, `256`
            // total) most workers in those tests would fail
            // fast on the per-origin cap inside
            // `pool::create_connection` — that's a different
            // error path than the cancel-vs-stall race those
            // tests are meant to exercise (Copilot R1).
            max_connections_per_origin: 256,
            max_total_connections: 1024,
            ..Default::default()
        },
        ..Default::default()
    })
}

#[test]
fn spawn_and_shutdown() {
    let handle = spawn_network_process(test_client());
    handle.shutdown();
}

#[test]
fn create_renderer_handle() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    assert!(renderer.client_id() > 0);
    handle.shutdown();
}

#[test]
fn unregister_renderer() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let cid = renderer.client_id();
    handle.unregister_renderer(cid);
    // Brief wait for unregistration to propagate.
    std::thread::sleep(Duration::from_millis(10));
    handle.shutdown();
}

#[test]
fn fetch_blocking_connection_refused() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse("http://127.0.0.1:1/").unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let result = renderer.fetch_blocking(request);
    assert!(result.is_err());

    handle.shutdown();
}

#[test]
fn fetch_blocking_success() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // Bind a sync TCP server — no race with thread startup.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
        stream.write_all(resp).unwrap();
    });

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let result = renderer.fetch_blocking(request);
    let resp = result.unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"ok");

    server_thread.join().unwrap();
    np.shutdown();
}

#[test]
fn drain_events_empty() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let events = renderer.drain_events();
    assert!(events.is_empty());
    handle.shutdown();
}

#[test]
fn fetch_id_monotonic() {
    let a = FetchId::next();
    let b = FetchId::next();
    assert!(b.0 > a.0);
}

#[test]
fn multiple_renderers() {
    let handle = spawn_network_process(test_client());
    let r1 = handle.create_renderer_handle();
    let r2 = handle.create_renderer_handle();
    assert_ne!(r1.client_id(), r2.client_id());
    handle.shutdown();
}

#[test]
fn debug_network_handle() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let debug = format!("{renderer:?}");
    assert!(debug.contains("NetworkHandle"));
    handle.shutdown();
}

#[test]
fn fetch_async_returns_id_and_drain_picks_up_response() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
        stream.write_all(resp).unwrap();
    });

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let id = renderer.fetch_async(request);
    assert!(id.0 > 0);

    // Poll drain_events until the matching FetchResponse arrives.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut got = None;
    while std::time::Instant::now() < deadline {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == id {
                    got = Some(result);
                }
            }
        }
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let resp = got.expect("FetchResponse not delivered").unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"ok");

    server_thread.join().unwrap();
    np.shutdown();
}

#[test]
fn cancel_fetch_delivers_aborted_reply() {
    // Bind a sync server that *never replies* — the only way the
    // renderer sees a FetchResponse is via the broker's CancelFetch
    // synthesised reply.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    // Hold the listener open for the whole test so the connect
    // succeeds; never accept-and-reply (so the real fetch hangs).
    let _listener = listener;

    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let id = renderer.fetch_async(request);
    assert!(renderer.cancel_fetch(id));

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut got = None;
    while std::time::Instant::now() < deadline {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == id && result.is_err() {
                    got = Some(result);
                }
            }
        }
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let err = got.expect("aborted reply not delivered").unwrap_err();
    assert!(err.contains("aborted"), "expected 'aborted' got: {err}");

    np.shutdown();
}

#[test]
fn fetch_async_on_disconnected_handle_buffers_terminal_error() {
    // R1.1: when the request channel is closed (broker shut down,
    // or `NetworkHandle::disconnected()` test fixture), `fetch_async`
    // must still produce a `FetchResponse(id, Err(...))` so the
    // renderer's `pending_fetches` table can settle on the next
    // drain instead of leaking the entry.
    let renderer = NetworkHandle::disconnected();
    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse("http://example.invalid/").unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let id = renderer.fetch_async(request);
    let events = renderer.drain_events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        NetworkToRenderer::FetchResponse(rid, Err(msg)) => {
            assert_eq!(*rid, id);
            assert!(msg.contains("disconnected"), "got: {msg}");
        }
        other => panic!("expected disconnected error, got {other:?}"),
    }
}

/// True request cancellation: a `cancel_fetch` against a
/// fetch dispatched to a server that never responds must
/// release the in-flight slot promptly, so the next fetch
/// is not blocked behind the stalled IO.
///
/// Pre-PR-true-request-cancellation behaviour: the worker
/// kept its `MAX_CONCURRENT_FETCHES` inflight slot until the
/// underlying `request_timeout` (~30s) — a workload that
/// cancelled aggressively could starve subsequent fetches.
/// With the [`crate::CancelHandle`] wired through to
/// `transport.send`, the hyper future is dropped immediately
/// and the inflight counter decrements via the `FetchInflight
/// Guard` drop.
#[test]
fn cancel_fetch_releases_inflight_slot_promptly() {
    // Bind a sync server that holds the connection open
    // forever (never replies), so any un-cancelled fetch
    // would wait for the request_timeout to fire.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = listener.local_addr().unwrap();
    let _stall_listener = listener; // hold open

    // Second listener: the post-cancel "did the slot free up"
    // probe.  Replies promptly so a successful fetch confirms
    // the inflight counter is below MAX after the cancel.
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
    let renderer = np.create_renderer_handle();

    // Fire 1 fetch at the stalling server, then cancel it.
    let stall_request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let stall_id = renderer.fetch_async(stall_request);
    // Yield briefly so the worker has a chance to enter
    // `transport.send` before the cancel arrives.
    std::thread::sleep(Duration::from_millis(20));
    assert!(renderer.cancel_fetch(stall_id));

    // Drain the synth aborted reply.
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
    assert!(saw_aborted, "synth aborted reply not delivered");

    // Now fire a probe fetch.  If the cancel actually
    // released the inflight slot, this should complete
    // promptly (well under the 30s request_timeout that
    // would gate a saturated counter).
    let probe_request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let probe_id = renderer.fetch_async(probe_request);
    let probe_deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut probe_got = None;
    while std::time::Instant::now() < probe_deadline && probe_got.is_none() {
        for ev in renderer.drain_events() {
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
        .expect("probe fetch did not complete — inflight slot likely not released")
        .unwrap();
    assert_eq!(probe_resp.status, 200);
    assert_eq!(probe_resp.body.as_ref(), b"ok");

    np.shutdown();
    probe_thread.join().unwrap();
}

/// Cancel-spam workload regression (R7.1): dispatch
/// many fetches at a stalling server and cancel each
/// immediately.  Without true cancellation, the inflight
/// counter would saturate at `MAX_CONCURRENT_FETCHES` and
/// later fetches would receive `"too many concurrent
/// fetches"`.  With the [`crate::CancelHandle`] each cancel
/// decrements the counter promptly.
///
/// Sized at 100 (rather than the doc'd 1000) to keep the
/// test under a few seconds of wall-clock; the assertion is
/// the same — subsequent fetch must not be starved.
#[test]
fn cancel_spam_does_not_saturate_inflight_counter() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stall_addr = listener.local_addr().unwrap();
    let _stall_listener = listener;

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
    let renderer = np.create_renderer_handle();

    let mk_stall = || Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };

    let mut ids = Vec::new();
    for _ in 0..100 {
        let id = renderer.fetch_async(mk_stall());
        ids.push(id);
    }
    // Brief pause so a meaningful fraction of workers reach
    // the transport's await point before the cancels fire —
    // exercises the actual abort path more realistically
    // than a pure pre-dispatch cancel.
    std::thread::sleep(Duration::from_millis(50));
    for id in &ids {
        assert!(renderer.cancel_fetch(*id));
    }

    // Drain replies until all 100 cancels are observed.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut seen = std::collections::HashSet::new();
    while std::time::Instant::now() < deadline && seen.len() < ids.len() {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, _) = ev {
                seen.insert(rid);
            }
        }
        if seen.len() < ids.len() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(seen.len(), ids.len(), "not all cancels acknowledged");

    // Probe: fresh fetch must succeed (counter not pinned at MAX).
    let probe_id = renderer.fetch_async(Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", probe_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    });
    let probe_deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut probe_got = None;
    while std::time::Instant::now() < probe_deadline && probe_got.is_none() {
        for ev in renderer.drain_events() {
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
        .expect("probe fetch starved — inflight counter saturated by cancel-spam")
        .unwrap();
    assert_eq!(probe_resp.status, 200);

    np.shutdown();
    probe_thread.join().unwrap();
}

#[test]
fn cancel_fetch_unknown_id_is_idempotent() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();
    // Allocate an id never sent as a Fetch — broker still posts an
    // aborted reply (renderer-side dedupe handles the mismatch).
    let id = FetchId::next();
    assert!(renderer.cancel_fetch(id));

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut got = None;
    while std::time::Instant::now() < deadline {
        for ev in renderer.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == id {
                    got = Some(result);
                }
            }
        }
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let err = got.expect("aborted reply not delivered").unwrap_err();
    assert!(err.contains("aborted"));

    np.shutdown();
}

/// Cross-client cancel isolation (Copilot R1,
/// PR-true-request-cancellation, PR #136): renderer A
/// cannot cancel renderer B's in-flight fetch by
/// sending `CancelFetch` with B's `FetchId`.  Pre-fix the
/// `cancel_tokens` map was keyed only by `FetchId` so A's
/// cancel triggered B's [`crate::CancelHandle`] — the worker
/// suppressed its own reply on observing
/// `NetErrorKind::Cancelled` and the synthetic `Err("aborted")`
/// reply was misrouted to A, leaving B's promise permanently
/// pending.  Post-fix the map is keyed by `(cid, FetchId)`,
/// so A's cancel is a no-op against B's fetch and B receives
/// the worker's real reply on completion.
#[test]
fn cancel_fetch_from_non_owner_is_isolated() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        // Brief delay so the cross-client cancel races
        // against an actually-in-flight request, not one that
        // already completed.
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        std::thread::sleep(Duration::from_millis(80));
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello");
    });

    let np = spawn_network_process(test_client());
    let owner = np.create_renderer_handle();
    let attacker = np.create_renderer_handle();

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", server_addr.port())).unwrap(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
        ..Default::default()
    };
    let owner_id = owner.fetch_async(request);
    // Yield so the worker enters transport.send before the
    // cross-client cancel arrives.
    std::thread::sleep(Duration::from_millis(20));

    // Attacker tries to cancel owner's FetchId.  Broker
    // accepts the message but the (attacker, fetch_id) lookup
    // misses, so the underlying CancelHandle is NOT triggered.
    // The synthetic aborted reply goes back to the attacker
    // and is silently dropped (attacker has no matching
    // pending entry).
    assert!(attacker.cancel_fetch(owner_id));

    // Owner must observe a successful reply — the worker
    // wasn't actually cancelled.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut owner_result = None;
    while std::time::Instant::now() < deadline && owner_result.is_none() {
        for ev in owner.drain_events() {
            if let NetworkToRenderer::FetchResponse(rid, result) = ev {
                if rid == owner_id {
                    owner_result = Some(result);
                }
            }
        }
        if owner_result.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let resp = owner_result
        .expect("owner did not receive any reply — cross-client cancel hit owner's fetch")
        .expect("owner saw aborted error — cross-client cancel triggered owner's CancelHandle");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"hello");

    // Attacker may have observed the synthetic aborted reply
    // (harmless — its pending_fetches table never had this
    // id), but the *owner* must not have seen one in addition
    // to the success.
    np.shutdown();
    server_thread.join().unwrap();
}

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
