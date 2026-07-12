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

mod content_messages;
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
mod viewport;

#[cfg(test)]
#[path = "../app_fragment_nav_tests.rs"]
mod fragment_nav_tests;

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
use crate::ipc::{BrowserToContent, ModifierState};

use render::try_init_render_state;
use tab::TabManager;

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
    /// Device pixel ratio (`window.scale_factor()`); CSS-px → device-px scale. Held as
    /// **`f64`** (the lossless winit value) because it is also the dppx *content fact*
    /// (`window.devicePixelRatio` / `@media (resolution)`): an `f32` rounds a fractional
    /// scale like 1.2 to `1.2000000476837158`, which the exact (`RangeValue::Dppx`)
    /// media evaluator rejects for `(resolution: 1.2dppx)` and which JS observes as a
    /// wrong DPR (Codex R3). The compositor transform / input ÷scale tolerate `f32`, so
    /// `origin_phys`/`size_phys` narrow it there.
    pub(super) scale_factor: f64,
}

impl ContentAreaPlacement {
    /// The scale narrowed to `f32` for the **compositor transform / input ÷scale**, which
    /// tolerate it — the single deliberate-narrowing site (the `f64` `scale_factor` is
    /// lossless only for the dppx *content fact*: `devicePixelRatio` / `@media (resolution)`,
    /// C3 R3). Centralized so the narrowing intent + lint exemption live in one place.
    #[allow(clippy::cast_possible_truncation)]
    pub(super) fn scale_f32(&self) -> f32 {
        self.scale_factor as f32
    }

    /// Content-area top-left in physical surface px (`origin_logical × scale`).
    pub(super) fn origin_phys(&self) -> Point {
        Point::new(
            self.origin_logical.x * self.scale_f32(),
            self.origin_logical.y * self.scale_f32(),
        )
    }

