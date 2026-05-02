//! HTTP redirect tracking with SSRF re-validation.
//!
//! Follows 301, 302, 303, 307, and 308 redirects up to a configurable
//! maximum (default: 20). Each redirect target is validated against SSRF
//! rules before connecting.

use crate::error::{NetError, NetErrorKind};
use crate::preflight::{requires_preflight, run_preflight, PreflightCache};
use crate::transport::HttpTransport;
use crate::{CredentialsMode, RedirectMode, Request, RequestMode, Response};
use bytes::Bytes;

/// Follow redirects for a request, returning the final response.
///
/// - 301, 302, 303: change method to GET and drop body
/// - 307, 308: preserve method and body
/// - Each redirect URL is validated against SSRF rules (unless
///   `allow_private_ips` is true, for testing)
///
/// Honours [`Request::redirect`]:
/// - [`RedirectMode::Follow`] (default): auto-follow as above.
/// - [`RedirectMode::Error`]: return [`NetErrorKind::BadRedirect`]
///   on the first 3xx (no further hops).
/// - [`RedirectMode::Manual`]: return the 3xx response as-is so
///   callers can construct an `OpaqueRedirect`-typed Response
///   (WHATWG Fetch §4.4).
///
/// `preflight_cache` participates in the WHATWG Fetch §4.4 step 14
/// re-preflight on cross-origin CORS redirects: when the redirect
/// target would itself require a preflight ([`requires_preflight`]
/// against the post-redirect probe), an OPTIONS round-trip is
/// dispatched against `next_url` (or short-circuited via the
/// cache) before the actual redirected request is sent.  Pass
/// `None` for embedder-driven paths that never run cors-mode
/// fetches (e.g. [`crate::resource_loader::ResourceLoader`]); the
/// cors-redirect handling silently no-ops for `mode != Cors`
/// requests so a `None` cache is only observable when a misconfigured
/// caller routes a cors-mode request through such a path —
/// `cors_redirect_handle` then fails closed.
///
/// On success the returned [`Response`] carries:
/// - `url_list` = full redirect chain (one entry per hop, original
///   request URL first, [`Response::url`] last) — WHATWG Fetch §3.1.4.
/// - `is_redirect_tainted` = `true` once **any** hop crossed origin
///   per §4.4 step 14.3 (the *redirect-tainted origin flag*).  The
///   classifier in `crates/script/elidex-js/src/vm/host/cors.rs`
///   reads this so a chain that crossed origin even once is routed
///   through the cors path even when the final URL happens to be
///   same-origin with the initiator.
///
/// The second tuple element is the post-chain credentials mode —
/// an `Include` that crossed origin gets downgraded to `SameOrigin`
/// per §4.4 step 14.5.  [`crate::NetClient::send`] threads this
/// through the cookie-storage gate.
///
/// # Limitations (M2-1)
///
/// Cookies are not re-attached or stored on intermediate redirect hops.
/// `Set-Cookie` headers from 3xx responses are ignored, and the cookie
/// jar is not consulted for the redirected URL's domain. This means
/// authentication flows that set cookies during redirects (e.g. OAuth)
/// may not work correctly. The caller (`NetClient::send`) stores
/// cookies only from the final response.
///
/// Full per-hop cookie handling requires passing the `CookieJar` into
/// this function, which is a future improvement (Phase 3).
pub async fn follow_redirects(
    transport: &HttpTransport,
    mut request: Request,
    max_redirects: u32,
    preflight_cache: Option<&PreflightCache>,
    cancel: Option<&crate::CancelHandle>,
) -> Result<(Response, CredentialsMode), NetError> {
    let skip_ssrf = transport.config().allow_private_ips;
    let mut redirects = 0u32;
    let redirect_mode = request.redirect;
    // Accumulate the spec's "request URL list" (§3.1.4) across
    // hops.  The transport stamps a single-URL list onto each
    // hop's response; we override the *final* response's
    // `url_list` with the full chain so callers see one entry per
    // hop.  Seeded with the original request URL so even a
    // single-hop (no-redirect) response surfaces a one-element
    // list rather than the transport's identical clone.
    let mut url_list: Vec<url::Url> = vec![request.url.clone()];
    // Redirect-tainted origin flag (§4.4 step 14.3) — `true` once
    // any hop crossed origin.  Persists across subsequent hops so
    // a chain that briefly returned to the initiator origin still
    // surfaces tainted=true to the classifier.
    let mut tainted = false;

    loop {
        let mut response = transport.send(&request, cancel).await?;

        if !is_redirect(response.status) {
            response.url_list = url_list;
            response.is_redirect_tainted = tainted;
            response.credentialed_network = request.credentials == CredentialsMode::Include;
            return Ok((response, request.credentials));
        }
        // Honour the request's redirect mode (WHATWG Fetch §5.3).
        // `Manual` returns the 3xx as-is for opaque-redirect
        // Response wrapping; `Error` short-circuits so the JS
        // path surfaces a `TypeError("Failed to fetch")`.
        match redirect_mode {
            RedirectMode::Follow => {}
            RedirectMode::Manual => {
                response.url_list = url_list;
                response.is_redirect_tainted = tainted;
                response.credentialed_network = request.credentials == CredentialsMode::Include;
                return Ok((response, request.credentials));
            }
            RedirectMode::Error => {
                return Err(NetError::new(
                    NetErrorKind::BadRedirect,
                    format!(
                        "{} redirect blocked by request.redirect=error",
                        response.status
                    ),
                ));
            }
        }
        if redirects >= max_redirects {
            return Err(NetError::new(
                NetErrorKind::TooManyRedirects,
                format!("exceeded {max_redirects} redirects"),
            ));
        }

        redirects += 1;

        let next_url = resolve_next_url(&request, &response, skip_ssrf)?;
        let (method, body, headers) = prepare_next_hop_request(&request, &response, &next_url);

        let credentials = cors_redirect_handle(
            transport,
            preflight_cache,
            &request,
            &next_url,
            &method,
            &headers,
            &body,
            cancel,
        )
        .await?;

        // Update the redirect-tainted flag *after* any cors-mode
        // gating so a §4.4 step 14 fail-closed branch doesn't leak
        // a partially-tainted state through a returned error.
        if !is_same_origin(&request.url, &next_url) {
            tainted = true;
        }
        url_list.push(next_url.clone());

        request = Request {
            method,
            headers,
            url: next_url,
            body,
            credentials,
            ..request
        };
    }
}

