//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, frame rendering via Vello, and user input
//! event dispatch to the DOM.
//!
//! Supports two modes:
//! - **Threaded** (`TabManager`): each tab runs on a dedicated content thread,
//!   communicating via message passing.
//! - **Legacy inline** (`InteractiveState`): all processing on the main
//!   thread (used by `build_pipeline` test API).

pub(crate) mod events;
pub(crate) mod hover;
mod inline;
pub(crate) mod navigation;
mod render;
pub(crate) mod sw_coordinator;
#[allow(dead_code)] // Infrastructure for FetchEvent IPC wiring — callers added in next commit.
pub(crate) mod sw_fetch_relay;
pub(crate) mod tab;
mod threaded;

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use elidex_ecs::Entity;
use elidex_navigation::NavigationController;
use elidex_plugin::{Point, Size};
use wgpu::util::TextureBlitter;
use wgpu::{Instance, Surface};

use crate::chrome::{self, TabBarInfo};
use crate::ipc::{BrowserToContent, ContentToBrowser, ModifierState};

use render::try_init_render_state;
use tab::{Tab, TabId, TabManager};

/// Convert a `CookieSnapshot` to a `PersistedCookie` for DB persistence.
fn snap_to_persisted(
    snap: elidex_net::CookieSnapshot,
) -> elidex_storage_core::browser_db::cookies::PersistedCookie {
    use elidex_storage_core::browser_db::cookies::system_time_to_unix;
    elidex_storage_core::browser_db::cookies::PersistedCookie {
        host: snap.host,
        path: snap.path,
        name: snap.name,
        partition_key: snap.partition_key,
        value: snap.value,
        domain: snap.domain,
        host_only: snap.host_only,
        persistent: snap.persistent,
        expires: snap.expires.map(system_time_to_unix),
        secure: snap.secure,
        httponly: snap.http_only,
        samesite: snap.same_site,
        creation_time: system_time_to_unix(snap.creation_time),
        last_access_time: system_time_to_unix(snap.last_access_time),
    }
}

/// Convert a `PersistedCookie` to a `CookieSnapshot` for CookieJar loading.
fn persisted_to_snap(
    c: elidex_storage_core::browser_db::cookies::PersistedCookie,
) -> elidex_net::CookieSnapshot {
    use elidex_storage_core::browser_db::cookies::unix_to_system_time;
    let now = std::time::SystemTime::now();
    elidex_net::CookieSnapshot {
        name: c.name,
        value: c.value,
        domain: c.domain,
        host: c.host,
        path: c.path,
        partition_key: c.partition_key,
        host_only: c.host_only,
        persistent: c.persistent,
        secure: c.secure,
        http_only: c.httponly,
        same_site: c.samesite,
        expires: c.expires.and_then(unix_to_system_time),
        creation_time: unix_to_system_time(c.creation_time).unwrap_or(now),
        last_access_time: unix_to_system_time(c.last_access_time).unwrap_or(now),
    }
}

/// Platform-appropriate data directory for persistent storage.
///
/// - macOS: `~/Library/Application Support`
/// - Linux: `$XDG_DATA_HOME` or `~/.local/share`
/// - Windows: `%APPDATA%`
/// - Fallback: temp directory
fn dirs_next_data_dir() -> std::path::PathBuf {
    // Simple cross-platform implementation without extra dependencies.
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(home).join("Library/Application Support");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return std::path::PathBuf::from(xdg);
        }
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(home).join(".local/share");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return std::path::PathBuf::from(appdata);
        }
    }
    std::env::temp_dir()
}

/// Convert a winit mouse button to the DOM spec button number.
///
/// DOM spec: 0=primary, 1=auxiliary, 2=secondary, 3=back, 4=forward.
pub(crate) fn winit_button_to_dom(button: winit::event::MouseButton) -> u8 {
    match button {
        winit::event::MouseButton::Middle => 1,
        winit::event::MouseButton::Right => 2,
        winit::event::MouseButton::Back => 3,
        winit::event::MouseButton::Forward => 4,
        winit::event::MouseButton::Left | winit::event::MouseButton::Other(_) => 0,
    }
}

