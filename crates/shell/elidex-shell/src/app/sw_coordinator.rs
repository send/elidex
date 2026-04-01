//! Service Worker coordinator (browser thread).
//!
//! Manages SW registrations, lifecycle, update checks, and sync events.
//! Runs on the browser thread and communicates with content threads via IPC.

use std::collections::HashMap;

use elidex_api_sw::{
    SwHandle, SwPersistence, SwRegistration, SwRegistrationStore, SwState, SyncManager,
    UpdateChecker, UpdateViaCache,
};

/// Global QuotaManager for navigator.storage API.
///
/// Shared across all tabs. Will be replaced by OriginStorageManager
/// integration in M4-8.5.
static QUOTA: std::sync::LazyLock<elidex_storage_core::QuotaManager> =
    std::sync::LazyLock::new(elidex_storage_core::QuotaManager::new);

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
    /// Registers the SW, spawns a SW thread, and sends Install event.
    pub fn register(
        &mut self,
        script_url: &url::Url,
        scope: &url::Url,
        page_url: &url::Url,
        network_process: &elidex_net::broker::NetworkProcessHandle,
    ) {
        // Validate security constraints against the actual registering page URL.
        if let Err(msg) = elidex_api_sw::validate_registration(script_url, scope, page_url) {
            tracing::warn!(error = %msg, "SW registration rejected");
            return;
        }

        let reg = SwRegistration {
            scope: scope.clone(),
            script_url: script_url.clone(),
            state: SwState::Installing,
            script_hash: None,
            last_update_check: None,
            update_via_cache: UpdateViaCache::default(),
        };

        self.store.register(reg.clone());

        if let Some(ref persistence) = self.persistence {
            let _ = persistence.save(&reg);
        }

        // Spawn SW thread.
        let nh = network_process.create_renderer_handle();
        let (browser_ch, sw_ch) = elidex_plugin::channel_pair();
        let sw_script_url = script_url.clone();
        let sw_scope = scope.clone();
        let thread = std::thread::spawn(move || {
            elidex_js_boa::sw_thread::sw_thread_main(sw_script_url, sw_scope, sw_ch, nh);
        });

        let mut handle = SwHandle::new(scope.clone(), script_url.clone(), browser_ch, thread);
        handle.set_state(SwState::Installing);

        // Send Install event.
        handle.send(elidex_api_sw::ContentToSw::Install);

        self.handles.insert(scope.to_string(), handle);

        tracing::info!(
            scope = %scope,
            script = %script_url,
            "SW thread spawned, Install event sent"
        );
    }

    /// Drain responses from all active SW threads and advance lifecycle.
    ///
    /// Call this each frame from the browser thread event loop.
    pub fn tick(&mut self) {
        let mut to_remove = Vec::new();

        for (scope_key, handle) in &mut self.handles {
            while let Ok(msg) = handle.try_recv() {
                match msg {
                    elidex_api_sw::SwToContent::LifecycleComplete { event, success } => {
                        let scope = handle.scope().clone();
                        match event {
                            elidex_api_sw::LifecycleEvent::Install => {
                                if success {
                                    handle.set_state(SwState::Installed);
                                    // Auto-activate (simplified: no waiting for controlled clients)
                                    handle.send(elidex_api_sw::ContentToSw::Activate);
                                    handle.set_state(SwState::Activating);
                                } else {
                                    handle.set_state(SwState::Redundant);
                                    self.store.set_state(&scope, SwState::Redundant);
                                    to_remove.push(scope_key.clone());
                                }
                            }
                            elidex_api_sw::LifecycleEvent::Activate => {
                                if success {
                                    handle.set_state(SwState::Activated);
                                    self.store.set_state(&scope, SwState::Activated);
                                    if let Some(ref persistence) = self.persistence {
                                        if let Some(reg) = self.store.get_by_scope(&scope) {
                                            let _ = persistence.save(reg);
                                        }
                                    }
                                    tracing::info!(scope = %scope, "SW activated");
                                } else {
                                    handle.set_state(SwState::Redundant);
                                    self.store.set_state(&scope, SwState::Redundant);
                                    to_remove.push(scope_key.clone());
                                }
                            }
                        }
                    }
                    elidex_api_sw::SwToContent::SkipWaiting => {
                        // Force activation immediately.
                        let scope = handle.scope().clone();
                        if handle.state() == SwState::Installed {
                            handle.send(elidex_api_sw::ContentToSw::Activate);
                            handle.set_state(SwState::Activating);
                            self.store.set_state(&scope, SwState::Activating);
                        }
                    }
                    elidex_api_sw::SwToContent::Error { message, .. } => {
                        tracing::warn!(scope = %handle.scope(), error = %message, "SW error");
                    }
                    // FetchResponse, SyncComplete, etc. are handled by the content thread,
                    // not the browser thread coordinator.
                    _ => {}
                }
            }

            // Check if thread died unexpectedly — transition to Redundant.
            if !handle.is_alive() && handle.state() != SwState::Redundant {
                let scope = handle.scope().clone();
                tracing::warn!(scope = %scope, "SW thread terminated unexpectedly");
                self.store.set_state(&scope, SwState::Redundant);
                to_remove.push(scope_key.clone());
            }
        }

        for key in to_remove {
            self.handles.remove(&key);
        }
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

    /// Get quota estimate for an origin (for navigator.storage.estimate()).
    pub fn quota_estimate(
        &self,
        origin: &elidex_storage_core::OriginKey,
    ) -> elidex_storage_core::QuotaEstimate {
        // TODO(M4-8.5): use OriginStorageManager's QuotaManager.
        // For now, QuotaManager tracks in-memory only.
        let _ = &self.sync_manager; // suppress unused field warning
        QUOTA.estimate(origin)
    }

    /// Request persistent storage for an origin.
    pub fn quota_persist(&self, origin: &elidex_storage_core::OriginKey) -> bool {
        let _ = &self.persistence;
        QUOTA.request_persist(origin)
    }

    /// Check if an origin has persistent storage.
    pub fn quota_persisted(&self, origin: &elidex_storage_core::OriginKey) -> bool {
        let _ = &self.persistence;
        QUOTA.is_persisted(origin)
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
