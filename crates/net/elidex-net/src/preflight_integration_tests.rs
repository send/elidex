//! Integration-style tests for the WHATWG Fetch §4.8 CORS
//! preflight pipeline (extracted from the inline
//! `#[cfg(test)] mod preflight_integration_tests` to keep the
//! crate root under the 1000-line convention).
//!
//! Each test stands up a scripted TCP server (see
//! [`spawn_scripted_server`]) that records the raw request bytes
//! seen on each connection, so assertions can verify the OPTIONS
//! preflight + actual request shape.  Companion to the unit-style
//! coverage in [`super::tests`].

#![cfg(test)]

use super::*;
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use transport::TransportConfig;

/// Stand up a TCP server that answers each accepted
/// connection with the next scripted response, recording the
/// raw request bytes seen on each connection so test
/// assertions can verify the OPTIONS preflight + actual
/// request shape.
///
/// The returned `Arc<Mutex<Vec<String>>>` accumulates request
/// strings (UTF-8-lossy) in accept order.  The function
/// shuts down once `responses.len()` connections have been
/// served.
async fn spawn_scripted_server(responses: Vec<Vec<u8>>) -> (u16, StdArc<StdMutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let recorded: StdArc<StdMutex<Vec<String>>> = StdArc::new(StdMutex::new(Vec::new()));
    let recorded_clone = StdArc::clone(&recorded);
    // `Vec<u8>` (not `&'static [u8]`) so callers can build
    // dynamically formatted bodies without `Box::leak`'ing for
    // a `'static` upgrade (Copilot R4).  Literal slices use
    // `.to_vec()`; formatted strings use `.into_bytes()`.
    tokio::spawn(async move {
        for body in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Read until end-of-headers `\r\n\r\n` (or EOF) so
            // we don't truncate on a TCP fragment boundary —
            // tests assert on header content (ACRM / ACRH /
            // method line) which can land arbitrarily late
            // depending on hyper's write batching (Copilot R3).
            let mut buf = Vec::with_capacity(4096);
            let mut chunk = [0u8; 1024];
            loop {
                let n = stream.read(&mut chunk).await.expect("scripted server read");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let req = String::from_utf8_lossy(&buf).to_string();
            recorded_clone.lock().unwrap().push(req);
            stream
                .write_all(&body)
                .await
                .expect("scripted server write");
        }
    });
    (port, recorded)
}

fn test_client() -> NetClient {
    NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        },
        ..Default::default()
    })
}

fn cors_request(method: &str, port: u16, headers: Vec<(String, String)>) -> Request {
    Request {
        method: method.to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
        headers,
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    }
}

#[tokio::test]
async fn cors_simple_request_skips_preflight() {
    // Single-response server: cross-origin GET with no custom
    // headers → no preflight needed; a single GET round-trip
    // suffices.
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: http://example.com\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
    ]).await;

    let client = test_client();
    let request = cors_request("GET", port, vec![]);
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].starts_with("GET "));
}

#[tokio::test]
async fn cors_custom_header_issues_preflight_first() {
    // Two-response server: OPTIONS (204 with allow headers) → GET (200).
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Headers: x-custom\r\n\
          Access-Control-Allow-Methods: GET\r\n\
          Access-Control-Max-Age: 60\r\n\
          Content-Length: 0\r\n\
          Connection: close\r\n\r\n"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 2\r\nConnection: close\r\n\r\nok"
            .to_vec(),
    ])
    .await;

    let client = test_client();
    let request = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 2, "expected OPTIONS + GET");
    assert!(
        recorded[0].starts_with("OPTIONS "),
        "first request should be OPTIONS, got: {}",
        recorded[0]
    );
    assert!(
        recorded[0]
            .to_ascii_lowercase()
            .contains("access-control-request-method: get"),
        "OPTIONS should include ACRM"
    );
    assert!(
        recorded[0]
            .to_ascii_lowercase()
            .contains("access-control-request-headers: x-custom"),
        "OPTIONS should include ACRH"
    );
    assert!(recorded[1].starts_with("GET "));
}