/// Render state initialized after the window is created.
pub(super) struct RenderState {
    pub(super) window: Arc<Window>,
    /// Kept alive as a precaution. While wgpu 27's `Surface` does not
    /// reference the `Instance` directly, keeping it alive ensures
    /// correctness if future wgpu versions change this.
    pub(super) _instance: Instance,
    pub(super) surface: Surface<'static>,
    pub(super) gpu: crate::gpu::GpuContext,
    pub(super) renderer: elidex_render::VelloRenderer,
    pub(super) blitter: TextureBlitter,
    pub(super) egui_ctx: egui::Context,
    pub(super) egui_state: egui_winit::State,
    pub(super) egui_renderer: egui_wgpu::Renderer,
    pub(super) a11y_adapter: accesskit_winit::Adapter,
}

/// Browser-process (shell-owned) device-fact descriptor: the single source of
/// truth tying the three coordinate spaces — content/CSS px, window-logical px,
/// physical surface px — used by the producer (`SetViewport` size), the
/// compositor (paint transform + clip), and the input mapper (cursor → CSS px).
///
/// Built **only** by [`App::content_area_placement`] (the sole caller of
/// `chrome_content_offset` + `chrome::content_size` + `window.scale_factor()`)
/// and cached on [`App::placement`] so the three primitives are snapshotted
/// atomically once per frame / device-fact event. It is browser-process device
/// state (not per-DOM-entity content state) → a shell-local value, not an ECS
/// component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContentAreaPlacement {
    /// Content-area top-left in window-logical px (the chrome-reserved offset).
    pub(super) origin_logical: Point,
    /// Content-area size in CSS logical px (window-logical minus chrome).
    pub(super) size_logical: Size,
    /// Device pixel ratio (`window.scale_factor()`); CSS-px → device-px scale.
    pub(super) scale_factor: f32,
}

impl ContentAreaPlacement {
    /// Content-area top-left in physical surface px (`origin_logical × scale`).
    pub(super) fn origin_phys(&self) -> Point {
        Point::new(
            self.origin_logical.x * self.scale_factor,
            self.origin_logical.y * self.scale_factor,
        )
    }

    /// Content-area size in physical surface px (`size_logical × scale`).
    pub(super) fn size_phys(&self) -> Size {
        Size::new(
            self.size_logical.width * self.scale_factor,
            self.size_logical.height * self.scale_factor,
        )
    }
}

/// Legacy inline interactive state (all processing on the main thread).
///
/// Kept for backward compatibility with `build_pipeline()` test API.
pub(super) struct InteractiveState {
    pub(super) pipeline: crate::PipelineResult,
    pub(super) cursor_pos: Option<Point<f64>>,
    pub(super) hover_chain: Vec<Entity>,
    pub(super) active_chain: Vec<Entity>,
    pub(super) modifiers: Modifiers,
    pub(super) nav_controller: NavigationController,
    pub(super) window_title: String,
    pub(super) chrome: crate::chrome::ChromeState,
}

/// The initial content-thread spawn, **deferred** from `new_threaded*` until the
/// window — and thus the content-area [`ContentAreaPlacement`] — exists in
/// `resumed`. Spawning at `resumed` (not at construction) lets the initial tab be
/// born at its real viewport *by construction* (C1): the viewport is a spawn
/// argument, not a value raced-in after the first layout. `take()`-d once on the
/// first `resumed` (re-entry-guarded), so a suspend→resume cycle does not re-spawn.
enum PendingSpawn {
    /// Inline HTML/CSS (`new_threaded`).
    Html { html: String, css: String },
    /// A URL to load (`new_threaded_url`).
    Url(url::Url),
}

