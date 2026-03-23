//! Internal pipeline helpers: script execution and lifecycle event dispatch.

use std::rc::Rc;

use elidex_css::Stylesheet;
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_js_boa::JsRuntime;
use elidex_layout::layout_tree;
use elidex_net::FetchHandle;
use elidex_script_session::{DispatchEvent, SessionCore};

use elidex_plugin::ViewportOverflow;

use elidex_plugin::Size;

use crate::{resolve_with_compat, DEFAULT_VIEWPORT_HEIGHT, DEFAULT_VIEWPORT_WIDTH};

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
    let records: Vec<_> = session.flush(dom).into_iter().flatten().collect();
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
/// Returns `(SessionCore, JsRuntime, ViewportOverflow)` for the caller to include in `PipelineResult`.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_scripts_and_finalize(
    dom: &mut EcsDom,
    document: Entity,
    stylesheets: &[Stylesheet],
    script_sources: &[&str],
    fetch_handle: Rc<FetchHandle>,
    font_db: &Rc<elidex_text::FontDatabase>,
    current_url: Option<url::Url>,
    registry: &elidex_plugin::CssPropertyRegistry,
) -> (SessionCore, JsRuntime, ViewportOverflow) {
    let stylesheet_refs: Vec<&Stylesheet> = stylesheets.iter().collect();

    // Initial style resolution (with compat layer).
    let default_viewport = Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT);
    resolve_with_compat(dom, &stylesheet_refs, registry, default_viewport);

    // Script execution phase.
    let mut session = SessionCore::new();
    let mut runtime = JsRuntime::with_fetch(Some(fetch_handle));

    if let Some(url) = current_url {
        runtime.set_current_url(Some(url));
    }

    for source in script_sources {
        runtime.eval(source, &mut session, dom, document);
    }
    runtime.drain_timers(&mut session, dom, document);
    flush_with_ce_reactions(&mut runtime, &mut session, dom, document);

    // Dispatch lifecycle events.
    dispatch_lifecycle_events(&mut runtime, &mut session, dom, document);
    flush_with_ce_reactions(&mut runtime, &mut session, dom, document);

    // Re-resolve styles after DOM mutations from scripts (with compat layer).
    let viewport_overflow = resolve_with_compat(dom, &stylesheet_refs, registry, default_viewport);

    layout_tree(dom, default_viewport, font_db);

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
) -> elidex_render::PagedDisplayList {
    let stylesheet_refs: Vec<&Stylesheet> = stylesheets.iter().collect();
    let viewport = Size::new(page_ctx.page_width, page_ctx.page_height);
    resolve_with_compat(dom, &stylesheet_refs, registry, viewport);

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
    runtime.dispatch_event(&mut dcl_event, session, dom, document);
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
    runtime.dispatch_event(&mut load_event, session, dom, document);
}

/// Transition `document.readyState` and dispatch `readystatechange`.
///
/// Per HTML spec §8.2.6: The `readystatechange` event fires on the Document
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
    runtime.dispatch_event(&mut event, session, dom, document);
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
    let prevented = runtime.dispatch_event(&mut beforeunload, session, dom, document);
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
    runtime.dispatch_event(&mut unload, session, dom, document);
    flush_with_ce_reactions(runtime, session, dom, document);
    true
}
