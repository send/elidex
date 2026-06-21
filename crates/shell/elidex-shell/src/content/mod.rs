//! Content thread: owns the DOM, JS runtime, and rendering pipeline.
//!
//! The content thread runs a message loop, processing events from the browser
//! thread and sending back display list updates.

mod animation;
mod event_handlers;
mod event_loop;
pub(crate) mod focus;
mod form_input;
pub(crate) mod iframe;
mod ime;
mod navigation;
mod scroll;

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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
    /// Wake the browser event loop to schedule a repaint after a
    /// display/chrome-affecting send. Under `ControlFlow::Wait` a content-initiated
    /// frame (timer / rAF / animation / async DOM / `SetViewport` round-trip)
    /// would otherwise paint only on the next OS event; calling `wake()` schedules
    /// a redraw so the frame reaches a rendering opportunity (WHATWG HTML
    /// §8.1.7.3). Windowing-agnostic
    /// (`crate::WakeHandle = Box<dyn Fn() + Send>`) so the content thread stays
    /// winit-free.
    wake: crate::WakeHandle,
}

impl ContentState {
    /// Send a `ContentToBrowser` message AND wake the browser event loop so a
    /// content-initiated UI change reaches a rendering opportunity (WHATWG HTML
    /// §8.1.7.3) rather than stalling under `ControlFlow::Wait`.
    ///
    /// The chokepoint for every **rendering / chrome / window-action** variant —
    /// `DisplayListReady`, `TitleChanged`, `UrlChanged`, `NavigationState` (via the
    /// `send_*` helpers below) plus `OpenNewTab` / `FocusWindow` (routed at their
    /// call sites). Defined by `ContentToBrowser` *variant* (the read contract),
    /// not by which helper was called.
    ///
    /// Pure **non-rendering coordination** messages (`StorageChanged` /
    /// `IdbVersionChangeRequest` / `SwRegister` / `IdbConnectionsClosed` /
    /// `ManifestDiscovered`) keep the bare `self.channel.send`: they change no
    /// on-screen state, so they need no repaint. Their delivery latency under
    /// `ControlFlow::Wait` (processed on the next drain rather than an immediate
    /// wake) is a pre-existing, separate concern — slot
    /// `#11-content-message-coordination-wake`.
    fn notify_browser(&self, msg: ContentToBrowser) {
        let _ = self.channel.send(msg);
        (self.wake)();
    }

    /// Send the current display list to the browser thread (+ wake).
    fn send_display_list(&self) {
        self.notify_browser(ContentToBrowser::DisplayListReady(
            self.pipeline.display_list.clone(),
        ));
    }

    /// Send current navigation state (`can_go_back`/`can_go_forward`) (+ wake).
    fn send_navigation_state(&self) {
        self.notify_browser(ContentToBrowser::NavigationState {
            can_go_back: self.nav_controller.can_go_back(),
            can_go_forward: self.nav_controller.can_go_forward(),
        });
    }

    /// Send URL change notification to the browser thread (+ wake).
    fn send_url_changed(&self, url: &url::Url) {
        self.notify_browser(ContentToBrowser::UrlChanged(url.clone()));
    }

    /// Send a window-title change to the browser thread (+ wake). The single
    /// `TitleChanged` chokepoint (previously raw `channel.send` at two sites).
    fn send_title(&self, title: String) {
        self.notify_browser(ContentToBrowser::TitleChanged(title));
    }

    /// Send all post-navigation notifications to the browser thread
    /// (title, URL, navigation state, display list).
    fn notify_navigation(&self, url: &url::Url) {
        let title = format!("elidex \u{2014} {url}");
        self.send_title(title);
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
        wake: crate::WakeHandle,
    ) -> Self {
        Self {
            channel,
            nav_controller,
            hover_chain: Vec::new(),
            active_chain: Vec::new(),
            caret_visible: true,
            caret_last_toggle: Instant::now(),
            focusable_cache: None,
            viewport_scroll: elidex_ecs::ScrollState::default(),
            pipeline,
            iframes: iframe::IframeRegistry::new(),
            focused_iframe: None,
            wake,
        }
    }

