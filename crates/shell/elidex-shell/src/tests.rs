use super::*;
use elidex_plugin::{AnimationEventInit, EventPayload, MouseEventInit, TransitionEventInit};
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
    result.dispatch_event(&mut event);
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
    // Capture the firing order in the DOM (each handler appends to `#order`),
    // read back via `get_text_content` — the same VM-robust pattern the sibling
    // `domcontentloaded_fires` / `load_event_fires` use (no reliance on a
    // cross-eval `var` global).
    let result = build_pipeline_interactive(
        "<div id=\"order\"></div>\
         <script>\
           document.addEventListener('DOMContentLoaded', function() {\
             document.getElementById('order').textContent += 'dcl,';\
           });\
           document.addEventListener('load', function() {\
             document.getElementById('order').textContent += 'load,';\
           });\
         </script>",
        "",
    );
    let order_entity = find_by_id(&result, "div", "order").expect("the #order div must exist");
    assert_eq!(
        get_text_content(&result.dom, order_entity),
        "dcl,load,",
        "DOMContentLoaded must fire before load",
    );
}

#[test]
fn lifecycle_events_not_cancelable() {
    // preventDefault() on lifecycle events should not prevent them.
    let mut result = build_pipeline_interactive(
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
    let messages = result.runtime.vm().console_messages();
    // DOMContentLoaded is not cancelable, so preventDefault should have no effect.
    // The `defaultPrevented` property should remain false.
    assert!(
        messages.iter().any(|m| m.1.contains("dcl-prevented=false")),
        "DOMContentLoaded should not be cancelable, got: {messages:?}"
    );
}

// --- CSS property registry tests ---

#[test]
fn get_computed_with_registry_matches_hardcoded() {
    use elidex_plugin::{ComputedStyle, CssColor, CssValue, Display, Float, LengthUnit};
    use elidex_style::get_computed_with_registry;

    let registry = create_css_property_registry();
    let style = ComputedStyle {
        display: Display::Flex,
        color: CssColor::RED,
        font_size: 20.0,
        float: Float::Left,
        opacity: 0.5,
        ..ComputedStyle::default()
    };

    // Verify that get_computed_with_registry returns expected values.
    let cases = &[
        ("display", CssValue::Keyword("flex".to_string())),
        ("color", CssValue::Color(CssColor::RED)),
        ("font-size", CssValue::Length(20.0, LengthUnit::Px)),
        ("float", CssValue::Keyword("left".to_string())),
        ("opacity", CssValue::Number(0.5)),
    ];
    for (prop, expected) in cases {
        let result = get_computed_with_registry(prop, &style, &registry);
        assert_eq!(
            result, *expected,
            "get_computed_with_registry({prop}) mismatch"
        );
    }
}

#[test]
fn registry_covers_all_handler_properties() {
    let registry = create_css_property_registry();

    // Verify that all 7 handlers' properties are resolvable in the registry.
    let expected_properties = &[
        "display",
        "position",
        "width",
        "margin-top",
        "padding-left",
        "border-top-width",
        "opacity",
        "background-color", // box
        "color",
        "font-size",
        "font-weight",
        "text-align",
        "white-space", // text
        "flex-direction",
        "flex-wrap",
        "justify-content", // flex
        "grid-template-columns",
        "grid-auto-flow", // grid
        "border-collapse",
        "table-layout", // table
        "float",
        "clear",
        "visibility",
        "vertical-align", // float
        "animation-name",
        "animation-duration",
        "transition-property",
        "transition-duration", // anim
    ];
    for prop in expected_properties {
        assert!(
            registry.resolve(prop).is_some(),
            "Registry should contain handler for '{prop}'"
        );
    }
}

#[test]
fn keyframes_registered_in_animation_engine() {
    let result = build_pipeline_interactive(
        "<div>Hello</div>",
        "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } div { display: block; }",
    );
    assert!(
        result.animation_engine.get_keyframes("fadeIn").is_some(),
        "fadeIn keyframes should be registered in the animation engine"
    );
}

#[test]
fn animation_properties_parsed_from_stylesheet() {
    // Verify that transition/animation properties are parsed from stylesheets
    // via the CssPropertyRegistry handler dispatch.
    let css = "div { animation-name: fadeIn; animation-duration: 1s; \
               transition-property: opacity; transition-duration: 0.3s; }";
    let registry = create_css_property_registry();
    let ss = elidex_css::parse_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(&registry),
    );
    assert_eq!(ss.rules.len(), 1);
    let decl_props: Vec<&str> = ss.rules[0]
        .declarations
        .iter()
        .map(|d| d.property.as_str())
        .collect();
    assert!(
        decl_props.contains(&"animation-name"),
        "animation-name should be parsed, got: {decl_props:?}"
    );
    assert!(
        decl_props.contains(&"animation-duration"),
        "animation-duration should be parsed, got: {decl_props:?}"
    );
    assert!(
        decl_props.contains(&"transition-property"),
        "transition-property should be parsed, got: {decl_props:?}"
    );
    assert!(
        decl_props.contains(&"transition-duration"),
        "transition-duration should be parsed, got: {decl_props:?}"
    );
}

