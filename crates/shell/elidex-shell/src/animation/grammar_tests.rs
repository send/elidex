use crate::{build_pipeline_interactive, re_render};
use elidex_plugin::{ComputedStyle, Dimension};

fn sample(keyframes: &str, progress: f64) -> ComputedStyle {
    let css = format!("@keyframes test {{ {keyframes} }} div {{ flex-grow:4; margin-left:80px; animation-name:test; animation-duration:1s; animation-timing-function:linear }}");
    let mut result = build_pipeline_interactive("<div></div>", &css);
    let target = result.dom.query_by_tag("div")[0];
    let _events = result.animation_engine.tick(progress);
    re_render(&mut result);
    let style = result.dom.world().get::<&ComputedStyle>(target).unwrap();
    (*style).clone()
}

#[test]
fn property_offsets_ignore_unrelated_or_invalid_declarations() {
    for interior in ["opacity:.5", "margin-left:invalid"] {
        let frames = format!("from {{ flex:0 4 auto; margin-left:0px }} 50% {{ {interior}; animation-timing-function:steps(1, end) }} to {{ flex:calc(1 + 1) calc(-1) auto; margin-left:100px }}");
        for (t, grow, shrink, margin) in [(0.25, 0.5, 3., 25.), (0.75, 1.5, 1., 75.)] {
            let style = sample(&frames, t);
            assert_eq!(style.flex_grow, grow);
            assert_eq!(style.flex_shrink, shrink);
            assert_eq!(style.margin_left, Dimension::Length(margin));
        }
    }
}

#[test]
fn pending_direct_and_shorthand_override_same_offset_discretely() {
    for declaration in ["flex:var(--x)", "flex-grow:var(--x)"] {
        let frames = format!("from {{ flex-grow:0 }} to {{ flex-grow:2; {declaration} }}");
        assert_eq!(sample(&frames, 0.25).flex_grow, 0.);
        assert_eq!(sample(&frames, 0.75).flex_grow, 4.);
    }
}

#[test]
fn missing_endpoints_read_underlying_value() {
    assert_eq!(sample("to { flex-grow:2 }", 0.5).flex_grow, 3.);
    assert_eq!(sample("from { flex-grow:2 }", 0.5).flex_grow, 3.);
    assert_eq!(
        sample("from { margin-left:0px } to { margin-left:1in }", 0.5).margin_left,
        Dimension::Length(48.)
    );
    assert_eq!(
        sample("to { margin-left:1in }", 0.5).margin_left,
        Dimension::Length(88.)
    );
}

#[test]
fn shared_keyframes_use_each_targets_font_and_current_viewport() {
    let css = "@keyframes test { from {margin-left:0px;margin-top:0px} to {margin-left:2em;margin-top:10lvw} } div {animation-name:test;animation-duration:1s;animation-timing-function:linear} #a {font-size:12px} #b {font-size:24px}";
    let mut result = build_pipeline_interactive("<div id=a></div><div id=b></div>", css);
    let _events = result.animation_engine.tick(0.5);
    for width in [200., 400.] {
        result.viewport.width = width;
        re_render(&mut result);
        for entity in result.dom.query_by_tag("div") {
            let style = result.dom.world().get::<&ComputedStyle>(entity).unwrap();
            assert_eq!(style.margin_left, Dimension::Length(style.font_size));
            assert_eq!(style.margin_top, Dimension::Length(width * 0.05));
        }
    }
}

#[test]
fn keyframe_translation_uses_target_font_before_interpolating() {
    use elidex_plugin::{CssValue, LengthUnit, TransformFunction};
    let style = sample(
        "from {transform:translateX(0px)} to {transform:translateX(2em)}",
        0.5,
    );
    assert_eq!(
        style.transform,
        vec![TransformFunction::TranslateX(CssValue::Length(
            style.font_size,
            LengthUnit::Px
        ))]
    );
}

#[test]
fn parent_and_child_read_pre_animation_fonts_for_the_frame() {
    let css = "@keyframes parent {from {font-size:20px} to {font-size:40px}} @keyframes child {from {font-size:2em} to {font-size:2em}} div, span {animation-duration:1s;animation-timing-function:linear} div {font-size:20px;animation-name:parent} span {animation-name:child}";
    let mut result = build_pipeline_interactive("<div><span></span></div>", css);
    let _events = result.animation_engine.tick(0.5);
    re_render(&mut result);
    for (tag, size) in [("div", 30.), ("span", 40.)] {
        let entity = result.dom.query_by_tag(tag)[0];
        assert_eq!(
            result
                .dom
                .world()
                .get::<&ComputedStyle>(entity)
                .unwrap()
                .font_size,
            size
        );
    }
}

#[test]
fn two_targets_share_declared_keyframes_and_keep_distinct_underlying_values() {
    let css = "@keyframes test { to { flex-grow:calc(1 + 1); flex-grow:-1; margin-left:1in } } div { animation-name:test; animation-duration:1s; animation-timing-function:linear; margin-left:0px } #first {flex-grow:4} #second {flex-grow:8}";
    let mut result = build_pipeline_interactive("<div id=first></div><div id=second></div>", css);
    let rule = result
        .animation_engine
        .get_keyframes("test")
        .unwrap()
        .clone();
    let _events = result.animation_engine.tick(0.5);
    re_render(&mut result);
    let entities = result.dom.query_by_tag("div");
    assert_eq!(entities.len(), 2);
    for entity in entities {
        let attributes = result
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(entity)
            .unwrap();
        let expected = if attributes.get("id") == Some("first") {
            3.
        } else {
            5.
        };
        let style = result.dom.world().get::<&ComputedStyle>(entity).unwrap();
        assert_eq!(style.flex_grow, expected);
        assert_eq!(style.margin_left, Dimension::Length(48.));
    }
    assert_eq!(result.animation_engine.get_keyframes("test"), Some(&rule));
}
