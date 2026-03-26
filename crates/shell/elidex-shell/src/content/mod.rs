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
    #[allow(dead_code)] // Used when iframe event routing is implemented.
    focused_iframe: Option<Entity>,
    /// Entities awaiting lazy load (loading="lazy" iframes not yet in viewport).
    lazy_iframe_pending: Vec<Entity>,
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
            lazy_iframe_pending: Vec::new(),
        }
    }

    /// Re-render all in-process iframes that need it, then re-render the parent.
    ///
    /// Iterates registered iframes and calls `crate::re_render()` on each
    /// in-process iframe whose `needs_render` flag is set, then re-renders
    /// the parent document. This ensures child iframe display lists are
    /// up-to-date before the parent composites them.
    fn re_render_all_iframes(&mut self) {
        // Collect (entity, Arc<DisplayList>) pairs to avoid borrow conflict.
        // The Arc is cached in InProcessIframe to avoid re-cloning the full
        // DisplayList every frame — only re-created when needs_render is true.
        let mut updated: Vec<(
            elidex_ecs::Entity,
            std::sync::Arc<elidex_render::DisplayList>,
        )> = Vec::new();
        for (&entity, entry) in self.iframes.iter_mut() {
            if let iframe::IframeHandle::InProcess(ref mut ip) = entry.handle {
                if ip.needs_render {
                    crate::re_render(&mut ip.pipeline);
                    ip.needs_render = false;
                    // Note: clones the full DisplayList into Arc. This happens only when
                    // needs_render is true (not every frame). To eliminate the clone,
                    // PipelineResult.display_list would need to be Arc<DisplayList>,
                    // which is a broader structural change deferred to Phase 5.
                    let arc_dl = std::sync::Arc::new(ip.pipeline.display_list.clone());
                    ip.cached_display_list = Some(std::sync::Arc::clone(&arc_dl));
                    updated.push((entity, arc_dl));
                }
            }
        }
        // Store each iframe's display list on the parent DOM so the
        // display list builder can emit SubDisplayList items.
        for (entity, dl) in updated {
            // Remove then insert: hecs insert_one fails if component exists.
            let _ = self
                .pipeline
                .dom
                .world_mut()
                .remove_one::<elidex_render::IframeDisplayList>(entity);
            let _ = self
                .pipeline
                .dom
                .world_mut()
                .insert_one(entity, elidex_render::IframeDisplayList(dl));
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
        self.re_render_all_iframes();

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
        iframe::detect_iframe_mutations(&mutation_records, self);

        // Check lazy iframes: load those that have entered the viewport.
        // Uses LayoutBox position vs viewport bounds with 200px margin.
        iframe::check_lazy_iframes(self);

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
    let pipeline = crate::build_pipeline_interactive(html, css);
    let mut state = ContentState::new(channel, NavigationController::new(), pipeline);
    scroll::update_viewport_scroll_dimensions(&mut state);
    // Scan for <iframe> elements present in the initial parsed DOM.
    // Mutation-based detection only catches dynamically added iframes;
    // statically parsed iframes need an explicit initial scan.
    iframe::scan_initial_iframes(&mut state);
    state.send_display_list();
    run_event_loop(&mut state);
}

fn content_thread_main_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    url: &url::Url,
) {
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

        // --- Iframe frame tick: drain OOP messages + in-process timers ---
        // Drain display list updates and postMessage events from cross-origin iframe threads.
        let post_messages = state.iframes.drain_oop_messages();
        for (_iframe_entity, data, origin) in &post_messages {
            dispatch_message_event(state, data, origin);
        }

        // Deliver self-postMessage events queued by window.postMessage().
        let self_messages = state.pipeline.runtime.bridge().drain_post_messages();
        for (data, origin) in &self_messages {
            dispatch_message_event(state, data, origin);
        }
        if !self_messages.is_empty() || !post_messages.is_empty() {
            needs_render = true;
        }

        // Drain pending window.open(_blank) requests from timers/animations.
        // process_pending_actions handles these from input handlers, but
        // window.open() called from setTimeout/requestAnimationFrame needs
        // to be drained here too.
        for url in state.pipeline.runtime.bridge().drain_pending_open_tabs() {
            let _ = state
                .channel
                .send(crate::ipc::ContentToBrowser::OpenNewTab(url));
        }

        // Drain timers for in-process (same-origin) iframes.
        // Timers always run (for correctness), but layout/render is skipped
        // for iframes outside the viewport to save CPU.
        for (_, entry) in state.iframes.iter_mut() {
            if let iframe::IframeHandle::InProcess(ref mut ip) = entry.handle {
                if ip
                    .pipeline
                    .runtime
                    .next_timer_deadline()
                    .is_some_and(|d| d <= now)
                {
                    ip.pipeline.runtime.drain_timers(
                        &mut ip.pipeline.session,
                        &mut ip.pipeline.dom,
                        ip.pipeline.document,
                    );
                    // Always mark for re-render when timers fire. The actual
                    // layout/render is deferred until the iframe scrolls into
                    // the viewport (checked in re_render_all_iframes).
                    ip.needs_render = true;
                }
            }
        }

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
            // Send Shutdown to all OOP iframes and join threads.
            state.iframes.shutdown_all();
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

                state.re_render();
                state.send_display_list();
            }
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

/// Dispatch a `MessageEvent` on the parent document (WHATWG HTML §9.4.3).
fn dispatch_message_event(state: &mut ContentState, data: &str, origin: &str) {
    let mut event =
        elidex_script_session::DispatchEvent::new_composed("message", state.pipeline.document);
    event.bubbles = false;
    event.cancelable = false;
    event.payload = elidex_plugin::EventPayload::Message {
        data: data.to_string(),
        origin: origin.to_string(),
    };
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

#[cfg(test)]
#[path = "../content_tests.rs"]
mod content_tests;
