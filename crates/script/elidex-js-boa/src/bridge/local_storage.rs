//! Origin-scoped `localStorage` with disk persistence (WHATWG HTML §11.2).
//!
//! Data is stored as JSON files in the platform data directory:
//! `{data_dir}/elidex/localStorage/{origin_hash}.json`
//!
//! Writes use atomic file operations (write to temp file + rename) to
//! prevent corruption on crash.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Global registry of localStorage instances, keyed by origin string.
///
/// `Arc<Mutex<...>>` so multiple tabs with the same origin share one store.
static LOCAL_STORAGE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<LocalStore>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// In-memory + disk-backed storage for a single origin.
struct LocalStore {
    data: HashMap<String, String>,
    file_path: PathBuf,
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
        Self { data, file_path }
    }

    /// Write to disk atomically (temp file + rename).
    fn persist(&self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp_path = self.file_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_string(&self.data) {
            if std::fs::write(&tmp_path, json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &self.file_path);
            }
        }
    }
}

/// Compute the storage file path for an origin.
fn storage_file_path(origin: &str) -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elidex")
        .join("localStorage");

    // Hash the origin for a safe filename.
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    origin.hash(&mut hasher);
    let hash = hasher.finish();
    base.join(format!("{hash:016x}.json"))
}

/// Get or create the shared store for an origin.
fn get_store(origin: &str) -> Arc<Mutex<LocalStore>> {
    let mut registry = LOCAL_STORAGE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    registry
        .entry(origin.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(LocalStore::load(origin))))
        .clone()
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
    guard.persist();
}

pub(crate) fn local_storage_remove(origin: &str, key: &str) {
    let store = get_store(origin);
    let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.remove(key);
    guard.persist();
}

pub(crate) fn local_storage_clear(origin: &str) {
    let store = get_store(origin);
    let mut guard = store.lock().unwrap_or_else(|e| e.into_inner());
    guard.data.clear();
    guard.persist();
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
