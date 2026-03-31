//! Quota management for per-origin storage (W3C Storage Standard §4).
//!
//! Tracks storage usage per origin across all storage types (`IndexedDB`,
//! Cache API, localStorage) and enforces quota limits with LRU eviction.
//!
//! The `QuotaManager` lives in the browser thread and is the single
//! authority for storage quota decisions.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

/// Per-origin storage usage tracking.
pub struct OriginUsage {
    /// Total bytes used across all storage types.
    pub total_bytes: u64,
    /// Last access timestamp (for LRU eviction ordering).
    pub last_access: Instant,
}

/// Central quota manager for all per-origin storage.
///
/// Enforces global and per-origin limits, performs LRU eviction of
/// non-persistent origins when storage pressure is detected.
pub struct QuotaManager {
    /// Per-origin usage tracking.
    usage: HashMap<String, OriginUsage>,
    /// Global storage limit in bytes.
    /// Default: `min(available_disk / 2, 2 GiB)`.
    global_limit: u64,
    /// Per-origin storage limit in bytes.
    /// Default: `min(global_limit / 5, 500 MiB)`.
    per_origin_limit: u64,
    /// Origins that have been granted persistent storage
    /// (exempt from LRU eviction).
    persistent_origins: HashSet<String>,
    /// Base directory for origin storage data.
    data_dir: PathBuf,
}

/// Default global limit: 2 GiB.
const DEFAULT_GLOBAL_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

/// Default per-origin limit: 500 MiB.
const DEFAULT_PER_ORIGIN_LIMIT: u64 = 500 * 1024 * 1024;

/// Eviction target: reduce usage to 80% of global limit.
const EVICTION_TARGET_RATIO: f64 = 0.80;

impl QuotaManager {
    /// Create a new `QuotaManager` with default limits.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            usage: HashMap::new(),
            global_limit: DEFAULT_GLOBAL_LIMIT,
            per_origin_limit: DEFAULT_PER_ORIGIN_LIMIT,
            persistent_origins: HashSet::new(),
            data_dir,
        }
    }

    /// Create a `QuotaManager` with custom limits (for testing).
    #[cfg(test)]
    pub fn with_limits(data_dir: PathBuf, global_limit: u64, per_origin_limit: u64) -> Self {
        Self {
            usage: HashMap::new(),
            global_limit,
            per_origin_limit,
            persistent_origins: HashSet::new(),
            data_dir,
        }
    }

    /// Report storage usage for an origin.
    ///
    /// Called after write operations (`IndexedDB` put, Cache API store, etc.).
    pub fn report_usage(&mut self, origin: &str, total_bytes: u64) {
        let entry = self
            .usage
            .entry(origin.to_string())
            .or_insert_with(|| OriginUsage {
                total_bytes: 0,
                last_access: Instant::now(),
            });
        entry.total_bytes = total_bytes;
        entry.last_access = Instant::now();
    }

    /// Touch an origin's last-access timestamp.
    pub fn touch(&mut self, origin: &str) {
        if let Some(entry) = self.usage.get_mut(origin) {
            entry.last_access = Instant::now();
        }
    }

    /// Check if an origin has exceeded its per-origin quota.
    #[must_use]
    pub fn check_origin_quota(&self, origin: &str) -> bool {
        self.usage
            .get(origin)
            .is_some_and(|u| u.total_bytes > self.per_origin_limit)
    }

    /// Get the total usage across all origins.
    #[must_use]
    pub fn total_usage(&self) -> u64 {
        self.usage.values().map(|u| u.total_bytes).sum()
    }

    /// Get usage and quota for a specific origin (for `navigator.storage.estimate()`).
    #[must_use]
    pub fn estimate(&self, origin: &str) -> (u64, u64) {
        let usage = self.usage.get(origin).map_or(0, |u| u.total_bytes);
        (usage, self.per_origin_limit)
    }

    /// Grant persistent storage to an origin (for `navigator.storage.persist()`).
    ///
    /// Persistent origins are exempt from LRU eviction.
    /// Returns `true` if the grant was new.
    pub fn grant_persistent(&mut self, origin: &str) -> bool {
        self.persistent_origins.insert(origin.to_string())
    }

    /// Check if an origin has persistent storage (for `navigator.storage.persisted()`).
    #[must_use]
    pub fn is_persistent(&self, origin: &str) -> bool {
        self.persistent_origins.contains(origin)
    }

    /// Perform LRU eviction if global usage exceeds the limit.
    ///
    /// Evicts non-persistent origins ordered by `last_access` (oldest first)
    /// until usage drops below 80% of `global_limit`.
    ///
    /// Returns the list of evicted origin keys.
    pub fn evict_if_needed(&mut self) -> Vec<String> {
        let total = self.total_usage();
        if total <= self.global_limit {
            return Vec::new();
        }

        // 80% of global limit. Precision loss from u64→f64 is negligible for storage sizes.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let target = (self.global_limit as f64 * EVICTION_TARGET_RATIO) as u64;
        let mut current = total;
        let mut evicted = Vec::new();

        // Collect non-persistent origins sorted by last_access ascending (oldest first).
        let mut candidates: Vec<(String, Instant, u64)> = self
            .usage
            .iter()
            .filter(|(origin, _)| !self.persistent_origins.contains(origin.as_str()))
            .map(|(origin, usage)| (origin.clone(), usage.last_access, usage.total_bytes))
            .collect();
        candidates.sort_by_key(|(_, last_access, _)| *last_access);

        for (origin, _, bytes) in candidates {
            if current <= target {
                break;
            }

            // Delete origin storage directory (hex-encoded to be filesystem-safe).
            let origin_dir = self
                .data_dir
                .join("elidex")
                .join("origins")
                .join(elidex_plugin::hex_encode_for_path(&origin));
            if origin_dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&origin_dir) {
                    eprintln!("QuotaManager: failed to evict {origin}: {e}");
                    continue; // Skip bookkeeping — data still on disk.
                }
            }

            current -= bytes;
            self.usage.remove(&origin);
            evicted.push(origin);
        }

        evicted
    }
}

