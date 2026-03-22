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
    session.flush(dom);

    // Dispatch lifecycle events.
    dispatch_lifecycle_events(&mut runtime, &mut session, dom, document);
    session.flush(dom);

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

/// Dispatch `DOMContentLoaded` and `load` lifecycle events on the document.
///
/// Per the HTML spec:
/// - `DOMContentLoaded` fires after HTML parsing and script execution complete.
/// - `load` fires after all sub-resources (stylesheets, images) have loaded.
///
/// Both events bubble but are not cancelable.
fn dispatch_lifecycle_events(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: Entity,
) {
    // DOMContentLoaded: bubbles, not cancelable.
    let mut dcl_event = DispatchEvent::new("DOMContentLoaded", document);
    dcl_event.cancelable = false;
    runtime.dispatch_event(&mut dcl_event, session, dom, document);

    // Flush mutations from DOMContentLoaded handlers before dispatching load.
    session.flush(dom);

    // load: does NOT bubble (spec), not cancelable.
    let mut load_event = DispatchEvent::new("load", document);
    load_event.bubbles = false;
    load_event.cancelable = false;
    runtime.dispatch_event(&mut load_event, session, dom, document);
}
