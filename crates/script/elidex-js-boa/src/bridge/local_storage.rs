//! Origin-scoped `localStorage` with disk persistence (WHATWG HTML §11.2).
//!
//! Data is stored as JSON files in the platform data directory:
//! `{data_dir}/elidex/localStorage/{origin_hash}.json`
//!
//! Writes use atomic file operations (write to temp file + rename) to
//! prevent corruption on crash.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use sha2::{Digest, Sha256};

/// Global registry of localStorage instances, keyed by origin string.
///
/// `Arc<Mutex<...>>` so multiple tabs with the same origin share one store.
static LOCAL_STORAGE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<LocalStore>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Track which origins have been modified since the last flush.
static DIRTY_ORIGINS: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// In-memory + disk-backed storage for a single origin.
///
/// Uses `IndexMap` for insertion-order-preserving key iteration,
/// matching WHATWG `Storage.key(n)` semantics.
struct LocalStore {
    data: IndexMap<String, String>,
    file_path: PathBuf,
    /// Dirty flag: true when in-memory data has changed since last persist.
    dirty: bool,
    /// Incremental byte size counter (sum of `key.len()` + `value.len()` for all entries).
    byte_size: usize,
}

impl LocalStore {
    /// Load from disk or create empty.
    fn load(origin: &str) -> Self {
        let file_path = storage_file_path(origin);
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

    /// Mark the store as dirty (needs persist).
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Write to disk atomically (temp file + rename), only if dirty.
    fn persist(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp_path = self.file_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_string(&self.data) {
            if std::fs::write(&tmp_path, json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &self.file_path);
                self.dirty = false;
            }
        }
    }
}

/// Compute the storage file path for an origin.
///
/// Uses a SHA-256 hex digest of the origin string as the filename,
/// avoiding filesystem-unsafe characters and preventing collisions
/// between origins that differ only in characters like `:` vs `/`.
fn storage_file_path(origin: &str) -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elidex")
        .join("localStorage");

    let hash = Sha256::digest(origin.as_bytes());
    let hex = hash.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    });
    base.join(format!("{hex}.json"))
}

// Thread-local cache to avoid hitting the global registry mutex on every access.
// Caches the last-used origin's `Arc<Mutex<LocalStore>>` so that repeated
// operations on the same origin (the common case) skip the global lock.
thread_local! {
    static CACHED_STORE: std::cell::RefCell<Option<(String, Arc<Mutex<LocalStore>>)>> =
        const { std::cell::RefCell::new(None) };
}

/// Get or create the shared store for an origin.
fn get_store(origin: &str) -> Arc<Mutex<LocalStore>> {
    // Fast path: check thread-local cache.
    let cached = CACHED_STORE.with(|cache| {
        let cache = cache.borrow();
        if let Some((ref cached_origin, ref store)) = *cache {
            if cached_origin == origin {
                return Some(store.clone());
            }
        }
        None
    });
    if let Some(store) = cached {
        return store;
    }

    // Slow path: look up or create in global registry.
    let mut registry = LOCAL_STORAGE_REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let store = registry
        .entry(origin.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(LocalStore::load(origin))))
        .clone();

    // Populate thread-local cache.
    CACHED_STORE.with(|cache| {
        *cache.borrow_mut() = Some((origin.to_string(), store.clone()));
    });

    store
}

/// Mark an origin as dirty so `flush_dirty_stores` will persist it.
fn mark_origin_dirty(origin: &str) {
    let mut dirty = DIRTY_ORIGINS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    dirty.insert(origin.to_string());
}

/// Public API for localStorage, called from `HostBridge`.
pub(crate) fn local_storage_get(origin: &str, key: &str) -> Option<String> {
    let store = get_store(origin);
    let guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.data.get(key).cloned()
}

pub(crate) fn local_storage_set(origin: &str, key: &str, value: &str) {
    let store = get_store(origin);
    let mut guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let old_size = guard.data.get(key).map_or(0, |v| key.len() + v.len());
    guard.data.insert(key.to_string(), value.to_string());
    guard.byte_size = guard.byte_size - old_size + key.len() + value.len();
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_remove(origin: &str, key: &str) {
    let store = get_store(origin);
    let mut guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(old_val) = guard.data.shift_remove(key) {
        guard.byte_size -= key.len() + old_val.len();
    }
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_clear(origin: &str) {
    let store = get_store(origin);
    let mut guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.data.clear();
    guard.byte_size = 0;
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_len(origin: &str) -> usize {
    let store = get_store(origin);
    let guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.data.len()
}

pub(crate) fn local_storage_key(origin: &str, index: usize) -> Option<String> {
    let store = get_store(origin);
    let guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.data.keys().nth(index).cloned()
}

pub(crate) fn local_storage_byte_size(origin: &str) -> usize {
    let store = get_store(origin);
    let guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.byte_size
}

/// Persist all dirty localStorage stores to disk.
///
/// Call once per frame (after JS eval) rather than on every setItem/removeItem/clear.
/// Only iterates origins that have been modified since the last flush.
///
/// The registry lock is released before any disk I/O occurs. Only per-origin
/// locks are held during `persist()`, avoiding blocking other threads.
pub fn flush_dirty_stores() {
    let dirty: Vec<String> = {
        let mut dirty_set = DIRTY_ORIGINS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let origins: Vec<String> = dirty_set.drain().collect();
        origins
    };

    if dirty.is_empty() {
        return;
    }

    // Clone Arc handles while holding the registry lock, then drop it
    // before doing any disk I/O.
    let stores: Vec<Arc<Mutex<LocalStore>>> = {
        let registry = LOCAL_STORAGE_REGISTRY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        dirty
            .iter()
            .filter_map(|origin| registry.get(origin).cloned())
            .collect()
    };
    // Registry lock is dropped here.

    // Persist with only per-origin locks held.
    for store in &stores {
        let mut guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.persist();
    }
}
