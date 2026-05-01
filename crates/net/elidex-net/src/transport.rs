//! HTTP transport layer — sends requests via the connection pool.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;

use crate::error::{NetError, NetErrorKind};
use crate::pool::{ConnectionPool, PooledConnection};
use crate::{Request, Response};

/// HTTP version of a response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/1.0 or HTTP/1.1
    H1,
    /// HTTP/2
    H2,
}

/// Configuration for the HTTP transport.
#[derive(Clone, Debug)]
pub struct TransportConfig {
    /// Maximum connections per origin (default: 6).
    pub max_connections_per_origin: usize,
    /// Maximum total connections across all origins (default: 256).
    pub max_total_connections: usize,
    /// TCP connection timeout (default: 10s).
    pub connect_timeout: Duration,
    /// Overall request timeout (default: 30s).
    pub request_timeout: Duration,
    /// Maximum response body size in bytes (default: 50 MB).
    pub max_response_bytes: usize,
    /// Maximum number of redirects (default: 20).
    pub max_redirects: u32,
    /// User-Agent header value.
    pub user_agent: String,
    /// Allow connections to private/reserved IPs (for testing).
    pub allow_private_ips: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_connections_per_origin: 6,
            max_total_connections: 256,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_response_bytes: 50 * 1024 * 1024,
            max_redirects: 20,
            user_agent: "elidex/0.1".to_string(),
            allow_private_ips: false,
        }
    }
}

/// HTTP transport that sends requests through a connection pool.
pub struct HttpTransport {
    pool: Arc<ConnectionPool>,
    config: TransportConfig,
}