    /// Content-area size in physical surface px (`size_logical × scale`).
    pub(super) fn size_phys(&self) -> Size {
        Size::new(
            self.size_logical.width * self.scale_f32(),
            self.size_logical.height * self.scale_f32(),
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
    /// The window's current device facts (dppx / color-scheme / reduced-motion) —
    /// the shell-owned SoT the VM's getterless media surface replaced (B20). Inline
    /// mode has no dynamic device-facts writer, so this is seed-at-construction +
    /// read-at-rebuild only (parity: facts were static-after-construction under boa).
    pub(super) device_facts: crate::ipc::DeviceFacts,
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
    /// Per-origin storage manager (Cache API / IDB connections), owned by the
    /// App at least authority. `Some` only in the threaded path (built in
    /// [`Self::init_browser_db`]); inline/legacy modes have no SW → `None`.
    origin_storage: Option<elidex_storage_core::OriginStorageManager>,
    /// Process-wide `localStorage` backend (WHATWG HTML §11.2), disk-backed +
    /// origin-scoped. Owned once at the browser-process level (mirroring the
    /// shared `CookieJar` on the `NetworkProcessHandle`) and cloned to every
    /// spawned content thread so same-origin tabs share ONE in-memory registry
    /// and one on-disk JSON tree. Threaded into the pipeline construction seam
    /// (`build_pipeline_*` → `run_scripts_and_finalize` → `install_web_storage`)
    /// and to the per-turn `flush_dirty` in the content event loop (F14 /
    /// §4.3.3).
    web_storage: std::sync::Arc<elidex_storage_core::WebStorageManager>,
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
    /// Viewport-producer state: placement SoT, pending initial spawn, and
    /// shared viewport cell (see [`viewport::ViewportProducer`]).
    viewport: viewport::ViewportProducer,
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
            origin_storage: None,
            // ONE process-wide manager, cloned to each content thread at spawn so
            // same-origin tabs share the live registry (mirrors the shared cookie
            // jar). Disk root = platform data_dir/elidex/localStorage (F14).
            web_storage: std::sync::Arc::new(
                elidex_storage_core::WebStorageManager::with_default_profile(),
            ),
            cookie_gen: 0,
            wake_proxy: Some(wake_proxy),
            viewport: viewport::ViewportProducer::new(
                crate::DEFAULT_VIEWPORT_WIDTH,
                crate::DEFAULT_VIEWPORT_HEIGHT,
            ),
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
        // Per-origin storage manager for SW Cache API connections (least-authority
        // §6.5). `OriginStorageManager::new` does no I/O — connections open lazily
        // per origin — so this is independent of the `BrowserDb::open` result.
        self.origin_storage = Some(elidex_storage_core::OriginStorageManager::new(
            profile_dir.clone(),
        ));
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
        app.viewport.pending_initial_spawn = Some(PendingSpawn::Html { html, css });
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
        app.viewport.pending_initial_spawn = Some(PendingSpawn::Url(url));
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
                // Inline pipelines are built with default facts (1× / Light); no
                // window → no dynamic writer, so this static seed IS parity (B20).
                device_facts: crate::ipc::DeviceFacts::default(),
            }),
            pending_focus: false,
            network_process: None, // Legacy mode — no broker.
            sw_coordinator: sw_coordinator::SwCoordinator::new(),
            browser_db: None,
            origin_storage: None, // Inline/legacy mode — no SW, no per-origin storage.
            // Inline/legacy mode has no content thread, but its in-app navigation
            // rebuild (`load_url_into_pipeline`) still constructs pipelines that
            // must persist `localStorage` — own one disk-backed manager here too.
            web_storage: std::sync::Arc::new(
                elidex_storage_core::WebStorageManager::with_default_profile(),
            ),
            cookie_gen: 0,
            wake_proxy: None, // Inline mode is synchronous — nothing to wake.
            viewport: viewport::ViewportProducer::new(
                crate::DEFAULT_VIEWPORT_WIDTH,
                crate::DEFAULT_VIEWPORT_HEIGHT,
            ),
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
                // Inline pipelines are built with default facts (1× / Light); no
                // window → no dynamic writer, so this static seed IS parity (B20).
                device_facts: crate::ipc::DeviceFacts::default(),
            }),
            pending_focus: false,
            network_process: None, // Legacy mode — no broker.
            sw_coordinator: sw_coordinator::SwCoordinator::new(),
            browser_db: None,
            origin_storage: None, // Inline/legacy mode — no SW, no per-origin storage.
            // Inline/legacy mode has no content thread, but its in-app navigation
            // rebuild (`load_url_into_pipeline`) still constructs pipelines that
            // must persist `localStorage` — own one disk-backed manager here too.
            web_storage: std::sync::Arc::new(
                elidex_storage_core::WebStorageManager::with_default_profile(),
            ),
            cookie_gen: 0,
            wake_proxy: None, // Inline mode is synchronous — nothing to wake.
            viewport: viewport::ViewportProducer::new(
                crate::DEFAULT_VIEWPORT_WIDTH,
                crate::DEFAULT_VIEWPORT_HEIGHT,
            ),
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

    /// Forward a window event to egui's `State` for its own bookkeeping.
    ///
    /// The `Resized` / `ScaleFactorChanged` / `ThemeChanged` arms in `window_event`
    /// handle-and-`return` before the per-mode dispatch (`handle_window_event_threaded`
    /// / `_inline`) where egui normally sees events, so egui-winit's cached
    /// `native_pixels_per_point` (`ScaleFactorChanged`) and `system_theme`
    /// (`ThemeChanged`) would otherwise stay stale after a monitor/theme change — the
    /// browser chrome would keep rendering at the old DPI scale / color theme while the
    /// content device facts update. These are non-input events (no `consumed` routing to
    /// preserve), so this forwards on `render_state` alone (egui exists whenever it
    /// does) — narrower than the per-mode input gating and sufficient for the cached
    /// window state. Honors egui's `repaint` request like the per-mode handlers do.
    fn forward_to_egui(&mut self, event: &WindowEvent) {
        if let Some(state) = self.render_state.as_mut() {
            let response = state.egui_state.on_window_event(&state.window, event);
            if response.repaint {
                state.window.request_redraw();
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
        self.viewport.placement = Some(placement);
        let facts = Self::device_facts(placement, &state.window);
        // Publish the real size + device facts to the viewport cell **before**
        // spawning, so the deferred initial tab's build reads them by construction
        // (the cell is the pull source the spawn reads, not a snapshot): size →
        // cascade/layout + bridge `innerWidth`; dppx/color-scheme → bridge
        // `devicePixelRatio`/`matchMedia` before initial scripts (C3). Normally bumps
        // seq 0 → 1 (DEFAULT → real) and flips facts off the 1×/Light baseline; the
        // returned `DeviceDelta` gates the re-resume fan-out below.
        let delta = self
            .viewport
            .viewport_cell
            .publish_device_state(placement.size_logical, facts);
        self.render_state = Some(state);

        // Spawn the deferred initial content thread (C1) at the real viewport + device
        // facts — it is born resolving styles, running initial scripts, and laying out
        // at the state it reads from the just-published cell, never a guessed default.
        self.spawn_pending_initial_tab();

        // Re-resume after `suspended` (plan-memo Q3): content threads persist across a
        // suspend but `placement` was dropped, so the content area / device facts may
        // have changed while the window was gone. Fan the rebuilt state to every
        // persisted tab only for what actually changed — an unchanged size must not
        // bump `applied_viewport_seq`; unchanged facts emit no re-eval. The
        // just-spawned initial tab was already born with both via the cell read, so
        // its deliveries here are guarded no-ops on the content side. Atomic delivery
        // (C3 R2): when the size changed, `broadcast_viewport` carries the settled facts
        // too; `broadcast_device_facts` is for a facts-only change — mutually exclusive,
        // so facts never double-send.
        if delta.size_changed {
            self.broadcast_viewport();
        } else if delta.facts_changed {
            self.broadcast_device_facts();
        }

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
        self.viewport.placement = None;
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
                // egui first (its cached window size), then sync the GPU surface — this
                // arm returns before the per-mode dispatch where egui normally runs.
                self.forward_to_egui(&event);
                let Some(state) = self.render_state.as_mut() else {
                    return;
                };
                state
                    .gpu
                    .resize(&state.surface, new_size.width, new_size.height);
                if new_size.width > 0 && new_size.height > 0 {
                    state.window.request_redraw();
                }
                // Placement recompute + publish (size + device facts) + reclip all move
                // to the redraw-top chokepoint (`handle_redraw_threaded`), so `placement`
                // and `seq` update **atomically** there (C3 D2/F1): this arm touches
                // neither, so across the event→redraw gap both stay old (coherent), and
                // gap-input maps old-placement/old-seq and is **applied** against the
                // still-old layout (`placement_seq == applied_viewport_seq`, not dropped/
                // superseded — Codex R2 correction; plan-memo D2 (1)). The arm only syncs
                // the GPU surface to the new physical size and requests the redraw whose
                // settled-state recompute does the rest.
                return;
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                // A DPI change. On the **common** path a `Resized` follows (winit forces
                // it when the DPI-adjusted physical size differs) and does the
                // `gpu.resize`; on the X11 / constrained-WM corner where physical size is
                // held constant, **no** `Resized` follows (winit 0.30 x11), so this is the
                // *only* event for that DPI change. Either way it must drive the redraw
                // whose settled-state recompute ships the new `size_logical` + dppx —
                // closing the former carved `#11-shell-viewport-scalefactorchanged-x11-
                // coverage` gap (C3 D1/F3). Sync the surface to the current physical size
                // (a no-op when physical is unchanged) and request the redraw; the arm
                // neither recomputes `placement` nor publishes (D2 atomicity) — redraw-top
                // reads the settled `(inner_size, scale_factor)`, so the bogus
                // `old_phys/new_scale` intermediate never materializes.
                //
                // Forward to egui first so egui-winit updates its cached
                // `native_pixels_per_point` — otherwise the chrome keeps rendering at the
                // old DPI scale while the content device facts update (this arm returns
                // before the per-mode dispatch where egui normally sees the event).
                self.forward_to_egui(&event);
                let Some(state) = self.render_state.as_mut() else {
                    return;
                };
                let phys = state.window.inner_size();
                state.gpu.resize(&state.surface, phys.width, phys.height);
                if phys.width > 0 && phys.height > 0 {
                    state.window.request_redraw();
                }
                return;
            }
            WindowEvent::ThemeChanged(_) => {
                // The OS color scheme changed (macOS / Windows; winit emits no theme
                // event on X11 / Wayland, where `prefers-color-scheme` stays `Light`).
                // Forward to egui first so egui-winit updates its cached `system_theme`
                // — otherwise the chrome keeps the old theme while the content
                // `prefers-color-scheme` fact updates (this arm returns before the
                // per-mode dispatch). Then drive a redraw; redraw-top re-reads
                // `window.theme()` (as it re-reads `scale_factor`) and publishes the new
                // `prefers-color-scheme` fact — no recompute/publish in the arm (D2).
                self.forward_to_egui(&event);
                if let Some(state) = &self.render_state {
                    state.window.request_redraw();
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
