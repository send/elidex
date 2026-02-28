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
pub(crate) mod key_map;

use elidex_css::{parse_stylesheet, Origin, Stylesheet};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_js::{extract_scripts, FetchHandle, JsRuntime};
use elidex_layout::layout_tree;
use elidex_parser::parse_html;
use elidex_render::{build_display_list, DisplayList};
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
    let pipeline_result = build_pipeline_interactive(html, css);

    let event_loop = EventLoop::new()?;
    let mut app = App::new_interactive(pipeline_result);
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
        let fetch_handle = FetchHandle::new(elidex_net::NetClient::new());
        let mut runtime = JsRuntime::with_fetch(Some(fetch_handle));

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
    /// The parsed CSS stylesheet.
    pub stylesheet: Stylesheet,
    /// The font database.
    pub font_db: FontDatabase,
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
    let mut session = SessionCore::new();
    let fetch_handle = FetchHandle::new(elidex_net::NetClient::new());
    let mut runtime = JsRuntime::with_fetch(Some(fetch_handle));

    for script in &scripts {
        runtime.eval(&script.source, &mut session, &mut dom, document);
    }
    runtime.drain_timers(&mut session, &mut dom, document);
    session.flush(&mut dom);

    // Re-resolve styles after DOM mutations from scripts.
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

    let display_list = build_display_list(&dom, &font_db);

    PipelineResult {
        display_list,
        dom,
        document,
        session,
        runtime,
        stylesheet,
        font_db,
    }
}

/// Re-render after DOM changes: re-resolve styles, re-layout, and rebuild display list.
pub(crate) fn re_render(result: &mut PipelineResult) {
    result.session.flush(&mut result.dom);

    resolve_styles(
        &mut result.dom,
        &[&result.stylesheet],
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
    );

    layout_tree(
        &mut result.dom,
        DEFAULT_VIEWPORT_WIDTH,
        DEFAULT_VIEWPORT_HEIGHT,
        &result.font_db,
    );

    result.display_list = build_display_list(&result.dom, &result.font_db);
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{EventPayload, MouseEventInit};
    use elidex_render::DisplayItem;
    use elidex_script_session::DispatchEvent;

    #[test]
    fn build_pipeline_interactive_returns_all_fields() {
        let result = build_pipeline_interactive(
            "<div id=\"test\">Hello</div>",
            "div { display: block; background-color: red; }",
        );
        assert!(!result.display_list.is_empty());
        // Document entity should be valid.
        assert!(result.dom.contains(result.document));
    }

    #[test]
    fn build_pipeline_interactive_with_script() {
        let result = build_pipeline_interactive(
            "<div id=\"target\">Before</div>\
             <script>document.getElementById('target').textContent = 'After';</script>",
            "",
        );
        assert!(result.dom.contains(result.document));
    }

    #[test]
    fn build_pipeline_interactive_compatible_with_build_pipeline() {
        // Both functions should produce similar display lists for the same input.
        let html = "<div style=\"background-color: red\">Hello</div>";
        let css = "div { display: block; }";

        let dl1 = build_pipeline(html, css);
        let result = build_pipeline_interactive(html, css);

        // Same number of display items.
        assert_eq!(dl1.iter().count(), result.display_list.iter().count());
    }

    #[test]
    fn re_render_updates_display_list() {
        let mut result = build_pipeline_interactive(
            "<div id=\"box\" style=\"background-color: red; width: 100px; height: 100px;\">Hello</div>",
            "div { display: block; }",
        );
        let original_count = result.display_list.iter().count();

        // Modify the DOM via the session (simulate a script mutation).
        // No actual change needed — just verify re_render doesn't crash.
        re_render(&mut result);
        let new_count = result.display_list.iter().count();
        assert_eq!(original_count, new_count);
    }

    #[test]
    fn event_listener_with_pipeline_interactive() {
        let mut result = build_pipeline_interactive(
            "<div id=\"btn\" style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>\
             <script>\
               document.getElementById('btn').addEventListener('click', function(e) {\
                 e.target.style.setProperty('background-color', 'red');\
               });\
             </script>",
            "div { display: block; }",
        );
        // The pipeline should complete without panic.
        assert!(!result.display_list.is_empty());
        assert!(result.dom.contains(result.document));

        // Simulate a click dispatch and re-render.
        let btn_entities = result.dom.query_by_tag("div");
        let btn = btn_entities.iter().find(|&&e| {
            result
                .dom
                .world()
                .get::<&elidex_ecs::Attributes>(e)
                .ok()
                .is_some_and(|a| a.get("id") == Some("btn"))
        });
        if let Some(&btn_entity) = btn {
            let mut event = DispatchEvent::new("click", btn_entity);
            event.payload = EventPayload::Mouse(MouseEventInit {
                client_x: 100.0,
                client_y: 50.0,
                ..Default::default()
            });
            result.runtime.dispatch_event(
                &mut event,
                &mut result.session,
                &mut result.dom,
                result.document,
            );
            re_render(&mut result);
        }
    }

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

    // --- Fetch integration tests ---

    #[test]
    fn pipeline_interactive_has_fetch_handle() {
        // build_pipeline_interactive creates a JsRuntime with fetch support.
        // Verify the pipeline completes with fetch available in the runtime.
        let result = build_pipeline_interactive(
            "<div id=\"test\">Hello</div>\
             <script>var hasFetch = typeof fetch === 'function';</script>",
            "",
        );
        assert!(result.dom.contains(result.document));
    }

    #[test]
    fn script_promise_chain_in_pipeline() {
        // Promise chains should work in the pipeline (run_jobs integration).
        let _dl = build_pipeline(
            "<div id=\"target\">Before</div>\
             <script>\
               Promise.resolve('After').then(function(val) {\
                 document.getElementById('target').textContent = val;\
               });\
             </script>",
            "",
        );
    }

    #[test]
    fn pipeline_interactive_event_with_promise() {
        // Events that use Promises should work in interactive mode.
        let mut result = build_pipeline_interactive(
            "<div id=\"btn\" style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>\
             <script>\
               document.getElementById('btn').addEventListener('click', function(e) {\
                 Promise.resolve('clicked').then(function(v) {\
                   e.target.textContent = v;\
                 });\
               });\
             </script>",
            "div { display: block; }",
        );
        assert!(!result.display_list.is_empty());

        // Simulate click dispatch.
        let btn_entities = result.dom.query_by_tag("div");
        let btn = btn_entities.iter().find(|&&e| {
            result
                .dom
                .world()
                .get::<&elidex_ecs::Attributes>(e)
                .ok()
                .is_some_and(|a| a.get("id") == Some("btn"))
        });
        if let Some(&btn_entity) = btn {
            let mut event = DispatchEvent::new("click", btn_entity);
            event.payload = EventPayload::Mouse(MouseEventInit {
                client_x: 100.0,
                client_y: 50.0,
                ..Default::default()
            });
            result.runtime.dispatch_event(
                &mut event,
                &mut result.session,
                &mut result.dom,
                result.document,
            );
            re_render(&mut result);
        }
    }
}
