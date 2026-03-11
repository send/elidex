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
use elidex_dom_compat::{get_presentational_hints, legacy_ua_stylesheet, parse_compat_stylesheet};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_js_boa::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_net::FetchHandle;
use elidex_parser::parse_html;
use elidex_render::{build_display_list, DisplayList};
use elidex_script_session::SessionCore;
use elidex_style::resolve_styles_with_compat;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use app::App;

/// Default viewport width for the initial layout pass.
const DEFAULT_VIEWPORT_WIDTH: f32 = 1024.0;
/// Default viewport height for the initial layout pass.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 768.0;

/// HTML content for a blank new-tab page.
const BLANK_TAB_HTML: &str = "<html><body><h1>New Tab</h1></body></html>";
/// CSS for the blank new-tab page.
const BLANK_TAB_CSS: &str = "body { background-color: #ffffff; color: #333333; font-family: sans-serif; } h1 { text-align: center; margin-top: 200px; }";

/// Resolve styles with the compat layer (legacy UA + presentational hints).
fn resolve_with_compat(dom: &mut EcsDom, author_stylesheets: &[&Stylesheet]) {
    let legacy_ua = legacy_ua_stylesheet();
    resolve_styles_with_compat(
        dom,
        author_stylesheets,
        &[legacy_ua],
        &get_presentational_hints,
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
    );
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

    let stylesheets = vec![parse_compat_stylesheet(css, elidex_css::Origin::Author)];
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let font_db = Rc::new(FontDatabase::new());

    let scripts = extract_scripts(&dom, document);
    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
        &font_db,
        None,
    );

    let display_list = build_display_list(&dom, &font_db);

    PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: None,
        fetch_handle,
    }
}

/// Re-render after DOM changes: re-resolve styles, re-layout, and rebuild display list.
pub(crate) fn re_render(result: &mut PipelineResult) {
    result.session.flush(&mut result.dom);

    let stylesheet_refs: Vec<&Stylesheet> = result.stylesheets.iter().collect();
    resolve_with_compat(&mut result.dom, &stylesheet_refs);

    layout_tree(
        &mut result.dom,
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
        &result.font_db,
    );

    result.display_list = build_display_list(&result.dom, &result.font_db);
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

    let script_sources: Vec<&str> = scripts.iter().map(|s| s.source.as_str()).collect();

    let (session, runtime) = pipeline::run_scripts_and_finalize(
        &mut dom,
        document,
        &stylesheets,
        &script_sources,
        Rc::clone(&fetch_handle),
        &font_db,
        Some(url.clone()),
    );

    let display_list = build_display_list(&dom, &font_db);

    PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheets,
        font_db,
        url: Some(url),
        fetch_handle,
    }
}

/// Build a pipeline from a URL.
///
/// Creates a `FetchHandle`, loads the document, and runs the full pipeline.
pub fn build_pipeline_from_url(
    url: &url::Url,
) -> Result<PipelineResult, elidex_navigation::LoadError> {
    let fetch_handle = Rc::new(FetchHandle::new(elidex_net::NetClient::new()));
    let loaded = elidex_navigation::load_document(url, &fetch_handle)?;
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
