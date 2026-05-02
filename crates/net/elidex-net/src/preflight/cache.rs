//! CORS preflight cache (WHATWG Fetch §4.8 step 19 + step 22).
//!
//! Caches the [`super::PreflightAllowance`] result of a successful
//! preflight, keyed on `(origin, url, method, header-name-set)`.
//! Subsequent requests with the same key skip the OPTIONS round-
//! trip while still re-validating the actual request's method /
//! headers against the cached allowance.
//!
//! Cache entries expire after `Access-Control-Max-Age` seconds
//! (capped to [`super::MAX_AGE_CAP_SECONDS`]).

use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::Instant;

use super::{is_broker_injected_header, is_cors_safelisted_request_header, PreflightAllowance};
use crate::Request;

/// Cache key for a preflight result (WHATWG Fetch §4.8 step 22).
///
/// The key includes the actual request's method + the set of
/// non-safelisted header names, so a request that adds a new
/// non-safelisted header (or switches method) must re-preflight
/// even if the URL+origin matched a previous cache entry.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PreflightCacheKey {
    /// `request.origin.ascii_serialization()`.  `Some(_)` is
    /// required — preflight is only meaningful for cross-origin
    /// requests, which always have a document origin.
    origin: String,
    /// `request.url.as_str()` — full URL minus fragment (already
    /// stripped by `url::Url`).
    url: String,
    /// `request.method` (case-preserving).
    method: String,
    /// ASCII-lowercased set of non-safelisted author header
    /// names from the actual request.  `BTreeSet` guarantees
    /// stable equality regardless of insertion order.
    header_set: BTreeSet<String>,
}

impl PreflightCacheKey {
    /// Build a cache key from the actual request.  Returns
    /// `None` if the request has no origin (preflight only
    /// applies to requests with a document origin).
    ///
    /// The header set is built from non-safelisted **author**
    /// header names only — broker-injected headers (`Origin` /
    /// `Referer` etc. per [`is_broker_injected_header`]) are
    /// excluded so the cache key matches the
    /// `Access-Control-Request-Headers` shape.  Otherwise the key
    /// would vary on auto-injected headers and force needless
    /// cache misses (Copilot R1).
    pub fn from_request(request: &Request) -> Option<Self> {
        let origin = request.origin.as_ref()?.ascii_serialization();
        let header_set = request
            .headers
            .iter()
            .filter(|(name, value)| {
                !is_broker_injected_header(name) && !is_cors_safelisted_request_header(name, value)
            })
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect();
        Some(Self {
            origin,
            url: request.url.as_str().to_string(),
            method: request.method.clone(),
            header_set,
        })
    }
}

/// One entry in the preflight cache.
#[derive(Clone, Debug)]
struct PreflightCacheEntry {
    allowance: PreflightAllowance,
    expiry: Instant,
}

/// Sweep low-water mark — opportunistically purge expired
/// entries during [`PreflightCache::store`] when the live entry
/// count reaches the **high-water mark** (`2 * SWEEP_THRESHOLD`).
/// Using a 2× high-water mark amortizes the O(N) `retain` scan
/// to one sweep per ~`SWEEP_THRESHOLD` inserts (vs. one sweep
/// per insert at a single-threshold trigger — Copilot R8
/// PR #134).
const SWEEP_THRESHOLD: usize = 256;

/// In-memory preflight cache.
///
/// Thread-safe: the inner `Mutex` is held only for the
/// `lookup`/`store` ops, never across `await`.
///
/// **Memory policy**: lookups evict the matched-key entry on
/// expiry; stores opportunistically sweep ALL expired entries
/// once the cache exceeds `SWEEP_THRESHOLD`.  This bounds
/// memory growth even when a misbehaving caller issues many
/// unique `(origin, url, method, header-set)` tuples that are
/// never re-queried (Copilot R4 PR #134).
#[derive(Debug, Default)]
pub struct PreflightCache {
    entries: Mutex<HashMap<PreflightCacheKey, PreflightCacheEntry>>,
}

impl PreflightCache {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a cache entry.  Returns `Some(allowance)` if the
    /// key matches AND the entry has not expired.  Expired
    /// entries are evicted lazily on lookup.
    pub fn lookup(&self, key: &PreflightCacheKey) -> Option<PreflightAllowance> {
        let mut guard = self.entries.lock().expect("preflight cache mutex poisoned");
        match guard.get(key) {
            Some(entry) if entry.expiry > Instant::now() => Some(entry.allowance.clone()),
            Some(_) => {
                guard.remove(key);
                None
            }
            None => None,
        }
    }

