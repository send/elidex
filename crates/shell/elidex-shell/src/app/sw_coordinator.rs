//! Service Worker coordinator (browser thread).
//!
//! Manages SW registrations, lifecycle, update checks, and sync events.
//! Runs on the browser thread and communicates with content threads via IPC.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use elidex_storage_core::SqliteConnection;

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
/// Client type (WHATWG SW §4.2 `ClientType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ClientType {
    Window,
    Worker,
    SharedWorker,
}

/// Frame type (WHATWG SW §4.2 `FrameType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FrameType {
    TopLevel,
    Nested,
    Auxiliary,
    None,
}

/// Visibility state (Page Visibility spec §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum VisibilityState {
    Visible,
    Hidden,
}

/// Tracked state for a controlled client (WHATWG SW §4.2 Client).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ClientState {
    pub id: String,
    pub url: String,
    pub client_type: ClientType,
    pub frame_type: FrameType,
    pub visibility: VisibilityState,
    pub focused: bool,
}

/// A back-channel update produced by `tick()`'s lifecycle advance, buffered
/// for the app loop to broadcast to same-origin content tabs (WHATWG SW
/// §3.1/§3.4, DR-B).
///
/// `tick()` has no per-content reply channel (unlike `register()`), so it
/// stages updates here and `App` drains them after `tick()` and reconstructs
/// the per-tab `BrowserToContent` message (which is not `Clone`).  boa's
/// content loop drops these today; the content→VM consumer wire is D-26.
#[derive(Clone, Debug)]
pub enum SwClientBroadcast {
    /// A worker's lifecycle state advanced → `BrowserToContent::SwStateChanged`.
    StateChanged { scope: url::Url, state: SwState },
    /// Control was established for a scope → `BrowserToContent::SwControllerSet`.
    ControllerSet { scope: url::Url },
}

impl SwClientBroadcast {
    /// The registration scope this update concerns (its origin is the
    /// same-origin broadcast routing key).
    #[must_use]
    pub fn scope(&self) -> &url::Url {
        match self {
            Self::StateChanged { scope, .. } | Self::ControllerSet { scope } => scope,
        }
    }
}