impl QuotaManager {
    /// Remove all usage tracking for an origin (e.g., "clear site data").
    pub fn clear_origin(&mut self, origin: &str) {
        self.usage.remove(origin);
        self.persistent_origins.remove(origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::AtomicU64;
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_manager() -> QuotaManager {
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("elidex-quota-test-{}-{id}", std::process::id()));
        QuotaManager::with_limits(dir, 1000, 500)
    }

    #[test]
    fn new_manager_empty() {
        let mgr = test_manager();
        assert_eq!(mgr.total_usage(), 0);
        assert!(!mgr.check_origin_quota("https://example.com"));
    }

    #[test]
    fn report_usage() {
        let mut mgr = test_manager();
        mgr.report_usage("https://example.com", 100);
        assert_eq!(mgr.total_usage(), 100);
        let (usage, quota) = mgr.estimate("https://example.com");
        assert_eq!(usage, 100);
        assert_eq!(quota, 500);
    }

    #[test]
    fn origin_quota_exceeded() {
        let mut mgr = test_manager();
        mgr.report_usage("https://example.com", 501);
        assert!(mgr.check_origin_quota("https://example.com"));
    }

    #[test]
    fn origin_quota_within_limit() {
        let mut mgr = test_manager();
        mgr.report_usage("https://example.com", 500);
        assert!(!mgr.check_origin_quota("https://example.com"));
    }

    #[test]
    fn persistent_storage_grant() {
        let mut mgr = test_manager();
        assert!(!mgr.is_persistent("https://example.com"));
        assert!(mgr.grant_persistent("https://example.com"));
        assert!(mgr.is_persistent("https://example.com"));
        // Second grant returns false (already granted).
        assert!(!mgr.grant_persistent("https://example.com"));
    }

    #[test]
    fn eviction_not_needed() {
        let mut mgr = test_manager();
        mgr.report_usage("https://a.com", 400);
        mgr.report_usage("https://b.com", 400);
        let evicted = mgr.evict_if_needed();
        assert!(evicted.is_empty());
    }

    #[test]
    fn eviction_lru_order() {
        let mut mgr = test_manager();
        let now = Instant::now();
        // Set up origins with controlled last_access (no sleep needed).
        mgr.report_usage("https://old.com", 400);
        mgr.usage.get_mut("https://old.com").unwrap().last_access =
            now.checked_sub(std::time::Duration::from_secs(30)).unwrap();
        mgr.report_usage("https://new.com", 400);
        mgr.usage.get_mut("https://new.com").unwrap().last_access =
            now.checked_sub(std::time::Duration::from_secs(10)).unwrap();
        mgr.report_usage("https://newest.com", 400);
        // Total = 1200 > limit 1000. Evict oldest first.
        let evicted = mgr.evict_if_needed();
        assert!(evicted.contains(&"https://old.com".to_string()));
        assert!(mgr.total_usage() <= 1000);
    }

    #[test]
    fn eviction_skips_persistent() {
        let now = Instant::now();
        let mut mgr = test_manager();
        mgr.report_usage("https://persistent.com", 600);
        mgr.grant_persistent("https://persistent.com");
        mgr.report_usage("https://evictable.com", 500);
        mgr.usage
            .get_mut("https://evictable.com")
            .unwrap()
            .last_access = now.checked_sub(std::time::Duration::from_secs(10)).unwrap();
        // Total = 1100 > limit 1000. Only evictable.com can be evicted.
        let evicted = mgr.evict_if_needed();
        assert!(evicted.contains(&"https://evictable.com".to_string()));
        assert!(!evicted.contains(&"https://persistent.com".to_string()));
    }

    #[test]
    fn clear_origin() {
        let mut mgr = test_manager();
        mgr.report_usage("https://example.com", 100);
        mgr.grant_persistent("https://example.com");
        mgr.clear_origin("https://example.com");
        assert_eq!(mgr.total_usage(), 0);
        assert!(!mgr.is_persistent("https://example.com"));
    }

    #[test]
    fn estimate_unknown_origin() {
        let mgr = test_manager();
        let (usage, quota) = mgr.estimate("https://unknown.com");
        assert_eq!(usage, 0);
        assert_eq!(quota, 500);
    }

    #[test]
    fn touch_updates_access_time() {
        let mut mgr = test_manager();
        mgr.report_usage("https://example.com", 100);
        // Set last_access to the past so touch() will produce a later Instant.
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(10))
            .unwrap();
        mgr.usage
            .get_mut("https://example.com")
            .unwrap()
            .last_access = past;
        mgr.touch("https://example.com");
        let after = mgr.usage["https://example.com"].last_access;
        assert!(after > past);
    }
}
