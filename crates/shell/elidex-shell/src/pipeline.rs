//! Internal pipeline helpers: script execution and lifecycle event dispatch.

use std::rc::Rc;
use std::sync::Arc;

use elidex_css::media::Medium;
use elidex_css::Stylesheet;
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_js_boa::JsRuntime;
use elidex_layout::layout_tree;
use elidex_script_session::{DispatchEvent, ScriptContext, SessionCore};

use elidex_plugin::ViewportOverflow;

use elidex_plugin::{EngineMode, Size};

use crate::resolve_with_mode;

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
}

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
    frame_security: Option<&FrameSecurity>,
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
    match frame_security {
        Some(security) => {
            runtime.bridge().set_sandbox_flags(security.sandbox_flags);
            runtime.bridge().set_origin(security.origin.clone());
            runtime.bridge().set_iframe_depth(security.iframe_depth);
        }
        // Top-level document: unsandboxed, origin derived from the URL.
        None => {
            if let Some(url) = current_url {
                runtime
                    .bridge()
                    .set_origin(elidex_plugin::SecurityOrigin::from_url(url));
            }
        }
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
