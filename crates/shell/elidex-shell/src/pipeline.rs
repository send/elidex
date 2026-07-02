//! Internal pipeline helpers: script execution, lifecycle event dispatch, and
//! the `build_pipeline_*` builder family (co-located with [`FrameSecurity`] and
//! the `run_scripts_and_finalize` chokepoint they feed).

use std::rc::Rc;
use std::sync::Arc;

use elidex_css::media::Medium;
use elidex_css::Stylesheet;
use elidex_dom_compat::parse_compat_stylesheet_with_registry;
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_html_parser::parse_progressive_str;
use elidex_js_boa::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_render::build_display_list;
use elidex_script_session::{DispatchEvent, ScriptContext, SessionCore};
use elidex_text::FontDatabase;

use elidex_plugin::ViewportOverflow;

use elidex_plugin::{EngineMode, Size, Vector};

use crate::animation::{create_animation_engine, sync_css_animations};
use crate::{
    create_css_property_registry, resolve_with_mode, sync_stylesheets_to_bridge, PipelineResult,
    DEFAULT_VIEWPORT_HEIGHT, DEFAULT_VIEWPORT_WIDTH,
};

/// Security state of a (sub-)frame document, installed on the JS bridge
/// **before the first eval** (WHATWG HTML §7.1.5 sandboxing / §7.1.1 origin).
///
/// Carried by the iframe load paths (`content/iframe/load.rs`, including the
/// OOP thread's initial build and its `Navigate` re-build in
/// `content/iframe/thread.rs`) into the pipeline builders so
/// `run_scripts_and_finalize` installs it at the same pre-eval seam that
/// seeds the cookie jar / viewport / device facts.
/// Invariant (S5-4b, closes `#11-iframe-origin-before-initial-scripts`):
/// **security installs precede the first eval on ALL iframe paths**
/// (in-process AND out-of-process) — a sandboxed iframe's initial scripts must
/// observe the opaque origin (and the sandbox flags, e.g. the `allow-scripts`
/// eval gate), not the URL-derived tuple origin. This is the `set_origin`
/// contract the engine documents (`elidex-js` `HostData::set_origin`: the
/// embedder "installs it before scripts run"). `None` = top-level document
/// (origin derived from the URL, unsandboxed, depth 0).
pub struct FrameSecurity {
    /// The document origin, with sandbox / credentialless opaqueness already
    /// applied (`apply_sandbox_origin`).
    pub origin: elidex_plugin::SecurityOrigin,
    /// Parsed `sandbox` attribute flags (`None` = no `sandbox` attribute).
    pub sandbox_flags: Option<elidex_plugin::IframeSandboxFlags>,
    /// Iframe nesting depth (`MAX_IFRAME_DEPTH` enforcement across `EcsDom`s).
    pub iframe_depth: usize,
    /// Whether this is a `credentialless` iframe. Persisted on the bridge (like
    /// the sandbox flags) so a same-frame navigation can re-derive the opaque
    /// origin a credentialless browsing context keeps across navigations. Note
    /// the credentialless→opaque-origin semantics itself is pre-existing
    /// (`apply_sandbox_origin`, predates S5-4b); `FrameSecurity` only carries
    /// the flag so `Navigate` stays consistent with the initial load.
    pub credentialless: bool,
    /// The document's referrer — the parent document URL for an iframe (WHATWG
    /// HTML §4.8.5). Installed at the same pre-eval chokepoint as origin/flags
    /// so the initial scripts read a populated `document.referrer` instead of
    /// `""`. `None` = no referrer (top-level, or parent has no URL).
    pub referrer: Option<String>,
}

/// Deferred inputs for deriving a (sub-)frame's [`FrameSecurity`] **after** the
/// document loads — used by the URL-loading rebuild (`build_pipeline_from_url`,
/// the OOP iframe `Navigate` path) where the origin must come from the final
/// post-redirect `loaded.url`, not the requested URL (S5-4b F-a/F-c). The
/// srcdoc / inherited-origin paths derive their origin before the build and
/// pass a fully-formed [`FrameSecurity`] instead.
pub struct FrameSecurityInputs {
    /// Parsed `sandbox` attribute flags (`None` = no `sandbox` attribute).
    pub sandbox_flags: Option<elidex_plugin::IframeSandboxFlags>,
    /// Whether this is a `credentialless` iframe (opaques the origin).
    pub credentialless: bool,
    /// Iframe nesting depth.
    pub iframe_depth: usize,
    /// Referrer URL (parent document URL); carried across the rebuild unchanged.
    pub referrer: Option<String>,
}

