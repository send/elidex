//! HTTP redirect tracking with SSRF re-validation.
//!
//! Follows 301, 302, 303, 307, and 308 redirects up to a configurable
//! maximum (default: 20). Each redirect target is validated against SSRF
//! rules before connecting.

use crate::error::{NetError, NetErrorKind};
use crate::transport::HttpTransport;
use crate::{Request, Response};
use bytes::Bytes;

/// Follow redirects for a request, returning the final response.
///
/// - 301, 302, 303: change method to GET and drop body
/// - 307, 308: preserve method and body
/// - Each redirect URL is validated against SSRF rules (unless
///   `allow_private_ips` is true, for testing)
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
) -> Result<Response, NetError> {
    let skip_ssrf = transport.config().allow_private_ips;
    let mut redirects = 0u32;

    loop {
        let response = transport.send(&request).await?;

        if !is_redirect(response.status) {
            return Ok(response);
        }
        if redirects >= max_redirects {
            return Err(NetError::new(
                NetErrorKind::TooManyRedirects,
                format!("exceeded {max_redirects} redirects"),
            ));
        }

        redirects += 1;

        // Extract Location header
        let location = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
            .map(|(_, v)| v.clone());

        let location = location.ok_or_else(|| {
            NetError::new(
                NetErrorKind::Other,
                format!("{} redirect without Location header", response.status),
            )
        })?;

        // Resolve relative URL against the current request URL
        let next_url = request.url.join(&location).map_err(|e| {
            NetError::with_source(
                NetErrorKind::InvalidUrl,
                format!("invalid redirect URL: {location}"),
                e,
            )
        })?;

        // SSRF re-validation on the redirect target (defense-in-depth).
        //
        // This is a URL-level check only. The real DNS-level guard is in
        // `Connector::resolve_and_validate()`, which validates resolved IPs.
        if !skip_ssrf {
            elidex_plugin::url_security::validate_url(&next_url)?;
        }

        // Determine method/body for redirected request.
        //
        // RFC 9110 §15.4:
        // - 303: always change to GET
        // - 301, 302: change POST to GET (browser behavior), preserve other methods
        // - 307, 308: preserve method and body
        let changes_method = matches!(
            (response.status, request.method.as_str()),
            (303, _) | (301 | 302, "POST")
        );
        let (method, body) = if changes_method {
            ("GET".to_string(), Bytes::new())
        } else {
            (request.method.clone(), request.body.clone())
        };

        let mut headers = filter_headers_for_redirect(&request.headers, &request.url, &next_url);

        // RFC 9110 §15.4: when changing method to GET, strip request body headers.
        if changes_method {
            headers.retain(|(k, _)| {
                let lower = k.to_ascii_lowercase();
                lower != "content-type"
                    && lower != "content-length"
                    && lower != "content-encoding"
                    && lower != "transfer-encoding"
            });
        }

        request = Request {
            method,
            headers,
            url: next_url,
            body,
        };
    }
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
        };

        let response = follow_redirects(&transport, request, 20).await.unwrap();
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
        };

        let result = follow_redirects(&transport, request, 20).await;
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
        };

        let response = follow_redirects(&transport, request, 20).await.unwrap();
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
}
