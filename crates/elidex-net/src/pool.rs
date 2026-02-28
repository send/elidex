//! Connection pooling per origin (scheme + host + port).
//!
//! - HTTP/1.1: up to N idle connections per origin (default: 6)
//! - HTTP/2: single multiplexed connection per origin via `SendRequest` clone

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::{http1, http2};

use hyper_util::rt::TokioIo;

use crate::connector::{Connector, StreamWrapper};
use crate::error::{NetError, NetErrorKind};

/// Idle connection timeout (90 seconds).
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Origin key for connection pooling.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OriginKey {
    scheme: String,
    host: String,
    port: u16,
}

impl OriginKey {
    fn from_url(url: &url::Url) -> Result<Self, NetError> {
        let scheme = url.scheme().to_ascii_lowercase();
        let host = url
            .host_str()
            .ok_or_else(|| NetError::new(NetErrorKind::InvalidUrl, "URL has no host"))?
            .to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "https" => 443,
            "http" => 80,
            _ => {
                return Err(NetError::new(
                    NetErrorKind::InvalidUrl,
                    format!("unsupported scheme: {scheme}"),
                ));
            }
        };
        let port = url.port().unwrap_or(default_port);
        Ok(Self { scheme, host, port })
    }

    fn use_tls(&self) -> bool {
        self.scheme == "https"
    }
}

/// A connection that can send HTTP requests.
///
/// H1 and H2 senders are opaque to external code; `Debug` shows the variant only.
pub enum PooledConnection {
    /// HTTP/1.1 connection.
    H1(http1::SendRequest<Full<Bytes>>),
    /// HTTP/2 connection (clone of a shared sender).
    H2(http2::SendRequest<Full<Bytes>>),
}

impl std::fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::H1(_) => f.write_str("PooledConnection::H1(..)"),
            Self::H2(_) => f.write_str("PooledConnection::H2(..)"),
        }
    }
}

/// An idle HTTP/1.1 connection with timestamp.
struct IdleH1 {
    sender: http1::SendRequest<Full<Bytes>>,
    idle_since: Instant,
}

/// Per-origin pool state.
struct OriginPool {
    idle_h1: Vec<IdleH1>,
    active_h1: usize,
    h2_sender: Option<http2::SendRequest<Full<Bytes>>>,
}

impl OriginPool {
    fn new() -> Self {
        Self {
            idle_h1: Vec::new(),
            active_h1: 0,
            h2_sender: None,
        }
    }

    /// Evict idle connections older than `IDLE_TIMEOUT` and dead H2 senders.
    fn evict_stale(&mut self) {
        let now = Instant::now();
        self.idle_h1
            .retain(|c| now.duration_since(c.idle_since) < IDLE_TIMEOUT);
        // Clean up dead H2 sender
        if let Some(ref sender) = self.h2_sender {
            if !sender.is_ready() {
                self.h2_sender = None;
            }
        }
    }
}

/// Default maximum total connections across all origins.
const DEFAULT_MAX_TOTAL_CONNECTIONS: usize = 256;

/// Connection pool managing HTTP/1.1 and HTTP/2 connections per origin.
pub struct ConnectionPool {
    connector: Connector,
    max_per_origin: usize,
    pools: Mutex<HashMap<OriginKey, OriginPool>>,
    global_semaphore: tokio::sync::Semaphore,
}

impl std::fmt::Debug for ConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pool_count = self.pools.lock().map(|p| p.len()).unwrap_or(0);
        f.debug_struct("ConnectionPool")
            .field("max_per_origin", &self.max_per_origin)
            .field("origins", &pool_count)
            .finish_non_exhaustive()
    }
}

impl ConnectionPool {
    /// Create a new pool with the given connector and per-origin limit.
    pub fn new(connector: Connector, max_per_origin: usize) -> Self {
        Self::with_global_limit(connector, max_per_origin, DEFAULT_MAX_TOTAL_CONNECTIONS)
    }

    /// Create a new pool with explicit global connection limit.
    pub fn with_global_limit(
        connector: Connector,
        max_per_origin: usize,
        max_total: usize,
    ) -> Self {
        Self {
            connector,
            max_per_origin,
            pools: Mutex::new(HashMap::new()),
            global_semaphore: tokio::sync::Semaphore::new(max_total),
        }
    }