    /// Store a cache entry.  `max_age == Duration::ZERO` is a
    /// no-op (spec says don't cache).
    ///
    /// Opportunistically sweeps expired entries when the cache
    /// reaches the **high-water mark** (`2 * SWEEP_THRESHOLD`),
    /// then refills back up to that mark before sweeping
    /// again.  This amortises the O(N) `retain` scan to one
    /// sweep per `SWEEP_THRESHOLD` inserts rather than one per
    /// insert once the cache is large (Copilot R8 PR #134).
    pub fn store(&self, key: PreflightCacheKey, allowance: PreflightAllowance) {
        if allowance.max_age.is_zero() {
            return;
        }
        let now = Instant::now();
        let expiry = now + allowance.max_age;
        let mut guard = self.entries.lock().expect("preflight cache mutex poisoned");
        if guard.len() >= 2 * SWEEP_THRESHOLD {
            guard.retain(|_, entry| entry.expiry > now);
        }
        guard.insert(key, PreflightCacheEntry { allowance, expiry });
    }

    /// Clear all cache entries (used by tests + embedder reset).
    pub fn clear(&self) {
        let mut guard = self.entries.lock().expect("preflight cache mutex poisoned");
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CredentialsMode, RedirectMode, RequestMode};
    use bytes::Bytes;
    use std::time::Duration;

    fn req(method: &str, url: &str, origin: &str, headers: Vec<(String, String)>) -> Request {
        Request {
            method: method.to_string(),
            url: url::Url::parse(url).unwrap(),
            headers,
            body: Bytes::new(),
            origin: Some(url::Url::parse(origin).unwrap().origin()),
            redirect: RedirectMode::Follow,
            credentials: CredentialsMode::SameOrigin,
            mode: RequestMode::Cors,
        }
    }

    fn allowance(max_age_secs: u64) -> PreflightAllowance {
        PreflightAllowance {
            allowed_methods: Some(vec!["PUT".into()]),
            allowed_headers: Some(Vec::new()),
            allow_credentials: false,
            max_age: Duration::from_secs(max_age_secs),
        }
    }

    #[test]
    fn lookup_after_store_returns_allowance() {
        let cache = PreflightCache::new();
        let r = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let key = PreflightCacheKey::from_request(&r).unwrap();
        cache.store(key.clone(), allowance(60));
        let hit = cache.lookup(&key);
        assert!(hit.is_some());
    }

    #[test]
    fn lookup_after_expiry_returns_none() {
        let cache = PreflightCache::new();
        let r = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let key = PreflightCacheKey::from_request(&r).unwrap();
        // Inject an entry with expiry in the past.
        let expired = PreflightCacheEntry {
            allowance: allowance(60),
            expiry: Instant::now()
                .checked_sub(Duration::from_secs(1))
                .expect("monotonic clock supports 1s rewind"),
        };
        cache.entries.lock().unwrap().insert(key.clone(), expired);
        assert!(cache.lookup(&key).is_none());
        // Expired entry must be evicted on miss.
        assert!(cache.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn store_max_age_zero_is_noop() {
        let cache = PreflightCache::new();
        let r = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let key = PreflightCacheKey::from_request(&r).unwrap();
        let mut a = allowance(60);
        a.max_age = Duration::ZERO;
        cache.store(key.clone(), a);
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn different_methods_do_not_collide() {
        let r1 = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let r2 = req(
            "DELETE",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let k1 = PreflightCacheKey::from_request(&r1).unwrap();
        let k2 = PreflightCacheKey::from_request(&r2).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_header_sets_do_not_collide() {
        let cache = PreflightCache::new();
        let r1 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("X-A".into(), "1".into())],
        );
        let r2 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("X-B".into(), "1".into())],
        );
        let k1 = PreflightCacheKey::from_request(&r1).unwrap();
        let k2 = PreflightCacheKey::from_request(&r2).unwrap();
        assert_ne!(k1, k2);
        cache.store(k1.clone(), allowance(60));
        assert!(cache.lookup(&k1).is_some());
        assert!(cache.lookup(&k2).is_none());
    }

    #[test]
    fn header_set_ignores_safelisted_names() {
        let r1 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![
                ("Accept".into(), "*/*".into()),
                ("X-Custom".into(), "1".into()),
            ],
        );
        let r2 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let k1 = PreflightCacheKey::from_request(&r1).unwrap();
        let k2 = PreflightCacheKey::from_request(&r2).unwrap();
        // Adding/removing a safelisted name does not change the
        // cache key — only non-safelisted names participate.
        assert_eq!(k1, k2);
    }