    /// Echo the committed viewport scroll offset to the two JS-observable
    /// consumers: the document-root `ScrollState` component (read by
    /// `getBoundingClientRect` via `accumulated_scroll_offset`, CSSOM View §5)
    /// and the script bridge (`window.scrollX` / `scrollY`).
    ///
    /// Both viewport-scroll commit paths route through this single sink so they
    /// cannot diverge: the `re_render` path (JS `scrollTo` / `scrollBy` applied
    /// via `take_pending_scroll`) and the wheel fast path
    /// (`scroll::handle_wheel`), which patches the display list in place without
    /// re-rendering and would otherwise leave `scrollX` / `scrollY` and
    /// `getBoundingClientRect` stale after user wheel scrolling until some
    /// unrelated render happened.
    fn echo_viewport_scroll(&mut self) {
        // Store viewport scroll on the document root so getBoundingClientRect
        // includes it via accumulated_scroll_offset (CSSOM View §5).
        let _ = self
            .pipeline
            .dom
            .world_mut()
            .insert_one(self.pipeline.document, self.viewport_scroll.clone());
        // Sync scroll offset to the script bridge so scrollX/scrollY reflect
        // current state.
        self.pipeline.runtime.bridge().set_scroll_offset(
            self.viewport_scroll.scroll_offset.x,
            self.viewport_scroll.scroll_offset.y,
        );
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

        // Drain any pending JS scroll (scrollTo/scrollBy) and apply the requested
        // offset so the display list builds toward it. The CLAMP is deferred to
        // AFTER `crate::re_render` recomputes layout (below): a script that
        // mutated layout and scrolled in the same turn — e.g. appended tall
        // content then `scrollTo` its bottom — must clamp against the NEW content
        // size, not the stale pre-layout one (Codex R6 "clamp script scrolls
        // after layout is refreshed").
        let pending_scroll = self.pipeline.runtime.bridge().take_pending_scroll();
        if let Some((x, y)) = pending_scroll {
            self.viewport_scroll.scroll_offset = elidex_plugin::Vector::new(x, y);
        }

        // Sync viewport scroll offset to pipeline for display list building.
        self.pipeline.scroll_offset = self.viewport_scroll.scroll_offset;
        // Echo to the JS-observable consumers (scrollX/scrollY + the document-root
        // ScrollState for getBoundingClientRect) through the shared chokepoint.
        self.echo_viewport_scroll();
        // Reconcile the focused-iframe side field against the parent's canonical
        // FOCUS bit — pass 1 of 2, the SYNCHRONOUS case, BEFORE the iframe render
        // pass below. A parent-side script `focus()` this turn may have moved the
        // FOCUS bit off the `<iframe>` element without `handle_click`'s blur path,
        // leaving a STILL-VISIBLE in-process child painting `:focus` / caret with
        // a stale `activeElement`. `current_focus` reads the canonical bit the
        // script already moved, so `reconcile_focused_iframe` blurs the child and
        // flags it `needs_render` here, before `re_render_all_iframes` rebuilds
        // the child display list — otherwise the visible iframe's blur lands a
        // frame late (Codex S2 R5). The complementary ASYNCHRONOUS case (the
        // iframe element made non-focusable by a *buffered* mutation, e.g.
        // `iframe.hidden = true`) can only be reconciled after `crate::re_render`
        // flushes + GCs the bit — see pass 2 below.
        event_handlers::reconcile_focused_iframe(self);

        // Re-render in-process iframes before the parent so child display
        // lists are up-to-date when the parent composites them.
        iframe::re_render_all_iframes(self);

        let mutation_records = crate::re_render(&mut self.pipeline);

        // Now that layout boxes reflect this turn's mutations, refresh the
        // viewport scroll dimensions and clamp the offset against the fresh
        // content size, then re-sync the (possibly clamped) offset to the display
        // list + the JS-observable echo when it moved. Two paths need this:
        //  - a script `scrollTo`/`scrollBy` applied pre-layout used the unclamped
        //    request (the pre-layout echo above), so "scroll to the bottom of
        //    just-added content" lands on the NEW max instead of the stale old
        //    max and lost (Codex R6); and
        //  - a resize / content shrink with no pending scroll can push the
        //    existing offset past the new max, which must likewise re-clamp +
        //    re-sync this frame rather than leaving the display list / bridge /
        //    document-root `ScrollState` stale until some later render (F4).
        // `clamp_scroll` only ever shrinks the offset, so the display list built
        // in `crate::re_render` (with the pre-layout offset) already carries a
        // `PushScrollOffset` wrapper to patch — no 0→non-zero rebuild needed here.
        let pre_clamp_offset = self.viewport_scroll.scroll_offset;
        scroll::update_viewport_scroll_dimensions(self);
        let clamped_offset = self.viewport_scroll.scroll_offset;
        if pending_scroll.is_some() || clamped_offset != pre_clamp_offset {
            self.pipeline.scroll_offset = clamped_offset;
            self.pipeline
                .display_list
                .update_scroll_offset(clamped_offset);
            self.echo_viewport_scroll();
        }

        // Invalidate focusable cache when DOM structure or focusability changes.
        if should_invalidate_focusable_cache(&mutation_records) {
            self.focusable_cache = None;
        }

        // Reconcile the focused-iframe side field — pass 2 of 2, the ASYNCHRONOUS
        // case, AFTER `crate::re_render`'s focusability GC. When a parent mutation
        // makes the focused `<iframe>` non-focusable (e.g. `iframe.hidden = true`),
        // that buffered attribute is only flushed + reconciled (the parent's
        // `current_focus ⟹ is_focusable` GC, WHATWG HTML "update the rendering"
        // step 17) INSIDE `crate::re_render` above — after pass 1 ran with the bit
        // still set. By now the GC has cleared the bit, so `current_focus` no
        // longer points at the `<iframe>` and this pass blurs the child, clearing
        // its JS-observable `activeElement` and dropping `focused_iframe` so key
        // routing stops. A non-focusable iframe is not composited, so its display
        // list staleness is moot — `blur_iframe_focus` flags `needs_render` for a
        // later un-hide (Codex S2 R8). (The parent / in-process / OOP focusability
        // GC itself shares the one `crate::re_render` chokepoint — see there.)
        event_handlers::reconcile_focused_iframe(self);

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
        let is_text_focused =
            elidex_dom_api::focus::current_focus(&self.pipeline.dom, self.pipeline.document)
                .is_some_and(|target| {
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
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    html: String,
    css: String,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(channel, network_handle, cookie_jar, &html, &css, wake);
    })
}

/// Spawn the content thread with a URL to load.
///
/// Returns a `JoinHandle` for the thread.
pub(crate) fn spawn_content_thread_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    url: url::Url,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main_url(channel, network_handle, cookie_jar, &url, wake);
    })
}