/// Resolve the redirect target URL from a 3xx response's
/// `Location` header against the current request URL, then run
/// SSRF revalidation on the resolved URL (skipped when the
/// transport is configured for private-IP testing).  Returns a
/// `NetError` of kind `Other` when no `Location` is present and
/// `InvalidUrl` for unparseable Locations.
fn resolve_next_url(
    request: &Request,
    response: &Response,
    skip_ssrf: bool,
) -> Result<url::Url, NetError> {
    let location = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("location"))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| {
            NetError::new(
                NetErrorKind::Other,
                format!("{} redirect without Location header", response.status),
            )
        })?;
    let next_url = request.url.join(&location).map_err(|e| {
        NetError::with_source(
            NetErrorKind::InvalidUrl,
            format!("invalid redirect URL: {location}"),
            e,
        )
    })?;
    // SSRF re-validation on the redirect target (defense-in-depth).
    // This is a URL-level check only — the real DNS-level guard is
    // in `Connector::resolve_and_validate()`, which validates
    // resolved IPs.
    if !skip_ssrf {
        elidex_plugin::url_security::validate_url(&next_url)?;
    }
    Ok(next_url)
}

/// Build the method / body / headers triple for the redirected
/// request per RFC 9110 §15.4:
/// - 303: always change to GET (and drop body)
/// - 301, 302: change POST to GET (browser behaviour), preserve
///   other methods
/// - 307, 308: preserve method and body
///
/// Cross-origin redirects strip credential headers
/// ([`filter_headers_for_redirect`]), and method-changing
/// redirects additionally drop request-body headers.
fn prepare_next_hop_request(
    request: &Request,
    response: &Response,
    next_url: &url::Url,
) -> (String, Bytes, Vec<(String, String)>) {
    let changes_method = matches!(
        (response.status, request.method.as_str()),
        (303, _) | (301 | 302, "POST")
    );
    let (method, body) = if changes_method {
        ("GET".to_string(), Bytes::new())
    } else {
        (request.method.clone(), request.body.clone())
    };
    let mut headers = filter_headers_for_redirect(&request.headers, &request.url, next_url);
    if changes_method {
        headers.retain(|(k, _)| {
            let lower = k.to_ascii_lowercase();
            lower != "content-type"
                && lower != "content-length"
                && lower != "content-encoding"
                && lower != "transfer-encoding"
        });
    }
    (method, body, headers)
}

