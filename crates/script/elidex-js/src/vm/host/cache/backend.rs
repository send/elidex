//! Cache API backend wrapper (`#11-cache-api-vm` / D-19 PR-1, DR-A).
//!
//! Owns the origin-keyed `Arc<Mutex<SqliteConnection>>` that the engine-
//! independent `elidex-cache-api` storage ops run against, and exposes a
//! single lock-and-call [`CacheBackend::with_conn`] so the `MutationGuard`
//! lifetime never leaks into the native bodies (mirrors the boa
//! `bridge/cache.rs` `with_cache` helper, but holding a `Mutex` so the
//! handle is `Send + Sync` and can later cross to the service-worker
//! thread — DR-A / §4.1).
//!
//! ## Sharing model (DR-A)
//!
//! The shell installs one `Arc<CacheBackend>` per origin into every
//! browsing-context VM's [`HostData`](super::super::super::host_data::HostData)
//! (via `install_cache_storage`), so the window realm — and, in PR-2, the
//! SW realm — observe one shared cache store within a session.  When no
//! shell backend is installed (headless / unit-test VMs), the VM lazily
//! mints an in-memory one ([`CacheBackend::in_memory`], boa `ensure_cache_backend`
//! parity).  `elidex-cache-api` is connection-agnostic, so swapping in an
//! on-disk connection later (slot `#11-cache-shared-ondisk-store`) is a
//! wiring change with no crate/API change.

#![cfg(feature = "engine")]

use std::sync::{Arc, Mutex};

use elidex_storage_core::SqliteConnection;

/// Owns the shared Cache API SQLite connection for one origin.
pub(crate) struct CacheBackend {
    /// The shared origin handle.  `Arc<Mutex<…>>` (not `Rc`) so the same
    /// connection can be handed to the service-worker thread in PR-2:
    /// `SqliteConnection` is `Send` but `!Sync`, so the `Mutex` makes the
    /// wrapper `Send + Sync` (DR-A / F12).
    conn: Arc<Mutex<SqliteConnection>>,
}

impl CacheBackend {
    /// Wrap a shared origin `Arc<Mutex<SqliteConnection>>` (DR-A): the
    /// `OriginStorageManager::cache_connection` handle the coordinator hands
    /// to both the window VM and the SW thread so they observe one shared
    /// cache store within a session (D-19 PR-2 / D-26).  `Send + Sync`, so it
    /// crosses the spawn boundary into the SW thread.
    pub(crate) fn new(conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { conn }
    }

    /// Mint a fresh in-memory backend (boa `ensure_cache_backend` parity).
    /// Used when no shell-owned origin handle was installed — the data is
    /// per-VM and lost on `Vm::unbind`, matching the storage fallback.
    pub(crate) fn in_memory() -> Result<Self, String> {
        let conn = SqliteConnection::open_in_memory()
            .map_err(|e| format!("failed to open cache backend: {e}"))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run `f` against the locked connection.  The guard is dropped before
    /// the call returns, so no `MutexGuard` is held across the Promise
    /// deliver boundary (DR-A).  Poisoned-lock recovery is `expect`: a
    /// poisoned cache mutex means a prior holder panicked mid-op, which is
    /// an engine bug, not a recoverable JS condition.
    pub(crate) fn with_conn<R>(&self, f: impl FnOnce(&SqliteConnection) -> R) -> R {
        let guard = self.conn.lock().expect("cache connection mutex poisoned");
        f(&guard)
    }
}