// `FrameSecurityInputs::into_frame_security` (the post-redirect origin
// derivation) lives in `content/iframe/load.rs`, beside the `apply_sandbox_origin`
// policy it applies — see that module. Keeping the derivation next to its policy
// lets `apply_sandbox_origin` stay private to `content/iframe`; the URL-loading
// builder below (`build_pipeline_from_url`) invokes the resolver by method
// dispatch rather than reaching up into the sandbox-origin policy.

/// Flush pending DOM mutations and drain custom element reactions.
///
/// This helper combines the three steps that must always run together:
/// 1. `session.flush(dom)` — apply buffered mutations
/// 2. `enqueue_ce_reactions_from_mutations()` — scan for CE lifecycle triggers
/// 3. `drain_custom_element_reactions_public()` — invoke CE callbacks
fn flush_with_ce_reactions(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: Entity,
) {
    let records: Vec<_> = session.flush(dom);
    runtime.enqueue_ce_reactions_from_mutations(&records, dom);
    runtime.drain_custom_element_reactions_public(session, dom, document);
}

/// Common script execution and finalization phase shared by pipeline builders.
///
/// Performs:
/// 1. Initial style resolution
/// 2. Script execution (eval each source, drain timers, flush mutations)
/// 3. Lifecycle event dispatch (`DOMContentLoaded`, `load`)
/// 4. Post-script style re-resolution and layout
///
/// The `registry` is passed in from the caller to avoid creating a duplicate
/// registry when the caller already holds one (e.g. for storage in `PipelineResult`).
///
/// `frame_security` (`Some` on iframe builds, in-process and OOP-thread) is
/// installed on the bridge **before** step 2 — see [`FrameSecurity`] for the
/// ordering invariant.
///
/// Returns `(SessionCore, JsRuntime, ViewportOverflow)` for the caller to include in `PipelineResult`.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_scripts_and_finalize(
    dom: &mut EcsDom,
    document: Entity,
    stylesheets: &[Stylesheet],
    script_sources: &[&str],
    network_handle: Option<Rc<elidex_net::broker::NetworkHandle>>,
    cookie_jar: Option<std::sync::Arc<elidex_net::CookieJar>>,
    font_db: &Arc<elidex_text::FontDatabase>,
    current_url: Option<&url::Url>,
    registry: &elidex_plugin::CssPropertyRegistry,
    viewport: Size,
    device_facts: crate::ipc::DeviceFacts,
    engine_mode: EngineMode,
    frame_security: Option<FrameSecurity>,
) -> (SessionCore, JsRuntime, ViewportOverflow) {
    let stylesheet_refs: Vec<&Stylesheet> = stylesheets.iter().collect();

    // Initial style resolution (with compat layer) at the real content-area
    // viewport — so `@media (width)` evaluates correctly at the first cascade.
    resolve_with_mode(
        dom,
        &stylesheet_refs,
        registry,
        viewport,
        Medium::Screen,
        engine_mode,
    );

    // Script execution phase.
    let mut session = SessionCore::new();
    let mut runtime = JsRuntime::with_network(network_handle);
    // Set cookie jar BEFORE script execution so document.cookie works during page load.
    if let Some(jar) = cookie_jar {
        runtime.bridge().set_cookie_jar(jar);
    }

    if let Some(url) = current_url {
        runtime.set_current_url(Some(url.clone()));
        // Top-level document (`frame_security: None`): unsandboxed, origin
        // derived from the URL.
        if frame_security.is_none() {
            runtime
                .bridge()
                .set_origin(elidex_plugin::SecurityOrigin::from_url(url));
        }
    }

    // Security-install chokepoint (S5-4b): sandbox flags + origin + iframe
    // depth land BEFORE the first eval below, so a frame's *initial* scripts
    // already observe them — the `allow-scripts` eval gate applies to the
    // initial scripts, and a sandboxed (no `allow-same-origin`) iframe's
    // scripts see the opaque origin, never the URL-derived tuple origin
    // (WHATWG HTML §7.1.5 sandboxed scripts / sandboxed origin flags; the
    // engine's `set_origin` contract, `elidex-js` `HostData::set_origin` —
    // installed "before scripts run"). The out-of-process iframe path routes
    // through this SAME seam: its thread-side builds (`iframe/load.rs`
    // `make_out_of_process_entry`, `iframe/thread.rs` `handle_navigate`) pass
    // `Some` too — no post-build install sequence anywhere. Closes
    // `#11-iframe-origin-before-initial-scripts`.
    if let Some(security) = frame_security {
        runtime.bridge().set_sandbox_flags(security.sandbox_flags);
        runtime.bridge().set_origin(security.origin);
        runtime.bridge().set_iframe_depth(security.iframe_depth);
        runtime.bridge().set_credentialless(security.credentialless);
        // Referrer rides the same pre-eval install so the initial scripts read a
        // populated `document.referrer` (the parent document URL, §4.8.5), not
        // the empty default — previously a post-build `set_referrer` landed it
        // only after the initial scripts had already run (and never on the OOP
        // path).
        runtime.bridge().set_referrer(security.referrer);
    }

    // Seed the JS bridge viewport + device facts BEFORE running scripts so initial
    // scripts read the real `window.innerWidth`/`matchMedia`/`devicePixelRatio` (the
    // bridge defaults to 800×600 / 1× / Light otherwise). This is the bridge half of
    // the single construction-input injection (`run_scripts_and_finalize` feeds
    // cascade + bridge + layout from one `viewport`+`device_facts`); it mirrors the
    // per-message resize / `SetDeviceFacts` paths (`event_loop.rs`). Device facts ride
    // the same construction seam as the size (C3) so a tab on a HiDPI / dark display
    // is born with the right `devicePixelRatio` + `prefers-color-scheme`, not 1×/Light
    // raced-in after the first script.
    runtime
        .bridge()
        .set_viewport(viewport.width, viewport.height);
    runtime.bridge().set_device_pixel_ratio(device_facts.dppx);
    runtime.bridge().set_color_scheme(device_facts.color_scheme);

    for source in script_sources {
        let mut ctx = ScriptContext::new(&mut session, dom, document);
        elidex_script_session::ScriptEngine::eval(&mut runtime, source, &mut ctx);
    }
    {
        let mut ctx = ScriptContext::new(&mut session, dom, document);
        elidex_script_session::ScriptEngine::drain_timers(&mut runtime, &mut ctx);
    }
    flush_with_ce_reactions(&mut runtime, &mut session, dom, document);

    // Dispatch lifecycle events.
    dispatch_lifecycle_events(&mut runtime, &mut session, dom, document);
    flush_with_ce_reactions(&mut runtime, &mut session, dom, document);

    // Re-resolve styles after DOM mutations from scripts (with compat layer).
    let viewport_overflow = resolve_with_mode(
        dom,
        &stylesheet_refs,
        registry,
        viewport,
        Medium::Screen,
        engine_mode,
    );

    layout_tree(dom, viewport, font_db);

    (session, runtime, viewport_overflow)
}

