//! SSE connection layer — raw TCP/TLS, HTTP request building, response
//! validation, redirect following, and CORS checking.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

/// Combined async read+write trait for SSE stream abstraction (TLS or plain TCP).
pub(super) trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncReadWrite for T {}

/// Error classification for SSE connection attempts.
pub(super) enum SseConnectError {
    /// Fatal: HTTP non-200 or wrong Content-Type. Don't reconnect.
    Fatal(String),
    /// Recoverable: network error. Auto-reconnect after retry delay.
    Recoverable(String),
}

/// Result of reading an SSE HTTP response status and headers.
enum SseResponseResult {
    /// 200 OK with valid `text/event-stream` content type.
    Ok,
    /// Redirect (301, 302, 303, 307, 308) with a `Location` URL.
    Redirect(String),
}

/// Result of a single SSE connection attempt (before redirect resolution).
pub(super) enum SseConnectResult {
    /// Successfully connected with valid 200 + text/event-stream response.
    Connected(BufReader<Box<dyn AsyncReadWrite>>),
    /// Server responded with a redirect; follow the Location.
    Redirect(String),
}

/// Maximum number of HTTP redirects to follow (matches elidex-net redirect.rs).
const MAX_SSE_REDIRECTS: u32 = 20;

/// Connect to an SSE endpoint and return a buffered reader over the body stream.
///
/// Establishes a raw TCP (or TLS for HTTPS) connection, sends an HTTP/1.1 GET
/// request, validates the response status and `Content-Type`, follows redirects
/// (up to 20 hops), and returns a `BufReader` positioned at the start of the
/// response body for incremental line-by-line reading.
///
/// If `origin` is `Some`, an `Origin` header is sent and CORS validation is
/// performed on the final (non-redirect) response.
pub(super) async fn connect_sse_stream(
    url: &url::Url,
    headers: &[(String, String)],
    origin: Option<&str>,
) -> Result<BufReader<Box<dyn AsyncReadWrite>>, SseConnectError> {
    let mut current_url = url.clone();

    for _ in 0..MAX_SSE_REDIRECTS {
        let reader = connect_sse_single(&current_url, headers, origin).await?;
        match reader {
            SseConnectResult::Connected(reader) => return Ok(reader),
            SseConnectResult::Redirect(location) => {
                // Resolve relative Location against the current URL.
                current_url = current_url.join(&location).map_err(|e| {
                    SseConnectError::Fatal(format!(
                        "SSE: invalid redirect Location '{location}': {e}"
                    ))
                })?;
                // Re-validate: redirect target must be http/https (not ws/ftp/etc.)
                // and must pass SSRF checks.
                match current_url.scheme() {
                    "http" | "https" => {}
                    s => {
                        return Err(SseConnectError::Fatal(format!(
                            "SSE: redirect to unsupported scheme '{s}'"
                        )));
                    }
                }
                if let Err(e) = elidex_plugin::url_security::validate_url(&current_url) {
                    return Err(SseConnectError::Fatal(format!(
                        "SSE: redirect target blocked: {e}"
                    )));
                }
            }
        }
    }

    Err(SseConnectError::Fatal(
        "SSE: too many redirects (> 20)".to_string(),
    ))
}