#[allow(dead_code)]
pub struct SwCoordinator {
    store: SwRegistrationStore,
    persistence: Option<SwPersistence>,
    handles: HashMap<String, SwHandle>,
    update_checker: UpdateChecker,
    sync_manager: SyncManager,
    /// Active clients tracked for clients.matchAll()/get().
    client_states: HashMap<String, ClientState>,
    /// Back-channel updates staged by `tick()` for the app loop to broadcast
    /// to same-origin content tabs (drained via `drain_client_broadcasts`).
    client_broadcasts: Vec<SwClientBroadcast>,
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
            client_states: HashMap::new(),
            client_broadcasts: Vec::new(),
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
            client_states: HashMap::new(),
            handles: HashMap::new(),
            update_checker: UpdateChecker::new(),
            sync_manager: SyncManager::new(),
            client_broadcasts: Vec::new(),
        }
    }

    /// Handle a SW registration request from a content thread.
    ///
    /// Registers the SW, spawns a SW thread, and sends Install event.
    /// Sends `SwRegistered(success: false)` back on validation failure.
    pub fn register(
        &mut self,
        script_url: &url::Url,
        scope: &url::Url,
        page_url: &url::Url,
        update_via_cache: UpdateViaCache,
        cache_conn: Arc<Mutex<SqliteConnection>>,
        network_process: &elidex_net::broker::NetworkProcessHandle,
        reply_channel: &elidex_plugin::LocalChannel<
            crate::ipc::BrowserToContent,
            crate::ipc::ContentToBrowser,
        >,
    ) {
        // Validate security constraints against the actual registering page URL.
        // `validate_registration` owns the whole scheme/origin/scope-path/secure
        // decision (engine-indep); we only forward its message (WHATWG SW §3.1).
        if let Err(err) = elidex_api_sw::validate_registration(script_url, scope, page_url) {
            tracing::warn!(error = %err, "SW registration rejected");
            let _ = reply_channel.send(crate::ipc::BrowserToContent::SwRegistered(Box::new(
                crate::ipc::SwRegisteredData {
                    scope: scope.clone(),
                    success: false,
                    error: Some(err),
                    worker: None,
                    update_via_cache,
                },
            )));
            return;
        }

        let reg = SwRegistration {
            scope: scope.clone(),
            script_url: script_url.clone(),
            state: SwState::Installing,
            script_hash: None,
            last_update_check: None,
            update_via_cache,
        };

        self.store.register(reg.clone());

        if let Some(ref persistence) = self.persistence {
            let _ = persistence.save(&reg);
        }

        // Marshal the initial client set for the SW realm's `clients.matchAll()`
        // (WHATWG SW §4.2), filtered to clients whose origin matches the SW's
        // scope origin. `client_states` is empty today (no `register_client`
        // caller yet, §6.1), so this yields `[]` = boa parity; it is written
        // correctly regardless.
        let scope_origin = scope.origin();
        let initial_clients: Vec<elidex_api_sw::ClientSnapshot> = self
            .client_states
            .values()
            .filter(|c| {
                url::Url::parse(&c.url)
                    .map(|u| u.origin() == scope_origin)
                    .unwrap_or(false)
            })
            .map(client_state_to_snapshot)
            .collect();

        // Spawn SW thread.
        let nh = network_process.create_renderer_handle();
        let (browser_ch, sw_ch) = elidex_plugin::channel_pair();
        let sw_script_url = script_url.clone();
        let sw_scope = scope.clone();
        let thread = std::thread::spawn(move || {
            elidex_js::vm::sw_thread::sw_thread_main(
                sw_script_url,
                sw_scope,
                sw_ch,
                nh,
                cache_conn,
                initial_clients,
            );
        });

        let mut handle = SwHandle::new(scope.clone(), script_url.clone(), browser_ch, thread);
        handle.set_state(SwState::Installing);

        // Send Install event.
        handle.send(elidex_api_sw::ContentToSw::Install);

        self.handles.insert(scope.to_string(), handle);

        // WHATWG SW §3.1: register() resolves once the registration is *created*
        // (not after activation).  Notify the registrant so its register()
        // promise can settle with the new registration (DR-B success path).
        let _ = reply_channel.send(crate::ipc::BrowserToContent::SwRegistered(Box::new(
            crate::ipc::SwRegisteredData {
                scope: scope.clone(),
                success: true,
                error: None,
                worker: Some(elidex_api_sw::SwWorkerSnapshot {
                    script_url: script_url.to_string(),
                    state: elidex_api_sw::SwState::Installing,
                }),
                update_via_cache,
            },
        )));

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
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::StateChanged {
                                            scope: scope.clone(),
                                            state: SwState::Installed,
                                        });
                                    // Auto-activate (simplified: no waiting for controlled clients)
                                    handle.send(elidex_api_sw::ContentToSw::Activate);
                                    handle.set_state(SwState::Activating);
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::StateChanged {
                                            scope: scope.clone(),
                                            state: SwState::Activating,
                                        });
                                } else {
                                    handle.set_state(SwState::Redundant);
                                    self.store.set_state(&scope, SwState::Redundant);
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::StateChanged {
                                            scope: scope.clone(),
                                            state: SwState::Redundant,
                                        });
                                    to_remove.push(scope_key.clone());
                                }
                            }
                            elidex_api_sw::LifecycleEvent::Activate => {
                                if success {
                                    handle.set_state(SwState::Activated);
                                    self.store.set_state(&scope, SwState::Activated);
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::StateChanged {
                                            scope: scope.clone(),
                                            state: SwState::Activated,
                                        });
                                    // The active worker now controls its scope.
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::ControllerSet {
                                            scope: scope.clone(),
                                        });
                                    if let Some(ref persistence) = self.persistence {
                                        if let Some(reg) = self.store.get_by_scope(&scope) {
                                            let _ = persistence.save(reg);
                                        }
                                    }
                                    tracing::info!(scope = %scope, "SW activated");
                                } else {
                                    handle.set_state(SwState::Redundant);
                                    self.store.set_state(&scope, SwState::Redundant);
                                    self.client_broadcasts
                                        .push(SwClientBroadcast::StateChanged {
                                            scope: scope.clone(),
                                            state: SwState::Redundant,
                                        });
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
                            self.client_broadcasts
                                .push(SwClientBroadcast::StateChanged {
                                    scope,
                                    state: SwState::Activating,
                                });
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
                self.client_broadcasts
                    .push(SwClientBroadcast::StateChanged {
                        scope,
                        state: SwState::Redundant,
                    });
                to_remove.push(scope_key.clone());
            }
        }

        for key in to_remove {
            self.handles.remove(&key);
        }
    }

    /// Take the back-channel updates staged by `tick()` so the app loop can
    /// broadcast them to same-origin content tabs (DR-B, WHATWG SW §3.1/§3.4).
    pub fn drain_client_broadcasts(&mut self) -> Vec<SwClientBroadcast> {
        std::mem::take(&mut self.client_broadcasts)
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

    /// Handle a `ServiceWorkerRegistration.update()` request (WHATWG SW §3.2.8).
    ///
    /// Minimal-settle form: this does NOT re-fetch or re-install the SW script
    /// (the full update algorithm — byte comparison, install, skipWaiting — is
    /// carved to `#11-sw-update-full-algorithm`). It records the update check
    /// and settles the update() promise with the registration's current worker,
    /// which shares the `SwRegistered` deliver path with `register()`.
    pub fn update(
        &mut self,
        scope: &url::Url,
        reply_channel: &elidex_plugin::LocalChannel<
            crate::ipc::BrowserToContent,
            crate::ipc::ContentToBrowser,
        >,
    ) {
        // Clone the needed fields out of the `&SwRegistration` before taking
        // `&mut self` for `record_update_check` / building the reply.
        let found = self
            .store
            .get_by_scope(scope)
            .map(|reg| (reg.script_url.clone(), reg.state, reg.update_via_cache));
        match found {
            Some((script_url, state, update_via_cache)) => {
                self.record_update_check(&script_url);
                let _ = reply_channel.send(crate::ipc::BrowserToContent::SwRegistered(Box::new(
                    crate::ipc::SwRegisteredData {
                        scope: scope.clone(),
                        success: true,
                        error: None,
                        worker: Some(elidex_api_sw::SwWorkerSnapshot {
                            script_url: script_url.to_string(),
                            state,
                        }),
                        update_via_cache,
                    },
                )));
            }
            None => {
                let _ = reply_channel.send(crate::ipc::BrowserToContent::SwRegistered(Box::new(
                    crate::ipc::SwRegisteredData {
                        scope: scope.clone(),
                        success: false,
                        error: Some(elidex_api_sw::SwRegisterError::TypeError(
                            "no registration to update".into(),
                        )),
                        worker: None,
                        update_via_cache: UpdateViaCache::default(),
                    },
                )));
            }
        }
    }

    /// Handle a `ServiceWorkerRegistration.unregister()` request (WHATWG SW
    /// §3.2.9) and settle its promise. Per §6.6 there is no teardown ack:
    /// `unregister()` resolves on REMOVAL, and the synchronous handle drop +
    /// immediate reply is spec-aligned.
    pub fn unregister_and_reply(
        &mut self,
        scope: &url::Url,
        reply_channel: &elidex_plugin::LocalChannel<
            crate::ipc::BrowserToContent,
            crate::ipc::ContentToBrowser,
        >,
    ) {
        let success = self.unregister(scope);
        let _ = reply_channel.send(crate::ipc::BrowserToContent::SwUnregistered {
            scope: scope.clone(),
            success,
        });
    }

    /// Deliver a `ServiceWorker.postMessage()` (WHATWG SW §3.1.4) to the worker
    /// at `scope`. Fire-and-forget: no reply. `origin`/`client_id` are the
    /// sender's, captured on the content thread at enqueue.
    pub fn post_message_to_worker(
        &self,
        scope: &url::Url,
        data: String,
        origin: String,
        client_id: String,
    ) {
        if let Some(handle) = self.handles.get(scope.as_str()) {
            handle.send(elidex_api_sw::ContentToSw::PostMessage {
                data,
                origin,
                client_id,
            });
        }
    }

    /// Get quota estimate for an origin (for navigator.storage.estimate()).
    #[allow(clippy::unused_self)]
    pub fn quota_estimate(
        &self,
        origin: &elidex_storage_core::OriginKey,
    ) -> elidex_storage_core::QuotaEstimate {
        // TODO(M4-8.5): use OriginStorageManager's QuotaManager.
        // For now, QuotaManager tracks in-memory only.
        QUOTA.estimate(origin)
    }

    /// Request persistent storage for an origin.
    #[allow(clippy::unused_self)]
    pub fn quota_persist(&self, origin: &elidex_storage_core::OriginKey) -> bool {
        QUOTA.request_persist(origin)
    }

    /// Check if an origin has persistent storage.
    #[allow(clippy::unused_self)]
    pub fn quota_persisted(&self, origin: &elidex_storage_core::OriginKey) -> bool {
        QUOTA.is_persisted(origin)
    }

    /// Shut down all active SW threads.
    pub fn shutdown_all(&mut self) {
        for (_, mut handle) in self.handles.drain() {
            handle.shutdown();
        }
    }

    // --- Client tracking (WHATWG SW §4.2-4.3) ---

    /// Register or update a controlled client.
    pub fn register_client(&mut self, state: ClientState) {
        self.client_states.insert(state.id.clone(), state);
    }

    /// Unregister a client (e.g., tab closed).
    pub fn unregister_client(&mut self, client_id: &str) {
        self.client_states.remove(client_id);
    }

    /// Get all tracked clients (for `clients.matchAll()`).
    pub fn all_clients(&self) -> Vec<&ClientState> {
        self.client_states.values().collect()
    }

    /// Get a specific client by ID (for `clients.get(id)`).
    pub fn get_client(&self, id: &str) -> Option<&ClientState> {
        self.client_states.get(id)
    }

    /// Check if any client is visible (for Background Sync).
    pub fn has_foreground_client(&self) -> bool {
        self.client_states
            .values()
            .any(|c| c.visibility == VisibilityState::Visible)
    }
}

