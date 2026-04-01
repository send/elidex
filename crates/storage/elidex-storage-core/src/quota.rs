use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::origin_manager::OriginKey;

/// Per-origin quota information.
#[derive(Debug, Clone)]
pub struct QuotaInfo {
    /// Current usage in bytes.
    pub usage: u64,
    /// Whether the origin has requested persistent storage.
    pub persistent: bool,
    /// Last time the origin's storage was accessed.
    pub last_access: Instant,
}

/// Quota estimation result for `navigator.storage.estimate()`.
#[derive(Debug, Clone)]
pub struct QuotaEstimate {
    /// Bytes currently used by this origin.
    pub usage: u64,
    /// Total quota available for this origin.
    pub quota: u64,
}

/// Manages per-origin storage quotas (Ch.22).
///
/// Current implementation: fixed global limit (default 10 GB), per-origin = limit / 5.
/// Tracks usage and persistent flag per origin. `eviction_candidates()` returns
/// non-persistent origins sorted by LRU for the caller to evict.
///
/// TODO(M4-8.5): disk-aware limit (`min(20% of available disk, 10 GB)`),
/// automatic eviction enforcement.
pub struct QuotaManager {
    origins: Mutex<HashMap<OriginKey, QuotaInfo>>,
    /// Global storage limit in bytes.
    global_limit: u64,
}

const DEFAULT_GLOBAL_LIMIT: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            origins: Mutex::new(HashMap::new()),
            global_limit: DEFAULT_GLOBAL_LIMIT,
        }
    }

    pub fn with_global_limit(global_limit: u64) -> Self {
        Self {
            origins: Mutex::new(HashMap::new()),
            global_limit,
        }
    }

    /// Report storage usage for an origin.
    pub fn report_usage(&self, origin: &OriginKey, usage: u64) {
        let mut origins = self.origins.lock().unwrap();
        let info = origins.entry(origin.clone()).or_insert_with(|| QuotaInfo {
            usage: 0,
            persistent: false,
            last_access: Instant::now(),
        });
        info.usage = usage;
        info.last_access = Instant::now();
    }

    /// Get the quota estimate for an origin (for `navigator.storage.estimate()`).
    pub fn estimate(&self, origin: &OriginKey) -> QuotaEstimate {
        let origins = self.origins.lock().unwrap();
        let usage = origins.get(origin).map_or(0, |i| i.usage);
        // global_limit * 0.20 — integer arithmetic avoids f64 cast warnings.
        let per_origin_quota = self.global_limit / 5;
        QuotaEstimate {
            usage,
            quota: per_origin_quota,
        }
    }

    /// Check if an origin can store additional bytes.
    pub fn check_quota(&self, origin: &OriginKey, additional_bytes: u64) -> bool {
        let origins = self.origins.lock().unwrap();
        let current = origins.get(origin).map_or(0, |i| i.usage);
        // global_limit * 0.20 — integer arithmetic avoids f64 cast warnings.
        let per_origin_quota = self.global_limit / 5;
        current.saturating_add(additional_bytes) <= per_origin_quota
    }

    /// Request persistent storage for an origin.
    pub fn request_persist(&self, origin: &OriginKey) -> bool {
        let mut origins = self.origins.lock().unwrap();
        let info = origins.entry(origin.clone()).or_insert_with(|| QuotaInfo {
            usage: 0,
            persistent: false,
            last_access: Instant::now(),
        });
        info.persistent = true;
        true
    }

    /// Check if an origin has persistent storage.
    pub fn is_persisted(&self, origin: &OriginKey) -> bool {
        let origins = self.origins.lock().unwrap();
        origins.get(origin).is_some_and(|i| i.persistent)
    }

    /// Get origins eligible for LRU eviction (non-persistent, sorted by last_access ascending).
    pub fn eviction_candidates(&self) -> Vec<OriginKey> {
        let origins = self.origins.lock().unwrap();
        let mut candidates: Vec<_> = origins
            .iter()
            .filter(|(_, info)| !info.persistent)
            .map(|(key, info)| (key.clone(), info.last_access))
            .collect();
        candidates.sort_by_key(|(_, t)| *t);
        candidates.into_iter().map(|(key, _)| key).collect()
    }

    /// Total usage across all origins.
    pub fn total_usage(&self) -> u64 {
        let origins = self.origins.lock().unwrap();
        origins.values().map(|i| i.usage).sum()
    }

    /// Remove tracking for an origin (after eviction or data clear).
    pub fn remove_origin(&self, origin: &OriginKey) {
        let mut origins = self.origins.lock().unwrap();
        origins.remove(origin);
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for QuotaManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let origins = self.origins.lock().unwrap();
        f.debug_struct("QuotaManager")
            .field("global_limit", &self.global_limit)
            .field("tracked_origins", &origins.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(name: &str) -> OriginKey {
        OriginKey::from_parts("https", name, 443)
    }

    #[test]
    fn estimate_empty() {
        let qm = QuotaManager::new();
        let est = qm.estimate(&origin("example.com"));
        assert_eq!(est.usage, 0);
        assert!(est.quota > 0);
    }

    #[test]
    fn report_and_estimate() {
        let qm = QuotaManager::new();
        let o = origin("example.com");
        qm.report_usage(&o, 1024);
        let est = qm.estimate(&o);
        assert_eq!(est.usage, 1024);
    }

    #[test]
    fn check_quota_under_limit() {
        let qm = QuotaManager::with_global_limit(1_000_000);
        let o = origin("example.com");
        assert!(qm.check_quota(&o, 100_000)); // 100K < 200K (20% of 1M)
    }

    #[test]
    fn check_quota_over_limit() {
        let qm = QuotaManager::with_global_limit(1_000_000);
        let o = origin("example.com");
        qm.report_usage(&o, 190_000);
        assert!(!qm.check_quota(&o, 20_000)); // 190K + 20K > 200K
    }

    #[test]
    fn persist() {
        let qm = QuotaManager::new();
        let o = origin("example.com");
        assert!(!qm.is_persisted(&o));
        qm.request_persist(&o);
        assert!(qm.is_persisted(&o));
    }

    #[test]
    fn eviction_candidates_excludes_persistent() {
        let qm = QuotaManager::new();
        let o1 = origin("evict.com");
        let o2 = origin("keep.com");

        qm.report_usage(&o1, 100);
        qm.report_usage(&o2, 200);
        qm.request_persist(&o2);

        let candidates = qm.eviction_candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], o1);
    }

    #[test]
    fn eviction_lru_order() {
        let qm = QuotaManager::new();
        let o1 = origin("old.com");
        let o2 = origin("new.com");

        qm.report_usage(&o1, 100);
        std::thread::sleep(std::time::Duration::from_millis(10));
        qm.report_usage(&o2, 200);

        let candidates = qm.eviction_candidates();
        assert_eq!(candidates[0], o1); // oldest first
    }

    #[test]
    fn total_usage() {
        let qm = QuotaManager::new();
        qm.report_usage(&origin("a.com"), 100);
        qm.report_usage(&origin("b.com"), 200);
        assert_eq!(qm.total_usage(), 300);
    }

    #[test]
    fn remove_origin() {
        let qm = QuotaManager::new();
        let o = origin("remove.com");
        qm.report_usage(&o, 100);
        assert_eq!(qm.estimate(&o).usage, 100);
        qm.remove_origin(&o);
        assert_eq!(qm.estimate(&o).usage, 0);
    }
}
