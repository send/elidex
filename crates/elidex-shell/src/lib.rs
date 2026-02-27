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
mod gpu;

use elidex_css::{parse_stylesheet, Origin};
use elidex_layout::layout_tree;
use elidex_parser::parse_html;
use elidex_render::build_display_list;
use elidex_style::resolve_styles;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use app::App;

/// Default viewport width for the initial layout pass.
const DEFAULT_VIEWPORT_WIDTH: f32 = 1024.0;
/// Default viewport height for the initial layout pass.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 768.0;

/// Run the full browser pipeline and display the result in a window.
///
/// Parses HTML, applies CSS, computes layout, builds a display list,
/// and opens a window rendering the result via Vello + wgpu.
///
/// This function blocks until the window is closed.
pub fn run(html: &str, css: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Phase 1 pipeline: parse → style → layout → display list.
    let display_list = build_pipeline(html, css);

    // Create the event loop and application.
    let event_loop = EventLoop::new()?;
    let mut app = App::new(display_list);
    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Execute the rendering pipeline without opening a window.
///
/// Useful for testing the parse → style → layout → display list chain.
#[must_use]
pub fn build_pipeline(html: &str, css: &str) -> elidex_render::DisplayList {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;

    let stylesheet = parse_stylesheet(css, Origin::Author);
    resolve_styles(
        &mut dom,
        &[&stylesheet],
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
    );

    let font_db = FontDatabase::new();
    layout_tree(
        &mut dom,
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
        &font_db,
    );

    build_display_list(&dom, &font_db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_render::DisplayItem;

    #[test]
    fn empty_html_produces_display_list() {
        let dl = build_pipeline("", "");
        // Empty HTML still parses (html5ever creates html/head/body).
        // UA stylesheet gives body a background, but it's transparent by default.
        // So the display list may or may not be empty depending on UA styles.
        let _ = dl;
    }

    #[test]
    fn background_color_in_pipeline() {
        let dl = build_pipeline(
            "<div style=\"background-color: red\">Hello</div>",
            "div { display: block; }",
        );
        let has_rect = dl
            .iter()
            .any(|item| matches!(item, DisplayItem::SolidRect { .. }));
        assert!(
            has_rect,
            "Expected at least one SolidRect for red background"
        );
    }

    #[test]
    fn pipeline_with_stylesheet() {
        let dl = build_pipeline(
            "<div class=\"box\">Test</div>",
            ".box { display: block; background-color: blue; width: 200px; height: 100px; }",
        );
        let rects: Vec<_> = dl
            .iter()
            .filter(|item| matches!(item, DisplayItem::SolidRect { .. }))
            .collect();
        assert!(!rects.is_empty(), "Expected SolidRect for blue box");
    }
}
