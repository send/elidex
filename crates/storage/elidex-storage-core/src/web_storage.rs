//! WHATWG HTML §11.2 Web Storage backend (`localStorage` + `sessionStorage`).
//!
//! Engine-independent algorithm + persistence. JS bindings live in the
//! engine crates (`elidex-js-boa::globals::storage`,
//! `elidex-js::vm::host::storage`); both delegate here so quota math,
//! origin scoping, and JSON-on-disk layout stay in a single place per
//! the CLAUDE.md Layering mandate.
//!
//! ## Persistence
//!
//! `localStorage` is JSON-on-disk, one file per origin, hashed via
//! SHA-256 to a hex stem. Atomic writes (temp file + rename) survive
//! crashes mid-flush. The set of files lives in
//! `{profile_dir}/localStorage/{sha256(origin)}.json`; embedders pick
//! `profile_dir` (defaults to the platform `data_dir`).
//!
//! `sessionStorage` is per-VM in-memory only — its lifetime matches the
//! `Vm` it lives on, never written to disk. Each VM gets its own
//! `SessionStorageState` (origin scoping is implicit since the state is
//! not shared across VMs).
//!
//! ## Quota
//!
//! Per WHATWG §11.2.1 the storage quota is per-origin, ~5 MB. We track
//! `byte_size` incrementally (sum of `key.len() + value.len()` in UTF-8
//! bytes — same convention as the boa precedent that this module
//! supersedes). [`STORAGE_QUOTA_BYTES`] is the cap; `set` returns
//! [`StorageError::quota_exceeded`] when adding a new entry would
//! overflow.
//!
//! ## Layering note
//!
//! The JS-visible names `getItem` / `setItem` / `removeItem` / `clear`
//! / `key` / `length` map onto the methods here directly; the binding
//! layer is responsible for ToString coercion of arguments and for
//! mapping [`crate::StorageErrorKind::QuotaExceeded`] to the platform's
//! `QuotaExceededError` DOMException.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use sha2::{Digest, Sha256};

use crate::error::StorageError;

/// Per-origin storage quota in UTF-8 bytes (sum of key + value lengths).
///
/// 5 MiB matches WHATWG HTML §11.2.1 wording ("around five megabytes")
/// and the de-facto browser baseline.
pub const STORAGE_QUOTA_BYTES: usize = 5 * 1024 * 1024;

/// Discriminates the two `Storage` flavours per WHATWG HTML §11.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageArea {
    /// Persistent, origin-scoped, shared across documents in the same origin.
    Local,
    /// Ephemeral, per-VM (browsing-context), cleared on VM teardown.
    Session,
}

/// In-memory + disk-backed entries for a single origin's localStorage.
struct LocalStore {
    data: IndexMap<String, String>,
    file_path: PathBuf,
    /// True when in-memory data has changed since last successful persist.
    dirty: bool,
    /// Sum of `key.len() + value.len()` over all entries. Maintained
    /// incrementally to keep `set` O(1) on the quota check.
    byte_size: usize,
}

impl LocalStore {
    fn load(file_path: PathBuf) -> Self {
        let data = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|contents| {
                    serde_json::from_str::<IndexMap<String, String>>(&contents).ok()
                })
                .unwrap_or_default()
        } else {
            IndexMap::new()
        };
        let byte_size = data.iter().map(|(k, v)| k.len() + v.len()).sum();
        Self {
            data,
            file_path,
            dirty: false,
            byte_size,
        }
    }

    fn persist(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp_path = self.file_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_string(&self.data) {
            if std::fs::write(&tmp_path, json).is_ok()
                && std::fs::rename(&tmp_path, &self.file_path).is_ok()
            {
                self.dirty = false;
            }
        }
    }
}

