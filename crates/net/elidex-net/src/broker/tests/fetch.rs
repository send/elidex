//! Broker integration tests for fetch dispatch (blocking + async + disconnected fallback).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).

use std::time::Duration;

use super::super::*;
use super::test_client;
use crate::Request;
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