/// Perform a single SSE connection attempt (no redirect following).
async fn connect_sse_single(
    url: &url::Url,
    headers: &[(String, String)],
    origin: Option<&str>,
) -> Result<SseConnectResult, SseConnectError> {
    let scheme = url.scheme();
    let host = url
        .host_str()
        .ok_or_else(|| SseConnectError::Recoverable("SSE: URL has no host".to_string()))?;
    let default_port: u16 = if scheme == "https" { 443 } else { 80 };
    let port = url.port().unwrap_or(default_port);

    // Connect TCP using (host, port) tuple — avoids IPv6 bracket issues with
    // format!("{host}:{port}") since ToSocketAddrs handles bare IPv6 addresses.
    let tcp = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(|e| {
            SseConnectError::Recoverable(format!("SSE: TCP connect to {host}:{port} failed: {e}"))
        })?;

    // SSRF re-validation: check the resolved IP address is not private.
    // This prevents DNS rebinding attacks where a hostname resolves to a
    // private IP after the initial URL validation at the JS layer.
    if let Ok(peer) = tcp.peer_addr() {
        if elidex_plugin::url_security::is_private_ip(peer.ip()) {
            return Err(SseConnectError::Fatal(format!(
                "SSE: resolved IP {peer} is a private address (SSRF blocked)"
            )));
        }
    }

    let req_buf = build_sse_request(url, host, headers, origin);

    // Build the stream: TLS for https, plain TCP otherwise.
    let stream: Box<dyn AsyncReadWrite> = if scheme == "https" {
        let tls_config = crate::tls::build_tls_config();
        let mut sse_tls_config = (*tls_config).clone();
        sse_tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let connector = tokio_rustls::TlsConnector::from(Arc::new(sse_tls_config));
        let server_name = crate::tls::server_name(host).map_err(|e| {
            SseConnectError::Recoverable(format!("SSE: invalid server name {host}: {e}"))
        })?;
        let tls_stream = connector.connect(server_name, tcp).await.map_err(|e| {
            SseConnectError::Recoverable(format!("SSE: TLS handshake with {host} failed: {e}"))
        })?;
        Box::new(tls_stream)
    } else {
        Box::new(tcp)
    };

    // Send HTTP request and validate response (shared for TLS and plain TCP).
    send_and_validate_sse(stream, &req_buf, origin).await
}

/// Send the HTTP request on a connected stream, validate the response, and
/// return either a connected body reader or a redirect location.
async fn send_and_validate_sse(
    mut stream: Box<dyn AsyncReadWrite>,
    req_buf: &str,
    origin: Option<&str>,
) -> Result<SseConnectResult, SseConnectError> {
    stream.write_all(req_buf.as_bytes()).await.map_err(|e| {
        SseConnectError::Recoverable(format!("SSE: failed to send HTTP request: {e}"))
    })?;
    stream
        .flush()
        .await
        .map_err(|e| SseConnectError::Recoverable(format!("SSE: flush failed: {e}")))?;

    // Use a single BufReader with the target capacity for both header validation
    // and body streaming. Avoids double-buffering (BufReader<BufReader<Stream>>)
    // which would cause data loss from the inner buffer being invisible to the outer.
    let mut reader = BufReader::with_capacity(65536, stream);
    match validate_sse_response(&mut reader, origin)
        .await
        .map_err(SseConnectError::Fatal)?
    {
        SseResponseResult::Ok => Ok(SseConnectResult::Connected(reader)),
        SseResponseResult::Redirect(loc) => Ok(SseConnectResult::Redirect(loc)),
    }
}

/// Build the raw HTTP/1.1 GET request string for an SSE connection.
///
/// If `origin` is `Some`, an `Origin` header is included for CORS.
fn build_sse_request(
    url: &url::Url,
    host: &str,
    headers: &[(String, String)],
    origin: Option<&str>,
) -> String {
    // Build path + query without fragment (fragments must not be sent in HTTP requests).
    let mut path_and_query = url.path().to_owned();
    if let Some(query) = url.query() {
        path_and_query.push('?');
        path_and_query.push_str(query);
    }
    // Build Host header: bracket IPv6 addresses, include port for non-default ports.
    let default_port: u16 = if url.scheme() == "https" { 443 } else { 80 };
    let host_header = if host.contains(':') {
        // IPv6 address — wrap in brackets per RFC 2732.
        match url.port() {
            Some(p) if p != default_port => format!("[{host}]:{p}"),
            _ => format!("[{host}]"),
        }
    } else {
        match url.port() {
            Some(p) if p != default_port => format!("{host}:{p}"),
            _ => host.to_string(),
        }
    };
    let mut req_buf = format!(
        "GET {path_and_query} HTTP/1.1\r\nHost: {host_header}\r\nAccept: text/event-stream\r\nCache-Control: no-cache\r\n"
    );
    if let Some(orig) = origin {
        use std::fmt::Write;
        let _ = write!(req_buf, "Origin: {orig}\r\n");
    }
    for (name, value) in headers {
        use std::fmt::Write;
        // Prevent header injection by stripping CR/LF from header values.
        let sanitized = value.replace(['\r', '\n'], "");
        let _ = write!(req_buf, "{name}: {sanitized}\r\n");
    }
    req_buf.push_str("\r\n");
    req_buf
}