#[tokio::test]
async fn preflight_method_rejection_blocks_request() {
    // OPTIONS responds without listing PUT in ACAM → preflight
    // fails closed; actual PUT is never sent.
    let (port, recorded) = spawn_scripted_server(vec![b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: GET\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
        .to_vec()])
    .await;
    let client = test_client();
    let request = cors_request("PUT", port, vec![]);
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1, "actual PUT must NOT be dispatched");
    assert!(recorded[0].starts_with("OPTIONS "));
}

#[tokio::test]
async fn preflight_5xx_blocks_request() {
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
    ])
    .await;
    let client = test_client();
    let request = cors_request("PUT", port, vec![]);
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
}

#[tokio::test]
async fn preflight_acao_mismatch_blocks_request() {
    let (port, _recorded) = spawn_scripted_server(vec![b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://attacker.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
        .to_vec()])
    .await;
    let client = test_client();
    let request = cors_request("PUT", port, vec![]);
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
}

#[tokio::test]
async fn preflight_cache_hit_skips_options_round_trip() {
    // First request: OPTIONS + actual.  Cache stores
    // allowance with max-age=60.  Second identical request
    // should skip OPTIONS and dispatch only the actual.
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Headers: x-custom\r\n\
          Access-Control-Allow-Methods: GET\r\n\
          Access-Control-Max-Age: 60\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 2\r\nConnection: close\r\n\r\nok"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 2\r\nConnection: close\r\n\r\nok"
            .to_vec(),
    ])
    .await;

    let client = test_client();
    let request1 = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
    let request2 = cors_request("GET", port, vec![("X-Custom".into(), "1".into())]);
    client.send(request1).await.unwrap();
    client.send(request2).await.unwrap();
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 3, "OPTIONS + GET + GET (cache hit on 2nd)");
    assert!(recorded[0].starts_with("OPTIONS "));
    assert!(recorded[1].starts_with("GET "));
    assert!(recorded[2].starts_with("GET "));
}

/// Regression for Copilot R1 finding 6: a `mode = Cors` +
/// `credentials = Include` request that gets redirected
/// cross-origin must NOT persist Set-Cookie from the
/// cross-origin response (per WHATWG Fetch §4.4 step 14
/// credentials downgrade).  Pre-fix `NetClient::send`
/// snapshotted credentials BEFORE `follow_redirects` so the
/// downgrade had no effect on the storage gate; post-fix
/// `follow_redirects` returns the post-redirect credentials
/// so the gate honours the downgrade.
#[tokio::test]
async fn cors_redirect_with_include_downgrades_credentials_for_set_cookie_storage() {
    // Server A: returns 302 → server B (cross-origin via
    // different port).  Server B: returns 200 with Set-Cookie.
    let (port_b, _rec_b) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nSet-Cookie: leak=cross_origin; Path=/\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
    ]).await;
    let location = format!("http://127.0.0.1:{port_b}/landing");
    let response_a = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let (port_a, _rec_a) = spawn_scripted_server(vec![response_a.into_bytes()]).await;

    let client = test_client();
    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port_a}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::Include,
        redirect: RedirectMode::Follow,
    };
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    // Cookie jar must remain empty: the §4.4 step 14
    // downgrade flipped credentials Include → SameOrigin
    // mid-redirect, and the cross-origin response URL fails
    // the SameOrigin storage gate (response.url.origin() !=
    // request.origin).
    assert!(
        client.cookie_jar().is_empty(),
        "cross-origin Set-Cookie must not leak under credentials-downgrade"
    );
}

/// Regression for Copilot R5 finding 2: a SIMPLE cors-mode
/// GET (no preflight needed per §4.8.1) with `origin=None`
/// must STILL fail closed — pre-R5 the broker entry only
/// gated through `requires_preflight`, so simple cors GET
/// without origin context bypassed the §4.4 / §4.8 fail-
/// closed gate entirely.  Closed at the broker entry now,
/// before middleware / preflight detection / dispatch.
#[tokio::test]
async fn simple_cors_mode_without_origin_fails_closed() {
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    ])
    .await;
    let client = test_client();
    let request = Request {
        // Simple safelisted-method request — would NOT trigger
        // preflight under §4.8.1, so the R2 origin-None gate
        // inside `run_preflight` doesn't catch it.
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: None, // ← the bug condition
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    let recorded = recorded.lock().unwrap();
    assert!(
        recorded.is_empty(),
        "simple cors-mode without origin must NOT be dispatched"
    );
}

