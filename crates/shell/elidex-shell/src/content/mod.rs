//! Content thread: owns the DOM, JS runtime, and rendering pipeline.
//!
//! The content thread runs a message loop, processing events from the browser
//! thread and sending back display list updates.

mod animation;
mod event_handlers;
pub(crate) mod focus;
mod form_input;
pub(crate) mod iframe;
mod ime;
mod navigation;
mod scroll;

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use elidex_ecs::Entity;
use elidex_navigation::NavigationController;

use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};
use crate::PipelineResult;

/// Default poll interval when no timers or animations are active.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Target frame interval (~60 fps) for the animation frame loop.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Caret blink interval (500ms per HTML spec recommendation).
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

/// Monotonic counter for script-created animation keyframes names.
/// Ensures multiple `element.animate()` calls on the same element
/// produce distinct keyframes without overwriting previous ones.
static SCRIPT_ANIM_COUNTER: AtomicU64 = AtomicU64::new(0);

/// State owned by the content thread.
///
/// `hover_chain` and `active_chain` are bounded by [`elidex_ecs::MAX_ANCESTOR_DEPTH`]
/// (the depth limit enforced by [`collect_hover_chain`](crate::app::hover::collect_hover_chain)).
struct ContentState {
    pipeline: PipelineResult,
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    nav_controller: NavigationController,
    hover_chain: Vec<Entity>,
    active_chain: Vec<Entity>,
    focus_target: Option<Entity>,
    /// Value of the focused text control when it gained focus (for change event on blur).
    focus_initial_value: Option<String>,
    /// Whether the caret is currently visible (toggles every 500ms).
    caret_visible: bool,
    /// Last time the caret toggled visibility.
    caret_last_toggle: Instant,
    /// Cached list of focusable entities for Tab navigation (invalidated on DOM changes).
    focusable_cache: Option<Vec<Entity>>,
    /// Viewport-level scroll state (not attached to a DOM entity).
    viewport_scroll: elidex_ecs::ScrollState,
    /// Registry of child iframes (same-origin in-process + cross-origin out-of-process).
    iframes: iframe::IframeRegistry,
    /// Which iframe currently has focus (`None` = parent document has focus).
    /// Set by `try_route_click_to_iframe`, checked by `try_route_key_to_iframe`.
    focused_iframe: Option<Entity>,
}

impl ContentState {
    /// Send the current display list to the browser thread.
    fn send_display_list(&self) {
        let _ = self.channel.send(ContentToBrowser::DisplayListReady(
            self.pipeline.display_list.clone(),
        ));
    }

    /// Send current navigation state (`can_go_back`/`can_go_forward`) to the browser thread.
    fn send_navigation_state(&self) {
        let _ = self.channel.send(ContentToBrowser::NavigationState {
            can_go_back: self.nav_controller.can_go_back(),
            can_go_forward: self.nav_controller.can_go_forward(),
        });
    }

    /// Send URL change notification to the browser thread.
    fn send_url_changed(&self, url: &url::Url) {
        let _ = self.channel.send(ContentToBrowser::UrlChanged(url.clone()));
    }

    /// Send all post-navigation notifications to the browser thread
    /// (title, URL, navigation state, display list).
    fn notify_navigation(&self, url: &url::Url) {
        let title = format!("elidex \u{2014} {url}");
        let _ = self.channel.send(ContentToBrowser::TitleChanged(title));
        self.send_url_changed(url);
        self.send_navigation_state();
        self.send_display_list();
    }

    /// Push or replace a URL in the navigation controller.
    fn push_or_replace(&mut self, url: url::Url, replace: bool) {
        if replace {
            self.nav_controller.replace(url);
        } else {
            self.nav_controller.push(url);
        }
    }

    /// Create a new `ContentState` from an initialized pipeline and channel.
    fn new(
        channel: LocalChannel<ContentToBrowser, BrowserToContent>,
        nav_controller: NavigationController,
        pipeline: PipelineResult,
    ) -> Self {
        Self {
            channel,
            nav_controller,
            hover_chain: Vec::new(),
            active_chain: Vec::new(),
            focus_target: None,
            focus_initial_value: None,
            caret_visible: true,
            caret_last_toggle: Instant::now(),
            focusable_cache: None,
            viewport_scroll: elidex_ecs::ScrollState::default(),
            pipeline,
            iframes: iframe::IframeRegistry::new(),
            focused_iframe: None,
        }
    }