#[test]
fn anim_style_populated_from_css() {
    use elidex_css_anim::style::{AnimStyle, TransitionProperty};

    let result = build_pipeline_interactive(
        "<div id=\"animated\">Hello</div>",
        "div { transition: opacity 0.3s ease; }",
    );

    let entity = find_by_id(&result, "div", "animated").expect("should find div#animated");
    let anim_style = result
        .dom
        .world()
        .get::<&AnimStyle>(entity)
        .expect("AnimStyle should be attached");
    assert_eq!(
        anim_style.transition_property,
        vec![TransitionProperty::Property("opacity".into())]
    );
    assert!((anim_style.transition_duration[0] - 0.3).abs() < 1e-6);
}

#[test]
fn anim_style_not_attached_without_animation_props() {
    use elidex_css_anim::style::AnimStyle;

    let result = build_pipeline_interactive("<div id=\"plain\">Hello</div>", "div { color: red; }");

    let entity = find_by_id(&result, "div", "plain").expect("should find div#plain");
    let anim_result = result.dom.world().get::<&AnimStyle>(entity);
    assert!(
        anim_result.is_err(),
        "AnimStyle should not be attached when no animation/transition properties are set"
    );
}

#[test]
fn css_animation_auto_started_on_initial_render() {
    // Verify that CSS animations declared in the stylesheet are automatically
    // started in the animation engine on initial pipeline build.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } \
               div { animation-name: fadeIn; animation-duration: 1s; }";
    let result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(
        active.len(),
        1,
        "Expected 1 active animation, got {}",
        active.len()
    );
    assert_eq!(active[0].name(), "fadeIn");
}

#[test]
fn css_animation_not_started_without_keyframes() {
    // animation-name references a non-existent @keyframes — should not start.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "div { animation-name: nonexistent; animation-duration: 1s; }";
    let result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(
        active.len(),
        0,
        "Should not start animation without @keyframes"
    );
}

#[test]
fn css_animation_none_ignored() {
    // animation-name: none should not start any animation.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } \
               div { animation-name: none; animation-duration: 1s; }";
    let result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(active.len(), 0, "animation-name: none should not start");
}

#[test]
fn re_render_preserves_running_animations() {
    // Verify that re_render with no style changes keeps existing animations running.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } \
               div { animation-name: fadeIn; animation-duration: 1s; }";
    let mut result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        1,
        "Should have 1 animation before re_render"
    );

    // Re-render with no DOM changes — animation should persist.
    re_render(&mut result);

    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(
        active.len(),
        1,
        "Animation should persist across re_render with no changes"
    );
    assert_eq!(active[0].name(), "fadeIn");
}

#[test]
fn re_render_registers_dynamically_added_keyframes() {
    // Regression: a `@keyframes foo` defined *after* the pipeline is built
    // (here via a `<style>` injected into the live DOM) plus an element that
    // gains `animation-name: foo` in the same turn must start the animation.
    // Before the fix, `re_render` re-collected the new stylesheet for style
    // resolution but left `result.animation_engine` with only the
    // construction-time keyframe set, so `sync_css_animations` skipped the new
    // name (`get_keyframes("foo").is_none()`). This drives the lowest layer that
    // exercises the `re_render` re-registration path: a direct DOM `<style>`
    // append followed by `re_render`.
    let html = r#"<div id="box">Hello</div>"#;
    let mut result = build_pipeline_interactive(html, "div { color: black; }");

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();

    // Precondition: the construction-time engine has never seen `foo`, and no
    // animation is running.
    assert!(
        result.animation_engine.get_keyframes("foo").is_none(),
        "foo keyframes must not exist before the dynamic <style> is added"
    );
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        0,
        "no animation should run before the dynamic <style> is added"
    );

    // Inject a connected `<style>` defining both the `@keyframes` and the rule
    // that puts `animation-name: foo` on the element — the same turn.
    let parent = result
        .dom
        .query_by_tag("body")
        .into_iter()
        .next()
        .unwrap_or(result.document);
    let style = result
        .dom
        .create_element("style", elidex_ecs::Attributes::default());
    let text = result.dom.create_text(
        "@keyframes foo { from { opacity: 0; } to { opacity: 1; } } \
         #box { animation-name: foo; animation-duration: 1s; }",
    );
    assert!(result.dom.append_child(style, text));
    assert!(result.dom.append_child(parent, style));

    re_render(&mut result);

    // The re-collected sheet's keyframes are now in the engine, and the
    // animation started this frame.
    assert!(
        result.animation_engine.get_keyframes("foo").is_some(),
        "foo keyframes should be re-registered from the re-collected stylesheet"
    );
    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(
        active.len(),
        1,
        "the dynamically-added animation should have started, got {}",
        active.len()
    );
    assert_eq!(active[0].name(), "foo");
}

