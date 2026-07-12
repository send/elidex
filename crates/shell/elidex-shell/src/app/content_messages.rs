//! Browser-side drain of the content→browser IPC message stream.
//!
//! Extracted verbatim from `app/mod.rs` (no behavior change): the
//! [`ContentToBrowser`] dispatcher is the browser-thread counterpart to the
//! OS-event dispatcher in `threaded.rs`, and a self-contained cohesion seam, so it
//! lives in its own module to keep `app/mod.rs` under the project's 1000-line
//! guideline (touch-time split). [`App::drain_content_messages`] is invoked once per
//! frame from `handle_redraw_threaded`; the per-tab drain cap lives here with it.

use std::sync::Arc;

use crate::ipc::{BrowserToContent, ContentToBrowser};

use super::sw_coordinator;
use super::tab::TabId;
use super::App;

impl App {
    /// Maximum messages to drain per tab per frame.
    ///
    /// Prevents a runaway content thread from monopolizing the browser thread's
    /// event loop. Any remaining messages will be drained on the next frame.
    const MAX_DRAIN_PER_TAB: usize = 1000;

    /// Drain all pending messages from all tabs.
    #[allow(clippy::too_many_lines)]
    pub(super) fn drain_content_messages(&mut self) {
        let Some(mgr) = &mut self.tab_manager else {
            return;
        };
        let mut new_tab_urls: Vec<url::Url> = Vec::new();
        // Collect (source_tab_id, storage_change) for cross-tab broadcast.
        let mut storage_changes: Vec<(TabId, crate::ipc::StorageChangedMsg)> = Vec::new();
        // Collect IDB versionchange requests for cross-tab broadcast.
        // (source_tab, request_id, origin, db_name, old_version, new_version)
        let mut idb_version_change_requests: Vec<(TabId, u64, String, String, u64, Option<u64>)> =
            Vec::new();
        for tab in mgr.tabs_mut() {
            let mut drained = 0;
            while drained < Self::MAX_DRAIN_PER_TAB {
                let Ok(msg) = tab.channel.try_recv() else {
                    break;
                };
                drained += 1;
                match msg {
                    ContentToBrowser::DisplayListReady(dl) => {
                        tab.display_list = dl;
                    }
                    ContentToBrowser::TitleChanged(title) => {
                        tab.window_title = title;
                    }
                    ContentToBrowser::NavigationState {
                        can_go_back,
                        can_go_forward,
                    } => {
                        tab.can_go_back = can_go_back;
                        tab.can_go_forward = can_go_forward;
                    }
                    ContentToBrowser::UrlChanged(url) => {
                        tab.chrome.set_url(&url);
                        tab.current_origin = Some(url.origin().ascii_serialization());
                        // Record visit in browser.sqlite history.
                        if let Some(ref db) = self.browser_db {
                            let title = &tab.window_title;
                            if let Err(e) = db.history().record_visit(
                                &url,
                                title,
                                elidex_storage_core::browser_db::history::TransitionType::Link,
                            ) {
                                tracing::debug!(error = %e, "failed to record history visit");
                            }
                        }
                    }
                    ContentToBrowser::NavigationFailed { url, error } => {
                        eprintln!("Navigation to {url} failed: {error}");
                    }
                    ContentToBrowser::OpenNewTab(url) => {
                        new_tab_urls.push(url);
                    }
                    ContentToBrowser::FocusWindow => {
                        self.pending_focus = true;
                    }
                    ContentToBrowser::StorageChanged {
                        origin,
                        key,
                        old_value,
                        new_value,
                        url,
                    } => {
                        storage_changes.push((
                            tab.id,
                            crate::ipc::StorageChangedMsg {
                                origin,
                                key,
                                old_value,
                                new_value,
                                url,
                            },
                        ));
                    }
                    ContentToBrowser::IdbVersionChangeRequest {
                        request_id,
                        origin,
                        db_name,
                        old_version,
                        new_version,
                    } => {
                        // Broadcast versionchange to all other same-origin tabs.
                        // Note: origin is trusted here because it's computed by the
                        // bridge from SecurityOrigin::from_url (not user-supplied).
                        idb_version_change_requests.push((
                            tab.id,
                            request_id,
                            origin,
                            db_name,
                            old_version,
                            new_version,
                        ));
                    }
                    // No-op at browser level — tracked for future use.
                    ContentToBrowser::SwRegister {
                        script_url,
                        scope,
                        origin: _,
                        page_url,
                        update_via_cache,
                    } => {
                        if let Some(ref np) = self.network_process {
                            // Acquire the per-origin Cache API connection (owned
                            // Arc) up front, releasing the `origin_storage` borrow
                            // immediately so `register` can take `&mut self.…`.
                            let cache_conn = self.origin_storage.as_ref().and_then(|osm| {
                                elidex_storage_core::OriginKey::from_url(&scope)
                                    .and_then(|key| osm.cache_connection(&key).ok())
                            });
                            match cache_conn {
                                Some(cache_conn) => {
                                    self.sw_coordinator.register(
                                        &script_url,
                                        &scope,
                                        &page_url,
                                        update_via_cache,
                                        cache_conn,
                                        np,
                                        &tab.channel,
                                    );
                                }
                                None => {
                                    // A SW with no Cache API connection is
                                    // non-functional; skip the spawn, but the
                                    // register() promise must STILL settle (never
                                    // hang) — reply failure (§4 Part 1 hung-promise
                                    // invariant).
                                    let _ =
                                        tab.channel
                                            .send(crate::ipc::BrowserToContent::SwRegistered(
                                            Box::new(crate::ipc::SwRegisteredData {
                                                scope: scope.clone(),
                                                success: false,
                                                error: Some(
                                                    elidex_api_sw::SwRegisterError::TypeError(
                                                        "service worker cache storage unavailable"
                                                            .to_owned(),
                                                    ),
                                                ),
                                                worker: None,
                                                update_via_cache,
                                            }),
                                        ));
                                }
                            }
                        }
                    }
                    ContentToBrowser::SwUpdate { scope } => {
                        self.sw_coordinator.update(&scope, &tab.channel);
                    }
                    ContentToBrowser::SwUnregister { scope } => {
                        self.sw_coordinator
                            .unregister_and_reply(&scope, &tab.channel);
                    }
                    ContentToBrowser::SwPostMessage {
                        scope,
                        data,
                        origin,
                        client_id,
                    } => {
                        self.sw_coordinator
                            .post_message_to_worker(&scope, data, origin, client_id);
                    }
                    ContentToBrowser::ManifestDiscovered { url } => {
                        tracing::debug!(manifest_url = %url, "manifest discovered");
                        // TODO(M4-8): fetch manifest JSON, parse, apply to window
                    }
                    ContentToBrowser::StorageEstimate { origin_url } => {
                        if let Some(origin_key) =
                            elidex_storage_core::OriginKey::from_url(&origin_url)
                        {
                            let est = self.sw_coordinator.quota_estimate(&origin_key);
                            let _ = tab.channel.send(
                                crate::ipc::BrowserToContent::StorageEstimateResult {
                                    usage: est.usage,
                                    quota: est.quota,
                                },
                            );
                        }
                    }
                    ContentToBrowser::StoragePersist { origin_url } => {
                        if let Some(origin_key) =
                            elidex_storage_core::OriginKey::from_url(&origin_url)
                        {
                            let granted = self.sw_coordinator.quota_persist(&origin_key);
                            let _ = tab.channel.send(
                                crate::ipc::BrowserToContent::StoragePersistResult { granted },
                            );
                        }
                    }
                    ContentToBrowser::StoragePersisted { origin_url } => {
                        if let Some(origin_key) =
                            elidex_storage_core::OriginKey::from_url(&origin_url)
                        {
                            let persisted = self.sw_coordinator.quota_persisted(&origin_key);
                            let _ = tab.channel.send(
                                crate::ipc::BrowserToContent::StoragePersistedResult { persisted },
                            );
                        }
                    }
                    ContentToBrowser::IdbConnectionsClosed { .. } => {}
                    ContentToBrowser::SwFetchRequest {
                        fetch_id,
                        request,
                        client_id: _,
                        resulting_client_id: _,
                    } => {
                        // Route the FetchEvent to the controlling SW via the relay.
                        // TODO(M4-10): Use SwFetchRelay to dispatch ContentToSw::FetchEvent
                        // to the SW handle and route the response back. Currently sends
                        // passthrough because SwCoordinator.handles is not directly accessible
                        // from drain_content_messages (ownership boundary). Full wiring requires
                        // async fetch in M4-10 (elidex-js VM event loop).
                        if let Some(reg) = self.sw_coordinator.find_controller(&request.url) {
                            let scope = reg.scope.clone();
                            let _ =
                                tab.channel
                                    .send(crate::ipc::BrowserToContent::SwFetchResponse {
                                        fetch_id,
                                        response: None, // passthrough
                                    });
                            tracing::debug!(
                                scope = %scope,
                                url = %request.url,
                                "SW FetchEvent relay — passthrough (full wiring pending)"
                            );
                        } else {
                            // No controlling SW — passthrough.
                            let _ =
                                tab.channel
                                    .send(crate::ipc::BrowserToContent::SwFetchResponse {
                                        fetch_id,
                                        response: None,
                                    });
                        }
                    }
                }
            }
        }

        // Broadcast storage changes to other same-origin tabs (WHATWG HTML §11.2.1).
        for (source_tab_id, change) in &storage_changes {
            for tab in mgr.tabs_mut() {
                if tab.id == *source_tab_id {
                    continue;
                }
                // Only send to tabs whose origin matches the storage change origin.
                let tab_matches = tab
                    .current_origin
                    .as_ref()
                    .is_some_and(|o| *o == change.origin);
                if !tab_matches {
                    continue;
                }
                let _ = tab.channel.send(BrowserToContent::StorageEvent {
                    key: change.key.clone(),
                    old_value: change.old_value.clone(),
                    new_value: change.new_value.clone(),
                    url: change.url.clone(),
                });
            }
        }

        // Broadcast IDB versionchange to other same-origin tabs (W3C IndexedDB §2.4).
        for (source_tab_id, request_id, origin, db_name, old_version, new_version) in
            &idb_version_change_requests
        {
            for tab in mgr.tabs_mut() {
                if tab.id == *source_tab_id {
                    continue;
                }
                let tab_matches = tab.current_origin.as_ref().is_some_and(|o| o == origin);
                if !tab_matches {
                    continue;
                }
                let _ = tab.channel.send(BrowserToContent::IdbVersionChange {
                    request_id: *request_id,
                    db_name: db_name.clone(),
                    old_version: *old_version,
                    new_version: *new_version,
                });
            }
            // After broadcasting, immediately send IdbUpgradeReady to the requester.
            // TODO(M4-10): Wait for IdbConnectionsClosed from all tabs or timeout,
            // then send IdbUpgradeReady or IdbBlocked (W3C IndexedDB §2.4).
            for tab in mgr.tabs_mut() {
                if tab.id == *source_tab_id {
                    let _ = tab.channel.send(BrowserToContent::IdbUpgradeReady {
                        request_id: *request_id,
                        db_name: db_name.clone(),
                    });
                    break;
                }
            }
        }

        // Update window title only when the active tab's title changed.
        if let Some(tab) = mgr.active_tab() {
            if let Some(state) = &self.render_state {
                if state.window.title() != tab.window_title {
                    state.window.set_title(&tab.window_title);
                }
            }
        }

        // Tick SW coordinator — drain lifecycle responses, advance state.
        self.sw_coordinator.tick();

        // Broadcast SW back-channel updates to same-origin tabs (WHATWG SW
        // §3.1/§3.4, DR-B — drives navigator.serviceWorker state/controller).
        // `BrowserToContent` is not `Clone`, so reconstruct per recipient.
        for update in self.sw_coordinator.drain_client_broadcasts() {
            let scope_origin = update.scope().origin().ascii_serialization();
            for tab in mgr.tabs_mut() {
                let same_origin = tab
                    .current_origin
                    .as_ref()
                    .is_some_and(|o| *o == scope_origin);
                if !same_origin {
                    continue;
                }
                let msg = match &update {
                    sw_coordinator::SwClientBroadcast::StateChanged { scope, state } => {
                        BrowserToContent::SwStateChanged {
                            scope: scope.clone(),
                            state: *state,
                        }
                    }
                    sw_coordinator::SwClientBroadcast::ControllerSet { scope } => {
                        BrowserToContent::SwControllerSet {
                            scope: scope.clone(),
                        }
                    }
                };
                let _ = tab.channel.send(msg);
            }
        }

        // Open new tabs requested by window.open(). Born at the real viewport (C1):
        // the new content thread reads the shared `viewport_cell` (the `Arc`-clone
        // below) at build time — post-`resumed` the cell holds the window's published
        // size, and a clone is a disjoint `self.viewport_cell` read that coexists with
        // the active `&mut mgr` borrow. The cell is keyed to the active tab's chrome;
        // exact while every tab uses the default (`Top`) tab-bar position
        // (cf. `open_new_tab`) → slot #11-window-level-tab-bar-position.
        for url in new_tab_urls {
            let (browser_chan, content_chan) = crate::ipc::channel_pair();
            let title = format!("elidex \u{2014} {url}");
            let chrome = crate::chrome::ChromeState::new(Some(&url));
            if let Some(np) = &self.network_process {
                let nh = np.create_renderer_handle();
                let jar = Arc::clone(np.cookie_jar());
                let web_storage = Arc::clone(&self.web_storage);
                // Mint via the disjoint `wake_proxy` field (an associated fn, not
                // `&self`) so it coexists with the active `&mut mgr` borrow.
                let wake = Self::wake_or_noop(self.wake_proxy.as_ref());
                let thread = crate::content::spawn_content_thread_url(
                    content_chan,
                    nh,
                    jar,
                    web_storage,
                    url,
                    Arc::clone(&self.viewport.viewport_cell),
                    wake,
                );
                mgr.create_tab(browser_chan, thread, chrome, title);
            }
        }
    }
}
