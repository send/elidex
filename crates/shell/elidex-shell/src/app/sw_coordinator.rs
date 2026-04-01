//! Service Worker coordinator (browser thread).
//!
//! Manages SW registrations, lifecycle, update checks, and sync events.
//! Runs on the browser thread and communicates with content threads via IPC.

use std::collections::HashMap;

use elidex_api_sw::{
    SwHandle, SwPersistence, SwRegistration, SwRegistrationStore, SwState, SyncManager,
    UpdateChecker, UpdateViaCache,
};

/// Browser-thread Service Worker coordinator.
///
/// Owns the registration store, persistence layer, active SW handles,
/// update checker, and sync manager.
#[allow(dead_code)]
pub struct SwCoordinator {
    store: SwRegistrationStore,
    persistence: Option<SwPersistence>,
    handles: HashMap<String, SwHandle>,
    update_checker: UpdateChecker,
    sync_manager: SyncManager,
}

#[allow(dead_code)]
impl SwCoordinator {
    /// Create a new coordinator (without persistence — in-memory only).
    pub fn new() -> Self {
        Self {
            store: SwRegistrationStore::new(),
            persistence: None,
            handles: HashMap::new(),
            update_checker: UpdateChecker::new(),
            sync_manager: SyncManager::new(),
        }
    }

    /// Create with persistence (loads saved registrations from SQLite).
    pub fn with_persistence(persistence: SwPersistence) -> Self {
        let mut store = SwRegistrationStore::new();

        // Load persisted registrations.
        if let Ok(registrations) = persistence.load_all() {
            for reg in registrations {
                store.register(reg);
            }
        }

        Self {
            store,
            persistence: Some(persistence),
            handles: HashMap::new(),
            update_checker: UpdateChecker::new(),
            sync_manager: SyncManager::new(),
        }
    }

    /// Handle a SW registration request from a content thread.
    ///
    /// Creates a registration entry. The actual script fetch + SW thread
    /// spawn is deferred until the network process is available.
    pub fn register(&mut self, script_url: &url::Url, scope: &url::Url) {
        let reg = SwRegistration {
            scope: scope.clone(),
            script_url: script_url.clone(),
            state: SwState::Parsed,
            script_hash: None,
            last_update_check: None,
            update_via_cache: UpdateViaCache::default(),
        };

        self.store.register(reg.clone());

        if let Some(ref persistence) = self.persistence {
            let _ = persistence.save(&reg);
        }

        tracing::info!(
            scope = %scope,
            script = %script_url,
            "SW registered"
        );
    }

    /// Check if a URL is controlled by an active SW.
    pub fn find_controller(&self, url: &url::Url) -> Option<&SwRegistration> {
        elidex_api_sw::find_registration(self.store.all(), url)
    }

    /// Check if an update should be performed for a SW (soft update on navigation).
    pub fn should_update(&self, script_url: &url::Url) -> bool {
        self.update_checker.should_soft_update(script_url)
    }

    /// Record that an update check was performed.
    pub fn record_update_check(&mut self, script_url: &url::Url) {
        self.update_checker.record_check(script_url);
    }

    /// Get the sync manager (for Background Sync events).
    pub fn sync_manager(&self) -> &SyncManager {
        &self.sync_manager
    }

    /// Get mutable sync manager.
    pub fn sync_manager_mut(&mut self) -> &mut SyncManager {
        &mut self.sync_manager
    }

    /// Unregister a SW by scope.
    pub fn unregister(&mut self, scope: &url::Url) -> bool {
        if let Some(handle) = self.handles.remove(scope.as_str()) {
            drop(handle); // sends Shutdown
        }
        let removed = self.store.unregister(scope);
        if removed {
            if let Some(ref persistence) = self.persistence {
                let _ = persistence.delete(scope);
            }
        }
        removed
    }

    /// Shut down all active SW threads.
    pub fn shutdown_all(&mut self) {
        for (_, mut handle) in self.handles.drain() {
            handle.shutdown();
        }
    }
}

impl Default for SwCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SwCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwCoordinator")
            .field("registrations", &self.store.all().len())
            .field("active_handles", &self.handles.len())
            .finish_non_exhaustive()
    }
}