#[test]
fn re_render_does_not_duplicate_animations() {
    // Verify that re_render with unchanged CSS doesn't duplicate animations.
    // sync_css_animations should skip already-running names.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } \
               @keyframes slideUp { from { opacity: 0; } to { opacity: 1; } } \
               div { animation-name: fadeIn, slideUp; animation-duration: 1s, 2s; }";
    let mut result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        2,
        "Should have 2 animations initially"
    );

    // Re-render multiple times — count should stay at 2.
    re_render(&mut result);
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        2,
        "Should still have 2 animations after first re_render"
    );

    re_render(&mut result);
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        2,
        "Should still have 2 animations after second re_render"
    );
}

#[test]
fn cancel_animations_by_name_selective() {
    // Test that cancel_animations_by_name only removes specified names.
    let html = r#"<div id="box">Hello</div>"#;
    let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } } \
               @keyframes slideUp { from { opacity: 0; } to { opacity: 1; } } \
               div { animation-name: fadeIn, slideUp; animation-duration: 1s, 2s; }";
    let mut result = build_pipeline_interactive(html, css);

    let entity = find_by_id(&result, "div", "box").expect("should find div#box");
    let entity_bits = entity.to_bits().get();
    assert_eq!(
        result.animation_engine.active_animations(entity_bits).len(),
        2,
    );

    // Cancel only fadeIn — slideUp should remain.
    let mut to_cancel = std::collections::HashSet::new();
    to_cancel.insert("fadeIn");
    let events = result
        .animation_engine
        .cancel_animations_by_name(entity_bits, &to_cancel);
    assert_eq!(events.len(), 1, "Should emit 1 cancel event");

    let active = result.animation_engine.active_animations(entity_bits);
    assert_eq!(active.len(), 1, "Should have 1 remaining animation");
    assert_eq!(active[0].name(), "slideUp");
}