/// Marshal a browser-thread [`ClientState`] into the engine-independent,
/// `Send` [`elidex_api_sw::ClientSnapshot`] pushed to the SW thread (WHATWG
/// SW §4.2 `Client`). Pure enum-family + field copy; the local and api enums
/// are variant-for-variant identical.
fn client_state_to_snapshot(c: &ClientState) -> elidex_api_sw::ClientSnapshot {
    elidex_api_sw::ClientSnapshot {
        id: c.id.clone(),
        url: c.url.clone(),
        client_type: match c.client_type {
            ClientType::Window => elidex_api_sw::ClientType::Window,
            ClientType::Worker => elidex_api_sw::ClientType::Worker,
            ClientType::SharedWorker => elidex_api_sw::ClientType::SharedWorker,
        },
        frame_type: match c.frame_type {
            FrameType::TopLevel => elidex_api_sw::FrameType::TopLevel,
            FrameType::Nested => elidex_api_sw::FrameType::Nested,
            FrameType::Auxiliary => elidex_api_sw::FrameType::Auxiliary,
            FrameType::None => elidex_api_sw::FrameType::None,
        },
        visibility: match c.visibility {
            VisibilityState::Visible => elidex_api_sw::VisibilityState::Visible,
            VisibilityState::Hidden => elidex_api_sw::VisibilityState::Hidden,
        },
        focused: c.focused,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_state_to_snapshot_converts_all_enum_variants() {
        let cases = [
            (
                ClientType::Window,
                FrameType::TopLevel,
                VisibilityState::Visible,
                true,
            ),
            (
                ClientType::Worker,
                FrameType::Nested,
                VisibilityState::Hidden,
                false,
            ),
            (
                ClientType::SharedWorker,
                FrameType::Auxiliary,
                VisibilityState::Visible,
                true,
            ),
            (
                ClientType::Window,
                FrameType::None,
                VisibilityState::Hidden,
                false,
            ),
        ];

        for (client_type, frame_type, visibility, focused) in cases {
            let state = ClientState {
                id: "client-id".to_owned(),
                url: "https://example.com/page".to_owned(),
                client_type,
                frame_type,
                visibility,
                focused,
            };
            let snap = client_state_to_snapshot(&state);

            assert_eq!(snap.id, state.id);
            assert_eq!(snap.url, state.url);
            assert_eq!(snap.focused, focused);

            let expected_type = match client_type {
                ClientType::Window => elidex_api_sw::ClientType::Window,
                ClientType::Worker => elidex_api_sw::ClientType::Worker,
                ClientType::SharedWorker => elidex_api_sw::ClientType::SharedWorker,
            };
            let expected_frame = match frame_type {
                FrameType::TopLevel => elidex_api_sw::FrameType::TopLevel,
                FrameType::Nested => elidex_api_sw::FrameType::Nested,
                FrameType::Auxiliary => elidex_api_sw::FrameType::Auxiliary,
                FrameType::None => elidex_api_sw::FrameType::None,
            };
            let expected_vis = match visibility {
                VisibilityState::Visible => elidex_api_sw::VisibilityState::Visible,
                VisibilityState::Hidden => elidex_api_sw::VisibilityState::Hidden,
            };
            assert_eq!(snap.client_type, expected_type);
            assert_eq!(snap.frame_type, expected_frame);
            assert_eq!(snap.visibility, expected_vis);
        }
    }

    // --- 2e-b settle-deliver round-trips ---
    //
    // These exercise the browser→content half of the SW client-request cutover:
    // the `SwCoordinator` handlers 2e-b adds (`update` / `unregister_and_reply`
    // / `post_message_to_worker`) must emit the `BrowserToContent` reply that the
    // content-thread `deliver_sw_client_update` bracket settles the client
    // promise from. The content/VM half (promise resolution + event firing) is
    // covered by `elidex-js` `tests_service_worker_client` (register/unregister
    // resolve, worker identity). NOTE: the shell lib does not yet compile during
    // the in-progress S5-6b flip (20 stage-2f errors), so these do not run until
    // stage 3 — they are written to pass then.

    /// A `BrowserToContent` reply channel end (what a coordinator handler sends
    /// on) plus the content-side end that reads it back.
    fn reply_pair() -> (
        elidex_plugin::LocalChannel<crate::ipc::BrowserToContent, crate::ipc::ContentToBrowser>,
        elidex_plugin::LocalChannel<crate::ipc::ContentToBrowser, crate::ipc::BrowserToContent>,
    ) {
        elidex_plugin::channel_pair::<crate::ipc::BrowserToContent, crate::ipc::ContentToBrowser>()
    }

    fn reg_at(scope: &str, script: &str, state: SwState, uvc: UpdateViaCache) -> SwRegistration {
        SwRegistration {
            scope: url::Url::parse(scope).unwrap(),
            script_url: url::Url::parse(script).unwrap(),
            state,
            script_hash: None,
            last_update_check: None,
            update_via_cache: uvc,
        }
    }

    #[test]
    fn update_existing_registration_settles_with_worker_and_cache() {
        let mut coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/app/").unwrap();
        coord.store.register(reg_at(
            "https://example.com/app/",
            "https://example.com/sw.js",
            SwState::Activated,
            UpdateViaCache::None,
        ));

        let (browser_end, content_end) = reply_pair();
        coord.update(&scope, &browser_end);

        match content_end.try_recv() {
            Ok(crate::ipc::BrowserToContent::SwRegistered(data)) => {
                assert_eq!(data.scope, scope);
                assert!(data.success);
                assert!(data.error.is_none());
                let worker = data.worker.expect("worker snapshot on success");
                assert_eq!(worker.script_url, "https://example.com/sw.js");
                assert_eq!(worker.state, SwState::Activated);
                assert_eq!(data.update_via_cache, UpdateViaCache::None);
            }
            other => panic!("expected SwRegistered, got {other:?}"),
        }
    }

    #[test]
    fn update_missing_registration_replies_typeerror() {
        let mut coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/nope/").unwrap();

        let (browser_end, content_end) = reply_pair();
        coord.update(&scope, &browser_end);

        match content_end.try_recv() {
            Ok(crate::ipc::BrowserToContent::SwRegistered(data)) => {
                assert!(!data.success);
                assert!(data.worker.is_none());
                assert!(matches!(
                    data.error,
                    Some(elidex_api_sw::SwRegisterError::TypeError(_))
                ));
            }
            other => panic!("expected SwRegistered failure, got {other:?}"),
        }
    }

    #[test]
    fn unregister_and_reply_removes_and_settles_true() {
        let mut coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/app/").unwrap();
        coord.store.register(reg_at(
            "https://example.com/app/",
            "https://example.com/sw.js",
            SwState::Activated,
            UpdateViaCache::default(),
        ));
        assert!(coord.store.get_by_scope(&scope).is_some());

        let (browser_end, content_end) = reply_pair();
        coord.unregister_and_reply(&scope, &browser_end);

        match content_end.try_recv() {
            Ok(crate::ipc::BrowserToContent::SwUnregistered { scope: s, success }) => {
                assert_eq!(s, scope);
                assert!(success);
            }
            other => panic!("expected SwUnregistered, got {other:?}"),
        }
        assert!(coord.store.get_by_scope(&scope).is_none());
    }

    #[test]
    fn unregister_and_reply_missing_settles_false() {
        let mut coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/nope/").unwrap();

        let (browser_end, content_end) = reply_pair();
        coord.unregister_and_reply(&scope, &browser_end);

        match content_end.try_recv() {
            Ok(crate::ipc::BrowserToContent::SwUnregistered { success, .. }) => {
                assert!(!success);
            }
            other => panic!("expected SwUnregistered, got {other:?}"),
        }
    }

    #[test]
    fn post_message_to_worker_reaches_sw_channel() {
        let mut coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/app/").unwrap();
        let script = url::Url::parse("https://example.com/sw.js").unwrap();

        // Install a handle whose worker end we can read back.
        let (parent_ch, worker_ch) =
            elidex_plugin::channel_pair::<elidex_api_sw::ContentToSw, elidex_api_sw::SwToContent>();
        let handle = SwHandle::new(scope.clone(), script, parent_ch, std::thread::spawn(|| {}));
        coord.handles.insert(scope.to_string(), handle);

        coord.post_message_to_worker(
            &scope,
            "payload".to_owned(),
            "https://example.com".to_owned(),
            "client-42".to_owned(),
        );

        match worker_ch.try_recv() {
            Ok(elidex_api_sw::ContentToSw::PostMessage {
                data,
                origin,
                client_id,
            }) => {
                assert_eq!(data, "payload");
                assert_eq!(origin, "https://example.com");
                assert_eq!(client_id, "client-42");
            }
            other => panic!("expected ContentToSw::PostMessage, got {other:?}"),
        }
    }

    #[test]
    fn post_message_to_worker_no_handle_is_noop() {
        let coord = SwCoordinator::new();
        let scope = url::Url::parse("https://example.com/gone/").unwrap();
        // No handle registered — must not panic (fire-and-forget).
        coord.post_message_to_worker(
            &scope,
            "x".to_owned(),
            "https://example.com".to_owned(),
            "c".to_owned(),
        );
    }
}