/// winit application that renders a display list to a window.
pub struct App {
    pub(super) render_state: Option<RenderState>,
    /// Multi-tab manager (threaded mode).
    tab_manager: Option<TabManager>,
    /// Window-level cursor position (shared across tabs).
    pub(super) cursor_pos: Option<Point<f64>>,
    /// Window-level modifier state (shared across tabs).
    pub(super) modifiers: Modifiers,
    /// Whether the cursor was in the content area on the last move event.
    /// Used to send exactly one `CursorLeft` when the cursor moves into the chrome area.
    cursor_in_content: bool,
    /// Legacy inline interactive state.
    pub(super) interactive: Option<InteractiveState>,
    /// Pending window focus request from `window.focus()`.
    pub(super) pending_focus: bool,
    /// Network Process broker handle (singleton, owns `NetClient` + `CookieJar`).
    network_process: Option<elidex_net::broker::NetworkProcessHandle>,
    /// Service Worker coordinator (manages registrations, lifecycle, sync).
    sw_coordinator: sw_coordinator::SwCoordinator,
    /// Browser-owned centralized database (cookies, history, bookmarks, etc.).
    browser_db: Option<elidex_storage_core::BrowserDb>,
    /// Last-synced CookieJar generation (for dirty-check persistence).
    cookie_gen: u64,
    /// Proxy to wake the winit event loop for content-initiated repaints.
    ///
    /// `Some` in threaded mode (built from the `EventLoop` in `run`/`run_url`);
    /// `None` in legacy inline mode (synchronous, no content thread to wake).
    /// [`App::wake_or_noop`] mints a per-content-thread [`crate::WakeHandle`] from
    /// a clone of this proxy at each spawn (`new_threaded*` initial tab,
    /// `window.open`, `open_new_tab`).
    wake_proxy: Option<winit::event_loop::EventLoopProxy<crate::WakeEvent>>,
    /// Cached content-area placement SoT (size↔origin↔scale), the single
    /// descriptor the producer/compositor/input mapper all read.
    ///
    /// `Some` once the window exists — **seeded in `resumed` together with
    /// `render_state`** and recomputed at redraw top + on `Resized`, so any
    /// input event that passes the `render_state.is_some()` gate sees a built
    /// placement by construction. `None` before the first `resumed` / after
    /// `suspended`.
    placement: Option<ContentAreaPlacement>,
    /// The initial tab's content-thread spawn, deferred until `resumed` so it is
    /// born at the real viewport (C1). `Some` between `new_threaded*` and the
    /// first `resumed`; `None` afterwards (inline mode is always `None`).
    pending_initial_spawn: Option<PendingSpawn>,
}

impl App {
    /// Build a content-thread wake closure from a clone of the event-loop proxy.
    /// The single way a [`crate::WakeHandle`] is minted (used by both the
    /// `new_threaded*` initial-tab spawn and [`App::wake_or_noop`] for later tabs).
    fn wake_from_proxy(
        proxy: &winit::event_loop::EventLoopProxy<crate::WakeEvent>,
    ) -> crate::WakeHandle {
        let proxy = proxy.clone();
        Box::new(move || {
            let _ = proxy.send_event(crate::WakeEvent::Repaint);
        })
    }

    /// Mint a content-thread wake from an optional proxy, falling back to a no-op
    /// (inline mode has no proxy — synchronous, nothing to wake). Takes the proxy
    /// by `Option<&_>` rather than `&self` so spawn sites can call it while
    /// holding a disjoint `&mut self.tab_manager` borrow.
    fn wake_or_noop(
        proxy: Option<&winit::event_loop::EventLoopProxy<crate::WakeEvent>>,
    ) -> crate::WakeHandle {
        match proxy {
            Some(p) => Self::wake_from_proxy(p),
            None => Box::new(|| {}),
        }
    }
    /// Create a threaded-mode `App` from a pre-initialized `TabManager`
    /// and `NetworkProcessHandle`.
    fn from_tab_manager(
        mgr: TabManager,
        np: elidex_net::broker::NetworkProcessHandle,
        wake_proxy: winit::event_loop::EventLoopProxy<crate::WakeEvent>,
    ) -> Self {
        let mut app = Self {
            render_state: None,
            tab_manager: Some(mgr),
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
            interactive: None,
            pending_focus: false,
            network_process: Some(np),
            sw_coordinator: sw_coordinator::SwCoordinator::new(),
            browser_db: None,
            cookie_gen: 0,
            wake_proxy: Some(wake_proxy),
            placement: None,
            pending_initial_spawn: None,
        };
        app.init_browser_db();
        app
    }