/// Spawn a blank new-tab content thread.
///
/// Renders a minimal "New Tab" page.
pub(crate) fn spawn_content_thread_blank(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(
            channel,
            network_handle,
            cookie_jar,
            crate::BLANK_TAB_HTML,
            crate::BLANK_TAB_CSS,
            wake,
        );
    })
}

fn content_thread_main(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    html: &str,
    css: &str,
    wake: crate::WakeHandle,
) {
    if let Err(e) = elidex_sandbox::apply_sandbox(&elidex_plugin::PlatformSandbox::Unsandboxed) {
        eprintln!("Sandbox enforcement failed (fatal): {e}");
        return;
    }

    let nh = std::rc::Rc::new(network_handle);
    let pipeline = crate::build_pipeline_interactive_with_network(html, css, nh, cookie_jar);
    let mut state = ContentState::new(channel, NavigationController::new(), pipeline, wake);
    scroll::update_viewport_scroll_dimensions(&mut state);
    // Scan for <iframe> elements present in the initial parsed DOM.
    // Mutation-based detection only catches dynamically added iframes;
    // statically parsed iframes need an explicit initial scan.
    iframe::scan_initial_iframes(&mut state);
    // Re-render after initial scan so statically parsed iframes are composited
    // into the parent display list before the first send.
    state.re_render();
    state.send_display_list();
    event_loop::run_event_loop(&mut state);
}

fn content_thread_main_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    url: &url::Url,
    wake: crate::WakeHandle,
) {
    if let Err(e) = elidex_sandbox::apply_sandbox(&elidex_plugin::PlatformSandbox::Unsandboxed) {
        eprintln!("Sandbox enforcement failed (fatal): {e}");
        return;
    }

    let nh = std::rc::Rc::new(network_handle);
    let loaded = match elidex_navigation::load_document(url, &nh, None) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Content thread: failed to load {url}: {e}");
            let _ = channel.send(ContentToBrowser::NavigationFailed {
                url: url.clone(),
                error: format!("{e}"),
            });
            return;
        }
    };
    // Extract manifest URL before pipeline builder consumes LoadedDocument.
    let manifest_url = loaded.manifest_url.clone();
    let font_db = std::sync::Arc::new(elidex_text::FontDatabase::new());
    let pipeline = crate::build_pipeline_from_loaded(loaded, nh, font_db, Some(cookie_jar));

    let mut nav_controller = NavigationController::new();
    nav_controller.push(url.clone());

    let mut state = ContentState::new(channel, nav_controller, pipeline, wake);
    scroll::update_viewport_scroll_dimensions(&mut state);

    // Notify browser thread of manifest discovery.
    if let Some(manifest) = manifest_url {
        let _ = state
            .channel
            .send(ContentToBrowser::ManifestDiscovered { url: manifest });
    }

    // Scan for <iframe> elements present in the initial parsed DOM.
    iframe::scan_initial_iframes(&mut state);
    state.re_render();
    state.notify_navigation(url);
    event_loop::run_event_loop(&mut state);
}

// run_event_loop + handle_message are in event_loop.rs.

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
/// Changes to these attributes affect whether an element is focusable or its
/// position in the sequential focus navigation order (HTML §6.6.3), so a
/// mutation to any of them invalidates the shell's Tab-order `focusable_cache`.
/// The complete set that `elidex_dom_api::focus::is_focusable` reads: `tabindex`
/// (criterion 1 + order), `disabled` (criterion 3), `contenteditable` (editing
/// host, criterion 1), `hidden` (criterion 5 subtree), `href` (the `<a>`/`<area>`
/// link default), and `type` (an `<input>`'s `type=hidden` is never focusable).
/// Missing `href`/`type` here previously left a stale Tab order after a script
/// added/removed a link's `href` or flipped an input's `type` (Codex S2).
const FOCUSABLE_ATTRIBUTES: &[&str] = &[
    "tabindex",
    "disabled",
    "contenteditable",
    "hidden",
    "href",
    "type",
];

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
    state.pipeline.dispatch_event(&mut event);
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
    state.pipeline.dispatch_event(&mut event);
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