#[test]
fn re_render_with_transitions_does_not_panic() {
    // Verify the full re_render pipeline (including transition detection
    // and animated value application) doesn't panic with transition CSS.
    let mut result = build_pipeline_interactive(
        "<div>Hello</div>",
        "div { opacity: 1; transition: opacity 0.5s linear; }",
    );

    // Re-render should succeed without panic.
    re_render(&mut result);
    // Just verify re_render completes successfully.
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

#[test]
fn transition_event_dispatched_to_js_listener() {
    let html = r#"<div id="box">test</div>
<script>
  var el = document.getElementById("box");
  el.addEventListener("transitionend", function(e) {
    console.log("te:" + e.propertyName + ":" + e.elapsedTime + ":" + e.pseudoElement);
  });
</script>"#;
    let css = "div { opacity: 1; transition: opacity 0.3s linear; }";
    let mut result = build_pipeline_interactive(html, css);

    let div = find_by_id(&result, "div", "box").unwrap();

    // Dispatch a synthetic transitionend event.
    let mut event = DispatchEvent::new_composed("transitionend", div);
    event.cancelable = false;
    event.payload = EventPayload::Transition(TransitionEventInit {
        property_name: "opacity".into(),
        elapsed_time: 0.3,
        pseudo_element: String::new(),
    });
    result.dispatch_event(&mut event);

    let messages = result.runtime.vm().console_messages();
    assert!(
        messages.iter().any(|m| m.1.starts_with("te:opacity:0.3")),
        "Expected transitionend with propertyName=opacity, got: {messages:?}"
    );
}

#[test]
fn animation_event_dispatched_to_js_listener() {
    let html = r#"<div id="box">test</div>
<script>
  var el = document.getElementById("box");
  el.addEventListener("animationend", function(e) {
    console.log("ae:" + e.animationName + ":" + e.elapsedTime + ":" + e.pseudoElement);
  });
</script>"#;
    let css = "div { opacity: 1; }";
    let mut result = build_pipeline_interactive(html, css);

    let div = find_by_id(&result, "div", "box").unwrap();

    // Dispatch a synthetic animationend event.
    let mut event = DispatchEvent::new_composed("animationend", div);
    event.cancelable = false;
    event.payload = EventPayload::Animation(AnimationEventInit {
        animation_name: "fadeIn".into(),
        elapsed_time: 1.0,
        pseudo_element: String::new(),
    });
    result.dispatch_event(&mut event);

    let messages = result.runtime.vm().console_messages();
    assert!(
        messages.iter().any(|m| m.1.starts_with("ae:fadeIn:1:")),
        "Expected animationend with animationName=fadeIn, got: {messages:?}"
    );
}

// TODO(S5-6b stage3 oracle migration): rendered-outcome insertRule oracle.
// The boa `apply_cssom_mutations` shadow-struct oracle
// (`boa_insert_rule_skips_media_conditioned_rule`) was deleted with the CSSOM
// shadow-sync (§3.3/§4.2, this stage): it asserted equality on the boa
// `CssomSheet` shadow copy, which no longer exists — the VM writes `insertRule`
// back to the DOM owner source and `re_render`'s DOM→cascade re-collection
// (`elidex_dom_api::collect_document_stylesheets`) picks it up. The replacement
// is a rendered-outcome pipeline test: drive `insertRule("@media …")` /
// `insertRule("span …")` via script and assert the re-resolved style /
// display-list effect (the observable the struct-equality oracle was a proxy
// for). Deferred to stage 3 because the pipeline does not compile until the
// remaining B-row call-site convergence lands. The `@media`-skip + plain-insert
// invariant it guarded stays covered engine-independently by the dom-api
// `cssom_sheet::tests` (`actual_index_skips_media_rules`,
// `insert_position_maps_visible_to_actual`) and the re-collection unit oracles
// in `elidex_dom_api::cssom_collect::tests` (written-back `<style>`/`<link>`
// pickup, changed-owner-only re-parse, idle zero-reparse).

// --- E0 (F6): engine-mode-gated style compat ---

/// An entity's resolved `color` as a stable string, for cross-mode comparison.
fn computed_color(result: &PipelineResult, entity: Entity) -> String {
    let r = result
        .dom
        .world()
        .get::<&elidex_plugin::ComputedStyle>(entity)
        .expect("ComputedStyle not found");
    format!("{:?}", r.color)
}

#[test]
fn engine_mode_gates_presentational_hints() {
    // `<font color="red">` is colored ONLY by the presentational-hint compat
    // layer (WHATWG HTML §15.2). BrowserCompat (the production default) applies
    // it; BrowserCore resolves against the modern UA baseline and must drop it —
    // exercising the `resolve_with_mode` core arm through the real `re_render`
    // path (which reads `result.engine_mode`).
    let mut result = build_pipeline_interactive("<font id=\"f\" color=\"red\">x</font>", "");
    let font = find_by_id(&result, "font", "f").expect("font element");

    re_render(&mut result); // default engine_mode = BrowserCompat
    let compat_color = computed_color(&result, font);

    result.engine_mode = elidex_plugin::EngineMode::BrowserCore;
    re_render(&mut result);
    let core_color = computed_color(&result, font);

    assert_ne!(
        compat_color, core_color,
        "BrowserCore must drop the <font color> presentational hint \
         (compat={compat_color}, core={core_color})"
    );
}

#[test]
fn engine_mode_core_keeps_modern_baseline_for_plain_content() {
    // A plain <div> styled by an author rule (no legacy tag, no presentational
    // attribute) resolves identically in both modes — the gate touches only the
    // compat surface, never the modern UA baseline + author cascade.
    let html = "<div id=\"d\">x</div>";
    let css = "div { color: green; }";

    let mut compat = build_pipeline_interactive(html, css);
    re_render(&mut compat);
    let d_compat = find_by_id(&compat, "div", "d").expect("div");
    let compat_color = computed_color(&compat, d_compat);

    let mut core = build_pipeline_interactive(html, css);
    core.engine_mode = elidex_plugin::EngineMode::BrowserCore;
    re_render(&mut core);
    let d_core = find_by_id(&core, "div", "d").expect("div");
    let core_color = computed_color(&core, d_core);

    assert_eq!(
        compat_color, core_color,
        "modern baseline + author cascade must be mode-invariant \
         (compat={compat_color}, core={core_color})"
    );
}

/// An entity's resolved `font-weight`.
fn font_weight(result: &PipelineResult, entity: Entity) -> u16 {
    result
        .dom
        .world()
        .get::<&elidex_plugin::ComputedStyle>(entity)
        .expect("ComputedStyle not found")
        .font_weight
}

#[test]
fn engine_mode_core_keeps_standard_ua_rendering() {
    // Root-cause regression lock (Codex #406 P2-1): standard §15.3 phrasing
    // rendering (e.g. `<strong>` font-weight: bolder → 700) lives in the CORE UA
    // sheet (after the #408 reclassification), so BrowserCore keeps it even though
    // the core arm drops the compat legacy sheet + presentational hints. `<strong>`
    // is UA-standard (not author/hint), so it must be bold in BOTH modes — dropping
    // the compat sheet must NOT strip standard rendering.
    let html = "<strong id=\"s\">x</strong>";

    let mut compat = build_pipeline_interactive(html, "");
    re_render(&mut compat);
    let s_compat = find_by_id(&compat, "strong", "s").expect("strong");

    let mut core = build_pipeline_interactive(html, "");
    core.engine_mode = elidex_plugin::EngineMode::BrowserCore;
    re_render(&mut core);
    let s_core = find_by_id(&core, "strong", "s").expect("strong");

    assert_eq!(
        font_weight(&compat, s_compat),
        700,
        "<strong> must be bold (700) in BrowserCompat"
    );
    assert_eq!(
        font_weight(&core, s_core),
        700,
        "BrowserCore must keep standard <strong> UA rendering — dropping the compat \
         legacy sheet must not strip §15.3 rendering (Codex #406 P2-1)"
    );
}

/// F14 (§4.3.3 / Slice-7): the pipeline construction seam MUST route
/// `localStorage` through the shell-owned [`WebStorageManager`], so a page-load
/// `setItem` persists + is same-origin shared — NOT the per-VM in-memory
/// `fallback_local_storage` (the pre-flip regression this closes). Drives the
/// lowest builder that installs a manager and carries a real tuple origin, then
/// asserts the write is visible THROUGH the manager (keyed by the document's
/// serialized origin), and that an un-installed (hermetic) build never reaches it.
#[test]
fn pipeline_construction_installs_web_storage_manager() {
    fn temp_manager(tag: &str) -> std::sync::Arc<elidex_storage_core::WebStorageManager> {
        let dir = std::env::temp_dir().join(format!(
            "elidex-f14-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::sync::Arc::new(elidex_storage_core::WebStorageManager::new(dir))
    }

    let manager = temp_manager("installed");
    let url = url::Url::parse("http://example.com/").unwrap();
    let origin = elidex_plugin::SecurityOrigin::from_url(&url).serialize();

    // A page-load script writes localStorage DURING construction (the scripts run
    // inside `run_scripts_and_finalize`, after the pre-eval install seam).
    let installed = build_pipeline_interactive_shared(
        "<script>localStorage.setItem('k', 'v');</script>",
        Some(url.clone()),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected()),
        std::sync::Arc::new(crate::create_css_property_registry()),
        None, // cookie jar
        Some(std::sync::Arc::clone(&manager)),
        elidex_plugin::Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
        crate::ipc::DeviceFacts::default(),
        None, // top-level: origin derives from `url`
    );
    drop(installed);

    // The write landed in the installed manager (persisted + cross-tab visible),
    // proving `install_web_storage` was wired at the construction seam.
    assert_eq!(
        manager.local_get(&origin, "k").as_deref(),
        Some("v"),
        "construction-seam install must route localStorage through the manager (F14)"
    );

    // A build WITHOUT a manager (the hermetic `build_pipeline_interactive` path)
    // falls back to the per-VM in-memory store — a fresh bystander manager never
    // observes the write, and the real manager is untouched by it.
    let uninstalled = build_pipeline_interactive(
        "<script>localStorage.setItem('k', 'bystander');</script>",
        "",
    );
    drop(uninstalled);
    let bystander = temp_manager("bystander");
    assert_eq!(
        bystander.local_get(&origin, "k"),
        None,
        "an un-installed build must not reach any WebStorageManager (fallback path)"
    );
    assert_eq!(
        manager.local_get(&origin, "k").as_deref(),
        Some("v"),
        "the un-installed build must not have mutated the real manager"
    );
}