/// Build a paged media pipeline result.
///
/// Performs style resolution, layout in paged mode, and builds a
/// [`PagedDisplayList`](elidex_render::PagedDisplayList) with one display
/// list per page. This is the entry point for print/PDF output.
///
/// The DOM must already have been parsed and stylesheets collected.
#[must_use]
#[allow(dead_code)] // Exposed for future print/PDF integration.
pub(super) fn build_paged_pipeline(
    dom: &mut EcsDom,
    stylesheets: &[Stylesheet],
    font_db: &elidex_text::FontDatabase,
    page_ctx: &elidex_plugin::PagedMediaContext,
    registry: &elidex_plugin::CssPropertyRegistry,
    engine_mode: EngineMode,
) -> elidex_render::PagedDisplayList {
    let stylesheet_refs: Vec<&Stylesheet> = stylesheets.iter().collect();
    let viewport = Size::new(page_ctx.page_width, page_ctx.page_height);
    // Paged/print output → `Medium::Print` so `@media print` rules apply and
    // `@media screen` rules do not (mediaqueries-5 §2.3 / CSS Conditional §2).
    resolve_with_mode(
        dom,
        &stylesheet_refs,
        registry,
        viewport,
        Medium::Print,
        engine_mode,
    );

    elidex_render::build_paged_display_lists_interleaved(dom, font_db, page_ctx)
}