/// Regression for Copilot R2 finding 1: a `mode = Cors`
/// request that reaches the preflight stage but has no
/// origin must fail closed rather than silently bypass the
/// CORS gate.  In normal flow the VM-side fetch path always
/// sets `Request.origin` (see `attach_default_origin`), so
/// reaching this branch means a misconfigured embedder
/// caller — fail closed defensively.
#[tokio::test]
async fn cors_mode_without_origin_fails_closed_at_preflight() {
    // Server should never be hit — preflight must reject
    // before dispatch.
    let (port, recorded) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    ])
    .await;
    let client = test_client();
    let request = Request {
        method: "PUT".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port}/data")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: None, // ← the bug condition
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    let recorded = recorded.lock().unwrap();
    assert!(
        recorded.is_empty(),
        "actual request must NOT be dispatched when origin is missing in cors mode"
    );
}

/// PR-cors-redirect-preflight: cross-origin CORS redirects
/// to a non-simple-method target now re-issue the §4.8
/// preflight against the redirect URL, then dispatch the
/// actual request when the second preflight succeeds.
/// Previously this path failed closed with `CorsBlocked`.
#[tokio::test]
async fn cors_redirect_re_preflights_and_succeeds() {
    // Landing server: receives a re-issued preflight at
    // `/dest`, responds with allowance, then receives the
    // PUT and replies 200.  Spawned first so the origin
    // server's 302 can encode the actual landing port.
    let (land_port, land_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 7\r\nConnection: close\r\n\r\nlanding"
            .to_vec(),
    ])
    .await;
    // Origin server: receives the initial PUT preflight
    // (`OPTIONS /start`), responds with allowance, then
    // receives the actual PUT and emits a 302 to the
    // cross-origin landing server.
    let location_header = format!("http://127.0.0.1:{land_port}/dest");
    let redirect_response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let (origin_port, origin_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        redirect_response.into_bytes(),
    ])
    .await;

    let client = test_client();
    let request = Request {
        method: "PUT".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body.as_ref(), b"landing");

    // Origin server saw OPTIONS then PUT.  Landing server
    // saw OPTIONS (re-issued preflight) then PUT.  Each
    // server records exactly two requests.
    let origin_reqs = origin_rec.lock().unwrap().clone();
    assert_eq!(origin_reqs.len(), 2);
    assert!(origin_reqs[0].starts_with("OPTIONS "));
    assert!(origin_reqs[1].starts_with("PUT "));
    let land_reqs = land_rec.lock().unwrap().clone();
    assert_eq!(land_reqs.len(), 2);
    assert!(land_reqs[0].starts_with("OPTIONS "));
    assert!(land_reqs[1].starts_with("PUT "));

    // Final response carries the redirect-tainted flag —
    // the chain crossed origin from 127.0.0.1 → 127.0.0.1
    // (different ports = different origins per RFC 6454).
    assert!(
        response.is_redirect_tainted,
        "redirect-tainted flag must be set after a cross-origin redirect"
    );
    // url_list captures both hops.
    assert_eq!(
        response.url_list.len(),
        2,
        "url_list must record both redirect hops"
    );
}

/// Re-preflight failure at the redirect target surfaces
/// `CorsBlocked` (the actual request is never dispatched).
#[tokio::test]
async fn cors_redirect_re_preflight_failure_blocks() {
    // First spawn the landing server with a *failing*
    // preflight response (no ACAO).
    let (land_port, _land_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    ])
    .await;
    let location_header = format!("http://127.0.0.1:{land_port}/dest");
    let response_2 = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let (origin_port, _origin_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        response_2.into_bytes(),
    ])
    .await;

    let client = test_client();
    let request = Request {
        method: "PUT".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let err = client.send(request).await.unwrap_err();
    assert_eq!(err.kind, NetErrorKind::CorsBlocked);
}

