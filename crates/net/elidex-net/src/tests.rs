//! Unit tests for `elidex-net` lib.rs (extracted from the inline
//! `#[cfg(test)] mod tests` to keep the crate root under the
//! 1000-line convention).
//!
//! These exercise [`super::should_attach_cookies`],
//! [`super::should_store_set_cookie_from`], and the basic
//! [`super::NetClient`] / [`super::Request`] surface.  Companion to
//! the integration-style scenarios in
//! [`super::preflight_integration_tests`].

#![cfg(test)]

use elidex_plugin::SecurityOrigin;

use super::*;

#[test]
fn net_client_default() {
    let client = NetClient::new();
    assert!(client.cookie_jar().is_empty());
}

#[test]
fn net_client_config_defaults() {
    let config = NetClientConfig::default();
    assert!(!config.file_access);
    assert!(!config.https_only);
}

#[test]
fn request_clone() {
    let req = Request {
        method: "POST".to_string(),
        url: url::Url::parse("https://example.com").unwrap(),
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: Bytes::from("hello"),
        ..Default::default()
    };
    let cloned = req.clone();
    assert_eq!(cloned.method, "POST");
    assert_eq!(cloned.body.as_ref(), b"hello");
}

#[tokio::test]
async fn send_to_local_server() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
        stream.write_all(response).await.unwrap();
    });

    let client = NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        },
        ..Default::default()
    });

    let request = Request {
        method: "GET".to_string(),
        url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
        headers: Vec::new(),
        body: Bytes::new(),
        ..Default::default()
    };

    let response = client.send(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body.as_ref(), b"ok");
}

#[tokio::test]
async fn load_data_url() {
    let client = NetClient::new();
    let url = url::Url::parse("data:text/plain,Hello%20World").unwrap();
    let response = client.load(&url).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body.as_ref(), b"Hello World");
    assert_eq!(response.content_type.as_deref(), Some("text/plain"));
}

#[test]
fn should_attach_cookies_omit_returns_false() {
    let request = Request {
        url: url::Url::parse("http://example.com/").unwrap(),
        credentials: CredentialsMode::Omit,
        ..Default::default()
    };
    assert!(!should_attach_cookies(&request));
}

#[test]
fn should_attach_cookies_include_always_true() {
    let request = Request {
        url: url::Url::parse("http://example.com/").unwrap(),
        origin: Some(SecurityOrigin::from_url(
            &url::Url::parse("http://other.com/").unwrap(),
        )),
        credentials: CredentialsMode::Include,
        ..Default::default()
    };
    assert!(should_attach_cookies(&request));
}

#[test]
fn should_attach_cookies_same_origin_default_attaches_when_no_origin() {
    // Embedder-driven loads have no document-origin context;
    // PR5-cors preserves the pre-PR top-level navigation
    // attach behaviour when origin is None.
    let request = Request {
        url: url::Url::parse("http://example.com/").unwrap(),
        origin: None,
        credentials: CredentialsMode::SameOrigin,
        ..Default::default()
    };
    assert!(should_attach_cookies(&request));
}

#[test]
fn should_attach_cookies_same_origin_blocks_cross_origin() {
    let request = Request {
        url: url::Url::parse("http://api.other.com/data").unwrap(),
        origin: Some(SecurityOrigin::from_url(
            &url::Url::parse("http://example.com/").unwrap(),
        )),
        credentials: CredentialsMode::SameOrigin,
        ..Default::default()
    };
    assert!(!should_attach_cookies(&request));
}

#[test]
fn should_attach_cookies_same_origin_passes_same_origin_match() {
    let request = Request {
        url: url::Url::parse("http://example.com/data").unwrap(),
        origin: Some(SecurityOrigin::from_url(
            &url::Url::parse("http://example.com/page").unwrap(),
        )),
        credentials: CredentialsMode::SameOrigin,
        ..Default::default()
    };
    assert!(should_attach_cookies(&request));
}

/// S5-4d: an **opaque** initiator origin (sandboxed document /
/// `data:` script) strips cookies under `SameOrigin` — an opaque
/// origin equals no tuple URL origin *by type*, so the credential
/// strip is structural, not a per-call scheme check.  This is the
/// matrix's opaque row that a `url::Origin`-typed field could not
/// carry with a stable identity.
#[test]
fn should_attach_cookies_same_origin_opaque_initiator_strips() {
    let request = Request {
        url: url::Url::parse("http://example.com/api").unwrap(),
        origin: Some(SecurityOrigin::opaque()),
        credentials: CredentialsMode::SameOrigin,
        ..Default::default()
    };
    assert!(!should_attach_cookies(&request));
}

/// Counterpart: `Include` still attaches for an opaque initiator —
/// the strip is a `SameOrigin`-only origin-equality gate, not an
/// opaque-origin block.
#[test]
fn should_attach_cookies_include_opaque_initiator_attaches() {
    let request = Request {
        url: url::Url::parse("http://example.com/api").unwrap(),
        origin: Some(SecurityOrigin::opaque()),
        credentials: CredentialsMode::Include,
        ..Default::default()
    };
    assert!(should_attach_cookies(&request));
}

