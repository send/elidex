//! Window management and event loop shell for elidex.
//!
//! Provides the top-level integration that ties together parsing, styling,
//! layout, and rendering into a windowed application.
//!
//! # Usage
//!
//! ```ignore
//! elidex_shell::run("<h1>Hello</h1>", "h1 { color: red; }").unwrap();
//! ```

pub(crate) mod animation;
mod app;
pub(crate) mod chrome;
mod content;
mod gpu;
pub mod ipc;
pub(crate) mod key_map;
mod pipeline;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use elidex_css::Stylesheet;
use elidex_css_anim::engine::AnimationEngine;
use elidex_dom_compat::{
    get_presentational_hints, legacy_ua_stylesheet, parse_compat_stylesheet_with_registry,
};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_html_parser::parse_html;
use elidex_js_boa::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_net::FetchHandle;
use elidex_plugin::{Size, Vector, ViewportOverflow};
use elidex_render::{build_display_list, build_display_list_with_scroll, DisplayList};
use elidex_script_session::SessionCore;
use elidex_style::resolve_styles_with_compat;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use animation::{
    apply_active_animations, collect_computed_without_anim, collect_old_anim_styles,
    create_animation_engine, detect_and_start_transitions, sync_css_animations,
};

use app::App;

/// Build the CSS property registry with all standard property handlers.
///
/// Delegates to [`elidex_style::create_css_property_registry`].
#[must_use]
pub fn create_css_property_registry() -> elidex_plugin::CssPropertyRegistry {
    elidex_style::create_css_property_registry()
}

/// Default viewport width for the initial layout pass.
const DEFAULT_VIEWPORT_WIDTH: f32 = 1024.0;
/// Default viewport height for the initial layout pass.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 768.0;

/// HTML content for a blank new-tab page.
const BLANK_TAB_HTML: &str = "<html><body><h1>New Tab</h1></body></html>";
/// CSS for the blank new-tab page.
const BLANK_TAB_CSS: &str = "body { background-color: #ffffff; color: #333333; font-family: sans-serif; } h1 { text-align: center; margin-top: 200px; }";

/// Resolve styles with the compat layer (legacy UA + presentational hints).
///
/// Passes the CSS property registry to enable handler-based dispatch for
/// `is_inherited()`, `initial_value()`, and `get_computed()` queries.
fn resolve_with_compat(
    dom: &mut EcsDom,
    author_stylesheets: &[&Stylesheet],
    registry: &elidex_plugin::CssPropertyRegistry,
    viewport: Size,
) -> ViewportOverflow {
    let legacy_ua = legacy_ua_stylesheet();
    resolve_styles_with_compat(
        dom,
        author_stylesheets,
        &[legacy_ua],
        &get_presentational_hints,
        viewport,
        Some(registry),
    )
}

/// Run the full browser pipeline and display the result in a window.
///
/// Parses HTML, applies CSS, computes layout, builds a display list,
/// and opens a window rendering the result via Vello + wgpu.
///
/// Content processing (DOM, JS, style, layout) runs on a dedicated thread,
/// communicating with the browser thread via message passing.
///
/// This function blocks until the window is closed.
pub fn run(html: &str, css: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new_threaded(html.to_string(), css.to_string());
    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Execute the rendering pipeline without opening a window.
///
/// Useful for testing the parse → style → layout → display list chain.
/// Includes script execution phase: `<script>` tags are evaluated after
/// initial style resolution, followed by re-resolution and layout.
///
/// Delegates to [`build_pipeline_interactive`] and returns only the display list.
#[must_use]
pub fn build_pipeline(html: &str, css: &str) -> elidex_render::DisplayList {
    build_pipeline_interactive(html, css).display_list
}

/// Result of the interactive rendering pipeline.
///
/// Contains all state needed to handle user events and re-render.
pub struct PipelineResult {
    /// The initial display list.
    pub display_list: DisplayList,
    /// The ECS DOM.
    pub dom: EcsDom,
    /// The document root entity.
    pub document: Entity,
    /// The script session state.
    pub session: SessionCore,
    /// The JavaScript runtime.
    pub runtime: JsRuntime,
    /// All parsed CSS stylesheets.
    pub stylesheets: Vec<Stylesheet>,
    /// The font database (shared across navigations to avoid re-scanning).
    pub font_db: Rc<FontDatabase>,
    /// The URL of the current page, if loaded from a URL.
    pub url: Option<url::Url>,
    /// Shared fetch handle (for cookie sharing across navigation).
    pub fetch_handle: Rc<FetchHandle>,
    /// CSS property registry (cached to avoid re-creation on each re-render).
    pub registry: elidex_plugin::CssPropertyRegistry,
    /// CSS animation/transition engine.
    pub animation_engine: AnimationEngine,
    /// Current viewport dimensions for layout.
    pub viewport: Size,
    /// Whether the text input caret should be visible in the display list.
    ///
    /// Set by the content thread's caret blink timer. Defaults to `true`.
    pub caret_visible: bool,
    /// Cached form ancestor lookups (invalidated on DOM mutation).
    pub ancestor_cache: elidex_form::AncestorCache,
    /// Viewport-level overflow propagated from root/body element.
    pub viewport_overflow: ViewportOverflow,
    /// Viewport scroll offset synced from content thread before re-render.
    pub scroll_offset: Vector,
}

impl PipelineResult {
    /// Remove animation/transition state for entities that no longer exist in the DOM.
    pub(crate) fn prune_dead_animation_entities(&mut self) {
        self.animation_engine.prune_dead_entities(&|entity_id| {
            Entity::from_bits(entity_id).is_some_and(|entity| self.dom.world().contains(entity))
        });
    }
}

/// Execute the rendering pipeline and return all state for interactive use.
///
/// Like `build_pipeline`, but returns the full `PipelineResult` instead
/// of just the display list. This allows the shell to handle user events,
/// dispatch DOM events, and re-render.
#[must_use]
pub fn build_pipeline_interactive(html: &str, css: &str) -> PipelineResult {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    elidex_form::init_form_controls(&mut dom);

    let registry = create_css_property_registry();

    let stylesheets = vec![parse_compat_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    )];
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let font_db = Rc::new(FontDatabase::new());

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
        &font_db,
        None,
        &registry,
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
        fetch_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    result
}

