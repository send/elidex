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
pub(crate) mod scroll;

use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use elidex_ecs::Entity;
use elidex_navigation::NavigationController;
use elidex_script_session::HostDriver;

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
    /// This page/browsing-context's Service Worker client id (WHATWG SW §4.2
    /// `Client.id`). The shell is the SOLE minter (the VM has no window-side
    /// client id); minted once per `ContentState` at construction and fed to
    /// `SwFetchRequest.client_id` (→ `FetchEvent.clientId`, SW §4.6.3) and, when
    /// the PostMessage arm lands (2e-b), `ContentToSw::PostMessage.client_id` —
    /// the ONE generator so all SW client-id reads agree (§6.4).
    client_id: String,
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
    /// Latest browser-published content-area viewport (a *pull* source) — read at
    /// every build site, including the navigation rebuild in `handle_navigate`, so a
    /// resize landing during a blocking `load_document` is observed by construction
    /// rather than reconciled after the fact. Shared `Arc` (one per window; all tabs
    /// share the content area). See [`crate::ipc::ViewportCell`].
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
    /// High-water mark of the [`crate::ipc::ViewportCell`] `seq` this thread has
    /// consumed — set to the seq the current document **built** at, then advanced by
    /// each applied `SetViewport`. A delivery with `seq ≤` this is dropped as stale
    /// (already consumed by the build or a prior apply), which is what prevents the
    /// cell-read build from flashing backward to a queued intermediate resize.
    applied_viewport_seq: u64,
    /// High-water mark of the [`crate::ipc::ViewportCell`] `facts_seq` this thread has
    /// consumed — the device-facts analog of `applied_viewport_seq`, tracked
    /// separately because facts are orthogonal to the size `seq` (D3). Set to the
    /// `facts_seq` the current document **built** at, then advanced by each applied
    /// `SetDeviceFacts`. A delivery with `facts_seq ≤` this is dropped as stale (a
    /// dppx/color-scheme already folded into the facts the build read from the cell),
    /// so a navigation racing rapid DPI/theme changes does not replay an obsolete
    /// color-scheme/dppx backward.
    applied_facts_seq: u64,
    /// Wake the browser event loop to schedule a repaint after a
    /// display/chrome-affecting send. Under `ControlFlow::Wait` a content-initiated
    /// frame (timer / rAF / animation / async DOM / `SetViewport` round-trip)
    /// would otherwise paint only on the next OS event; calling `wake()` schedules
    /// a redraw so the frame reaches a rendering opportunity (WHATWG HTML
    /// §8.1.7.3). Windowing-agnostic
    /// (`crate::WakeHandle = Box<dyn Fn() + Send>`) so the content thread stays
    /// winit-free.
    wake: crate::WakeHandle,
    /// The document-root `inclusive_descendants_version` as of the last completed
    /// `re_render` — the §4.3.8 version-delta baseline replacing the boa
    /// flush-record stream + `needs_render` bool.
    last_render_dom_version: u64,
    /// The window's current device facts (dppx / color-scheme / reduced-motion) —
    /// the shell-owned SoT the VM's getterless media surface replaced (B20); pushed
    /// to the VM via `set_media_environment` and inherited by child iframes.
    device_facts: crate::ipc::DeviceFacts,
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

    /// Push or replace a URL in the navigation controller for a
    /// `pushState`/`replaceState` — a **same-document** history mutation, so the
    /// entry inherits the current document's identity (`document_sequence`); a
    /// later traversal between it and its document-siblings restores in place.
    fn push_or_replace(&mut self, url: url::Url, replace: bool) {
        if replace {
            self.nav_controller.replace_same_document(url);
        } else {
            self.nav_controller.push_same_document(url);
        }
    }

    /// Create a new `ContentState` from an initialized pipeline and channel.
    ///
    /// `applied_viewport_seq` must be the [`crate::ipc::ViewportCell`] seq the
    /// `pipeline` **built** at (the seq returned by the build site's `cell.read()`) —
    /// never a `0` default, or a queued `SetViewport` with `seq <` the real build seq
    /// would satisfy the staleness guard and apply a pre-build intermediate (the
    /// backward flash the seq exists to prevent). `applied_facts_seq` is the same
    /// contract for the device-facts generation (`ViewportSnapshot::facts_seq`).
    fn new(
        channel: LocalChannel<ContentToBrowser, BrowserToContent>,
        nav_controller: NavigationController,
        pipeline: PipelineResult,
        wake: crate::WakeHandle,
        viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
        applied_viewport_seq: u64,
        applied_facts_seq: u64,
        device_facts: crate::ipc::DeviceFacts,
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
            client_id: uuid::Uuid::new_v4().to_string(),
            iframes: iframe::IframeRegistry::new(),
            focused_iframe: None,
            wake,
            viewport_cell,
            applied_viewport_seq,
            applied_facts_seq,
            // §4.3.8: 0 → the first `re_render` sees a delta and harmlessly
            // invalidates the fresh (`None`) focusable cache + walks iframes
            // (`scan_initial_iframes` still owns the initial load).
            last_render_dom_version: 0,
            device_facts,
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
        // The `HostDriver` scroll surface is f64 (DOM coordinates); the shell's
        // `viewport_scroll` is f32 (render space) — widen at the seam.
        self.pipeline.runtime.set_scroll_offset(
            self.viewport_scroll.scroll_offset.x.into(),
            self.viewport_scroll.scroll_offset.y.into(),
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
        self.pipeline.sync_dirty_canvases();
        self.pipeline.caret_visible = self.caret_visible;

        // Drain any pending JS scroll (scrollTo/scrollBy) and apply the requested
        // offset so the display list builds toward it. The CLAMP is deferred to
        // AFTER `crate::re_render` recomputes layout (below): a script that
        // mutated layout and scrolled in the same turn — e.g. appended tall
        // content then `scrollTo` its bottom — must clamp against the NEW content
        // size, not the stale pre-layout one (Codex R6 "clamp script scrolls
        // after layout is refreshed").
        let pending_scroll = self.pipeline.runtime.take_pending_scroll();
        if let Some((x, y)) = pending_scroll {
            // f64 (DOM coordinates) → f32 (render-space `Vector`) at the seam.
            self.viewport_scroll.scroll_offset = elidex_plugin::Vector::new(x as f32, y as f32);
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

        crate::re_render(&mut self.pipeline);

        // §4.3.8 version-delta — the ONE "did the DOM tree change this turn?"
        // signal replacing the boa flush-record stream. Snapshot AFTER
        // `crate::re_render` so any external-record flush mutations are folded in.
        // Every DOM-mutation class (childList / attribute / characterData) bumps
        // the document root's `inclusive_descendants_version` via `rev_version`'s
        // ancestor propagation; a detached-subtree mutation does NOT move the root
        // version, which is correct — rendering / in-document iframes / focusables
        // are document-tree facts.
        let current_dom_version = self
            .pipeline
            .dom
            .inclusive_descendants_version(self.pipeline.document);
        let dom_changed = current_dom_version != self.last_render_dom_version;
        self.last_render_dom_version = current_dom_version;

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

        // Invalidate the focusable cache on any document-tree change. The
        // §4.3.8 version-delta replaces the record-fed
        // `should_invalidate_focusable_cache`: under the VM flip the flush record
        // stream starves, so the coarser "tree changed at all" signal is used —
        // a superset of the old childList/focusability-attribute triggers (a
        // stale Tab order is never worse than an occasional harmless rebuild).
        if dom_changed {
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

        // `MutationObserver` + CE delivery for the flushed records is now internal
        // to `crate::re_render` (via `deliver_records_and_drain`); re-delivering
        // here would DOUBLE-FIRE observers, so this stage no longer delivers the
        // records after layout — only the layout-derived observers below.

        // Deliver the queued ResizeObserver + IntersectionObserver callbacks
        // (spec order: resize before intersection) after layout is complete. The
        // VM reads the viewport from bound state, so no `Rect` is passed.
        self.pipeline.deliver_layout_observations();

        // Detect iframe additions/removals/re-navigations via the §4.3.8
        // version-delta: when the document tree changed this turn, a full-document
        // diff-scan reconciles the registry (the record-fed `detect_iframe_mutations`
        // starved under the VM flush). Gated on `dom_changed` so the full walk runs
        // only when something actually moved.
        let iframes_changed = if dom_changed {
            iframe::rescan_iframes_by_diff(self)
        } else {
            false
        };

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
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(
            channel,
            network_handle,
            cookie_jar,
            &html,
            &css,
            viewport_cell,
            wake,
        );
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
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main_url(
            channel,
            network_handle,
            cookie_jar,
            &url,
            viewport_cell,
            wake,
        );
    })
}

/// Spawn a blank new-tab content thread.
///
/// Renders a minimal "New Tab" page.
pub(crate) fn spawn_content_thread_blank(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    network_handle: elidex_net::broker::NetworkHandle,
    cookie_jar: std::sync::Arc<elidex_net::CookieJar>,
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
    wake: crate::WakeHandle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(
            channel,
            network_handle,
            cookie_jar,
            crate::BLANK_TAB_HTML,
            crate::BLANK_TAB_CSS,
            viewport_cell,
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
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
    wake: crate::WakeHandle,
) {
    if let Err(e) = elidex_sandbox::apply_sandbox(&elidex_plugin::PlatformSandbox::Unsandboxed) {
        eprintln!("Sandbox enforcement failed (fatal): {e}");
        return;
    }

    // Read the latest published viewport + device facts by construction (no
    // `load_document` blocks this HTML path, so the read is immediate). `build_seq`
    // becomes the document's high-water mark so the resume-time
    // `broadcast_viewport(seq)` re-delivery — and any queued resize at
    // `seq ≤ build_seq` — is dropped instead of double-painting. The device facts
    // (dppx / color-scheme) seed the bridge before initial scripts (C3), so a tab on
    // a HiDPI / dark display is born with the right `devicePixelRatio` / `matchMedia`.
    let snapshot = viewport_cell.read();
    let (viewport, build_seq, build_facts_seq) = (snapshot.size, snapshot.seq, snapshot.facts_seq);
    let nh = std::rc::Rc::new(network_handle);
    let pipeline = crate::build_pipeline_interactive_with_network(
        html,
        css,
        nh,
        cookie_jar,
        viewport,
        snapshot.facts,
    );
    let mut state = ContentState::new(
        channel,
        NavigationController::new(),
        pipeline,
        wake,
        viewport_cell,
        build_seq,
        build_facts_seq,
        snapshot.facts,
    );
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
    viewport_cell: std::sync::Arc<crate::ipc::ViewportCell>,
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
    // Read the cell **after** the blocking load returns, so a resize that landed
    // during the load is observed by construction (the build's `innerWidth`/`@media`
    // see the real size, never the spawn-time snapshot). `build_seq` is the document's
    // high-water mark: queued `SetViewport`s at `seq ≤ build_seq` (the resizes already
    // folded into this read) are dropped instead of flashing the document backward.
    // Device facts (dppx / color-scheme) ride the same construction read (C3).
    let snapshot = viewport_cell.read();
    let (viewport, build_seq, build_facts_seq) = (snapshot.size, snapshot.seq, snapshot.facts_seq);
    // Extract manifest URL before pipeline builder consumes LoadedDocument.
    let manifest_url = loaded.manifest_url.clone();
    let font_db = std::sync::Arc::new(elidex_text::FontDatabase::new());
    let pipeline = crate::build_pipeline_from_loaded(
        loaded,
        nh,
        font_db,
        Some(cookie_jar),
        viewport,
        snapshot.facts,
        // Top-level document: no frame security (URL-derived origin).
        None,
        // Initial document load (not a traversal) → no `history.state` seed.
        None,
    );

    let mut nav_controller = NavigationController::new();
    nav_controller.push(url.clone());

    let mut state = ContentState::new(
        channel,
        nav_controller,
        pipeline,
        wake,
        viewport_cell,
        build_seq,
        build_facts_seq,
        snapshot.facts,
    );
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

/// WHATWG HTML §9.3.3 "Posting messages" step 8.1: an iframe→parent message is
/// delivered iff the sender's resolved `targetOrigin` is `*` (any origin) or it
/// equals the parent window's origin key. Both keys are identity-preserving
/// `storage_origin_key`s (opaque → per-VM sentinel), so distinct opaque origins
/// never alias — there is NO lossy `"null"` special case (the send side
/// fail-closes on opaque URL targets and uses the sentinel for `/`). Fail-closed:
/// any `target_origin` that is neither `"*"` nor an exact key match is dropped.
// `#[allow(dead_code)]` is transient: the gated dispatch loop that consumes this
// lands in sub-commit 2f4-c, which removes this attribute.
#[allow(dead_code)]
fn parent_message_allowed(parent_key: &str, target_origin: &str) -> bool {
    target_origin == "*" || parent_key == target_origin
}

#[cfg(test)]
#[path = "../content_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "../content_tests.rs"]
mod content_tests;

#[cfg(test)]
#[path = "../content_iframe_security_tests.rs"]
mod iframe_security_tests;

#[cfg(test)]
#[path = "../content_window_open_tests.rs"]
mod window_open_tests;

#[cfg(test)]
#[path = "../content_history_drain_tests.rs"]
mod history_drain_tests;

#[cfg(test)]
#[path = "../content_fragment_nav_tests.rs"]
mod fragment_nav_tests;

#[cfg(test)]
#[path = "../viewport_tests.rs"]
mod viewport_tests;

#[cfg(test)]
mod parent_message_gate_tests {
    use super::parent_message_allowed;

    #[test]
    fn wildcard_target_allows_any_origin() {
        assert!(parent_message_allowed("https://parent.example", "*"));
        assert!(parent_message_allowed("opaque-origin:7", "*"));
    }

    #[test]
    fn equal_keys_allow() {
        assert!(parent_message_allowed(
            "https://parent.example",
            "https://parent.example"
        ));
    }

    #[test]
    fn distinct_keys_drop() {
        assert!(!parent_message_allowed(
            "https://parent.example",
            "https://evil.example"
        ));
    }

    #[test]
    fn distinct_opaque_sentinels_drop() {
        // Two DIFFERENT per-VM opaque sentinels must NOT alias — the whole point
        // of keying on the sentinel rather than the lossy `"null"`.
        assert!(!parent_message_allowed(
            "opaque-origin:1",
            "opaque-origin:2"
        ));
    }
}