    /// Checkout a connection for the given URL.
    ///
    /// Returns a reused idle connection if available, or creates a new one.
    /// Respects both per-origin and global connection limits.
    pub async fn checkout(&self, url: &url::Url) -> Result<PooledConnection, NetError> {
        let key = OriginKey::from_url(url)?;

        // Try to reuse an existing connection (no new connection needed)
        if let Some(conn) = self.try_reuse(&key) {
            return Ok(conn);
        }

        // Acquire global permit before creating a new connection.
        // The permit is dropped immediately — it gates *creation rate*, not
        // concurrent connection count. Per-origin `active_h1` tracks lifetime.
        // Under normal workloads the per-origin limit (6) is the effective cap;
        // this semaphore prevents a burst of connections to many distinct origins
        // from exceeding `max_total_connections` creations in flight.
        let _permit = self
            .global_semaphore
            .try_acquire()
            .map_err(|_| NetError::new(NetErrorKind::Other, "global connection limit reached"))?;

        // Create a new connection
        self.create_connection(&key).await
    }

    /// Return an HTTP/1.1 connection to the pool for reuse.
    pub fn checkin(&self, url: &url::Url, sender: http1::SendRequest<Full<Bytes>>) {
        let Ok(key) = OriginKey::from_url(url) else {
            return;
        };
        let mut pools = self
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pool = pools.entry(key).or_insert_with(OriginPool::new);
        pool.active_h1 = pool.active_h1.saturating_sub(1);

        // Only store if the connection is still usable
        if sender.is_ready() {
            pool.idle_h1.push(IdleH1 {
                sender,
                idle_since: Instant::now(),
            });
        }
    }

    /// Decrement `active_h1` for an origin without returning a connection.
    ///
    /// Used when a connection is dropped without being returned to the pool
    /// (e.g. on error).
    pub fn release_h1(&self, url: &url::Url) {
        let Ok(key) = OriginKey::from_url(url) else {
            return;
        };
        let mut pools = self
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pool) = pools.get_mut(&key) {
            pool.active_h1 = pool.active_h1.saturating_sub(1);
        }
    }

    /// Evict stale idle connections across all origins.
    ///
    /// Iterates every origin pool and removes idle H1 connections older than
    /// [`IDLE_TIMEOUT`] and dead H2 senders. Origins with no remaining
    /// connections are removed entirely.
    ///
    /// Note: [`try_reuse()`](Self::try_reuse) also evicts stale connections,
    /// but only for the single origin being checked out. This method provides
    /// a global sweep. A background eviction task can be added if dead
    /// connections cause noticeable latency on first request after idle periods.
    pub fn evict_stale(&self) {
        let mut pools = self
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pools.retain(|_, pool| {
            pool.evict_stale();
            // Keep origin entry if there are idle connections, active connections, or h2 sender
            !pool.idle_h1.is_empty() || pool.active_h1 > 0 || pool.h2_sender.is_some()
        });
    }

    /// Try to reuse an existing connection from the pool.
    fn try_reuse(&self, key: &OriginKey) -> Option<PooledConnection> {
        let mut pools = self
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pool = pools.get_mut(key)?;
        pool.evict_stale();

        // H2: clone the sender if available and still ready
        if let Some(ref sender) = pool.h2_sender {
            if sender.is_ready() {
                return Some(PooledConnection::H2(sender.clone()));
            }
            // H2 connection is dead, clear it
            pool.h2_sender = None;
        }

        // H1: pop an idle connection
        while let Some(idle) = pool.idle_h1.pop() {
            if idle.sender.is_ready() {
                pool.active_h1 += 1;
                return Some(PooledConnection::H1(idle.sender));
            }
        }

        None
    }

    /// Create a new connection and optionally register it in the pool.
    ///
    /// Speculatively increments `active_h1` before the async connect to
    /// prevent TOCTOU races where multiple tasks exceed `max_per_origin`.
    async fn create_connection(&self, key: &OriginKey) -> Result<PooledConnection, NetError> {
        // Speculatively reserve a slot before the async gap
        {
            let mut pools = self
                .pools
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let pool = pools.entry(key.clone()).or_insert_with(OriginPool::new);

            if pool.active_h1 + pool.idle_h1.len() >= self.max_per_origin
                && pool.h2_sender.is_none()
            {
                return Err(NetError::new(
                    NetErrorKind::Other,
                    format!(
                        "connection limit reached for {}://{}:{}",
                        key.scheme, key.host, key.port
                    ),
                ));
            }
            // Reserve slot speculatively — decremented on failure or H2 upgrade
            pool.active_h1 += 1;
        }

        let stream = match self
            .connector
            .connect(&key.host, key.port, key.use_tls())
            .await
        {
            Ok(s) => s,
            Err(e) => {
                // Connection failed — release the speculative slot
                self.release_speculative(key);
                return Err(e);
            }
        };

        let is_h2 = stream.is_h2();
        let io = TokioIo::new(StreamWrapper(stream));

        if is_h2 {
            // HTTP/2: create multiplexed connection
            let (sender, conn) = match http2::handshake(TokioExecutor, io).await {
                Ok(r) => r,
                Err(e) => {
                    self.release_speculative(key);
                    return Err(NetError::with_source(
                        NetErrorKind::Other,
                        "HTTP/2 handshake failed",
                        e,
                    ));
                }
            };

            // Drive connection in background
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::debug!("H2 connection driver error: {e}");
                }
            });

            // H2 doesn't use the H1 slot — release it and store H2 sender
            let mut pools = self
                .pools
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let pool = pools.entry(key.clone()).or_insert_with(OriginPool::new);
            pool.active_h1 = pool.active_h1.saturating_sub(1);
            pool.h2_sender = Some(sender.clone());

            Ok(PooledConnection::H2(sender))
        } else {
            // HTTP/1.1: slot already reserved speculatively
            let (sender, conn) = match http1::handshake(io).await {
                Ok(r) => r,
                Err(e) => {
                    self.release_speculative(key);
                    return Err(NetError::with_source(
                        NetErrorKind::Other,
                        "HTTP/1.1 handshake failed",
                        e,
                    ));
                }
            };

            // Drive connection in background
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::debug!("H1 connection driver error: {e}");
                }
            });

            Ok(PooledConnection::H1(sender))
        }
    }

    /// Release a speculatively reserved `active_h1` slot.
    fn release_speculative(&self, key: &OriginKey) {
        let mut pools = self
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pool) = pools.get_mut(key) {
            pool.active_h1 = pool.active_h1.saturating_sub(1);
        }
    }
}

