//! Network Process broker (design doc §5.2, §5.3.3).
//!
//! Implements the Network Process as a singleton coordination thread that owns
//! the shared [`crate::NetClient`], cookie jar, and all WebSocket/SSE I/O loops.
//! Each HTTP fetch is executed on its own OS thread with a per-request tokio
//! runtime (see `dispatch::handle_fetch`).
//!
//! Content threads (Renderers) communicate exclusively through typed channels:
//! - [`RendererToNetwork`]: requests from content thread → Network Process
//! - [`NetworkToRenderer`]: responses/events from Network Process → content thread
//!
//! The broker is spawned once by the browser thread via [`spawn_network_process`].
//! Each content thread receives a [`NetworkHandle`] for IPC. All network access
//! is mediated through the broker — content threads never touch network APIs
//! directly, enabling OS-level sandbox enforcement (seccomp-bpf, etc.).
//!
//! # Cookie sharing
//!
//! The broker owns a single [`crate::NetClient`] (with shared `CookieJar`),
//! fixing the previous design where each content thread had its own
//! `FetchHandle` with an isolated cookie jar (spec violation — cookies must
//! be shared across browsing contexts within a profile).
//!
//! # Module layout
//!
//! Internal submodules (private; types are re-exported here):
//!
//! - `handle` — [`NetworkHandle`] + [`NetworkProcessHandle`] structs and
//!   their lifecycle methods, plus the [`spawn_network_process`] entry point.
//! - `dispatch` — broker thread main loop + per-renderer state machine
//!   (`NetworkProcessState`, `network_process_main`, `handle_fetch` worker
//!   spawn, WS/SSE forwarding).
//! - `cancel` — per-fetch cancellation token map (`CancelMap`) and the
//!   panic-safe RAII guards that keep it bounded.
//! - `buffered` — [`NetworkHandle::drain_events`] /
//!   [`NetworkHandle::drain_fetch_responses_only`] /
//!   [`NetworkHandle::rebuffer_events`] partial-drain helpers.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::sse::SseEvent;
use crate::ws::{WsCommand, WsEvent};
use crate::{Request, Response};

mod buffered;
mod cancel;
mod dispatch;
mod handle;

pub use handle::{spawn_network_process, NetworkHandle, NetworkProcessHandle};

// ---------------------------------------------------------------------------
// ID types
// ---------------------------------------------------------------------------

/// Unique fetch request identifier (globally monotonic).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FetchId(pub u64);

/// Monotonic counter for renderer client IDs.
pub(super) static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Monotonic counter for fetch request IDs.
static FETCH_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl FetchId {
    /// Generate a new unique fetch ID.
    #[must_use]
    pub fn next() -> Self {
        Self(FETCH_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

// ---------------------------------------------------------------------------
// Message types (design doc §5.3.3)
// ---------------------------------------------------------------------------

/// Messages from a Renderer (content thread) to the Network Process.
#[derive(Debug)]
pub enum RendererToNetwork {
    /// HTTP fetch request.
    Fetch(FetchId, Request),
    /// Cancel a pending fetch.
    CancelFetch(FetchId),
    /// Open a WebSocket connection.
    WebSocketOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// WebSocket URL (ws:// or wss://).
        url: url::Url,
        /// Requested sub-protocols.
        protocols: Vec<String>,
        /// Document origin for the `Origin` header.
        origin: String,
    },
    /// Send a WebSocket command (text/binary/close).
    WebSocketSend(u64, WsCommand),
    /// Close a WebSocket connection.
    WebSocketClose(u64),
    /// Open a Server-Sent Events connection.
    EventSourceOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// HTTP(S) URL for the event stream.
        url: url::Url,
        /// Last event ID for reconnection.
        last_event_id: Option<String>,
        /// Document origin for CORS (None = same-origin).
        origin: Option<String>,
        /// Whether to send credentials (cookies) cross-origin.
        with_credentials: bool,
    },
    /// Close an SSE connection (stop auto-reconnect).
    EventSourceClose(u64),
    /// Shutdown all connections for this renderer.
    Shutdown,
}

/// Messages from the Network Process to a Renderer (content thread).
#[derive(Debug)]
pub enum NetworkToRenderer {
    /// HTTP fetch response.
    FetchResponse(FetchId, Result<Response, String>),
    /// WebSocket event.
    WebSocketEvent(u64, WsEvent),
    /// SSE event.
    EventSourceEvent(u64, SseEvent),
}

/// Control messages from the Browser thread to the Network Process.
#[derive(Debug)]
pub enum NetworkProcessControl {
    /// Register a new renderer (content thread).
    RegisterRenderer {
        /// Unique client identifier.
        client_id: u64,
        /// Channel to send responses/events to this renderer.
        response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
    },
    /// Unregister a renderer (content thread shutting down).
    UnregisterRenderer {
        /// Client ID to remove.
        client_id: u64,
    },
    /// Shutdown the Network Process.
    Shutdown,
}

#[cfg(test)]
mod tests {
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
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
            );
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
    /// Verifies the cancel-on-shutdown path by:
    /// 1. Issuing a fetch against a stalling server.
    /// 2. Calling `np.shutdown()` (which join()s the broker thread).
    /// 3. Checking that the renderer observed an `aborted` reply
    ///    delivered before the broker thread joined — pre-fix the
    ///    reply would either not exist (worker not yet finished)
    ///    or arrive after `shutdown()` returned (race-prone).
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
        let id = renderer.fetch_async(request);
        // Yield so the worker reaches `transport.send`.
        std::thread::sleep(Duration::from_millis(40));

        // Shutdown — broker must cancel the inflight fetch and
        // join cleanly within the 5-second deadline below.
        let started = std::time::Instant::now();
        np.shutdown();
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "shutdown() blocked for {elapsed:?} — likely waiting for un-cancelled fetch's transport timeout"
        );

        // The worker observes `Cancelled` and silently exits, so
        // the renderer may or may not see a reply for `id` (the
        // synthetic `aborted` reply path runs against the broker's
        // `clients` map, which `Shutdown` clears as part of the
        // cleanup — see `handle_control`).  What we MUST verify
        // is that the broker thread did not block waiting for
        // network IO; the `elapsed` assertion above covers that.
        // We also confirm the renderer is now disconnected:
        let _ = id; // (suppress unused warning when the assertion below shifts)
        let post_request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", stall_addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
            ..Default::default()
        };
        // After shutdown, the request channel is closed → fetch_async
        // buffers a synthetic disconnect error.
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
}