/// Simple-method (`GET`) redirect chain: even when crossing
/// origin, no preflight is required at the redirect target.
/// Sentinel against accidentally re-issuing OPTIONS for
/// every cross-origin GET redirect.
#[tokio::test]
async fn cors_redirect_simple_request_no_re_preflight() {
    // Landing: single GET response (no OPTIONS ahead of it
    // — if the broker mis-issues a preflight, this single-
    // response server hangs and the test times out).
    let (land_port, land_rec) = spawn_scripted_server(vec![b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 4\r\nConnection: close\r\n\r\nland"
        .to_vec()])
    .await;
    let location_header = format!("http://127.0.0.1:{land_port}/dest");
    let response_redirect = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nAccess-Control-Allow-Origin: http://example.com\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let (origin_port, _origin_rec) =
        spawn_scripted_server(vec![response_redirect.into_bytes()]).await;

    let client = test_client();
    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body.as_ref(), b"land");

    // Landing server saw exactly one request (the GET — no
    // preflight).
    let land_reqs = land_rec.lock().unwrap().clone();
    assert_eq!(
        land_reqs.len(),
        1,
        "simple-method redirect must not trigger re-preflight"
    );
    assert!(land_reqs[0].starts_with("GET "));
    // Tainted flag still set (origin differs).
    assert!(response.is_redirect_tainted);
}

/// PR-cors-redirect-preflight: a preflight cache hit on the
/// redirect target skips the re-issued OPTIONS, so the
/// landing server only sees the actual PUT on the second
/// run of the same chain.
#[tokio::test]
async fn cors_redirect_re_preflight_cache_hit() {
    // Landing server scripts: (run1) OPTIONS + PUT, (run2)
    // PUT only — the cache hit avoids the second OPTIONS.
    let (land_port, land_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Access-Control-Max-Age: 3600\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 1\r\nConnection: close\r\n\r\nA"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 1\r\nConnection: close\r\n\r\nB"
            .to_vec(),
    ])
    .await;
    let location_header = format!("http://127.0.0.1:{land_port}/dest");
    let redirect_response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let preflight: Vec<u8> = b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Access-Control-Max-Age: 3600\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
        .to_vec();
    // Origin server: run1 sees OPTIONS+PUT, run2's OPTIONS
    // is short-circuited by the cache hit on `/start` so it
    // sees only PUT.  Total 3 responses.
    let (origin_port, _origin_rec) = spawn_scripted_server(vec![
        preflight,
        redirect_response.clone().into_bytes(),
        redirect_response.into_bytes(),
    ])
    .await;

    let client = test_client();
    let mk_request = |port| Request {
        method: "PUT".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let first = client.send(mk_request(origin_port)).await.unwrap();
    assert_eq!(first.status, 200);
    let second = client.send(mk_request(origin_port)).await.unwrap();
    assert_eq!(second.status, 200);

    // Landing server saw OPTIONS+PUT on run1, then PUT only
    // on run2 (the OPTIONS was short-circuited by the cache
    // hit on the redirect target's preflight key).
    let land_reqs = land_rec.lock().unwrap().clone();
    assert_eq!(
        land_reqs.len(),
        3,
        "landing server should receive 3 requests across two runs (OPTIONS+PUT then PUT only)"
    );
    assert!(land_reqs[0].starts_with("OPTIONS "));
    assert!(land_reqs[1].starts_with("PUT "));
    assert!(land_reqs[2].starts_with("PUT "));
}