/// Executor for hyper's HTTP/2 that uses `tokio::spawn`.
#[derive(Clone, Copy, Debug)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_key_from_url_https() {
        let url = url::Url::parse("https://Example.Com/path").unwrap();
        let key = OriginKey::from_url(&url).unwrap();
        assert_eq!(key.scheme, "https");
        assert_eq!(key.host, "example.com");
        assert_eq!(key.port, 443);
        assert!(key.use_tls());
    }

    #[test]
    fn origin_key_from_url_http_with_port() {
        let url = url::Url::parse("http://localhost:8080/").unwrap();
        let key = OriginKey::from_url(&url).unwrap();
        assert_eq!(key.scheme, "http");
        assert_eq!(key.host, "localhost");
        assert_eq!(key.port, 8080);
        assert!(!key.use_tls());
    }

    #[test]
    fn origin_key_equality() {
        let url1 = url::Url::parse("https://example.com/a").unwrap();
        let url2 = url::Url::parse("https://example.com/b").unwrap();
        let url3 = url::Url::parse("http://example.com/a").unwrap();
        assert_eq!(
            OriginKey::from_url(&url1).unwrap(),
            OriginKey::from_url(&url2).unwrap()
        );
        assert_ne!(
            OriginKey::from_url(&url1).unwrap(),
            OriginKey::from_url(&url3).unwrap()
        );
    }

    #[test]
    fn origin_key_unsupported_scheme() {
        let url = url::Url::parse("ftp://example.com/").unwrap();
        assert!(OriginKey::from_url(&url).is_err());
    }

    #[test]
    fn origin_pool_evict_stale() {
        let mut pool = OriginPool::new();
        // Creating real hyper SendRequests requires a full connection
        // handshake, so this test only verifies eviction on an empty pool.
        // Real eviction behavior is exercised by integration tests in
        // transport.rs (send_to_local_server) where connections are
        // checked out and returned.
        pool.evict_stale();
        assert!(pool.idle_h1.is_empty());
        assert!(pool.h2_sender.is_none());
    }
}
