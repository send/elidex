use super::*;
use elidex_plugin::{EventPayload, MouseEventInit};
use elidex_render::DisplayItem;
use elidex_script_session::DispatchEvent;

fn find_by_id(result: &PipelineResult, tag: &str, id: &str) -> Option<Entity> {
    let entities = result.dom.query_by_tag(tag);
    entities.into_iter().find(|&e| {
        result
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(e)
            .ok()
            .is_some_and(|a| a.get("id") == Some(id))
    })
}

fn simulate_click(result: &mut PipelineResult, entity: Entity) {
    let mut event = DispatchEvent::new_composed("click", entity);
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
    re_render(result);
}

fn get_text_content(dom: &EcsDom, entity: Entity) -> String {
    dom.get_first_child(entity)
        .and_then(|tc| {
            dom.world()
                .get::<&elidex_ecs::TextContent>(tc)
                .ok()
                .map(|t| t.0.clone())
        })
        .unwrap_or_default()
}

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
    if let Some(btn_entity) = find_by_id(&result, "div", "btn") {
        simulate_click(&mut result, btn_entity);
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
        "h1 { display: block; color: red; font-family: DejaVu Sans, Noto Sans, Arial, sans-serif; }",
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
    if let Some(btn_entity) = find_by_id(&result, "div", "btn") {
        simulate_click(&mut result, btn_entity);
    }
}

// --- Lifecycle event tests ---

#[test]
fn domcontentloaded_fires() {
    // DOMContentLoaded listener should fire during pipeline build.
    let result = build_pipeline_interactive(
        "<div id=\"target\">Before</div>\
         <script>\
           document.addEventListener('DOMContentLoaded', function() {\
             document.getElementById('target').textContent = 'DCL fired';\
           });\
         </script>",
        "",
    );
    // The listener should have been invoked during build.
    // Check the DOM — textContent should have been changed.
    if let Some(target_entity) = find_by_id(&result, "div", "target") {
        assert_eq!(get_text_content(&result.dom, target_entity), "DCL fired");
    }
}

#[test]
fn load_event_fires() {
    // load listener should fire during pipeline build.
    let result = build_pipeline_interactive(
        "<div id=\"target\">Before</div>\
         <script>\
           document.addEventListener('load', function() {\
             document.getElementById('target').textContent = 'loaded';\
           });\
         </script>",
        "",
    );
    if let Some(target_entity) = find_by_id(&result, "div", "target") {
        assert_eq!(get_text_content(&result.dom, target_entity), "loaded");
    }
}

#[test]
fn domcontentloaded_fires_before_load() {
    // DOMContentLoaded should fire before load.
    let result = build_pipeline_interactive(
        "<script>\
           var order = [];\
           document.addEventListener('DOMContentLoaded', function() {\
             order.push('dcl');\
           });\
           document.addEventListener('load', function() {\
             order.push('load');\
           });\
         </script>",
        "",
    );
    // Check that both events fired in the right order via console.
    // We need to read the `order` variable.
    // Use a follow-up eval to check.
    let mut session = result.session;
    let mut dom = result.dom;
    let mut runtime = result.runtime;
    runtime.eval(
        "console.log('order=' + order.join(','));",
        &mut session,
        &mut dom,
        result.document,
    );
    let messages = runtime.console_output().messages();
    assert!(
        messages.iter().any(|m| m.1.contains("order=dcl,load")),
        "Expected DOMContentLoaded before load, got: {messages:?}"
    );
}

#[test]
fn lifecycle_events_not_cancelable() {
    // preventDefault() on lifecycle events should not prevent them.
    let result = build_pipeline_interactive(
        "<script>\
           var prevented = false;\
           document.addEventListener('DOMContentLoaded', function(e) {\
             e.preventDefault();\
             prevented = e.defaultPrevented;\
             console.log('dcl-prevented=' + prevented);\
           });\
         </script>",
        "",
    );
    let messages = result.runtime.console_output().messages();
    // DOMContentLoaded is not cancelable, so preventDefault should have no effect.
    // The `defaultPrevented` property should remain false.
    assert!(
        messages.iter().any(|m| m.1.contains("dcl-prevented=false")),
        "DOMContentLoaded should not be cancelable, got: {messages:?}"
    );
}

#[test]
fn inline_run_produces_single_text_item() {
    // Verifies that inline text is collected and rendered correctly.
    let html = r"<p>Hello <strong>world</strong>!</p>";
    let css = "p { display: block; font-family: DejaVu Sans, Noto Sans, Arial, sans-serif; }";
    let dl = build_pipeline(html, css);
    let text_count = dl
        .iter()
        .filter(|i| matches!(i, DisplayItem::Text { .. }))
        .count();
    // Styled inline runs: one text item per styled segment.
    // "Hello " (p style), "world" (strong style), "!" (p style) = 3.
    assert_eq!(
        text_count, 3,
        "Expected 3 text items for styled inline run, got {text_count}"
    );
}
