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
use elidex_js::{extract_scripts, JsRuntime};
use elidex_layout::layout_tree;
use elidex_parser::parse_html;
use elidex_render::build_display_list;
use elidex_script_session::SessionCore;
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
/// Includes script execution phase: `<script>` tags are evaluated after
/// initial style resolution, followed by re-resolution and layout.
#[must_use]
pub fn build_pipeline(html: &str, css: &str) -> elidex_render::DisplayList {
    let parse_result = parse_html(html);
    for err in &parse_result.errors {
        eprintln!("HTML parse warning: {err}");
    }
    let mut dom = parse_result.dom;
    let document = parse_result.document;

    let stylesheet = parse_stylesheet(css, Origin::Author);

    // Initial style resolution.
    resolve_styles(
        &mut dom,
        &[&stylesheet],
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
    );

    // Script execution phase.
    let scripts = extract_scripts(&dom, document);
    if !scripts.is_empty() {
        let mut session = SessionCore::new();
        let mut runtime = JsRuntime::new();

        for script in &scripts {
            runtime.eval(&script.source, &mut session, &mut dom, document);
        }

        // Drain any immediately-ready timers (delay=0).
        // Note: only timers with fire_at <= now execute; deferred timers
        // remain queued (no event loop in Phase 2).
        runtime.drain_timers(&mut session, &mut dom, document);

        // Flush any buffered mutations from the session to the DOM.
        session.flush(&mut dom);

        // Re-resolve styles after DOM mutations from scripts.
        resolve_styles(
            &mut dom,
            &[&stylesheet],
            DEFAULT_VIEWPORT_WIDTH,
            DEFAULT_VIEWPORT_HEIGHT,
        );
    }

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

    // --- Script execution integration tests ---

    #[test]
    fn script_does_not_crash_pipeline() {
        // A script that does nothing should not break the pipeline.
        let dl = build_pipeline(
            "<div>Hello</div><script>var x = 1;</script>",
            "div { display: block; }",
        );
        let _ = dl;
    }

    #[test]
    fn script_error_does_not_crash_pipeline() {
        // A script error should be caught and not propagate.
        let dl = build_pipeline(
            "<div>Hello</div><script>throw new Error('test error');</script>",
            "div { display: block; }",
        );
        let _ = dl;
    }

    #[test]
    fn multiple_scripts_execute_in_order() {
        // Multiple scripts should all execute without crashing.
        let dl = build_pipeline(
            "<div>Hello</div>\
             <script>var a = 1;</script>\
             <script>var b = 2;</script>\
             <script>var c = a + b;</script>",
            "div { display: block; }",
        );
        let _ = dl;
    }

    #[test]
    fn script_console_log_does_not_crash() {
        let dl = build_pipeline(
            "<div>Hello</div><script>console.log('hello from script');</script>",
            "",
        );
        let _ = dl;
    }

    #[test]
    fn script_set_timeout_zero_executes() {
        // setTimeout with 0 delay should execute during drain_timers.
        let dl = build_pipeline(
            "<div>Hello</div><script>setTimeout('console.log(\"timer\")', 0);</script>",
            "",
        );
        let _ = dl;
    }

    #[test]
    fn pipeline_without_scripts_still_works() {
        // Ensure the script integration path doesn't break pipelines without scripts.
        let dl = build_pipeline(
            "<h1>No Scripts</h1><p>Just content</p>",
            "h1 { display: block; color: red; }",
        );
        let has_items = !dl.is_empty();
        assert!(has_items, "Expected display items for content");
    }

    // --- DOM JS round-trip integration tests ---

    #[test]
    fn script_get_element_by_id() {
        // getElementById should find an element and allow setting textContent.
        let _dl = build_pipeline(
            "<div id=\"target\">Before</div>\
             <script>document.getElementById('target').textContent = 'After';</script>",
            "",
        );
        // Pipeline completes without panic (H-1 fix validates RefCell safety).
    }

    #[test]
    fn script_create_element_and_append() {
        // createElement + appendChild through the full pipeline.
        let _dl = build_pipeline(
            "<div id=\"root\"></div>\
             <script>\
               var el = document.createElement('span');\
               el.textContent = 'dynamic';\
               document.getElementById('root').appendChild(el);\
             </script>",
            "",
        );
    }

    #[test]
    fn script_query_selector() {
        // querySelector should find elements by CSS selector.
        let _dl = build_pipeline(
            "<div class=\"target\">original</div>\
             <script>\
               var el = document.querySelector('.target');\
               el.setAttribute('data-found', 'true');\
             </script>",
            "",
        );
    }

    #[test]
    fn script_style_set_property() {
        // element.style.setProperty should work through the pipeline.
        let _dl = build_pipeline(
            "<div id=\"box\">styled</div>\
             <script>\
               document.getElementById('box').style.setProperty('background-color', 'red');\
             </script>",
            "",
        );
    }

    #[test]
    fn script_remove_child() {
        // removeChild should work through the DomApiHandler path.
        let _dl = build_pipeline(
            "<div id=\"parent\"><span id=\"child\">remove me</span></div>\
             <script>\
               var parent = document.getElementById('parent');\
               var child = document.getElementById('child');\
               parent.removeChild(child);\
             </script>",
            "",
        );
    }

    #[test]
    fn script_error_isolation() {
        // First script errors, second still executes.
        let _dl = build_pipeline(
            "<div id=\"a\">one</div><div id=\"b\">two</div>\
             <script>document.getElementById('nonexistent').textContent = 'fail';</script>\
             <script>document.getElementById('b').textContent = 'ok';</script>",
            "",
        );
    }
}