    /// Sync canvas pixels and `caret_visible` to the pipeline, then re-render.
    ///
    /// Canvas `ImageData` sync is deferred to this point (once per frame)
    /// instead of per-draw-call for efficiency.
    ///
    /// After re-render, delivers observer callbacks (`MutationObserver`,
    /// `ResizeObserver`, `IntersectionObserver`) per WHATWG spec ordering:
    /// mutations first, then resize, then intersection.
    ///
    /// Invalidates `focusable_cache` when mutations affect DOM structure or
    /// focusability-related attributes (childList, tabindex, disabled, etc.).
    fn re_render(&mut self) {
        self.pipeline
            .runtime
            .bridge()
            .sync_dirty_canvases(&mut self.pipeline.dom);
        self.pipeline.caret_visible = self.caret_visible;

        // Apply any pending JS scroll (scrollTo/scrollBy) to viewport state.
        if let Some((x, y)) = self.pipeline.runtime.bridge().take_pending_scroll() {
            self.viewport_scroll.scroll_offset = elidex_plugin::Vector::new(x, y);
            // Clamp to valid range so JS cannot set out-of-bounds scroll positions.
            scroll::update_viewport_scroll_dimensions(self);
        }

        // Sync viewport scroll offset to pipeline for display list building.
        self.pipeline.scroll_offset = self.viewport_scroll.scroll_offset;
        // Store viewport scroll on document root so getBoundingClientRect
        // includes it via accumulated_scroll_offset (CSSOM View §5).
        let _ = self
            .pipeline
            .dom
            .world_mut()
            .insert_one(self.pipeline.document, self.viewport_scroll.clone());
        // Sync scroll offset to JS bridge so scrollX/scrollY reflect current state.
        self.pipeline.runtime.bridge().set_scroll_offset(
            self.viewport_scroll.scroll_offset.x,
            self.viewport_scroll.scroll_offset.y,
        );
        // Re-render in-process iframes before the parent so child display
        // lists are up-to-date when the parent composites them.
        iframe::re_render_all_iframes(self);

        let mutation_records = crate::re_render(&mut self.pipeline);

        // Invalidate focusable cache when DOM structure or focusability changes.
        if should_invalidate_focusable_cache(&mutation_records) {
            self.focusable_cache = None;
        }

        // Deliver observer callbacks after layout is complete.
        if !mutation_records.is_empty() {
            self.pipeline.runtime.deliver_mutation_records(
                &mutation_records,
                &mut self.pipeline.session,
                &mut self.pipeline.dom,
                self.pipeline.document,
            );
        }

        self.pipeline.runtime.deliver_resize_observations(
            &mut self.pipeline.session,
            &mut self.pipeline.dom,
            self.pipeline.document,
        );

        let viewport = elidex_plugin::Rect::new(
            0.0,
            0.0,
            self.pipeline.viewport.width,
            self.pipeline.viewport.height,
        );
        self.pipeline.runtime.deliver_intersection_observations(
            &mut self.pipeline.session,
            &mut self.pipeline.dom,
            self.pipeline.document,
            viewport,
        );

        // Detect iframe additions/removals from mutation records.
        // Added <iframe> entities trigger loading; removed ones trigger unloading.
        let iframes_changed = iframe::detect_iframe_mutations(&mutation_records, self);

        // Check lazy iframes: load those that have entered the viewport.
        let lazy_loaded = iframe::check_lazy_iframes(self);

        // Rebuild the parent display list if iframes were added/removed/navigated,
        // since the display list was already built before iframe mutations were processed.
        if iframes_changed || lazy_loaded {
            self.pipeline.display_list = elidex_render::build_display_list_with_scroll(
                &self.pipeline.dom,
                &self.pipeline.font_db,
                self.pipeline.caret_visible,
                self.pipeline.scroll_offset,
            );
        }

        // Update viewport scroll dimensions after layout completes.
        scroll::update_viewport_scroll_dimensions(self);
    }

    /// Reset caret blink timer (call on key input to keep caret visible).
    fn reset_caret_blink(&mut self) {
        self.caret_visible = true;
        self.caret_last_toggle = Instant::now();
    }