    /// Initialize browser.sqlite and load persisted data.
    ///
    /// Call once during startup, after the Network Process is spawned.
    /// Loads cookies from BrowserDb into the shared CookieJar.
    fn init_browser_db(&mut self) {
        // Use platform data directory. Falls back to temp if unavailable.
        // A proper profile selection UI will be added when the shell supports
        // multiple user profiles.
        let profile_dir = dirs_next_data_dir().join("elidex");
        match elidex_storage_core::BrowserDb::open(&profile_dir) {
            Ok(db) => {
                // Load persisted cookies into the shared CookieJar.
                if let Some(ref np) = self.network_process {
                    if let Ok(persisted) = db.cookies().load_all() {
                        let snapshots: Vec<_> =
                            persisted.into_iter().map(persisted_to_snap).collect();
                        np.cookie_jar().load(snapshots);
                        self.cookie_gen = np.cookie_jar().generation();
                        tracing::info!(count = np.cookie_jar().len(), "loaded persisted cookies");
                    }
                }
                self.browser_db = Some(db);
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to open browser.sqlite — running without persistence");
            }
        }
    }

    /// Persist dirty cookies to browser.sqlite if the jar has changed.
    ///
    /// Called each frame from `handle_redraw_threaded`. Compares the CookieJar's
    /// generation counter against the last-synced value.
    fn sync_cookies_if_dirty(&mut self) {
        let (Some(ref db), Some(ref np)) = (&self.browser_db, &self.network_process) else {
            return;
        };
        let jar = np.cookie_jar();
        let current_gen = jar.generation();
        if current_gen == self.cookie_gen {
            return;
        }
        // Only persist persistent cookies (not session cookies).
        let persisted: Vec<_> = jar
            .snapshot()
            .into_iter()
            .filter(|c| c.persistent)
            .map(snap_to_persisted)
            .collect();
        match db.cookies().sync_all(&persisted) {
            Ok(()) => self.cookie_gen = current_gen,
            Err(e) => tracing::debug!(error = %e, "failed to sync cookies — will retry"),
        }
    }

    /// Spawn the singleton Network Process broker.
    ///
    /// (Helper placed above to avoid `items_after_statements` lint.)
    fn create_network_process() -> elidex_net::broker::NetworkProcessHandle {
        elidex_net::broker::spawn_network_process(elidex_net::NetClient::new())
    }

    /// Create a new threaded application from HTML/CSS.
    ///
    /// `wake_proxy` (from the `EventLoop` in `run`) wakes the loop for
    /// content-initiated repaints; it is stored and used to mint a
    /// [`crate::WakeHandle`] for every spawned tab. The **initial** content thread
    /// is *not* spawned here — it is deferred to `resumed` (see
    /// [`PendingSpawn`]) so it is born at the window's real viewport (C1).
    pub fn new_threaded(
        html: String,
        css: String,
        wake_proxy: winit::event_loop::EventLoopProxy<crate::WakeEvent>,
    ) -> Self {
        let np = Self::create_network_process();
        let mut app = Self::from_tab_manager(TabManager::new(), np, wake_proxy);
        app.pending_initial_spawn = Some(PendingSpawn::Html { html, css });
        app
    }

    /// Create a new threaded application from a URL.
    ///
    /// As with [`Self::new_threaded`], the initial content thread is deferred to
    /// `resumed` so it builds at the real viewport (C1).
    pub fn new_threaded_url(
        url: url::Url,
        wake_proxy: winit::event_loop::EventLoopProxy<crate::WakeEvent>,
    ) -> Self {
        let np = Self::create_network_process();
        let mut app = Self::from_tab_manager(TabManager::new(), np, wake_proxy);
        app.pending_initial_spawn = Some(PendingSpawn::Url(url));
        app
    }

    /// Create a new legacy (inline) interactive application from a pipeline result.
    #[allow(dead_code)]
    pub fn new_interactive(pipeline: crate::PipelineResult) -> Self {
        Self {
            render_state: None,
            tab_manager: None,
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
            interactive: Some(InteractiveState {
                chrome: crate::chrome::ChromeState::new(None),
                pipeline,
                cursor_pos: None,
                hover_chain: Vec::new(),
                active_chain: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller: NavigationController::new(),
                window_title: "elidex".to_string(),
            }),
            pending_focus: false,
            network_process: None, // Legacy mode — no broker.
            sw_coordinator: sw_coordinator::SwCoordinator::new(),
            browser_db: None,
            cookie_gen: 0,
            wake_proxy: None, // Inline mode is synchronous — nothing to wake.
            placement: None,
            pending_initial_spawn: None, // Inline mode spawns no content thread.
        }
    }