impl std::fmt::Debug for HttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpTransport")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport {
    /// Create a new transport with default configuration.
    pub fn new() -> Self {
        Self::with_config(TransportConfig::default())
    }

    /// Create a new transport with the given configuration.
    pub fn with_config(config: TransportConfig) -> Self {
        let connector =
            crate::connector::Connector::with_config(crate::connector::ConnectorConfig {
                connect_timeout: config.connect_timeout,
                allow_private_ips: config.allow_private_ips,
            });
        let pool = Arc::new(ConnectionPool::with_global_limit(
            connector,
            config.max_connections_per_origin,
            config.max_total_connections,
        ));
        Self { pool, config }
    }

    /// Access the transport configuration.
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Send a single HTTP request (no redirect following).
    ///
    /// `cancel`, when `Some`, lets the caller abort an in-flight
    /// request before its hyper future resolves —
    /// [`crate::CancelHandle::cancel`] drops the future via
    /// `tokio::select!` and the [`crate::FetchHandle`] /
    /// `MAX_CONCURRENT_FETCHES` inflight slot is released
    /// immediately rather than waiting on the underlying network
    /// IO to drain.  Pass the same handle to `run_preflight` and
    /// `follow_redirects` so a single `cancel()` aborts every
    /// hop of a fetch (preflight OPTIONS + main request +
    /// redirect chain).  Pass `None` only for paths that have no
    /// caller-visible abort surface — direct embedder-driven
    /// loads outside the broker fetch dispatcher.
    pub async fn send(
        &self,
        request: &Request,
        cancel: Option<&crate::CancelHandle>,
    ) -> Result<Response, NetError> {
        // Fast-path: caller already cancelled before we even
        // entered the transport.  Skip the connection checkout
        // entirely so cancel-spam workloads don't consume pool
        // slots for guaranteed-to-abort fetches.
        if let Some(c) = cancel {
            if c.is_cancelled() {
                return Err(NetError::new(
                    NetErrorKind::Cancelled,
                    "request cancelled before dispatch",
                ));
            }
        }
        // Validate HTTP method
        validate_method(&request.method)?;

        // Wrap the entire operation (checkout + send + body read) in request_timeout.
        // When `cancel` is provided, `tokio::select!` drops the
        // hyper future the moment cancel fires — the connection
        // pool's `SendGuard` then returns the half-used H1
        // socket as "broken" (closed) so the next checkout can't
        // accidentally read a stale response.
        let url = request.url.clone();
        let send_future = async {
            let conn = self.pool.checkout(&request.url).await?;

            // Build hyper request
            let authority = build_authority(&request.url)?;
            let path_and_query = request.url[url::Position::BeforePath..].to_string();

            let mut builder = HyperRequest::builder()
                .method(request.method.as_str())
                .uri(&path_and_query);

            // Set Host header (including port for non-default ports)
            builder = builder.header("host", &authority);

            // Set User-Agent if not already present
            let has_ua = request
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("user-agent"));
            if !has_ua {
                builder = builder.header("user-agent", &self.config.user_agent);
            }

            // Copy request headers
            for (name, value) in &request.headers {
                builder = builder.header(name.as_str(), value.as_str());
            }

            let body = Full::new(request.body.clone());
            let hyper_req = builder.body(body).map_err(|e| {
                NetError::with_source(NetErrorKind::Other, "failed to build request", e)
            })?;

            let (hyper_resp, version) = self.send_guard(conn, &request.url).send(hyper_req).await?;

            // Collect response
            let status = hyper_resp.status().as_u16();
            let headers: Vec<(String, String)> = hyper_resp
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        String::from_utf8_lossy(v.as_bytes()).into_owned(),
                    )
                })
                .collect();

            // Read body with size limit
            let body = self.read_body(hyper_resp.into_body()).await?;

            Ok(Response {
                status,
                headers,
                body,
                url: request.url.clone(),
                version,
                // WHATWG Fetch §3.1.4 — a request's URL list is
                // always at least one URL (the request URL).  The
                // redirect loop overrides this with the full chain;
                // for direct (non-redirected) sends the single-hop
                // list mirrors the spec contract.
                url_list: vec![request.url.clone()],
                // The transport never sees redirects; only the
                // redirect loop in `redirect::follow_redirects`
                // promotes this to `true` when a chain crosses
                // origin (WHATWG Fetch §4.4 step 14.3).
                is_redirect_tainted: false,
                // Stamp from the actual request's credentials
                // mode so direct `transport.send` callers (paths
                // that bypass the redirect loop) see a correct
                // `credentialed_network` instead of a placeholder
                // `false` (Copilot R3 PR-cors-redirect-preflight).
                // The redirect loop overrides this with the
                // post-redirect value on its final-hop response,
                // so multi-hop chains always end up with the
                // §4.4 step 14.5 downgrade applied.
                credentialed_network: request.credentials == crate::CredentialsMode::Include,
            })
        };
        // Compose: cancel future races against the request future,
        // both wrapped in the request_timeout.  The
        // `cancellation_future` branch is only present when the
        // caller passed a `CancelHandle` — the no-cancel path
        // resolves identically to pre-PR behaviour (single
        // timeout-wrapped future).
        let cancelled_err = || {
            NetError::new(
                NetErrorKind::Cancelled,
                format!("request to {url} cancelled"),
            )
        };
        let result = match cancel {
            Some(c) => tokio::time::timeout(self.config.request_timeout, async {
                tokio::select! {
                    biased;
                    () = c.cancelled() => Err(cancelled_err()),
                    res = send_future => res,
                }
            })
            .await
            .map_err(|_| {
                NetError::new(NetErrorKind::Timeout, format!("request to {url} timed out"))
            })?,
            None => tokio::time::timeout(self.config.request_timeout, send_future)
                .await
                .map_err(|_| {
                    NetError::new(NetErrorKind::Timeout, format!("request to {url} timed out"))
                })?,
        };

        result
    }

    /// Prepare a connection for sending, returning a `SendGuard` that handles
    /// H1 connection return to the pool on drop.
    fn send_guard(&self, conn: PooledConnection, url: &url::Url) -> SendGuard {
        SendGuard {
            conn: Some(conn),
            pool: self.pool.clone(),
            url: url.clone(),
        }
    }

    /// Read and collect response body with size limit.
    async fn read_body(&self, body: hyper::body::Incoming) -> Result<Bytes, NetError> {
        let max = self.config.max_response_bytes;
        let limited = http_body_util::Limited::new(body, max);
        limited
            .collect()
            .await
            .map(http_body_util::Collected::to_bytes)
            .map_err(|e| {
                // Type-safe detection via downcast instead of string matching
                if e.is::<http_body_util::LengthLimitError>() {
                    NetError::new(
                        NetErrorKind::ResponseTooLarge,
                        format!("response body exceeded {max} bytes"),
                    )
                } else {
                    NetError::new(
                        NetErrorKind::Other,
                        format!("failed to read response body: {e}"),
                    )
                }
            })
    }
}