    /// Regression for Copilot R1 finding 4: cache key must not
    /// vary on broker-injected headers (`Origin` / `Referer`),
    /// otherwise the auto-injected values would force cache
    /// misses on every cross-origin fetch.
    #[test]
    fn header_set_ignores_broker_injected_origin_and_referer() {
        let r1 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let r2 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![
                ("Origin".into(), "https://example.com".into()),
                ("Referer".into(), "https://example.com/page".into()),
                ("X-Custom".into(), "1".into()),
            ],
        );
        let k1 = PreflightCacheKey::from_request(&r1).unwrap();
        let k2 = PreflightCacheKey::from_request(&r2).unwrap();
        assert_eq!(k1, k2, "broker-injected headers must not affect cache key");
    }

    #[test]
    fn header_set_is_case_insensitive() {
        let r1 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let r2 = req(
            "POST",
            "https://api.other.com/x",
            "https://example.com/",
            vec![("x-custom".into(), "1".into())],
        );
        let k1 = PreflightCacheKey::from_request(&r1).unwrap();
        let k2 = PreflightCacheKey::from_request(&r2).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn from_request_returns_none_when_no_origin() {
        let mut r = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        r.origin = None;
        assert!(PreflightCacheKey::from_request(&r).is_none());
    }

    /// Regression for Copilot R4 finding 3: the cache must
    /// opportunistically evict expired entries during
    /// [`PreflightCache::store`] so unique-key churn doesn't
    /// accumulate dead entries indefinitely.  Pre-fix only the
    /// matched-key entry was evicted on lookup; never-queried
    /// expired keys would leak.
    #[test]
    fn store_sweeps_expired_entries_when_threshold_exceeded() {
        let cache = PreflightCache::new();
        // Fill the cache with `2 * SWEEP_THRESHOLD` already-expired
        // entries (bypassing `store()` so we can plant pre-
        // expired data) — the high-water mark for sweep.  Then
        // call `store()` with a fresh entry — the sweep should
        // evict all `2 * SWEEP_THRESHOLD` expired entries,
        // leaving only the new one.
        let now = Instant::now();
        let past = now
            .checked_sub(Duration::from_secs(1))
            .expect("monotonic clock supports 1s rewind");
        {
            let mut guard = cache.entries.lock().unwrap();
            for i in 0..(2 * SWEEP_THRESHOLD) {
                let r = req(
                    "PUT",
                    &format!("https://api.other.com/x{i}"),
                    "https://example.com/",
                    vec![],
                );
                let key = PreflightCacheKey::from_request(&r).unwrap();
                guard.insert(
                    key,
                    PreflightCacheEntry {
                        allowance: allowance(60),
                        expiry: past,
                    },
                );
            }
        }
        // Now insert one fresh entry — should trigger sweep
        // because cache size == 2 * SWEEP_THRESHOLD.
        let fresh = req(
            "PUT",
            "https://api.other.com/fresh",
            "https://example.com/",
            vec![],
        );
        let fresh_key = PreflightCacheKey::from_request(&fresh).unwrap();
        cache.store(fresh_key.clone(), allowance(60));

        let live_count = cache.entries.lock().unwrap().len();
        assert_eq!(
            live_count, 1,
            "sweep must evict expired entries; only the fresh one remains"
        );
        assert!(cache.lookup(&fresh_key).is_some());
    }

    /// Sentinel: stores below the sweep threshold do NOT evict —
    /// avoiding the O(N) scan on every store when the cache is
    /// small.
    #[test]
    fn store_below_threshold_does_not_sweep() {
        let cache = PreflightCache::new();
        let r1 = req(
            "PUT",
            "https://api.other.com/a",
            "https://example.com/",
            vec![],
        );
        let r2 = req(
            "PUT",
            "https://api.other.com/b",
            "https://example.com/",
            vec![],
        );
        let key1 = PreflightCacheKey::from_request(&r1).unwrap();
        let key2 = PreflightCacheKey::from_request(&r2).unwrap();
        // Plant an expired entry directly.
        let past = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .expect("monotonic clock supports 1s rewind");
        cache.entries.lock().unwrap().insert(
            key1.clone(),
            PreflightCacheEntry {
                allowance: allowance(60),
                expiry: past,
            },
        );
        cache.store(key2, allowance(60));
        // Below the threshold, the expired entry is still
        // present (lazy eviction on lookup only).
        assert_eq!(cache.entries.lock().unwrap().len(), 2);
    }

    #[test]
    fn clear_empties_cache() {
        let cache = PreflightCache::new();
        let r = req(
            "PUT",
            "https://api.other.com/x",
            "https://example.com/",
            vec![],
        );
        let key = PreflightCacheKey::from_request(&r).unwrap();
        cache.store(key.clone(), allowance(60));
        cache.clear();
        assert!(cache.lookup(&key).is_none());
    }
}