    /// Create a new legacy (inline) interactive application from a URL-loaded pipeline result.
    #[allow(dead_code)]
    pub fn new_interactive_with_url(pipeline: crate::PipelineResult, title: String) -> Self {
        let chrome = crate::chrome::ChromeState::new(pipeline.url.as_ref());
        let mut nav_controller = NavigationController::new();
        if let Some(url) = &pipeline.url {
            nav_controller.push(url.clone());
        }
        Self {
            render_state: None,
            tab_manager: None,
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
            interactive: Some(InteractiveState {
                chrome,
                pipeline,
                cursor_pos: None,
                hover_chain: Vec::new(),
                active_chain: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller,
                window_title: title,
            }),
            pending_focus: false,
            network_process: None, // Legacy mode — no broker.
            sw_coordinator: sw_coordinator::SwCoordinator::new(),
            browser_db: None,
            cookie_gen: 0,
            wake_proxy: None, // Inline mode is synchronous — nothing to wake.
            placement: None,
            pending_initial_spawn: None, // Inline mode spawns no content thread.
        }
    }

    /// Maximum messages to drain per tab per frame.
    ///
    /// Prevents a runaway content thread from monopolizing the browser thread's
    /// event loop. Any remaining messages will be drained on the next frame.
    const MAX_DRAIN_PER_TAB: usize = 1000;

    /// Drain all pending messages from all tabs.
    #[allow(clippy::too_many_lines)]
    fn drain_content_messages(&mut self) {
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
                    } => {
                        if let Some(ref np) = self.network_process {
                            self.sw_coordinator.register(
                                &script_url,
                                &scope,
                                &page_url,
                                np,
                                &tab.channel,
                            );
                        }
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

        // Open new tabs requested by window.open(). Post-`resumed`, so `placement`
        // is `Some` — born at the real viewport (C1); `DEFAULT` only as a defensive
        // fallback (disjoint `self.placement` read coexists with `&mut mgr`).
        let viewport = self.placement.map_or_else(
            || {
                Size::new(
                    crate::DEFAULT_VIEWPORT_WIDTH,
                    crate::DEFAULT_VIEWPORT_HEIGHT,
                )
            },
            |p| p.size_logical,
        );
        for url in new_tab_urls {
            let (browser_chan, content_chan) = crate::ipc::channel_pair();
            let title = format!("elidex \u{2014} {url}");
            let chrome = crate::chrome::ChromeState::new(Some(&url));
            if let Some(np) = &self.network_process {
                let nh = np.create_renderer_handle();
                let jar = Arc::clone(np.cookie_jar());
                // Mint via the disjoint `wake_proxy` field (an associated fn, not
                // `&self`) so it coexists with the active `&mut mgr` borrow.
                let wake = Self::wake_or_noop(self.wake_proxy.as_ref());
                let thread = crate::content::spawn_content_thread_url(
                    content_chan,
                    nh,
                    jar,
                    url,
                    viewport,
                    wake,
                );
                mgr.create_tab(browser_chan, thread, chrome, title);
            }
        }
    }

    /// Convert winit modifier state to IPC `ModifierState`.
    fn to_modifier_state(mods: winit::keyboard::ModifiersState) -> ModifierState {
        ModifierState {
            alt: mods.alt_key(),
            ctrl: mods.control_key(),
            meta: mods.super_key(),
            shift: mods.shift_key(),
        }
    }