    /// Update caret blink state. Returns true if visibility changed.
    ///
    /// Only blinks for editable text controls (not buttons/checkboxes/links)
    /// to avoid unnecessary 500ms re-render loops for non-text focus.
    fn update_caret_blink(&mut self) -> bool {
        let is_text_focused = self.focus_target.is_some_and(|target| {
            self.pipeline
                .dom
                .world()
                .get::<&elidex_form::FormControlState>(target)
                .ok()
                .is_some_and(|fcs| fcs.kind.is_text_control() && !fcs.disabled)
        });
        if !is_text_focused {
            return false;
        }
        if self.caret_last_toggle.elapsed() >= CARET_BLINK_INTERVAL {
            self.caret_visible = !self.caret_visible;
            self.caret_last_toggle = Instant::now();
            return true;
        }
        false
    }
}

/// Spawn the content thread with initial HTML/CSS.
///
/// Returns a `JoinHandle` for the thread. The thread will run until it
/// receives `Shutdown` or the channel disconnects.
pub(crate) fn spawn_content_thread(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    html: String,
    css: String,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(channel, &html, &css);
    })
}

/// Spawn the content thread with a URL to load.
///
/// Returns a `JoinHandle` for the thread.
pub(crate) fn spawn_content_thread_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    url: url::Url,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main_url(channel, &url);
    })
}

/// Spawn a blank new-tab content thread.
///
/// Renders a minimal "New Tab" page.
pub(crate) fn spawn_content_thread_blank(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(channel, crate::BLANK_TAB_HTML, crate::BLANK_TAB_CSS);
    })
}

fn content_thread_main(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    html: &str,
    css: &str,
) {
    // Apply sandbox before processing any content (design doc §8.1).
    // In SingleProcess mode this is a no-op (Unsandboxed).
    if let Err(e) = elidex_sandbox::apply_sandbox(&elidex_plugin::PlatformSandbox::Unsandboxed) {
        eprintln!("Sandbox enforcement failed (fatal): {e}");
        return;
    }

    let pipeline = crate::build_pipeline_interactive(html, css);
    let mut state = ContentState::new(channel, NavigationController::new(), pipeline);
    scroll::update_viewport_scroll_dimensions(&mut state);
    // Scan for <iframe> elements present in the initial parsed DOM.
    // Mutation-based detection only catches dynamically added iframes;
    // statically parsed iframes need an explicit initial scan.
    iframe::scan_initial_iframes(&mut state);
    // Re-render after initial scan so statically parsed iframes are composited
    // into the parent display list before the first send.
    state.re_render();
    state.send_display_list();
    run_event_loop(&mut state);
}

fn content_thread_main_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    url: &url::Url,
) {
    // Apply sandbox before processing any content (design doc §8.1).
    if let Err(e) = elidex_sandbox::apply_sandbox(&elidex_plugin::PlatformSandbox::Unsandboxed) {
        eprintln!("Sandbox enforcement failed (fatal): {e}");
        return;
    }

    let pipeline = match crate::build_pipeline_from_url(url) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Content thread: failed to load {url}: {e}");
            let _ = channel.send(ContentToBrowser::NavigationFailed {
                url: url.clone(),
                error: format!("{e}"),
            });
            return;
        }
    };

    let mut nav_controller = NavigationController::new();
    nav_controller.push(url.clone());

    let mut state = ContentState::new(channel, nav_controller, pipeline);
    scroll::update_viewport_scroll_dimensions(&mut state);
    // Scan for <iframe> elements present in the initial parsed DOM.
    iframe::scan_initial_iframes(&mut state);
    state.re_render();
    state.notify_navigation(url);
    run_event_loop(&mut state);
}