/// Re-render after DOM changes: re-resolve styles, re-layout, and rebuild display list.
///
/// Includes transition detection: saves old computed values for entities with
/// `AnimStyle`, re-resolves styles, compares old vs new values to detect
/// transitions, feeds them to the `AnimationEngine`, and applies animated
/// values to `ComputedStyle` before layout.
///
/// Returns the mutation records from the flush, for observer dispatch.
pub(crate) fn re_render(result: &mut PipelineResult) -> Vec<elidex_script_session::MutationRecord> {
    // Flush applies buffered mutations to the DOM.
    let raw_records = result.session.flush(&mut result.dom);
    let mutation_records: Vec<elidex_script_session::MutationRecord> =
        raw_records.into_iter().flatten().collect();

    // Invalidate ancestor cache when DOM mutations occurred.
    if !mutation_records.is_empty() {
        result.ancestor_cache.invalidate_all();
    }

    // Prune animations/transitions for destroyed entities unconditionally.
    // JS event handlers may destroy entities without generating style mutations,
    // so conditional pruning could leak animation state.
    result.prune_dead_animation_entities();

    // Phase 1: Save old computed values for entities with AnimStyle (transition detection).
    // Also snapshot entities without AnimStyle but with ComputedStyle, so that
    // entities gaining AnimStyle in this render cycle have a baseline for transitions.
    let old_styles = collect_old_anim_styles(&result.dom);
    let old_computed_no_anim = collect_computed_without_anim(&result.dom);

    // Phase 2: Re-resolve styles.
    let stylesheet_refs: Vec<&Stylesheet> = result.stylesheets.iter().collect();
    result.viewport_overflow = resolve_with_compat(
        &mut result.dom,
        &stylesheet_refs,
        &result.registry,
        result.viewport,
    );

    // Phase 3: Detect transitions by comparing old vs new computed values.
    // Includes entities that newly gained AnimStyle (transition-* properties).
    detect_and_start_transitions(result, &old_styles, &old_computed_no_anim);

    // Phase 3b: Start/cancel CSS animations based on animation-name changes.
    // CSS Animations L1 §4.2: when animation-name changes, old names are cancelled
    // and new names are started.
    sync_css_animations(result, &old_styles);

    // Phase 4: Apply animated values from active transitions/animations to ComputedStyle.
    apply_active_animations(result);

    layout_tree(&mut result.dom, result.viewport, &result.font_db);

    result.display_list = build_display_list_with_scroll(
        &result.dom,
        &result.font_db,
        result.caret_visible,
        result.scroll_offset,
    );

    mutation_records
}

/// Build a pipeline from a pre-loaded document (from [`elidex_navigation::load_document`]).
///
/// Merges all stylesheets, executes all scripts in document order,
/// resolves styles, computes layout, and builds the display list.
pub fn build_pipeline_from_loaded(
    loaded: elidex_navigation::LoadedDocument,
    fetch_handle: Rc<FetchHandle>,
    font_db: Rc<FontDatabase>,
) -> PipelineResult {
    let elidex_navigation::LoadedDocument {
        mut dom,
        document,
        stylesheets,
        scripts,
        url,
    } = loaded;

    elidex_form::init_form_controls(&mut dom);

    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let registry = create_css_property_registry();

    let (session, runtime, viewport_overflow) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
        &font_db,
        Some(url.clone()),
        &registry,
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
        fetch_handle,
        registry,
        animation_engine,
        viewport: Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        caret_visible: true,
        ancestor_cache: elidex_form::AncestorCache::new(),
        viewport_overflow,
        scroll_offset: Vector::<f32>::ZERO,
    };

    // Start CSS animations declared in initial styles.
    sync_css_animations(&mut result, &[]);

    result
}

/// Build a pipeline from a URL.
///
/// Creates a `FetchHandle`, loads the document, and runs the full pipeline.
pub fn build_pipeline_from_url(
    url: &url::Url,
) -> Result<PipelineResult, elidex_navigation::LoadError> {
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let loaded = elidex_navigation::load_document(url, &fetch_handle, None)?;
    let font_db = Rc::new(FontDatabase::new());
    Ok(build_pipeline_from_loaded(loaded, fetch_handle, font_db))
}

/// Run the browser from a URL string, opening a window.
///
/// Parses the URL, fetches the page and its resources, executes scripts,
/// renders the result, and runs the event loop.
///
/// Content processing runs on a dedicated thread.
///
/// This function blocks until the window is closed.
pub fn run_url(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url::Url::parse(url_str)
        .map_err(|e| elidex_navigation::LoadError::InvalidUrl(format!("{url_str}: {e}")))?;

    let event_loop = EventLoop::new()?;
    let mut app = App::new_threaded_url(url);
    event_loop.run_app(&mut app)?;

    Ok(())
}