    /// Build the content-area placement SoT from the current window + active-tab
    /// chrome position.
    ///
    /// The **only** caller of `window.scale_factor()`, [`chrome::content_size`],
    /// and [`chrome::chrome_content_offset`] (egui's own DPI read at render-init
    /// excepted) — the three primitives are snapshotted atomically (one
    /// `scale_factor` read per build). Callers cache the result in
    /// [`App::placement`]; nothing else recomputes a content-area size/origin.
    // Window dimensions (< 2^23 px) and the scale factor lose no meaningful
    // precision narrowing to the `f32` the layout/CSS coordinate space uses.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn content_area_placement(&self, window: &Window) -> ContentAreaPlacement {
        let scale_factor = window.scale_factor() as f32;
        let phys = window.inner_size();
        let win_logical_w = phys.width as f32 / scale_factor;
        let win_logical_h = phys.height as f32 / scale_factor;
        let position = self.tab_bar_position();
        ContentAreaPlacement {
            origin_logical: chrome::chrome_content_offset(position),
            size_logical: chrome::content_size(win_logical_w, win_logical_h, position),
            scale_factor,
        }
    }

    /// Send the cached content-area size (CSS logical px) to **one** tab's content
    /// thread — the single per-tab viewport-delivery primitive (C1).
    ///
    /// Content stays device-agnostic: only `size_logical` crosses the IPC, never
    /// `scale_factor`. An associated fn (not `&self`) so it composes with an active
    /// `&mut self.tab_manager` borrow (mirrors [`Self::wake_or_noop`]). No-op until
    /// `placement` is seeded. `BrowserToContent` is not `Clone`, so each recipient
    /// gets a freshly-constructed message.
    fn seed_tab_viewport(placement: Option<ContentAreaPlacement>, tab: &Tab) {
        if let Some(placement) = placement {
            if let Err(e) = tab.channel.send(BrowserToContent::SetViewport {
                width: placement.size_logical.width,
                height: placement.size_logical.height,
            }) {
                eprintln!("Failed to send viewport to content thread (disconnected): {e}");
            }
        }
    }

    /// Fan the cached viewport out to **every** tab — all share the window's
    /// content area, so a resize must reach background tabs too (their
    /// `innerWidth`/`matchMedia` stay spec-correct). Called on `Resized` (and C2's
    /// `ScaleFactorChanged`). Initial/`window.open`/new tabs are instead born at the
    /// real size via the construction-input spawn (C1), so they need no seed message.
    fn broadcast_viewport(&self) {
        if let Some(mgr) = &self.tab_manager {
            for tab in mgr.tabs() {
                Self::seed_tab_viewport(self.placement, tab);
            }
        }
    }

    /// Spawn the deferred initial content thread (C1) at the real `viewport`, now
    /// that the window exists. No-op if there is no pending spawn (already spawned,
    /// or inline mode) or no network process. The minted [`crate::WakeHandle`]
    /// comes from the disjoint `wake_proxy` field (mirrors the `window.open` /
    /// `open_new_tab` spawn sites).
    fn spawn_pending_initial_tab(&mut self, viewport: Size) {
        let Some(pending) = self.pending_initial_spawn.take() else {
            return;
        };
        // `pending` is `Some` only in threaded mode (set by `new_threaded*`), and
        // `from_tab_manager` guarantees `network_process` + `tab_manager` are then
        // `Some`. Surface a broken invariant rather than silently dropping the
        // initial tab (a blank window) — mirrors the `tab_manager.expect` in
        // `handle_redraw_threaded`.
        let np = self
            .network_process
            .as_ref()
            .expect("threaded-mode initial spawn requires a network process");
        let nh = np.create_renderer_handle();
        let jar = Arc::clone(np.cookie_jar());
        let wake = Self::wake_or_noop(self.wake_proxy.as_ref());
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let (thread, chrome, title) = match pending {
            PendingSpawn::Html { html, css } => (
                crate::content::spawn_content_thread(
                    content_ch, nh, jar, html, css, viewport, wake,
                ),
                crate::chrome::ChromeState::new(None),
                "elidex".to_string(),
            ),
            PendingSpawn::Url(url) => {
                let title = format!("elidex \u{2014} {url}");
                let chrome = crate::chrome::ChromeState::new(Some(&url));
                let thread = crate::content::spawn_content_thread_url(
                    content_ch, nh, jar, url, viewport, wake,
                );
                (thread, chrome, title)
            }
        };
        self.tab_manager
            .as_mut()
            .expect("threaded mode requires a tab manager")
            .create_tab(browser_ch, thread, chrome, title);
    }

    /// Send a message to the active tab's content thread.
    fn send_to_content(&self, msg: BrowserToContent) {
        if let Some(mgr) = &self.tab_manager {
            if let Some(tab) = mgr.active_tab() {
                if let Err(e) = tab.channel.send(msg) {
                    eprintln!("Failed to send to content thread (disconnected): {e}");
                }
            }
        }
    }

    /// Get the tab bar position from the active tab's chrome state.
    fn tab_bar_position(&self) -> chrome::TabBarPosition {
        self.tab_manager
            .as_ref()
            .and_then(|mgr| mgr.active_tab())
            .map_or(chrome::TabBarPosition::Top, |tab| {
                tab.chrome.tab_bar_position
            })
    }

    /// Build tab bar info for all tabs.
    fn tab_bar_infos(&self) -> Vec<TabBarInfo> {
        let Some(mgr) = &self.tab_manager else {
            return Vec::new();
        };
        let active_id = mgr.active_id();
        mgr.tabs()
            .iter()
            .map(|tab| TabBarInfo {
                id: tab.id,
                title: tab.window_title.clone(),
                is_active: Some(tab.id) == active_id,
            })
            .collect()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(mgr) = &mut self.tab_manager {
            mgr.shutdown_all();
        }
    }
}