/// Apply WHATWG Fetch §4.4 step 14 paired-infra for a
/// `mode = Cors` cross-origin redirect:
///
/// 1. Detect whether the redirected request would itself be
///    "non-simple" (per [`requires_preflight`]).  If so,
///    re-dispatch a preflight against `next_url` via
///    [`run_preflight`] (or short-circuit via the cache).  A
///    failed preflight is surfaced as
///    [`NetErrorKind::CorsBlocked`] so the JS-side fetch rejects
///    with `TypeError("Failed to fetch")`.
/// 2. Downgrade `Include` credentials to `SameOrigin` so the
///    redirected hop doesn't surface cookies / Authorization
///    picked up from the new origin's cookie jar (§4.4 step 14.5).
///
/// `mode != Cors` paths short-circuit with the input credentials
/// so embedder-driven loads (no `preflight_cache`) skip the §4.8
/// machinery entirely.
#[allow(clippy::too_many_arguments)] // §4.4 step 14 paired-infra inputs — bundling them into a struct adds boilerplate without clarifying the per-call data flow; the helper has exactly one call site.
async fn cors_redirect_handle(
    transport: &HttpTransport,
    preflight_cache: Option<&PreflightCache>,
    request: &Request,
    next_url: &url::Url,
    method: &str,
    headers: &[(String, String)],
    body: &Bytes,
    cancel: Option<&crate::CancelHandle>,
) -> Result<CredentialsMode, NetError> {
    if request.mode != RequestMode::Cors {
        return Ok(request.credentials);
    }
    // §4.4 step 14.5 — `Include` downgrades to `SameOrigin` only
    // when the **redirect hop itself** crosses origin (the
    // contract is hop-to-hop, not initiator-to-target).  A
    // same-origin hop preserves the credentials mode unchanged.
    let hop_crosses_origin = !is_same_origin(&request.url, next_url);
    let post_redirect_credentials =
        if hop_crosses_origin && request.credentials == CredentialsMode::Include {
            CredentialsMode::SameOrigin
        } else {
            request.credentials
        };
    // Probe representing the request that would actually go to
    // `next_url` after method / header / body / credentials
    // adjustments.  Used both for `requires_preflight` detection
    // and as the input to `build_preflight_request` inside
    // `run_preflight`.  Built with the **post-redirect**
    // credentials so `validate_preflight_response` doesn't apply
    // the strict credentialed branch (`ACAO: *` rejected,
    // `ACAC: true` required) when the actual redirected request
    // will go out without credentials (Copilot R2).
    let probe = Request {
        method: method.to_string(),
        headers: headers.to_vec(),
        url: next_url.clone(),
        body: body.clone(),
        origin: request.origin.clone(),
        redirect: request.redirect,
        credentials: post_redirect_credentials,
        mode: request.mode,
    };
    // §4.4 step 14.4 — re-issue preflight whenever the redirected
    // request itself is non-simple, regardless of whether the
    // *hop* crossed origin (Copilot R3): the §4.8 preflight cache
    // key is per-URL, so a hop that stays within the cross-origin
    // server but lands on a *different URL* than the original
    // preflight target still needs its own OPTIONS round-trip.
    // `requires_preflight(&probe)` already gates on initiator-vs-
    // probe.url same-origin, so genuinely same-origin landings
    // (relative to the initiator) correctly skip preflight.
    if requires_preflight(&probe) {
        // Without a cache reference (e.g. embedder-driven
        // `ResourceLoader` paths) we must fail closed since
        // dispatching a preflight without populating a cache
        // would silently re-OPTIONS on every same-target redirect
        // — a perf bug at minimum, and a contract violation
        // since the embedder has no way to scope or reset the
        // implicit cache.  In practice such callers never set
        // `mode = Cors`, so this branch is only reached for
        // misconfigured callers.
        let cache = preflight_cache.ok_or_else(|| {
            NetError::new(
                NetErrorKind::CorsBlocked,
                "cors-redirect preflight: cors-mode request routed through a path without a preflight cache",
            )
        })?;
        run_preflight(transport, cache, &probe, cancel).await?;
    }
    Ok(post_redirect_credentials)
}

