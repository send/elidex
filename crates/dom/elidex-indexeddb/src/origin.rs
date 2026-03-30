//! Per-origin `IndexedDB` storage management.
//!
//! Each origin gets its own `SQLite` database file under a data directory.
//! `OriginIdbManager` caches open connections by origin key.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::backend::{BackendError, IdbBackend};

/// Maximum storage per origin in bytes (50 MiB).
const DEFAULT_QUOTA_BYTES: u64 = 50 * 1024 * 1024;

/// Compute the `SQLite` database file path for a given origin.
///
/// Layout: `{data_dir}/elidex/origins/{origin_key}/idb.sqlite`
///
/// The origin key is sanitized to be filesystem-safe.
pub fn origin_db_path(data_dir: &Path, origin_key: &str) -> PathBuf {
    let safe_key = sanitize_origin_key(origin_key);
    data_dir
        .join("elidex")
        .join("origins")
        .join(safe_key)
        .join("idb.sqlite")
}

/// Sanitize an origin string for use as a directory name.
///
/// Replaces `://` with `_`, `/` and `:` with `_`, keeps alphanumeric, `.`, `-`, `_`.
fn sanitize_origin_key(origin: &str) -> String {
    origin
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Per-origin `IndexedDB` manager with connection caching.
pub struct OriginIdbManager {
    data_dir: PathBuf,
    backends: HashMap<String, IdbBackend>,
    quota_bytes: u64,
}

impl OriginIdbManager {
    /// Create a new manager with the given data directory.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            backends: HashMap::new(),
            quota_bytes: DEFAULT_QUOTA_BYTES,
        }
    }

    /// Create a manager with a custom quota (for testing).
    pub fn with_quota(data_dir: PathBuf, quota_bytes: u64) -> Self {
        Self {
            data_dir,
            backends: HashMap::new(),
            quota_bytes,
        }
    }

    /// Ensure a backend exists for the given origin (lazy init).
    fn ensure_backend(&mut self, origin_key: &str) -> Result<(), BackendError> {
        if !self.backends.contains_key(origin_key) {
            let path = origin_db_path(&self.data_dir, origin_key);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    BackendError::Internal(format!("Failed to create directory: {e}"))
                })?;
            }
            let backend = IdbBackend::open(&path)?;
            self.backends.insert(origin_key.to_owned(), backend);
        }
        Ok(())
    }

    /// Get or create a backend for the given origin.
    pub fn get_backend(&mut self, origin_key: &str) -> Result<&IdbBackend, BackendError> {
        self.ensure_backend(origin_key)?;
        Ok(self.backends.get(origin_key).expect("just inserted"))
    }

    /// Get a mutable backend for the given origin.
    pub fn get_backend_mut(&mut self, origin_key: &str) -> Result<&mut IdbBackend, BackendError> {
        self.ensure_backend(origin_key)?;
        Ok(self.backends.get_mut(origin_key).expect("just inserted"))
    }

    /// Check if the origin's storage exceeds the quota.
    ///
    /// Uses the `SQLite` file size as a proxy for storage usage.
    pub fn check_quota(&self, origin_key: &str) -> Result<(), BackendError> {
        let path = origin_db_path(&self.data_dir, origin_key);
        if path.exists() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if size > self.quota_bytes {
                return Err(BackendError::Internal(format!(
                    "QuotaExceededError: origin '{origin_key}' storage {size} bytes exceeds quota {} bytes",
                    self.quota_bytes
                )));
            }
        }
        Ok(())
    }

    /// Remove an origin's backend from the cache (e.g., after deleting all databases).
    pub fn evict(&mut self, origin_key: &str) {
        self.backends.remove(origin_key);
    }

    /// Returns the quota in bytes.
    pub fn quota_bytes(&self) -> u64 {
        self.quota_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_db_path_layout() {
        let path = origin_db_path(Path::new("/data"), "https://example.com");
        assert_eq!(
            path,
            PathBuf::from("/data/elidex/origins/https___example.com/idb.sqlite")
        );
    }

    #[test]
    fn sanitize_preserves_safe_chars() {
        assert_eq!(sanitize_origin_key("example.com"), "example.com");
        assert_eq!(sanitize_origin_key("a-b_c.d"), "a-b_c.d");
    }

    #[test]
    fn sanitize_replaces_unsafe_chars() {
        assert_eq!(
            sanitize_origin_key("https://example.com:8080"),
            "https___example.com_8080"
        );
    }

    #[test]
    fn manager_creates_backend_on_disk() {
        let tmp = std::env::temp_dir().join(format!("elidex_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let mut mgr = OriginIdbManager::new(tmp.clone());
        let backend = mgr.get_backend("https://test.example").unwrap();
        backend.set_version("testdb", 1).unwrap();
        assert_eq!(backend.get_version("testdb").unwrap(), Some(1));

        // Verify file was created
        let path = origin_db_path(&tmp, "https://test.example");
        assert!(path.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn manager_caches_connection() {
        let tmp = std::env::temp_dir().join(format!("elidex_test_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let mut mgr = OriginIdbManager::new(tmp.clone());
        mgr.get_backend("origin1").unwrap();
        mgr.get_backend("origin1").unwrap(); // should reuse

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
