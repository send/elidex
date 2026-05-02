//! Broker integration tests for cancel-fetch flows (synthetic reply, slot release, cancel spam, idempotence, cross-client isolation).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).

use std::time::Duration;

use super::super::*;
use super::test_client;
use crate::Request;
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