#[allow(clippy::too_many_lines)] // Event loop with iframe integration.
fn run_event_loop(state: &mut ContentState) {
    let mut last_frame = Instant::now();

    loop {
        let animations_running = state.pipeline.animation_engine.has_running();

        // Determine the poll timeout:
        // - When animations are running: target ~60 fps (16ms) using absolute deadline
        // - When JS timers are pending: wake at next timer deadline
        // - Otherwise: idle poll interval (100ms)
        // Use absolute frame deadline to avoid jitter from elapsed-time subtraction.
        let now_for_timeout = Instant::now();
        let timer_timeout = state
            .pipeline
            .runtime
            .next_timer_deadline()
            .map(|d| d.saturating_duration_since(now_for_timeout));
        let timeout = if animations_running {
            let next_frame = last_frame + FRAME_INTERVAL;
            // Minimum 1ms sleep to prevent CPU spin when frame deadline has passed.
            let frame_remaining = next_frame
                .saturating_duration_since(now_for_timeout)
                .max(Duration::from_millis(1));
            // Wake at whichever comes first: next frame or next timer.
            timer_timeout.map_or(frame_remaining, |t| frame_remaining.min(t))
        } else {
            timer_timeout.unwrap_or(DEFAULT_POLL_INTERVAL)
        };

        match state.channel.recv_timeout(timeout) {
            Ok(msg) => {
                if !handle_message(msg, state) {
                    break; // Shutdown
                }
                // Event handlers may destroy DOM elements with running animations.
                // Prune stale animation state to prevent unbounded memory growth.
                if state.pipeline.animation_engine.has_active() {
                    state.pipeline.prune_dead_animation_entities();
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // --- Frame tick: animations + timers ---
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        let mut needs_render = false;

        // Drain script-initiated animations (element.animate()) and apply to engine.
        apply_script_animations(state);

        // Tick animation engine if animations have active state (including fill values).
        // Use has_active() here (not has_running()) so fill-mode values are applied.
        // Guard against zero-dt (no elapsed time) but allow sub-millisecond ticks
        // for smooth playback on high-refresh (120Hz+) displays.
        if state.pipeline.animation_engine.has_active() && dt > Duration::ZERO {
            // Cap dt to prevent idle→active spike: when the engine was idle,
            // last_frame may be far in the past.  Without capping, newly started
            // transitions would instantly complete on the first tick.
            let dt_secs = dt.min(FRAME_INTERVAL * 2).as_secs_f64();
            let events = state.pipeline.animation_engine.tick(dt_secs);
            animation::dispatch_animation_events(&events, state);
            // Only update last_frame when animations are ticked, not on caret-only
            // renders, to avoid animation timing drift.
            last_frame = now;
            needs_render = true;
        }

        // Drain JS timers if any are ready.
        if state
            .pipeline
            .runtime
            .next_timer_deadline()
            .is_some_and(|d| d <= now)
        {
            state.pipeline.runtime.drain_timers(
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            needs_render = true;
        }

        // --- Iframe + messaging frame tick ---
        // Drain display list updates and postMessage events from OOP iframe threads.
        let post_messages = state.iframes.drain_oop_messages();
        for msg in &post_messages {
            dispatch_message_event(state, &msg.data, &msg.origin);
        }

        // Deliver self-postMessage events queued by window.postMessage().
        let self_messages = state.pipeline.runtime.bridge().drain_post_messages();
        for (data, origin) in &self_messages {
            dispatch_message_event(state, data, origin);
        }
        if !self_messages.is_empty() || !post_messages.is_empty() {
            needs_render = true;
        }

        // Drain pending localStorage changes and send to browser for cross-tab broadcast.
        for change in state.pipeline.runtime.bridge().drain_storage_changes() {
            let _ = state
                .channel
                .send(crate::ipc::ContentToBrowser::StorageChanged {
                    origin: change.origin,
                    key: change.key,
                    old_value: change.old_value,
                    new_value: change.new_value,
                    url: change.url,
                });
        }

        // Drain pending IDB versionchange requests for cross-tab broadcast.
        for req in state
            .pipeline
            .runtime
            .bridge()
            .drain_idb_versionchange_requests()
        {
            let _ = state
                .channel
                .send(crate::ipc::ContentToBrowser::IdbVersionChangeRequest {
                    request_id: req.request_id,
                    origin: req.origin,
                    db_name: req.db_name,
                    old_version: req.old_version,
                    new_version: req.new_version,
                });
        }

        // Batch-persist all dirty localStorage stores to disk (once per frame).
        elidex_js_boa::bridge::local_storage::flush_dirty_stores();

        // Drain pending window.open(_blank) requests from timers/animations.
        for url in state.pipeline.runtime.bridge().drain_pending_open_tabs() {
            let _ = state
                .channel
                .send(crate::ipc::ContentToBrowser::OpenNewTab(url));
        }

        // Drain pending window.focus() request.
        if state.pipeline.runtime.bridge().take_pending_focus() {
            let _ = state
                .channel
                .send(crate::ipc::ContentToBrowser::FocusWindow);
        }

        // Drain WebSocket and SSE events from I/O threads.
        {
            let (ws_events, sse_events) = state.pipeline.runtime.bridge().drain_realtime_events();
            let has_js_events = ws_events
                .iter()
                .any(|(_, e)| !matches!(e, elidex_net::ws::WsEvent::BytesSent(_)))
                || !sse_events.is_empty();
            if !ws_events.is_empty() || !sse_events.is_empty() {
                state.pipeline.runtime.dispatch_realtime_events(
                    ws_events,
                    sse_events,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if has_js_events {
                    needs_render = true;
                }
            }
        }

        // Drain worker messages.
        needs_render |= state.pipeline.runtime.drain_and_dispatch_worker_events(
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );

        // Drain timers for in-process (same-origin) iframes.
        iframe::tick_iframe_timers(state);

        // Caret blink update.
        if state.update_caret_blink() {
            needs_render = true;
        }

        if needs_render {
            state.re_render();
            state.send_display_list();
        }
    }
}

/// Handle a single message. Returns `false` for Shutdown.
#[allow(clippy::too_many_lines)]
fn handle_message(msg: BrowserToContent, state: &mut ContentState) -> bool {
    match msg {
        BrowserToContent::Shutdown => {
            // Dispatch beforeunload/unload lifecycle events before shutdown.
            // If beforeunload is cancelled, the user wants to stay on the page.
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if !proceed {
                // beforeunload cancelled — user wants to stay.
                // (In a real browser, this would show a confirmation dialog.)
                return true; // Continue event loop.
            }
            // Shutdown all child iframes before parent (WHATWG HTML §7.1.3).
            state.iframes.shutdown_all();
            // Close all WebSocket/SSE connections.
            state.pipeline.runtime.bridge().shutdown_all_realtime();
            // Terminate all workers.
            state.pipeline.runtime.bridge().shutdown_all_workers();
            return false;
        }

        BrowserToContent::Navigate(url) => {
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if !proceed {
                return true; // Blocked by beforeunload handler.
            }
            navigation::handle_navigate(state, &url, false, None);
        }

        BrowserToContent::MouseClick(ref click) => {
            event_handlers::handle_click(state, click);
        }

        BrowserToContent::MouseRelease { button: _ } => {
            event_handlers::handle_mouse_release(state);
        }

        BrowserToContent::MouseMove { point, .. } => {
            event_handlers::handle_mouse_move(state, point);
        }

        BrowserToContent::CursorLeft => {
            event_handlers::handle_cursor_left(state);
        }

        BrowserToContent::KeyDown {
            ref key,
            ref code,
            repeat,
            mods,
        } => {
            event_handlers::handle_key(state, "keydown", key, code, repeat, mods);
        }

        BrowserToContent::KeyUp {
            ref key,
            ref code,
            repeat,
            mods,
        } => {
            event_handlers::handle_key(state, "keyup", key, code, repeat, mods);
        }

        BrowserToContent::SetViewport { width, height } => {
            if width > 0.0 && width.is_finite() && height > 0.0 && height.is_finite() {
                state.pipeline.viewport = elidex_plugin::Size::new(width, height);
                // Sync viewport size to JS bridge for window.innerWidth/innerHeight.
                let bridge = state.pipeline.runtime.bridge().clone();
                bridge.set_viewport(width, height);

                // Re-evaluate media queries and dispatch "change" events to listeners.
                let changed = bridge.re_evaluate_media_queries(width, height);
                if !changed.is_empty() {
                    dispatch_media_query_changes(&changed, state);
                }

                // Dispatch "resize" event (CSSOM View §4.2).
                // Per spec this fires on Window; in our architecture window
                // listeners are wired to the document entity, so targeting
                // document is equivalent.
                let mut resize_event = elidex_script_session::DispatchEvent::new_composed(
                    "resize",
                    state.pipeline.document,
                );
                resize_event.bubbles = false;
                resize_event.cancelable = false;
                state.pipeline.runtime.dispatch_event(
                    &mut resize_event,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );

                state.re_render();
                state.send_display_list();
            }
        }

        BrowserToContent::VisibilityChanged { visible } => {
            // Dispatch "visibilitychange" event on the document (Page Visibility §4.1).
            let mut event = elidex_script_session::DispatchEvent::new_composed(
                "visibilitychange",
                state.pipeline.document,
            );
            event.bubbles = false;
            event.cancelable = false;
            // Store visibility state in the bridge for document.visibilityState.
            state.pipeline.runtime.bridge().set_visibility(visible);
            state.pipeline.runtime.dispatch_event(
                &mut event,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            state.re_render();
            state.send_display_list();
        }

        BrowserToContent::GoBack => {
            // Check navigability before dispatching unload events.
            if state.nav_controller.can_go_back() {
                let proceed = crate::pipeline::dispatch_unload_events(
                    &mut state.pipeline.runtime,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if proceed {
                    if let Some(url) = state.nav_controller.go_back().cloned() {
                        navigation::handle_navigate(state, &url, true, None);
                    }
                }
            }
        }

        BrowserToContent::GoForward => {
            if state.nav_controller.can_go_forward() {
                let proceed = crate::pipeline::dispatch_unload_events(
                    &mut state.pipeline.runtime,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if proceed {
                    if let Some(url) = state.nav_controller.go_forward().cloned() {
                        navigation::handle_navigate(state, &url, true, None);
                    }
                }
            }
        }

        BrowserToContent::Reload => {
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if proceed {
                if let Some(url) = state.pipeline.url.clone() {
                    navigation::handle_navigate(state, &url, true, None);
                }
            }
        }

        BrowserToContent::MouseWheel { delta, point } => {
            scroll::handle_wheel(state, delta, point);
        }

        BrowserToContent::Ime { kind } => {
            ime::handle_ime(state, kind);
        }

        BrowserToContent::StorageEvent {
            key,
            old_value,
            new_value,
            url,
        } => {
            dispatch_storage_event(state, key, old_value, new_value, url);
        }

        // --- IndexedDB cross-tab versionchange (W3C IndexedDB §2.4) ---
        BrowserToContent::IdbVersionChange {
            request_id,
            db_name,
            old_version,
            new_version,
        } => {
            // Fire versionchange event on open connections and close them.
            state.pipeline.runtime.dispatch_idb_versionchange(
                &db_name,
                old_version,
                new_version,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            // Notify browser that connections are closed.
            let _ = state.channel.send(ContentToBrowser::IdbConnectionsClosed {
                request_id,
                db_name,
            });
        }

        // IDB upgrade/blocked: TODO(M4-10). Storage API responses: no-op here.
        BrowserToContent::IdbUpgradeReady { .. }
        | BrowserToContent::IdbBlocked { .. }
        | BrowserToContent::StorageEstimateResult { .. }
        | BrowserToContent::StoragePersistResult { .. }
        | BrowserToContent::StoragePersistedResult { .. } => {}
    }
    true
}

/// Dispatch "change" events to `MediaQueryList` listeners whose result changed.
///
/// Creates a `MediaQueryListEvent`-like object with `matches` and `media` properties
/// and invokes each registered listener callback via `JsRuntime`.
fn dispatch_media_query_changes(changed: &[(u64, bool)], state: &mut ContentState) {
    state.pipeline.runtime.deliver_media_query_changes(
        changed,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Focusability-relevant attribute names.
///
/// Changes to these attributes affect whether an element is focusable or
/// its position in the sequential focus navigation order (HTML §6.6.3).
const FOCUSABLE_ATTRIBUTES: &[&str] = &["tabindex", "disabled", "contenteditable", "hidden"];

/// Check whether any mutation record requires invalidating the focusable cache.
///
/// Returns `true` for `ChildList` mutations (elements added/removed) and
/// `Attribute` mutations on focusability-relevant attributes.
fn should_invalidate_focusable_cache(records: &[elidex_script_session::MutationRecord]) -> bool {
    use elidex_script_session::MutationKind;

    records.iter().any(|r| match r.kind {
        MutationKind::ChildList => true,
        MutationKind::Attribute => r
            .attribute_name
            .as_deref()
            .is_some_and(|name| FOCUSABLE_ATTRIBUTES.contains(&name)),
        _ => false,
    })
}

/// Dispatch a `storage` event on window (WHATWG HTML §11.2.1).
///
/// Fired when another tab changes `localStorage` for the same origin.
/// Per spec this fires on `Window`; in our architecture window listeners
/// are wired to the document entity, so targeting document is equivalent.
fn dispatch_storage_event(
    state: &mut ContentState,
    key: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
    url: String,
) {
    let mut event =
        elidex_script_session::DispatchEvent::new_composed("storage", state.pipeline.document);
    event.bubbles = false;
    event.cancelable = false;
    event.payload = elidex_plugin::EventPayload::Storage {
        key,
        old_value,
        new_value,
        url,
    };
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Dispatch a `MessageEvent` on the parent document (WHATWG HTML §9.4.3).
fn dispatch_message_event(state: &mut ContentState, data: &str, origin: &str) {
    let mut event =
        elidex_script_session::DispatchEvent::new_composed("message", state.pipeline.document);
    event.bubbles = false;
    event.cancelable = false;
    event.payload = elidex_plugin::EventPayload::Message {
        data: data.to_string(),
        origin: origin.to_string(),
        last_event_id: String::new(),
    };
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Drain pending script-initiated animations from the bridge and apply them
/// to the `AnimationEngine`. Converts `ScriptAnimation` options into
/// `SingleAnimationSpec` + `KeyframesRule` and registers them.
fn apply_script_animations(state: &mut ContentState) {
    let bridge = state.pipeline.runtime.bridge();
    let pending = bridge.drain_script_animations();
    if pending.is_empty() {
        return;
    }

    let current_time = state.pipeline.animation_engine.timeline().current_time();

    for anim in pending {
        // Convert parsed keyframes to KeyframesRule.
        let mut keyframes = Vec::new();
        let num_kf = anim.keyframes.len();
        for (i, kf) in anim.keyframes.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let offset = kf.offset.unwrap_or_else(|| {
                if num_kf <= 1 {
                    1.0
                } else {
                    i as f64 / (num_kf - 1) as f64
                }
            });
            #[allow(clippy::cast_possible_truncation)]
            let declarations: Vec<elidex_plugin::PropertyDeclaration> = kf
                .declarations
                .iter()
                .map(|(prop, val)| elidex_plugin::PropertyDeclaration {
                    property: prop.clone(),
                    value: elidex_plugin::CssValue::Keyword(val.clone()),
                })
                .collect();
            keyframes.push(elidex_css_anim::parse::Keyframe {
                #[allow(clippy::cast_possible_truncation)]
                offset: offset as f32,
                declarations,
                timing_function: None,
            });
        }

        // Generate a unique name for this script animation.
        // Use a monotonic counter to avoid name collisions when multiple
        // animations are created on the same element without explicit ids.
        let name = if anim.options.id.is_empty() {
            let seq = SCRIPT_ANIM_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!("__script_anim_{}_{seq}", anim.entity_id)
        } else {
            anim.options.id.clone()
        };

        let rule = elidex_css_anim::parse::KeyframesRule {
            name: name.clone(),
            keyframes,
        };
        state.pipeline.animation_engine.register_keyframes(rule);

        // Convert options to SingleAnimationSpec.
        #[allow(clippy::cast_possible_truncation)]
        let duration = (anim.options.duration / 1000.0) as f32; // ms → seconds
        #[allow(clippy::cast_possible_truncation)]
        let delay = (anim.options.delay / 1000.0) as f32;

        let iteration_count = if anim.options.iterations.is_infinite() {
            elidex_css_anim::style::IterationCount::Infinite
        } else {
            #[allow(clippy::cast_possible_truncation)]
            elidex_css_anim::style::IterationCount::Number(anim.options.iterations as f32)
        };

        let direction = match anim.options.direction.as_str() {
            "reverse" => elidex_css_anim::style::AnimationDirection::Reverse,
            "alternate" => elidex_css_anim::style::AnimationDirection::Alternate,
            "alternate-reverse" => elidex_css_anim::style::AnimationDirection::AlternateReverse,
            _ => elidex_css_anim::style::AnimationDirection::Normal,
        };

        let fill_mode = match anim.options.fill.as_str() {
            "forwards" => elidex_css_anim::style::AnimationFillMode::Forwards,
            "backwards" => elidex_css_anim::style::AnimationFillMode::Backwards,
            "both" => elidex_css_anim::style::AnimationFillMode::Both,
            _ => elidex_css_anim::style::AnimationFillMode::None,
        };

        let spec = elidex_css_anim::SingleAnimationSpec {
            name,
            duration,
            timing_function: elidex_css_anim::timing::TimingFunction::Linear,
            delay,
            iteration_count,
            direction,
            fill_mode,
            play_state: elidex_css_anim::style::PlayState::Running,
        };

        let instance = elidex_css_anim::instance::AnimationInstance::new(&spec, current_time);
        state
            .pipeline
            .animation_engine
            .add_animation(anim.entity_id, instance);
    }
}

#[cfg(test)]
#[path = "../content_tests.rs"]
mod content_tests;
