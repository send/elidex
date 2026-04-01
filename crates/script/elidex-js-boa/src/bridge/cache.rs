//! HostBridge methods for Cache API state management.

use elidex_storage_core::SqliteConnection;

use super::HostBridge;

impl HostBridge {
    /// Get or lazily initialize the Cache API backend for this origin.
    ///
    /// Uses an in-memory SQLite database. Persistent file-backed storage
    /// will be integrated via `OriginStorageManager` when the shell provides a data directory.
    pub fn ensure_cache_backend(&self) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        if inner.cache_conn.is_none() {
            let conn = SqliteConnection::open_in_memory()
                .map_err(|e| format!("failed to open cache backend: {e}"))?;
            inner.cache_conn = Some(conn);
        }
        Ok(())
    }

    /// Execute a closure with the cache connection.
    pub fn with_cache<R>(
        &self,
        f: impl FnOnce(&SqliteConnection) -> R,
    ) -> Option<R> {
        let inner = self.inner.borrow();
        inner.cache_conn.as_ref().map(f)
    }
}