/// Dispatch lifecycle events on the document per the HTML spec.
///
/// Sequence:
/// 1. `readystatechange` (Interactive) — document transitions to "interactive"
/// 2. `DOMContentLoaded` — HTML parsing and script execution complete
/// 3. `readystatechange` (Complete) — document transitions to "complete"
/// 4. `load` — all sub-resources have loaded
fn dispatch_lifecycle_events(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: Entity,
) {
    // 1. Transition to "interactive" and fire readystatechange.
    transition_ready_state(
        runtime,
        session,
        dom,
        document,
        elidex_script_session::ReadyState::Interactive,
    );

    // 2. DOMContentLoaded: bubbles, not cancelable.
    let mut dcl_event = DispatchEvent::new("DOMContentLoaded", document);
    dcl_event.cancelable = false;
    elidex_script_session::script_dispatch_event(
        runtime,
        &mut dcl_event,
        &mut ScriptContext::new(session, dom, document),
    );
    flush_with_ce_reactions(runtime, session, dom, document);

    // 3. Transition to "complete" and fire readystatechange.
    transition_ready_state(
        runtime,
        session,
        dom,
        document,
        elidex_script_session::ReadyState::Complete,
    );

    // 4. load: does NOT bubble (spec), not cancelable.
    //
    // Per HTML spec §8.2.6, the `load` event fires on the Window object.
    // In our architecture, there is no separate Window entity — the document
    // entity serves as the event target. This is correct because:
    // - `window.onload` is aliased to document-level in our model
    // - `addEventListener('load', ...)` on document still fires
    // - The event does not bubble, so dispatching on document is equivalent
    let mut load_event = DispatchEvent::new("load", document);
    load_event.bubbles = false;
    load_event.cancelable = false;
    elidex_script_session::script_dispatch_event(
        runtime,
        &mut load_event,
        &mut ScriptContext::new(session, dom, document),
    );
}

/// Transition `document.readyState` and dispatch `readystatechange`.
///
/// Per HTML spec §3.1.5: The `readystatechange` event fires on the Document
/// object each time the readyState attribute's value changes.
fn transition_ready_state(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: Entity,
    new_state: elidex_script_session::ReadyState,
) {
    session.document_ready_state = new_state;
    let mut event = DispatchEvent::new("readystatechange", document);
    event.bubbles = false;
    event.cancelable = false;
    elidex_script_session::script_dispatch_event(
        runtime,
        &mut event,
        &mut ScriptContext::new(session, dom, document),
    );
    flush_with_ce_reactions(runtime, session, dom, document);
}

/// Dispatch `beforeunload` and `unload` events before navigation or shutdown.
///
/// Per HTML spec §7.1.8: `beforeunload` is cancelable (can prevent navigation),
/// `unload` is not cancelable. Both fire on the Window (document target).
///
/// Returns `true` if navigation should proceed (beforeunload not cancelled).
pub(crate) fn dispatch_unload_events(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: Entity,
) -> bool {
    // beforeunload: cancelable (returnValue or preventDefault can block navigation).
    let mut beforeunload = DispatchEvent::new("beforeunload", document);
    beforeunload.cancelable = true;
    beforeunload.bubbles = false;
    let prevented = elidex_script_session::script_dispatch_event(
        runtime,
        &mut beforeunload,
        &mut ScriptContext::new(session, dom, document),
    );
    // Always flush mutations from beforeunload handlers, regardless of
    // whether the event was prevented, so the page state remains consistent.
    flush_with_ce_reactions(runtime, session, dom, document);
    if prevented {
        return false; // Navigation blocked by beforeunload handler.
    }

    // unload: not cancelable, not bubble.
    let mut unload = DispatchEvent::new("unload", document);
    unload.bubbles = false;
    unload.cancelable = false;
    elidex_script_session::script_dispatch_event(
        runtime,
        &mut unload,
        &mut ScriptContext::new(session, dom, document),
    );
    flush_with_ce_reactions(runtime, session, dom, document);
    true
}