/// Read and validate HTTP response status line and headers from a buffered
/// reader. Returns `Ok` result classifying the response as success or redirect.
///
/// When `origin` is `Some`, CORS validation is performed: the response must
/// contain `Access-Control-Allow-Origin` matching `*` or the sent origin.
async fn validate_sse_response<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    origin: Option<&str>,
) -> Result<SseResponseResult, String> {
    // Read the response status line.
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .await
        .map_err(|e| format!("SSE: failed to read status line: {e}"))?;

    // Parse status code (e.g. "HTTP/1.1 200 OK\r\n").
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(format!("SSE: invalid status line: {status_line}"));
    }
    let status: u16 = parts[1]
        .parse()
        .map_err(|_| format!("SSE: invalid status code: {}", parts[1]))?;

    let is_redirect = matches!(status, 301 | 302 | 303 | 307 | 308);

    // Read headers until empty line.
    let mut content_type_ok = false;
    let mut location: Option<String> = None;
    let mut acao: Option<String> = None;
    loop {
        let mut header_line = String::new();
        reader
            .read_line(&mut header_line)
            .await
            .map_err(|e| format!("SSE: failed to read header: {e}"))?;
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            let name_lower = name.trim().to_ascii_lowercase();
            match name_lower.as_str() {
                "content-type" => {
                    let mime = value.trim().split(';').next().unwrap_or("").trim();
                    if mime.eq_ignore_ascii_case("text/event-stream") {
                        content_type_ok = true;
                    }
                }
                "location" => {
                    location = Some(value.trim().to_string());
                }
                "access-control-allow-origin" => {
                    acao = Some(value.trim().to_string());
                }
                _ => {}
            }
        }
    }

    // Handle redirects.
    if is_redirect {
        if let Some(loc) = location {
            return Ok(SseResponseResult::Redirect(loc));
        }
        return Err(format!(
            "SSE: HTTP {status} redirect without Location header"
        ));
    }

    if status != 200 {
        return Err(format!("SSE: HTTP {status} (expected 200)"));
    }

    if !content_type_ok {
        return Err("SSE: response Content-Type is not text/event-stream".to_string());
    }

    // CORS validation: if Origin was sent, check Access-Control-Allow-Origin.
    if let Some(sent_origin) = origin {
        match acao {
            Some(ref allowed) if allowed == "*" || allowed == sent_origin => {
                // CORS check passed.
            }
            _ => {
                return Err(format!(
                    "SSE: CORS check failed — Access-Control-Allow-Origin does not match origin '{sent_origin}'"
                ));
            }
        }
    }

    Ok(SseResponseResult::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_sse_request_basic() {
        let url = url::Url::parse("http://example.com/events?q=1").unwrap();
        let req = build_sse_request(&url, "example.com", &[], None);
        assert!(req.starts_with("GET /events?q=1 HTTP/1.1\r\n"));
        assert!(req.contains("Host: example.com\r\n"));
        assert!(req.contains("Accept: text/event-stream\r\n"));
        assert!(!req.contains("Origin:"));
        assert!(req.ends_with("\r\n\r\n"));
    }

    #[test]
    fn build_sse_request_with_extra_headers() {
        let url = url::Url::parse("http://example.com/stream").unwrap();
        let headers = vec![
            ("Last-Event-ID".to_string(), "99".to_string()),
            ("Cookie".to_string(), "session=abc".to_string()),
        ];
        let req = build_sse_request(&url, "example.com", &headers, None);
        assert!(req.contains("Last-Event-ID: 99\r\n"));
        assert!(req.contains("Cookie: session=abc\r\n"));
    }

    #[test]
    fn build_sse_request_with_origin() {
        let url = url::Url::parse("http://example.com/stream").unwrap();
        let req = build_sse_request(&url, "example.com", &[], Some("http://mysite.com"));
        assert!(req.contains("Origin: http://mysite.com\r\n"));
    }

    #[test]
    fn build_sse_request_sanitizes_header_values() {
        let url = url::Url::parse("http://example.com/stream").unwrap();
        let headers = vec![(
            "Last-Event-ID".to_string(),
            "42\r\nX-Injected: yes".to_string(),
        )];
        let req = build_sse_request(&url, "example.com", &headers, None);
        assert!(req.contains("Last-Event-ID: 42X-Injected: yes\r\n"));
        assert!(!req.contains("Last-Event-ID: 42\r\nX-Injected: yes\r\n"));
    }
}