/// Validate that the HTTP method conforms to the token grammar (RFC 9110 §5.6.2).
fn validate_method(method: &str) -> Result<(), NetError> {
    if method.is_empty() {
        return Err(NetError::new(NetErrorKind::Other, "HTTP method is empty"));
    }
    // HTTP token: [!#$%&'*+-.^_`|~0-9A-Za-z]+
    let is_token = method.bytes().all(|b| {
        matches!(b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' |
            b'^' | b'_' | b'`' | b'|' | b'~' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
        )
    });
    if !is_token {
        return Err(NetError::new(
            NetErrorKind::Other,
            format!("invalid HTTP method: {method:?}"),
        ));
    }
    Ok(())
}

/// Build the Host header authority value including port for non-default ports.
fn build_authority(url: &url::Url) -> Result<String, NetError> {
    let host = url
        .host_str()
        .ok_or_else(|| NetError::new(NetErrorKind::InvalidUrl, "URL has no host"))?;
    // IPv6 addresses contain ':' and need brackets in the authority.
    // url::Url::host_str() may already include brackets for IPv6, so
    // only add them if not already present.
    let host_part = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => {
            let is_default = matches!((url.scheme(), port), ("http", 80) | ("https", 443));
            if is_default {
                Ok(host_part)
            } else {
                Ok(format!("{host_part}:{port}"))
            }
        }
        None => Ok(host_part),
    }
}

/// Guard that returns an H1 connection to the pool after use.
///
/// On send, the H1 sender is checked and returned to the pool if still usable.
/// If the sender is consumed without sending (e.g. on error), the `active_h1`
/// counter is decremented on drop.
struct SendGuard {
    conn: Option<PooledConnection>,
    pool: Arc<ConnectionPool>,
    url: url::Url,
}

impl std::fmt::Debug for SendGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendGuard")
            .field("conn", &self.conn)
            .field("url", &self.url.as_str())
            .finish_non_exhaustive()
    }
}

impl SendGuard {
    /// Send the request and return the H1 sender to the pool if still usable.
    ///
    /// Holds the connection in `self.conn` *across* the
    /// `send_request().await` so that if the future is dropped
    /// (e.g. via [`crate::CancelHandle`] or the request_timeout),
    /// `Drop::drop` still observes `Some(_)` and routes through
    /// `release_h1()` — without this, a cancellation mid-await
    /// leaks the per-origin `active_h1` counter and eventually
    /// exhausts `max_connections_per_origin` (Copilot R1).
    async fn send(
        mut self,
        req: HyperRequest<Full<Bytes>>,
    ) -> Result<(hyper::Response<hyper::body::Incoming>, HttpVersion), NetError> {
        let resp = match self.conn.as_mut() {
            Some(PooledConnection::H1(sender)) => sender.send_request(req).await.map_err(|e| {
                NetError::with_source(NetErrorKind::Other, "HTTP/1.1 request failed", e)
            })?,
            Some(PooledConnection::H2(sender)) => sender.send_request(req).await.map_err(|e| {
                NetError::with_source(NetErrorKind::Other, "HTTP/2 request failed", e)
            })?,
            None => {
                return Err(NetError::new(
                    NetErrorKind::Other,
                    "connection already consumed",
                ));
            }
        };
        // Success: take ownership for checkin (H1) or to drop (H2).
        // Drop won't fire `release_h1` because `self.conn` is now
        // `None` — that's the signal the request completed cleanly.
        match self.conn.take() {
            Some(PooledConnection::H1(sender)) => {
                self.pool.checkin(&self.url, sender);
                Ok((resp, HttpVersion::H1))
            }
            Some(PooledConnection::H2(_)) => Ok((resp, HttpVersion::H2)),
            None => unreachable!("conn was Some before send_request"),
        }
    }
}