/// Compute the deterministic on-disk filename for an origin.
///
/// SHA-256 hex avoids filesystem-unsafe characters and prevents
/// collisions between origins differing only in `:` / `/` etc.
fn origin_file_stem(origin: &str) -> String {
    let hash = Sha256::digest(origin.as_bytes());
    hash.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Origin-scoped manager for `localStorage`.
///
/// Holds the per-origin store registry plus a shared profile directory.
/// Designed for embedder ownership: shells construct one per process
/// and clone the `Arc` into each VM's host data.
///
/// `sessionStorage` is *not* managed here — it is per-VM and carried
/// directly on the VM as a [`SessionStorageState`].
pub struct WebStorageManager {
    profile_dir: PathBuf,
    /// origin → shared `Arc<Mutex<LocalStore>>`. Cross-VM tabs of the
    /// same origin share one entry.
    registry: Mutex<HashMap<String, Arc<Mutex<LocalStore>>>>,
    /// Origins modified since last [`flush_dirty`] call.
    dirty_origins: Mutex<HashSet<String>>,
}

impl WebStorageManager {
    /// New manager rooted at the platform `data_dir/elidex` (or `.`
    /// when no `data_dir` is available).
    #[must_use]
    pub fn with_default_profile() -> Self {
        let profile_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elidex");
        Self::new(profile_dir)
    }

    /// New manager rooted at `profile_dir`. Used for tests + embedder
    /// configuration. The localStorage tree lives in
    /// `{profile_dir}/localStorage/`.
    #[must_use]
    pub fn new(profile_dir: PathBuf) -> Self {
        Self {
            profile_dir,
            registry: Mutex::new(HashMap::new()),
            dirty_origins: Mutex::new(HashSet::new()),
        }
    }

    fn local_storage_dir(&self) -> PathBuf {
        self.profile_dir.join("localStorage")
    }

    fn store_for(&self, origin: &str) -> Arc<Mutex<LocalStore>> {
        let mut registry = self
            .registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        registry
            .entry(origin.to_string())
            .or_insert_with(|| {
                let path = self
                    .local_storage_dir()
                    .join(format!("{}.json", origin_file_stem(origin)));
                Arc::new(Mutex::new(LocalStore::load(path)))
            })
            .clone()
    }

    fn mark_dirty(&self, origin: &str) {
        let mut dirty = self
            .dirty_origins
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        dirty.insert(origin.to_string());
    }

    /// Look up `key` in `origin`'s localStorage. `None` for absent.
    pub fn local_get(&self, origin: &str, key: &str) -> Option<String> {
        let store = self.store_for(origin);
        let guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.data.get(key).cloned()
    }

    /// Set `key` to `value` in `origin`'s localStorage. Returns the
    /// previous value (for StorageEvent `oldValue` pairing).
    ///
    /// Errors with [`StorageError::quota_exceeded`] when the new total
    /// byte count would exceed [`STORAGE_QUOTA_BYTES`].
    pub fn local_set(
        &self,
        origin: &str,
        key: &str,
        value: &str,
    ) -> Result<Option<String>, StorageError> {
        let store = self.store_for(origin);
        let mut guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let old_value = guard.data.get(key).cloned();
        let old_entry_bytes = old_value.as_ref().map_or(0, |v| key.len() + v.len());
        let new_entry_bytes = key.len() + value.len();
        let projected = guard.byte_size - old_entry_bytes + new_entry_bytes;

        if projected > STORAGE_QUOTA_BYTES {
            return Err(StorageError::quota_exceeded(format!(
                "localStorage quota exceeded for origin {origin}: {projected} > {STORAGE_QUOTA_BYTES}"
            )));
        }

        guard.data.insert(key.to_string(), value.to_string());
        guard.byte_size = projected;
        guard.dirty = true;
        drop(guard);
        self.mark_dirty(origin);
        Ok(old_value)
    }

    /// Remove `key` from `origin`'s localStorage. Returns the removed
    /// value, or `None` when absent (silent no-op per spec).
    pub fn local_remove(&self, origin: &str, key: &str) -> Option<String> {
        let store = self.store_for(origin);
        let mut guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let removed = guard.data.shift_remove(key);
        if let Some(ref old_val) = removed {
            guard.byte_size -= key.len() + old_val.len();
            guard.dirty = true;
            drop(guard);
            self.mark_dirty(origin);
        }
        removed
    }

    /// Clear all entries for `origin`. No-op when already empty.
    pub fn local_clear(&self, origin: &str) {
        let store = self.store_for(origin);
        let mut guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.data.is_empty() {
            return;
        }
        guard.data.clear();
        guard.byte_size = 0;
        guard.dirty = true;
        drop(guard);
        self.mark_dirty(origin);
    }

    /// Number of entries in `origin`'s localStorage.
    pub fn local_len(&self, origin: &str) -> usize {
        let store = self.store_for(origin);
        let guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.data.len()
    }

    /// `n`-th key in insertion order, or `None` for out-of-range.
    pub fn local_key(&self, origin: &str, index: usize) -> Option<String> {
        let store = self.store_for(origin);
        let guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.data.keys().nth(index).cloned()
    }

    /// Snapshot of all keys in insertion order. Used for `for..in`
    /// enumeration in the binding layer.
    pub fn local_keys(&self, origin: &str) -> Vec<String> {
        let store = self.store_for(origin);
        let guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.data.keys().cloned().collect()
    }

    /// Total byte size for `origin`. Mostly for tests + diagnostics.
    pub fn local_byte_size(&self, origin: &str) -> usize {
        let store = self.store_for(origin);
        let guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.byte_size
    }

    /// Persist all dirty origins. Call once per script-task end rather
    /// than per mutation; we batch to amortise file I/O.
    ///
    /// Releases the registry lock before doing any disk I/O — only
    /// per-origin locks are held during `persist`.
    pub fn flush_dirty(&self) {
        let dirty: Vec<String> = {
            let mut set = self
                .dirty_origins
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            set.drain().collect()
        };
        if dirty.is_empty() {
            return;
        }
        let stores: Vec<Arc<Mutex<LocalStore>>> = {
            let registry = self
                .registry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            dirty
                .iter()
                .filter_map(|origin| registry.get(origin).cloned())
                .collect()
        };
        for store in &stores {
            let mut guard = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.persist();
        }
    }
}

impl Default for WebStorageManager {
    fn default() -> Self {
        Self::with_default_profile()
    }
}

/// Per-VM `sessionStorage` state. In-memory only; cleared when the
/// containing VM is torn down.
#[derive(Default)]
pub struct SessionStorageState {
    data: IndexMap<String, String>,
    byte_size: usize,
}

impl SessionStorageState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    /// Mirror of [`WebStorageManager::local_set`] — returns previous
    /// value, errors on quota overrun.
    pub fn set(&mut self, key: &str, value: &str) -> Result<Option<String>, StorageError> {
        let old_value = self.data.get(key).cloned();
        let old_entry_bytes = old_value.as_ref().map_or(0, |v| key.len() + v.len());
        let new_entry_bytes = key.len() + value.len();
        let projected = self.byte_size - old_entry_bytes + new_entry_bytes;
        if projected > STORAGE_QUOTA_BYTES {
            return Err(StorageError::quota_exceeded(format!(
                "sessionStorage quota exceeded: {projected} > {STORAGE_QUOTA_BYTES}"
            )));
        }
        self.data.insert(key.to_string(), value.to_string());
        self.byte_size = projected;
        Ok(old_value)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let removed = self.data.shift_remove(key);
        if let Some(ref old_val) = removed {
            self.byte_size -= key.len() + old_val.len();
        }
        removed
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.byte_size = 0;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn key(&self, index: usize) -> Option<String> {
        self.data.keys().nth(index).cloned()
    }

    /// Snapshot of keys in insertion order, for enumeration.
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.byte_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> (tempfile::TempDir, WebStorageManager) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = WebStorageManager::new(dir.path().to_path_buf());
        (dir, mgr)
    }

    #[test]
    fn local_basic_get_set() {
        let (_dir, mgr) = manager();
        assert_eq!(mgr.local_get("https://a", "k"), None);
        assert_eq!(mgr.local_set("https://a", "k", "v").unwrap(), None);
        assert_eq!(mgr.local_get("https://a", "k").as_deref(), Some("v"));
        assert_eq!(mgr.local_len("https://a"), 1);
    }

    #[test]
    fn local_overwrite_returns_previous() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "k", "v1").unwrap();
        let prev = mgr.local_set("https://a", "k", "v2").unwrap();
        assert_eq!(prev.as_deref(), Some("v1"));
        assert_eq!(mgr.local_get("https://a", "k").as_deref(), Some("v2"));
        assert_eq!(mgr.local_len("https://a"), 1);
    }

    #[test]
    fn local_remove_returns_old() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "k", "v").unwrap();
        assert_eq!(mgr.local_remove("https://a", "k").as_deref(), Some("v"));
        assert_eq!(mgr.local_remove("https://a", "k"), None);
        assert_eq!(mgr.local_len("https://a"), 0);
    }

    #[test]
    fn local_clear() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "x", "1").unwrap();
        mgr.local_set("https://a", "y", "2").unwrap();
        mgr.local_clear("https://a");
        assert_eq!(mgr.local_len("https://a"), 0);
        assert_eq!(mgr.local_byte_size("https://a"), 0);
    }

    #[test]
    fn local_key_insertion_order() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "first", "1").unwrap();
        mgr.local_set("https://a", "second", "2").unwrap();
        mgr.local_set("https://a", "third", "3").unwrap();
        assert_eq!(mgr.local_key("https://a", 0).as_deref(), Some("first"));
        assert_eq!(mgr.local_key("https://a", 1).as_deref(), Some("second"));
        assert_eq!(mgr.local_key("https://a", 2).as_deref(), Some("third"));
        assert_eq!(mgr.local_key("https://a", 3), None);
    }

    #[test]
    fn local_keys_snapshot() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "x", "1").unwrap();
        mgr.local_set("https://a", "y", "2").unwrap();
        assert_eq!(
            mgr.local_keys("https://a"),
            vec!["x".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn local_origin_isolation() {
        let (_dir, mgr) = manager();
        mgr.local_set("https://a", "k", "alpha").unwrap();
        mgr.local_set("https://b", "k", "beta").unwrap();
        assert_eq!(mgr.local_get("https://a", "k").as_deref(), Some("alpha"));
        assert_eq!(mgr.local_get("https://b", "k").as_deref(), Some("beta"));
    }

    #[test]
    fn local_quota_overflow() {
        let (_dir, mgr) = manager();
        let big = "x".repeat(STORAGE_QUOTA_BYTES);
        let err = mgr
            .local_set("https://a", "k", &big)
            .expect_err("over quota");
        assert!(err.message.contains("quota exceeded"));
    }

    #[test]
    fn local_quota_overwrite_recovers_space() {
        let (_dir, mgr) = manager();
        let half = "x".repeat(STORAGE_QUOTA_BYTES / 2);
        let small = "x".repeat(8);
        mgr.local_set("https://a", "big", &half).unwrap();
        mgr.local_set("https://a", "big", &small).unwrap();
        let other = "x".repeat(STORAGE_QUOTA_BYTES / 2 - 100);
        mgr.local_set("https://a", "fits-now", &other)
            .expect("freed space");
    }

    #[test]
    fn local_quota_remove_recovers_space() {
        let (_dir, mgr) = manager();
        let half = "x".repeat(STORAGE_QUOTA_BYTES / 2 - 50);
        mgr.local_set("https://a", "a", &half).unwrap();
        mgr.local_set("https://a", "b", &half).unwrap();
        mgr.local_remove("https://a", "a");
        let more = "x".repeat(STORAGE_QUOTA_BYTES / 2 - 100);
        mgr.local_set("https://a", "c", &more)
            .expect("space freed by remove");
    }

    #[test]
    fn local_persistence_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mgr = WebStorageManager::new(dir.path().to_path_buf());
            mgr.local_set("https://a", "k", "v").unwrap();
            mgr.flush_dirty();
        }
        {
            let mgr = WebStorageManager::new(dir.path().to_path_buf());
            assert_eq!(mgr.local_get("https://a", "k").as_deref(), Some("v"));
        }
    }

    #[test]
    fn local_byte_size_tracks_changes() {
        let (_dir, mgr) = manager();
        assert_eq!(mgr.local_byte_size("https://a"), 0);
        mgr.local_set("https://a", "k", "v").unwrap();
        assert_eq!(mgr.local_byte_size("https://a"), 2);
        mgr.local_set("https://a", "k", "longer").unwrap();
        assert_eq!(mgr.local_byte_size("https://a"), 7);
        mgr.local_remove("https://a", "k");
        assert_eq!(mgr.local_byte_size("https://a"), 0);
    }

    #[test]
    fn session_basic() {
        let mut s = SessionStorageState::new();
        assert!(s.is_empty());
        assert_eq!(s.set("k", "v").unwrap(), None);
        assert_eq!(s.get("k").as_deref(), Some("v"));
        assert_eq!(s.len(), 1);
        assert_eq!(s.key(0).as_deref(), Some("k"));
    }

    #[test]
    fn session_overwrite_returns_previous() {
        let mut s = SessionStorageState::new();
        s.set("k", "v1").unwrap();
        let prev = s.set("k", "v2").unwrap();
        assert_eq!(prev.as_deref(), Some("v1"));
        assert_eq!(s.byte_size(), 3);
    }

    #[test]
    fn session_quota_overflow() {
        let mut s = SessionStorageState::new();
        let big = "x".repeat(STORAGE_QUOTA_BYTES);
        s.set("k", &big).expect_err("over quota");
    }

    #[test]
    fn session_clear_resets_byte_size() {
        let mut s = SessionStorageState::new();
        s.set("k", "v").unwrap();
        assert_eq!(s.byte_size(), 2);
        s.clear();
        assert_eq!(s.byte_size(), 0);
        assert!(s.is_empty());
    }
}