/// Copilot R3 regression (finding 1): cookie storage from
/// the **final** post-redirect response must re-evaluate
/// the SameOrigin check against `response.url` so a
/// same-origin → cross-origin redirect under
/// `CredentialsMode::SameOrigin` does NOT persist cookies
/// from the cross-origin response.
#[test]
fn should_store_set_cookie_blocks_cross_origin_redirect_under_same_origin() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://attacker.com/landing").unwrap();
    // Same-origin credentials, but final URL crossed origin
    // (the redirect chain landed at attacker.com).  Storage
    // decision must be `false`.
    assert!(!should_store_set_cookie_from(
        CredentialsMode::SameOrigin,
        Some(&source_origin),
        &final_url,
        false,
    ));
}

/// Counterpart sentinel: a same-origin → same-origin
/// redirect chain still stores cookies under SameOrigin.
#[test]
fn should_store_set_cookie_allows_same_origin_redirect_under_same_origin() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://example.com/landing").unwrap();
    assert!(should_store_set_cookie_from(
        CredentialsMode::SameOrigin,
        Some(&source_origin),
        &final_url,
        false,
    ));
}

/// Counterpart sentinel: `Include` always stores even
/// across cross-origin redirects.
#[test]
fn should_store_set_cookie_include_always_stores() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://attacker.com/landing").unwrap();
    assert!(should_store_set_cookie_from(
        CredentialsMode::Include,
        Some(&source_origin),
        &final_url,
        false,
    ));
}

/// Counterpart sentinel: `Omit` never stores.
#[test]
fn should_store_set_cookie_omit_never_stores() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://example.com/landing").unwrap();
    assert!(!should_store_set_cookie_from(
        CredentialsMode::Omit,
        Some(&source_origin),
        &final_url,
        false,
    ));
}

/// PR-cors-redirect-preflight: SameOrigin credentials must
/// reject cookie storage when the redirect chain crossed
/// origin even if the final URL landed back same-origin
/// (`redirect_tainted = true`).  Without this gate, a
/// cross-origin hop could emit a `Set-Cookie` that the
/// same-origin landing hop "blesses" through this gate.
#[test]
fn should_store_set_cookie_blocks_tainted_chain_under_same_origin() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://example.com/landing").unwrap();
    assert!(!should_store_set_cookie_from(
        CredentialsMode::SameOrigin,
        Some(&source_origin),
        &final_url,
        true,
    ));
}

/// Sentinel: `Include` ignores the redirect-tainted flag —
/// the spec doesn't restrict cookie storage on `Include`
/// chains; it's the caller's responsibility to keep that
/// path off untrusted endpoints.
#[test]
fn should_store_set_cookie_include_ignores_tainted_flag() {
    let source_origin =
        SecurityOrigin::from_url(&url::Url::parse("http://example.com/page").unwrap());
    let final_url = url::Url::parse("http://example.com/landing").unwrap();
    assert!(should_store_set_cookie_from(
        CredentialsMode::Include,
        Some(&source_origin),
        &final_url,
        true,
    ));
}

/// S5-4d: an opaque initiator never persists Set-Cookie under
/// `SameOrigin` (opaque equals no tuple response origin), even on
/// an untainted chain — the structural strip mirrors
/// [`should_attach_cookies_same_origin_opaque_initiator_strips`] on
/// the storage side.
#[test]
fn should_store_set_cookie_opaque_initiator_never_stores_under_same_origin() {
    let source_origin = SecurityOrigin::opaque();
    let final_url = url::Url::parse("http://example.com/landing").unwrap();
    assert!(!should_store_set_cookie_from(
        CredentialsMode::SameOrigin,
        Some(&source_origin),
        &final_url,
        false,
    ));
}

/// Regression test: `Request.origin` is a [`SecurityOrigin`]
/// (not a full URL) so the broker never sees the initiator's
/// path / query / fragment.  This test exercises the type-level
/// guarantee — the origin is derived via
/// [`SecurityOrigin::from_url`], which discards everything below
/// the origin tuple, so a path-leaking comparison cannot regress
/// back in.
#[test]
fn request_origin_is_origin_not_url_with_path() {
    let initiator = url::Url::parse("http://example.com/page?secret=1#frag").unwrap();
    let request = Request {
        url: url::Url::parse("http://example.com/api").unwrap(),
        // The path / query / fragment of the initiator are
        // discarded by `SecurityOrigin::from_url` — this is the contract.
        origin: Some(SecurityOrigin::from_url(&initiator)),
        credentials: CredentialsMode::SameOrigin,
        ..Default::default()
    };
    assert!(should_attach_cookies(&request));
    // serialize() of the origin must NOT contain
    // path / query / fragment.
    let serialised = request.origin.as_ref().unwrap().serialize();
    assert_eq!(serialised, "http://example.com");
    assert!(!serialised.contains("/page"));
    assert!(!serialised.contains("secret"));
    assert!(!serialised.contains("frag"));
}