impl Drop for SendGuard {
    fn drop(&mut self) {
        // If the connection was not consumed (error/cancel path),
        // decrement active_h1.  H2 senders share a single
        // multiplexed connection so we never per-request decrement
        // them — the H2 driver's lifetime governs the slot.
        if let Some(PooledConnection::H1(_)) = self.conn.take() {
            self.pool.release_h1(&self.url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_config_defaults() {
        let config = TransportConfig::default();
        assert_eq!(config.max_connections_per_origin, 6);
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.max_response_bytes, 50 * 1024 * 1024);
        assert_eq!(config.max_redirects, 20);
        assert_eq!(config.user_agent, "elidex/0.1");
        assert!(!config.allow_private_ips);
    }

    #[test]
    fn http_version_eq() {
        assert_eq!(HttpVersion::H1, HttpVersion::H1);
        assert_ne!(HttpVersion::H1, HttpVersion::H2);
    }

    /// Regression for Copilot R1 (transport.rs SendGuard
    /// conn-leak on cancel): when the in-flight `send_request`
    /// future is dropped (cancel/timeout), `SendGuard::Drop` must
    /// still call `pool.release_h1()` so the per-origin
    /// `active_h1` counter is decremented.  Pre-fix the original
    /// `self.conn.take()` ran *before* `await`, so a dropped
    /// future left `self.conn = None` and `Drop` was a no-op,
    /// permanently leaking one slot of the per-origin cap.
    ///
    /// Test shape: cap a transport at `max_connections_per_origin
    /// = 1`, fire and cancel a request against a never-replying
    /// server, then fire a second request at the same origin.
    /// Pre-fix the second request fails immediately with
    /// `"connection limit reached"`; post-fix it proceeds (the
    /// listener never `accept`s the second connect, so we
    /// short-circuit via a small connect_timeout once we've
    /// observed the slot was actually released — proving the
    /// pre-fix early-error path is gone).
    #[tokio::test]
    async fn send_drop_releases_h1_slot_on_cancel() {
        // Stalling listener: accept once, never reply.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        // Hold the listener so additional connects queue at the
        // OS rather than ECONNREFUSED.
        let _stall = listener;

        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            max_connections_per_origin: 1,
            connect_timeout: Duration::from_millis(200),
            // Keep the second request short — it goes to the
            // same stalling listener so the 200 OK never arrives,
            // but we only need to observe the *kind* of error
            // (Timeout, not connection-limit) to prove the slot
            // was released.
            request_timeout: Duration::from_millis(800),
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        // First request: cancel after a brief delay so the
        // worker reaches `send_request().await` (the borrow path
        // that pre-fix mishandled).
        let cancel = crate::CancelHandle::new();
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });
        let err = transport
            .send(&request, Some(&cancel))
            .await
            .expect_err("cancelled request must not return Ok");
        assert_eq!(err.kind, NetErrorKind::Cancelled);

        // Second request: must NOT immediately fail with
        // "connection limit reached".  Pre-fix this fired right
        // away because the pool's `active_h1` was still 1.
        // Post-fix the slot is released by SendGuard::Drop, so
        // we proceed to a fresh connect attempt against the
        // listener — which we short-circuit via the small
        // connect_timeout to keep the test fast.
        let err2 = transport
            .send(&request, None)
            .await
            .expect_err("stalling listener never replies; expect timeout, not pool exhaustion");
        assert!(
            !format!("{err2:#}").contains("connection limit reached"),
            "per-origin slot leaked across cancellation (pre-fix bug): {err2}"
        );
        // Belt-and-suspenders: the only acceptable error here is
        // a timeout (the listener never replies).  If we get any
        // other kind, something else regressed.
        assert_eq!(err2.kind, NetErrorKind::Timeout, "unexpected: {err2:#}");
    }

    #[tokio::test]
    async fn send_to_local_server() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a minimal HTTP server
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            let response =
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello";
            stream.write_all(response).await.unwrap();
        });

        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: true,
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let response = transport.send(&request, None).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.as_ref(), b"hello");
        assert_eq!(response.version, HttpVersion::H1);
    }

    #[test]
    fn build_authority_ipv4() {
        let url = url::Url::parse("http://example.com/path").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "example.com");
    }

    #[test]
    fn build_authority_ipv4_non_default_port() {
        let url = url::Url::parse("http://example.com:8080/path").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "example.com:8080");
    }

    #[test]
    fn build_authority_ipv6() {
        let url = url::Url::parse("http://[::1]:8080/path").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "[::1]:8080");
    }

    #[test]
    fn build_authority_ipv6_default_port() {
        let url = url::Url::parse("http://[::1]/path").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "[::1]");
    }

    #[tokio::test]
    async fn ssrf_blocks_private_ip() {
        let transport = HttpTransport::with_config(TransportConfig {
            allow_private_ips: false,
            ..Default::default()
        });

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse("http://127.0.0.1:1/").unwrap(),
            headers: Vec::new(),
            body: Bytes::new(),
            ..Default::default()
        };

        let result = transport.send(&request, None).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, NetErrorKind::SsrfBlocked);
    }
}
