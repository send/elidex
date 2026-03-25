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
        // Collect (entity, display_list) pairs to avoid borrow conflict.
        let mut updated: Vec<(elidex_ecs::Entity, elidex_render::DisplayList)> = Vec::new();
        for (&entity, entry) in self.iframes.iter_mut() {
            if let iframe::IframeHandle::InProcess(ref mut ip) = entry.handle {
                if ip.needs_render {
                    crate::re_render(&mut ip.pipeline);
                    ip.needs_render = false;
                    updated.push((entity, ip.pipeline.display_list.clone()));
                }
            }
        }
        // Store each iframe's display list on the parent DOM so the
        // display list builder can emit SubDisplayList items.
        for (entity, dl) in updated {
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
        detect_iframe_mutations(&mutation_records, self);

        // Check lazy iframes: load those that have entered the viewport.
        // Uses LayoutBox position vs viewport bounds with 200px margin.
        check_lazy_iframes(self);

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
    scan_initial_iframes(&mut state);
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
    scan_initial_iframes(&mut state);
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
        // Deliver postMessage events to parent document's JS runtime as MessageEvent.
        for (_iframe_entity, data, origin) in &post_messages {
            let message_init = elidex_plugin::EventPayload::Message {
                data: data.clone(),
                origin: origin.clone(),
            };
            let mut event = elidex_script_session::DispatchEvent::new_composed(
                "message",
                state.pipeline.document,
            );
            event.payload = message_init;
            state.pipeline.runtime.dispatch_event(
                &mut event,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
        }

        // Deliver self-postMessage events queued by window.postMessage().
        let self_messages = state.pipeline.runtime.bridge().drain_post_messages();
        for (data, origin) in &self_messages {
            let mut event = elidex_script_session::DispatchEvent::new_composed(
                "message",
                state.pipeline.document,
            );
            event.payload = elidex_plugin::EventPayload::Message {
                data: data.clone(),
                origin: origin.clone(),
            };
            state.pipeline.runtime.dispatch_event(
                &mut event,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
        }
        if !self_messages.is_empty() || !post_messages.is_empty() {
            needs_render = true;
        }

        // Drain timers for in-process (same-origin) iframes.
        // Timers always run (for correctness), but layout/render is skipped
        // for iframes outside the viewport to save CPU.
        for (&iframe_entity, entry) in state.iframes.iter_mut() {
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
                    // Only mark for re-render if the iframe is within the viewport.
                    // Iframes outside the viewport still run timers but skip
                    // the expensive layout/render pass.
                    let in_viewport = state
                        .pipeline
                        .dom
                        .world()
                        .get::<&elidex_plugin::LayoutBox>(iframe_entity)
                        .ok()
                        .is_some_and(|lb| {
                            let vp_w = state.pipeline.viewport.width;
                            let vp_h = state.pipeline.viewport.height;
                            let left = lb.content.origin.x;
                            let top = lb.content.origin.y;
                            let right = left + lb.content.size.width;
                            let bottom = top + lb.content.size.height;
                            right >= 0.0 && left <= vp_w && bottom >= 0.0 && top <= vp_h
                        });
                    if in_viewport {
                        ip.needs_render = true;
                    }
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

/// Detect iframe additions/removals from mutation records.
///
/// Scans `MutationRecord` added/removed nodes for entities with `IframeData`
/// components, and triggers iframe loading/unloading accordingly.
///
/// Also detects `src` attribute changes on existing `<iframe>` elements
/// to trigger re-navigation.
fn detect_iframe_mutations(
    records: &[elidex_script_session::MutationRecord],
    state: &mut ContentState,
) {
    use elidex_script_session::MutationKind;

    for record in records {
        match record.kind {
            MutationKind::ChildList => {
                // Check added nodes for <iframe> elements.
                for &entity in &record.added_nodes {
                    // Skip if already loaded (e.g., moved within DOM).
                    if state.iframes.get(entity).is_some() {
                        continue;
                    }
                    try_load_iframe_entity(state, entity);
                }
                // Check removed nodes for <iframe> elements.
                for &entity in &record.removed_nodes {
                    if let Some(removed_entry) = state.iframes.remove(entity) {
                        // Dispatch beforeunload/unload on the iframe's document
                        // before dropping it (WHATWG HTML §7.1.3).
                        if let iframe::IframeHandle::InProcess(mut ip) = removed_entry.handle {
                            crate::pipeline::dispatch_unload_events(
                                &mut ip.pipeline.runtime,
                                &mut ip.pipeline.session,
                                &mut ip.pipeline.dom,
                                ip.pipeline.document,
                            );
                        }
                        if state.focused_iframe == Some(entity) {
                            state.focused_iframe = None;
                        }
                    }
                }
            }
            MutationKind::Attribute => {
                // src attribute change on <iframe> → re-navigate.
                if record
                    .attribute_name
                    .as_deref()
                    .is_some_and(|name| name == "src")
                {
                    let target = record.target;
                    state.iframes.remove(target);
                    try_load_iframe_entity(state, target);
                }
            }
            _ => {}
        }
    }
}

/// Register a loaded iframe: store its display list on the parent DOM,
/// insert into the registry, and dispatch the `load` event.
fn register_iframe_entry(
    state: &mut ContentState,
    entity: elidex_ecs::Entity,
    entry: iframe::IframeEntry,
) {
    if let iframe::IframeHandle::InProcess(ref ip) = entry.handle {
        let _ = state.pipeline.dom.world_mut().insert_one(
            entity,
            elidex_render::IframeDisplayList(ip.pipeline.display_list.clone()),
        );
    }
    state.iframes.insert(entity, entry);
    dispatch_iframe_load_event(state, entity);
}

/// Count the iframe nesting depth of an entity by walking its DOM ancestors.
///
/// Returns the number of ancestor elements that have `IframeData` components.
/// Used for `MAX_IFRAME_DEPTH` enforcement to prevent runaway nesting.
fn count_iframe_ancestor_depth(dom: &elidex_ecs::EcsDom, entity: elidex_ecs::Entity) -> usize {
    let mut depth = 0;
    let mut current = dom.get_parent(entity);
    let mut steps = 0;
    while let Some(parent) = current {
        steps += 1;
        if steps > elidex_ecs::MAX_ANCESTOR_DEPTH {
            break;
        }
        if dom.world().get::<&elidex_ecs::IframeData>(parent).is_ok() {
            depth += 1;
        }
        current = dom.get_parent(parent);
    }
    depth
}

/// Check lazy iframes and load those near the viewport.
///
/// Uses `LayoutBox` position to determine if a lazy iframe is within 200px
/// of the viewport bounds. Once loaded, the entity is removed from the
/// pending list. Iframes without a `LayoutBox` (e.g., inside a `display:none`
/// parent) remain in the pending list until layout is computed.
fn check_lazy_iframes(state: &mut ContentState) {
    if state.lazy_iframe_pending.is_empty() {
        return;
    }

    let vp_width = state.pipeline.viewport.width;
    let vp_height = state.pipeline.viewport.height;
    let scroll_x = state.viewport_scroll.scroll_offset.x;
    let scroll_y = state.viewport_scroll.scroll_offset.y;
    let margin = 200.0_f32; // Load iframes within 200px of viewport edge.

    let visible_left = scroll_x - margin;
    let visible_right = scroll_x + vp_width + margin;
    let visible_top = scroll_y - margin;
    let visible_bottom = scroll_y + vp_height + margin;

    // Collect entities to load (to avoid borrow conflict with state).
    let to_load: Vec<elidex_ecs::Entity> = state
        .lazy_iframe_pending
        .iter()
        .copied()
        .filter(|&entity| {
            state
                .pipeline
                .dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(entity)
                .ok()
                .is_some_and(|lb| {
                    let left = lb.content.origin.x;
                    let right = left + lb.content.size.width;
                    let top = lb.content.origin.y;
                    let bottom = top + lb.content.size.height;
                    // Iframe overlaps the extended viewport (2D check).
                    right >= visible_left
                        && left <= visible_right
                        && bottom >= visible_top
                        && top <= visible_bottom
                })
        })
        .collect();

    if to_load.is_empty() {
        return;
    }

    // Remove loaded entities from pending list.
    state.lazy_iframe_pending.retain(|e| !to_load.contains(e));

    // Load each visible lazy iframe.
    for entity in to_load {
        // Re-read IframeData since we're loading now.
        let iframe_data = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(entity)
            .ok()
            .map(|d| (*d).clone());
        if let Some(data) = iframe_data {
            let parent_origin = state.pipeline.runtime.bridge().origin();
            let depth = count_iframe_ancestor_depth(&state.pipeline.dom, entity);
            let entry = iframe::load_iframe(
                entity,
                &data,
                &parent_origin,
                state.pipeline.url.as_ref(),
                &state.pipeline.font_db,
                &state.pipeline.fetch_handle,
                &state.pipeline.registry,
                depth,
            );
            register_iframe_entry(state, entity, entry);
        }
    }
}

/// Dispatch a "load" event on an iframe element entity in the parent document.
///
/// Per WHATWG HTML §4.8.5: when an iframe's content is loaded, a "load"
/// event fires on the `<iframe>` element (not on the iframe's document).
fn dispatch_iframe_load_event(state: &mut ContentState, iframe_entity: elidex_ecs::Entity) {
    let mut event = elidex_script_session::DispatchEvent::new("load", iframe_entity);
    event.bubbles = false;
    event.cancelable = false;
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Try to load an iframe for `entity` if it has `IframeData`.
///
/// Respects `loading="lazy"`: lazy iframes are skipped here and will be
/// loaded when an `IntersectionObserver` detects they are near the viewport.
fn try_load_iframe_entity(state: &mut ContentState, entity: elidex_ecs::Entity) {
    let iframe_data = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_ecs::IframeData>(entity)
        .ok()
        .map(|d| (*d).clone());
    if let Some(data) = iframe_data {
        // loading="lazy": defer loading until near viewport (WHATWG HTML §4.8.5).
        // Registers the entity in the pending list; the event loop checks
        // LayoutBox positions each frame to detect viewport proximity.
        if data.loading == elidex_ecs::LoadingAttribute::Lazy {
            if !state.lazy_iframe_pending.contains(&entity) {
                state.lazy_iframe_pending.push(entity);
            }
            return;
        }
        let parent_origin = state.pipeline.runtime.bridge().origin();
        let depth = count_iframe_ancestor_depth(&state.pipeline.dom, entity);
        let entry = iframe::load_iframe(
            entity,
            &data,
            &parent_origin,
            state.pipeline.url.as_ref(),
            &state.pipeline.font_db,
            &state.pipeline.fetch_handle,
            &state.pipeline.registry,
            depth,
        );
        register_iframe_entry(state, entity, entry);
    }
}

/// Walk the DOM tree and load any `<iframe>` elements found during initial parse.
///
/// Mutation-based detection (`detect_iframe_mutations`) only catches iframes
/// added via JS. This function handles iframes present in the initial HTML.
fn scan_initial_iframes(state: &mut ContentState) {
    let mut iframes_to_load = Vec::new();
    collect_iframe_entities(
        &state.pipeline.dom,
        state.pipeline.document,
        &mut iframes_to_load,
        0,
    );
    for entity in iframes_to_load {
        try_load_iframe_entity(state, entity);
    }
}

/// Recursively collect entities with `IframeData` components.
fn collect_iframe_entities(
    dom: &elidex_ecs::EcsDom,
    entity: elidex_ecs::Entity,
    result: &mut Vec<elidex_ecs::Entity>,
    depth: usize,
) {
    if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }
    if dom.world().get::<&elidex_ecs::IframeData>(entity).is_ok() {
        result.push(entity);
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        collect_iframe_entities(dom, c, result, depth + 1);
        child = dom.get_next_sibling(c);
    }
}

#[cfg(test)]
#[path = "../content_tests.rs"]
mod content_tests;
