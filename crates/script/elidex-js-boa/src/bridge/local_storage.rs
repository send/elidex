//! Origin-scoped `localStorage` with disk persistence (WHATWG HTML §11.2).
//!
//! Data is stored as JSON files in the platform data directory:
//! `{data_dir}/elidex/localStorage/{origin_hash}.json`
//!
//! Writes use atomic file operations (write to temp file + rename) to
//! prevent corruption on crash.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Global registry of localStorage instances, keyed by origin string.
///
/// `Arc<Mutex<...>>` so multiple tabs with the same origin share one store.
static LOCAL_STORAGE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<LocalStore>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Track which origins have been modified since the last flush.
static DIRTY_ORIGINS: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// In-memory + disk-backed storage for a single origin.
struct LocalStore {
    data: HashMap<String, String>,
    file_path: PathBuf,
    /// Dirty flag: true when in-memory data has changed since last persist.
    dirty: bool,
}

impl LocalStore {
    /// Load from disk or create empty.
    fn load(origin: &str) -> Self {
        let file_path = storage_file_path(origin);
        let data = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|contents| serde_json::from_str::<HashMap<String, String>>(&contents).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self { data, file_path, dirty: false }
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
/// Uses the origin string directly as the filename, replacing
/// non-filesystem-safe characters with `_`. This avoids relying on
/// `DefaultHasher` which is not guaranteed to be stable across Rust versions.
fn storage_file_path(origin: &str) -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elidex")
        .join("localStorage");

    let safe_name: String = origin
        .chars()
        .map(|c| match c {
            '/' | ':' | '?' | '#' | '\\' | '*' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    base.join(format!("{safe_name}.json"))
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
    let mut registry = LOCAL_STORAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
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
    let mut dirty = DIRTY_ORIGINS.lock().unwrap_or_else(|e| e.into_inner());
    dirty.insert(origin.to_string());
}

/// Public API for localStorage, called from HostBridge.

pub(crate) fn local_storage_get(origin: &str, key: &str) -> Option<String> {
    let store = get_store(origin);
    let guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.get(key).cloned()
}

pub(crate) fn local_storage_set(origin: &str, key: &str, value: &str) {
    let store = get_store(origin);
    let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.insert(key.to_string(), value.to_string());
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_remove(origin: &str, key: &str) {
    let store = get_store(origin);
    let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.remove(key);
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_clear(origin: &str) {
    let store = get_store(origin);
    let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.clear();
    guard.mark_dirty();
    mark_origin_dirty(origin);
}

pub(crate) fn local_storage_len(origin: &str) -> usize {
    let store = get_store(origin);
    let guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.len()
}

pub(crate) fn local_storage_key(origin: &str, index: usize) -> Option<String> {
    let store = get_store(origin);
    let guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.keys().nth(index).cloned()
}

pub(crate) fn local_storage_byte_size(origin: &str) -> usize {
    let store = get_store(origin);
    let guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.iter().map(|(k, v)| k.len() + v.len()).sum()
}

/// Persist all dirty localStorage stores to disk.
///
/// Call once per frame (after JS eval) rather than on every setItem/removeItem/clear.
/// Only iterates origins that have been modified since the last flush.
pub fn flush_dirty_stores() {
    let dirty: Vec<String> = {
        let mut dirty_set = DIRTY_ORIGINS.lock().unwrap_or_else(|e| e.into_inner());
        let origins: Vec<String> = dirty_set.drain().collect();
        origins
    };

    if dirty.is_empty() {
        return;
    }

    let registry = LOCAL_STORAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    for origin in &dirty {
        if let Some(store) = registry.get(origin) {
            let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
            guard.persist();
        }
    }
}