impl ApplicationHandler<crate::WakeEvent> for App {
    /// A content thread asked for a repaint (a content-initiated frame is
    /// pending: timer / rAF / animation / async DOM / `SetViewport` round-trip).
    /// Schedule a redraw; the redraw handler drains the pending content messages
    /// (`drain_content_messages`) before presenting, satisfying the
    /// wake→redraw→drain→paint ordering. Best-effort: if the window is not yet
    /// created (`render_state` is `None`, e.g. a wake arriving before `resumed`),
    /// this no-ops — `resumed` issues its own `request_redraw` and the channel is
    /// the source of truth for the pending frame, so no frame is lost.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: crate::WakeEvent) {
        match event {
            crate::WakeEvent::Repaint => {
                if let Some(state) = &self.render_state {
                    state.window.request_redraw();
                }
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_state.is_some() {
            return;
        }

        let Some(state) = try_init_render_state(event_loop) else {
            event_loop.exit();
            return;
        };

        // Build the placement SoT now that the window exists (the §2.3 init
        // invariant): any input that later passes the `render_state.is_some()`
        // gate sees a built placement. The deferred initial tab is not yet
        // spawned, so `tab_bar_position` falls back to its default (`Top`) —
        // identical to the initial tab's default chrome, so the size is correct.
        let placement = self.content_area_placement(&state.window);
        self.placement = Some(placement);
        self.render_state = Some(state);

        // Spawn the deferred initial content thread (C1) at the real viewport —
        // it is born resolving styles, running initial scripts, and laying out at
        // `placement.size_logical`, never a guessed `DEFAULT_VIEWPORT`. This
        // replaces the old initial `send_viewport()`: the size is a spawn
        // argument, not a value raced-in after the first layout.
        self.spawn_pending_initial_tab(placement.size_logical);

        // Set the initial window title from the now-present active tab.
        if let Some(state) = &self.render_state {
            if let Some(mgr) = &self.tab_manager {
                if let Some(tab) = mgr.active_tab() {
                    state.window.set_title(&tab.window_title);
                }
            } else if let Some(interactive) = &self.interactive {
                state.window.set_title(&interactive.window_title);
            }
            state.window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.render_state = None;
        self.placement = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.render_state.is_none() {
            return;
        }

        // Process AccessKit events first.
        if let Some(state) = &mut self.render_state {
            state.a11y_adapter.process_event(&state.window, &event);
        }

        // Handle events that always need processing regardless of egui.
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(new_size) => {
                let Some(state) = self.render_state.as_mut() else {
                    return;
                };
                state
                    .gpu
                    .resize(&state.surface, new_size.width, new_size.height);
                let window = Arc::clone(&state.window);
                if new_size.width > 0 && new_size.height > 0 {
                    window.request_redraw();
                }
                // Recompute the placement SoT (the window size changed) and
                // fan the new viewport out to **every** tab — all share the
                // window content area, so background tabs must re-lay-out too (C1).
                self.placement = Some(self.content_area_placement(&window));
                self.broadcast_viewport();
                // The content rect may have shrunk/moved out from under a
                // stationary cursor — clear stuck `:hover` if it left content.
                if let Some(placement) = self.placement {
                    self.reclip_cursor_after_placement_change(placement);
                }
                return;
            }
            _ => {}
        }

        // ---- Threaded mode ----
        if self.tab_manager.is_some() {
            self.handle_window_event_threaded(event_loop, event);
            return;
        }

        // ---- Legacy inline mode ----
        self.handle_window_event_inline(event_loop, event);
    }
}