// ---------------------------------------------------------------------------
// Pipeline builders (moved from lib.rs — touch-time 1000-line split, S5-4b F-f)
// ---------------------------------------------------------------------------

/// Execute the rendering pipeline and return all state for interactive use.
///
/// Like `build_pipeline`, but returns the full `PipelineResult` instead
/// of just the display list. This allows the shell to handle user events,
/// dispatch DOM events, and re-render.
#[must_use]
pub fn build_pipeline_interactive(html: &str, css: &str) -> PipelineResult {
    let parse_result = parse_progressive_str(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let registry = Arc::new(create_css_property_registry());

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let font_db = Arc::new(FontDatabase::new());

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        None, // No NetworkHandle in standalone mode.
        None, // No CookieJar.
        &font_db,
        None,
        &registry,
        // Standalone/test build: no window, so the default viewport is the
        // explicit choice (D6 — not a silent in-pipeline guess).
        Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        // No window → default device facts (1× / Light); C3 facts are a window thing.
        crate::ipc::DeviceFacts::default(),
        EngineMode::BrowserCompat,
        // Top-level document: no frame security (unsandboxed, URL-derived origin).
        None,
    );

    let display_list = build_display_list(&dom, &font_db);

    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: None,
        network_handle: Rc::new(elidex_net::broker::NetworkHandle::disconnected()),
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
        engine_mode: EngineMode::BrowserCompat,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    // Sync stylesheet data to the JS bridge for CSSOM access.
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Like [`build_pipeline_interactive`] but with a `NetworkHandle` for network access.
pub(crate) fn build_pipeline_interactive_with_network(
    html: &str,
    css: &str,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    cookie_jar: Arc<elidex_net::CookieJar>,
    viewport: Size,
    device_facts: crate::ipc::DeviceFacts,
) -> PipelineResult {
    let parse_result = parse_progressive_str(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let registry = Arc::new(create_css_property_registry());
    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let font_db = Arc::new(FontDatabase::new());
    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        Some(cookie_jar),
        &font_db,
        None,
        &registry,
        viewport,
        device_facts,
        EngineMode::BrowserCompat,
        // Top-level document: no frame security (unsandboxed, URL-derived origin).
        None,
    );

    let display_list = build_display_list(&dom, &font_db);
    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: None,
        network_handle,
        registry,
        animation_engine,
        viewport,
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
        engine_mode: EngineMode::BrowserCompat,
    };

    sync_css_animations(&mut result, &[]);
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Build a pipeline from HTML, sharing the parent's resources.
///
/// Like [`build_pipeline_interactive`], but uses the provided `font_db`,
/// `network_handle`, and `registry` instead of creating fresh instances.
// Mirrors `run_scripts_and_finalize`: the construction inputs (resources + viewport +
// device facts) are each distinct values fed straight through to the builder; bundling
// them into a struct would only move the argument list, not reduce it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_pipeline_interactive_shared(
    html: &str,
    url: Option<url::Url>,
    font_db: Arc<FontDatabase>,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    registry: Arc<elidex_plugin::CssPropertyRegistry>,
    cookie_jar: Option<Arc<elidex_net::CookieJar>>,
    viewport: Size,
    device_facts: crate::ipc::DeviceFacts,
    frame_security: Option<FrameSecurity>,
) -> PipelineResult {
    let parse_result = parse_progressive_str(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        "",
        elidex_css::Origin::Author,
        Some(&registry),
    )];

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        cookie_jar,
        &font_db,
        url.as_ref(),
        &registry,
        viewport,
        device_facts,
        EngineMode::BrowserCompat,
        frame_security,
    );

    let display_list = build_display_list(&dom, &font_db);
    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url,
        network_handle,
        registry,
        animation_engine,
        viewport,
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
        engine_mode: EngineMode::BrowserCompat,
    };

    sync_css_animations(&mut result, &[]);
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Build a pipeline from a pre-loaded document (from [`elidex_navigation::load_document`]).
///
/// Merges all stylesheets, executes all scripts in document order,
/// resolves styles, computes layout, and builds the display list.
///
/// `frame_security` is `Some` on iframe builds (in-process AND the OOP iframe
/// thread): it installs the sandbox flags / origin / depth on the bridge
/// **before** the initial scripts run (see [`FrameSecurity`]). Top-level
/// builds pass `None`.
pub fn build_pipeline_from_loaded(
    loaded: elidex_navigation::LoadedDocument,
    network_handle: Rc<elidex_net::broker::NetworkHandle>,
    font_db: Arc<FontDatabase>,
    cookie_jar: Option<Arc<elidex_net::CookieJar>>,
    viewport: Size,
    device_facts: crate::ipc::DeviceFacts,
    frame_security: Option<FrameSecurity>,
) -> PipelineResult {
    let elidex_navigation::LoadedDocument {
        mut dom,
        document,
        stylesheets,
        scripts,
        url,
        response_headers: _, // Used by iframe loading for CSP/X-Frame-Options checks.
        manifest_url: _,     // Handled by content thread → IPC → browser thread.
    } = loaded;

    elidex_form::init_form_controls(&mut dom);

    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let registry = Arc::new(create_css_property_registry());

    let (session, runtime, viewport_overflow) = run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Some(Rc::clone(&network_handle)),
        cookie_jar,
        &font_db,
        Some(&url),
        &registry,
        viewport,
        device_facts,
        EngineMode::BrowserCompat,
        frame_security,
    );

    let display_list = build_display_list(&dom, &font_db);

    let animation_engine = create_animation_engine(&stylesheets);

    let mut result = PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: Some(url),
        network_handle,
        registry,
        animation_engine,
        viewport,
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
        broker_keepalive: None,
        engine_mode: EngineMode::BrowserCompat,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    // Sync stylesheet data to the JS bridge for CSSOM access.
    sync_stylesheets_to_bridge(&result.runtime, &result.stylesheets);

    result
}

