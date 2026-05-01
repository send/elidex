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

/// Connection pool managing HTTP/1.1 and HTTP/2 connections per origin.
pub struct ConnectionPool {
    connector: Connector,
    max_per_origin: usize,
    pools: Mutex<HashMap<OriginKey, OriginPool>>,
    global_semaphore: tokio::sync::Semaphore,
}

impl std::fmt::Debug for ConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pool_count = self.pools.lock().map_or(0, |p| p.len());
        f.debug_struct("ConnectionPool")
            .field("max_per_origin", &self.max_per_origin)
            .field("origins", &pool_count)
            .finish_non_exhaustive()
    }
}

impl ConnectionPool {
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

        // Clean up empty OriginPool entries to prevent unbounded map growth
        if pool.idle_h1.is_empty() && pool.active_h1 == 0 && pool.h2_sender.is_none() {
            pools.remove(key);
        }

        None
    }

    /// Create a new connection and optionally register it in the pool.
    ///
    /// Speculatively increments `active_h1` before the async connect to
    /// prevent TOCTOU races where multiple tasks exceed `max_per_origin`.
    ///
    /// **Cancellation safety**: the speculative slot is wrapped in
    /// a [`SpeculativeSlotGuard`] so a future-drop mid-`connect` /
    /// mid-handshake (e.g. via [`crate::CancelHandle`] or the
    /// transport-level `request_timeout`) still releases the slot
    /// via the guard's `Drop`.  Without this, every cancellation
    /// landing inside the connect/handshake window leaked one
    /// per-origin slot permanently and eventually pinned the
    /// origin at `max_per_origin` (Copilot R5).
    async fn create_connection(&self, key: &OriginKey) -> Result<PooledConnection, NetError> {
        // Speculatively reserve a slot before the async gap.
        let mut slot = {
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
            // Reserve slot speculatively — guarded by
            // SpeculativeSlotGuard so error / cancel / panic
            // paths all release without explicit calls below.
            pool.active_h1 += 1;
            SpeculativeSlotGuard::new(self, key.clone())
        };

        let stream = self
            .connector
            .connect(&key.host, key.port, key.use_tls())
            .await?;

        let is_h2 = stream.is_h2();
        let io = TokioIo::new(StreamWrapper(stream));

        if is_h2 {
            // HTTP/2: create multiplexed connection
            let (sender, conn) = http2::handshake(TokioExecutor, io).await.map_err(|e| {
                NetError::with_source(NetErrorKind::Other, "HTTP/2 handshake failed", e)
            })?;

            // Drive connection in background
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::debug!("H2 connection driver error: {e}");
                }
            });

            // H2 doesn't use the H1 slot — release the
            // speculative reservation and store H2 sender.  We
            // disarm the guard *after* the manual decrement: the
            // guard only releases when armed, and disarming
            // post-decrement is the cheapest way to encode "the
            // slot has been handed off / released exactly once".
            let mut pools = self
                .pools
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let pool = pools.entry(key.clone()).or_insert_with(OriginPool::new);
            pool.active_h1 = pool.active_h1.saturating_sub(1);
            pool.h2_sender = Some(sender.clone());
            drop(pools);
            slot.disarm();

            Ok(PooledConnection::H2(sender))
        } else {
            // HTTP/1.1: slot already reserved speculatively
            let (sender, conn) = http1::handshake(io).await.map_err(|e| {
                NetError::with_source(NetErrorKind::Other, "HTTP/1.1 handshake failed", e)
            })?;

            // Drive connection in background
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::debug!("H1 connection driver error: {e}");
                }
            });

            // Success: hand the slot off to the caller's
            // `PooledConnection` lifecycle (release_h1 /
            // checkin) by disarming the guard.
            slot.disarm();
            Ok(PooledConnection::H1(sender))
        }
    }
}