/// Check if a status code is a redirect.
fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Filter headers for redirect — strip sensitive headers on cross-origin.
///
/// Per RFC 9110 §15.4, `Authorization`, `Cookie`, `Proxy-Authorization`,
/// and `Referer` headers are stripped when the redirect target differs in
/// origin (scheme, host, or port) from the original request.
fn filter_headers_for_redirect(
    headers: &[(String, String)],
    original_url: &url::Url,
    redirect_url: &url::Url,
) -> Vec<(String, String)> {
    if is_same_origin(original_url, redirect_url) {
        return headers.to_vec();
    }
    // Cross-origin: strip credentials
    headers
        .iter()
        .filter(|(k, _)| {
            let lower = k.to_ascii_lowercase();
            lower != "authorization"
                && lower != "cookie"
                && lower != "proxy-authorization"
                && lower != "referer"
        })
        .cloned()
        .collect()
}

/// Check if two URLs share the same origin (scheme + host + port).
fn is_same_origin(a: &url::Url, b: &url::Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_redirect_status() {
        assert!(is_redirect(301));
        assert!(is_redirect(302));
        assert!(is_redirect(303));
        assert!(is_redirect(307));
        assert!(is_redirect(308));
        assert!(!is_redirect(200));
        assert!(!is_redirect(404));
        assert!(!is_redirect(500));
    }

    #[tokio::test]
    async fn head_301_preserves_method() {
        use crate::transport::TransportConfig;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        tokio::spawn(async move {
            // First request: 301 redirect
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.starts_with("HEAD"), "expected HEAD request, got: {req}");
            let response = format!(
                "HTTP/1.1 301 Moved\r\nLocation: http://127.0.0.1:{port}/dest\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            drop(stream);

            // Second request: should still be HEAD (not GET)
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(
                req.starts_with("HEAD"),
                "expected HEAD preserved, got: {req}"
            );
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response).await.unwrap();
        });

        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        });

        let request = Request {
            method: "HEAD".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/src")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let (response, _) = follow_redirects(&transport, request, 20, None, None)
            .await
            .unwrap();
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn redirect_to_private_ip_blocked() {
        use crate::transport::TransportConfig;
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server that redirects to a private IP
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            let response = "HTTP/1.1 301 Moved\r\nLocation: http://10.0.0.1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string();
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        // Note: allow_private_ips is true, so the URL-level SSRF check
        // in follow_redirects is skipped. The DNS-level SSRF guard in
        // Connector::resolve_and_validate() is also skipped. The error
        // here comes from TCP connection failure to 10.0.0.1, not from
        // SSRF blocking. The actual SSRF protection on redirect targets
        // is tested below in redirect_ssrf_validation_unit_test.
        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            connect_timeout: std::time::Duration::from_secs(1),
            request_timeout: std::time::Duration::from_secs(2),
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let result = follow_redirects(&transport, request, 20, None, None).await;
        assert!(result.is_err());
    }

    /// Verify that `validate_url` correctly blocks private-IP redirect
    /// targets at the URL level (independently of `follow_redirects`).
    #[test]
    fn redirect_ssrf_validation_unit_test() {
        use crate::error::NetErrorKind;

        // Private IPv4
        let url = url::Url::parse("http://10.0.0.1/secret").unwrap();
        let err = elidex_plugin::url_security::validate_url(&url).unwrap_err();
        assert_eq!(Into::<NetError>::into(err).kind, NetErrorKind::SsrfBlocked);

        // Loopback
        let url = url::Url::parse("http://127.0.0.1/secret").unwrap();
        assert!(elidex_plugin::url_security::validate_url(&url).is_err());

        // Public IP should pass
        let url = url::Url::parse("http://93.184.216.34/").unwrap();
        assert!(elidex_plugin::url_security::validate_url(&url).is_ok());
    }

    #[tokio::test]
    async fn follow_redirect_301() {
        use crate::transport::TransportConfig;
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        // Spawn server: first request → 301, second request → 200
        tokio::spawn(async move {
            // First request: redirect
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            let response = format!(
                "HTTP/1.1 301 Moved\r\nLocation: http://127.0.0.1:{port}/dest\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            drop(stream);

            // Second request: final response
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(response).await.unwrap();
        });

        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/src")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let (response, _) = follow_redirects(&transport, request, 20, None, None)
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"ok");
    }

    #[test]
    fn filter_headers_same_origin_keeps_all() {
        let headers = vec![
            ("authorization".to_string(), "Bearer token".to_string()),
            ("accept".to_string(), "text/html".to_string()),
        ];
        let from = url::Url::parse("https://example.com/a").unwrap();
        let to = url::Url::parse("https://example.com/b").unwrap();
        let filtered = filter_headers_for_redirect(&headers, &from, &to);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_headers_cross_origin_strips_credentials() {
        let headers = vec![
            ("authorization".to_string(), "Bearer token".to_string()),
            ("cookie".to_string(), "sid=abc".to_string()),
            ("proxy-authorization".to_string(), "Basic xyz".to_string()),
            ("accept".to_string(), "text/html".to_string()),
        ];
        let from = url::Url::parse("https://example.com/a").unwrap();
        let to = url::Url::parse("https://attacker.com/b").unwrap();
        let filtered = filter_headers_for_redirect(&headers, &from, &to);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "accept");
    }

    #[test]
    fn filter_headers_cross_origin_strips_referer() {
        let headers = vec![
            (
                "referer".to_string(),
                "https://example.com/page".to_string(),
            ),
            ("accept".to_string(), "text/html".to_string()),
        ];
        let from = url::Url::parse("https://example.com/a").unwrap();
        let to = url::Url::parse("https://other.com/b").unwrap();
        let filtered = filter_headers_for_redirect(&headers, &from, &to);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "accept");
    }

    #[test]
    #[allow(clippy::many_single_char_names)] // Short URL variable names (a, b, c, ...) are clear in test context.
    fn is_same_origin_checks_scheme_host_port() {
        let a = url::Url::parse("https://example.com/a").unwrap();
        let b = url::Url::parse("https://example.com/b").unwrap();
        assert!(is_same_origin(&a, &b));

        let c = url::Url::parse("http://example.com/a").unwrap();
        assert!(!is_same_origin(&a, &c)); // different scheme

        let d = url::Url::parse("https://other.com/a").unwrap();
        assert!(!is_same_origin(&a, &d)); // different host

        let e = url::Url::parse("https://example.com:8443/a").unwrap();
        assert!(!is_same_origin(&a, &e)); // different port
    }

    /// Spawn a TCP listener that always answers with the supplied
    /// raw bytes on the first accepted connection.  Returns the
    /// bound port.  Used by the redirect-mode tests below.
    async fn spawn_one_shot(response_bytes: &'static [u8]) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            stream.write_all(response_bytes).await.unwrap();
        });
        port
    }

    #[tokio::test]
    async fn redirect_mode_error_rejects_3xx() {
        use crate::transport::TransportConfig;
        let port = spawn_one_shot(
            b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        });
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            redirect: RedirectMode::Error,
            ..Default::default()
        };
        let err = follow_redirects(&transport, request, 20, None, None)
            .await
            .unwrap_err();
        assert_eq!(err.kind, NetErrorKind::BadRedirect);
    }

    #[tokio::test]
    async fn redirect_mode_manual_returns_3xx_unfollowed() {
        use crate::transport::TransportConfig;
        let port = spawn_one_shot(
            b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        });
        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{port}/")).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            redirect: RedirectMode::Manual,
            ..Default::default()
        };
        let (response, _) = follow_redirects(&transport, request, 20, None, None)
            .await
            .unwrap();
        assert_eq!(response.status, 302);
    }
}