/// Build a pipeline from a URL.
///
/// Spawns a temporary Network Process broker to load the document (standalone mode).
/// Content threads should use `build_pipeline_from_loaded` with a proper `NetworkHandle`.
///
/// `frame_security` is `Some` on the OOP iframe thread's `Navigate` re-build
/// (`content/iframe/thread.rs`). It carries [`FrameSecurityInputs`] rather than
/// a fully-formed `FrameSecurity` because the frame's origin must be derived
/// from the **post-redirect** `loaded.url` (S5-4b F-a/F-c): this builder resolves
/// the fetch (following redirects) and only then computes
/// `FrameSecurity.origin = apply_sandbox_origin(from_url(loaded.url), flags,
/// credentialless)` — the same derivation the initial OOP load performs — before
/// installing it at the pre-eval chokepoint (see [`FrameSecurity`]). Standalone
/// top-level callers pass `None` (URL-derived origin, unsandboxed).
pub fn build_pipeline_from_url(
    url: &url::Url,
    viewport: Size,
    frame_security: Option<FrameSecurityInputs>,
) -> Result<PipelineResult, elidex_navigation::LoadError> {
    // Standalone mode: use a disconnected handle for pipeline (no broker).
    // load_document still routes through NetworkHandle::fetch_blocking which
    // returns "network process disconnected" for disconnected handles, so
    // we create a temporary broker for standalone URL loading.
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let network_handle = Rc::new(np.create_renderer_handle());
    let loaded = elidex_navigation::load_document(url, &network_handle, None)?;
    // Derive the frame security from the post-redirect loaded URL: the origin
    // (and its credentialless/sandbox opaqueness) is a property of the document
    // that actually loaded, not of the requested URL (F-a/F-c).
    let frame_security = frame_security.map(|inputs| inputs.into_frame_security(&loaded.url));
    let font_db = Arc::new(FontDatabase::new());
    let cookie_jar = Arc::clone(np.cookie_jar());
    let mut result = build_pipeline_from_loaded(
        loaded,
        network_handle,
        font_db,
        Some(cookie_jar),
        viewport,
        // Standalone (no window) → default device facts (1× / Light).
        crate::ipc::DeviceFacts::default(),
        frame_security,
    );
    result.broker_keepalive = Some(np); // Keep broker alive for pipeline lifetime.
    Ok(result)
}