/// RAII guard for the speculative `active_h1` reservation made
/// inside [`ConnectionPool::create_connection`].  When armed,
/// `Drop` decrements the per-origin counter — covering early-
/// returns from `?`, panics, *and* cancellation drops of the
/// containing future (Copilot R5).
///
/// Successful paths call [`Self::disarm`] to hand the slot off
/// to the caller's `PooledConnection` lifecycle (`release_h1` on
/// guarded send-failure or `checkin` on success); after disarm
/// the guard's `Drop` is a no-op.
///
/// Borrows the pool by reference: the guard's lifetime is bound
/// to a single `create_connection` invocation, and `&self`
/// survives for the entire async call (including cancel-drop of
/// the future), so a borrow is sufficient and avoids forcing
/// the pool to be `Arc`-wrapped.
struct SpeculativeSlotGuard<'a> {
    pool: &'a ConnectionPool,
    key: Option<OriginKey>,
}

impl<'a> SpeculativeSlotGuard<'a> {
    fn new(pool: &'a ConnectionPool, key: OriginKey) -> Self {
        Self {
            pool,
            key: Some(key),
        }
    }

    /// Mark the slot as handed off — `Drop` will not decrement.
    fn disarm(&mut self) {
        self.key = None;
    }
}

impl Drop for SpeculativeSlotGuard<'_> {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else {
            return;
        };
        let mut pools = self
            .pool
            .pools
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pool) = pools.get_mut(&key) {
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

    /// Helper: build a `ConnectionPool` instance suitable for
    /// guard-only tests (no real connect required).  We need a
    /// `Connector` to construct one — use a `tokio::net::TcpStream`
    /// connector pointing at an unused port; the connector is never
    /// actually invoked by these tests.
    fn empty_pool_for_guard_tests() -> ConnectionPool {
        ConnectionPool::with_global_limit(Connector::new(), 4, 16)
    }

    /// Regression for Copilot R5 (pool.rs SpeculativeSlotGuard):
    /// the guard must release the speculatively reserved
    /// `active_h1` slot when the containing future is dropped
    /// (cancel/timeout) mid-connect/handshake.  Pre-fix, the
    /// in-line `release_speculative` calls only ran on the `Err`
    /// arm of the explicit `match`, so a future-drop during the
    /// `connect()` or `handshake()` `await` left the slot
    /// permanently bumped.
    ///
    /// We test the guard itself: simulate the speculative
    /// reserve, drop the guard via panic + `catch_unwind`, then
    /// verify the per-origin counter is back to zero.
    #[test]
    fn speculative_slot_guard_releases_on_drop() {
        let pool = empty_pool_for_guard_tests();
        let url = url::Url::parse("http://example.test/").unwrap();
        let key = OriginKey::from_url(&url).unwrap();

        // Manually mimic the create_connection setup: speculative
        // increment under the lock, then construct the guard.
        {
            let mut pools = pool.pools.lock().unwrap();
            pools
                .entry(key.clone())
                .or_insert_with(OriginPool::new)
                .active_h1 += 1;
        }
        assert_eq!(
            pool.pools.lock().unwrap().get(&key).unwrap().active_h1,
            1,
            "precondition: slot reserved"
        );

        // Drop the guard via a panic-inside-scope: covers the
        // future-cancellation Drop path.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _slot = SpeculativeSlotGuard::new(&pool, key.clone());
            panic!("simulated mid-connect cancellation");
        }));
        assert!(result.is_err(), "panic was caught");
        assert_eq!(
            pool.pools.lock().unwrap().get(&key).unwrap().active_h1,
            0,
            "SpeculativeSlotGuard leaked the speculative slot on Drop"
        );
    }

    /// Sibling assertion: a disarmed guard is a no-op on Drop —
    /// the success path hands ownership of the slot to the
    /// caller's `PooledConnection` lifecycle and must not
    /// double-release.
    #[test]
    fn speculative_slot_guard_disarmed_does_not_release() {
        let pool = empty_pool_for_guard_tests();
        let url = url::Url::parse("http://example.test/").unwrap();
        let key = OriginKey::from_url(&url).unwrap();

        {
            let mut pools = pool.pools.lock().unwrap();
            pools
                .entry(key.clone())
                .or_insert_with(OriginPool::new)
                .active_h1 += 1;
        }
        {
            let mut slot = SpeculativeSlotGuard::new(&pool, key.clone());
            slot.disarm();
            // Drop happens at end of block.
        }
        assert_eq!(
            pool.pools.lock().unwrap().get(&key).unwrap().active_h1,
            1,
            "disarmed guard erroneously decremented the slot"
        );
    }
}