/// PR-cors-redirect-preflight: cookie storage gate honours
/// the redirect-tainted flag — a chain that crossed origin
/// must not persist `Set-Cookie` from the final response
/// even when the landing URL is back on the initiator
/// origin under `SameOrigin` credentials.
#[tokio::test]
async fn cors_redirect_tainted_chain_blocks_cookie_storage() {
    // Landing server: same origin as initiator (example.com)
    // — but reached through a cross-origin hop, so the chain
    // is tainted.  Set-Cookie on the final response must not
    // be stored.  We can't easily run example.com on
    // 127.0.0.1, so we use a same-origin-with-initiator
    // setup by aligning the request `origin` with the
    // landing port.
    let (land_port, _land_rec) = spawn_scripted_server(vec![
        b"HTTP/1.1 200 OK\r\nSet-Cookie: tainted=yes; Path=/\r\nContent-Length: 1\r\nConnection: close\r\n\r\nL".to_vec(),
    ])
    .await;
    let initiator_origin = url::Url::parse(&format!("http://127.0.0.1:{land_port}/page"))
        .unwrap()
        .origin();
    // Cross-origin redirector at a *different* port.
    let location_header = format!("http://127.0.0.1:{land_port}/dest");
    let redirect_response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location_header}\r\nAccess-Control-Allow-Origin: {origin_str}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        origin_str = initiator_origin.ascii_serialization(),
    );
    let (origin_port, _origin_rec) =
        spawn_scripted_server(vec![redirect_response.into_bytes()]).await;

    let client = test_client();
    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{origin_port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(initiator_origin),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert!(
        response.is_redirect_tainted,
        "chain crossed origin (different ports) — must be tainted"
    );

    // Cookie jar must NOT have stored the `tainted=yes`
    // cookie despite the final URL being same-origin with
    // the initiator (same port).  Cookie jar `cookie_header_for_url`
    // returns `None` when no cookies match.
    let landing_url = url::Url::parse(&format!("http://127.0.0.1:{land_port}/page")).unwrap();
    let attached = client.cookie_jar().cookie_header_for_url(&landing_url);
    assert!(
        attached.is_none(),
        "tainted-chain Set-Cookie must not be persisted under SameOrigin"
    );
}

/// PR-cors-redirect-preflight Copilot R3: a same-origin
/// redirect hop within a cross-origin server (e.g.
/// `https://api.other.com/start` → `/dest`, both on the same
/// cross-origin host) must still re-issue OPTIONS against
/// `/dest` because the §4.8 preflight cache is keyed
/// per-URL.  Without this the broker would skip the
/// per-URL preflight and dispatch the actual non-simple
/// request without a fresh allowance for `/dest`.
#[tokio::test]
async fn cors_redirect_same_origin_hop_to_different_url_re_preflights() {
    // Cross-origin server hosts both /start and /dest.  /start
    // gets the initial OPTIONS+PUT (PUT responds with 302 →
    // /dest).  /dest gets its own OPTIONS+PUT.  All on the
    // same port (= same origin from the initiator's POV) but
    // different URLs.
    let preflight: Vec<u8> = b"HTTP/1.1 204 No Content\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Access-Control-Allow-Methods: PUT\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
        .to_vec();
    // We use a relative `Location: /dest` so the port doesn't
    // need to be pre-known: it resolves against the current
    // request URL.
    let redirect_response: Vec<u8> = b"HTTP/1.1 302 Found\r\n\
          Location: /dest\r\n\
          Content-Length: 0\r\nConnection: close\r\n\r\n"
        .to_vec();
    let final_response: Vec<u8> = b"HTTP/1.1 200 OK\r\n\
          Access-Control-Allow-Origin: http://example.com\r\n\
          Content-Length: 1\r\nConnection: close\r\n\r\nD"
        .to_vec();
    // Server scripted: OPTIONS /start → PUT /start (302) →
    // OPTIONS /dest → PUT /dest (200).
    let (port, recorded) = spawn_scripted_server(vec![
        preflight.clone(),
        redirect_response,
        preflight,
        final_response,
    ])
    .await;

    let client = test_client();
    let request = Request {
        method: "PUT".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{port}/start")).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        origin: Some(url::Url::parse("http://example.com/").unwrap().origin()),
        mode: RequestMode::Cors,
        credentials: CredentialsMode::SameOrigin,
        redirect: RedirectMode::Follow,
    };
    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);

    // Server saw exactly 4 requests: OPTIONS /start, PUT /start,
    // OPTIONS /dest, PUT /dest.  Without the R3 fix, the second
    // OPTIONS would be skipped because `is_same_origin(/start, /dest)`
    // returned true, mis-treating the same-origin hop as
    // "no preflight needed".
    let reqs = recorded.lock().unwrap().clone();
    assert_eq!(reqs.len(), 4, "must see 4 requests (OPTIONS+PUT × 2)");
    assert!(reqs[0].starts_with("OPTIONS /start"));
    assert!(reqs[1].starts_with("PUT /start"));
    assert!(
        reqs[2].starts_with("OPTIONS /dest"),
        "redirect target on same cross-origin server must still be re-preflighted (R3): got {:?}",
        reqs[2].lines().next()
    );
    assert!(reqs[3].starts_with("PUT /dest"));
}
