use crate::{build_pipeline_interactive, re_render};
use elidex_plugin::{ComputedStyle, Dimension};

fn sample(keyframes: &str, progress: f64) -> ComputedStyle {
    sample_with_grow(keyframes, progress, 4.)
}

fn sample_with_grow(keyframes: &str, progress: f64, grow: f32) -> ComputedStyle {
    sample_on(
        keyframes,
        progress,
        &format!("flex-grow:{grow}; margin-left:80px"),
    )
}

fn sample_on(keyframes: &str, progress: f64, underlying: &str) -> ComputedStyle {
    let css = format!("@keyframes test {{ {keyframes} }} div {{ {underlying}; animation-name:test; animation-duration:1s; animation-timing-function:linear }}");
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
        for (t, grow, shrink, margin) in [
            (0.25, 0.5, 3., 25.),
            (0.5, 1., 2., 50.),
            (0.75, 1.5, 1., 75.),
        ] {
            let style = sample(&frames, t);
            assert_eq!(style.flex_grow, grow);
            assert_eq!(style.flex_shrink, shrink);
            let literal = sample(
                "from { flex-grow:0; flex-shrink:4 } to { flex-grow:2; flex-shrink:0 }",
                t,
            );
            assert_eq!(
                (style.flex_grow, style.flex_shrink),
                (literal.flex_grow, literal.flex_shrink)
            );
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
    assert_eq!(
        sample_with_grow("from { flex-grow:4 }", 0.5, 2.).flex_grow,
        3.
    );
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

#[test]
fn css_wide_flex_endpoints_resolve_before_interpolation() {
    for t in [0.25, 0.75] {
        let expected = sample("from {flex:0 1 auto} to {flex:2 1 auto}", t).flex_grow;
        for decl in ["flex:initial", "flex-grow:initial", "flex:unset"] {
            assert_eq!(
                sample(&format!("from {{{decl}}} to {{flex:2 1 auto}}"), t).flex_grow,
                expected
            );
        }
    }
}

#[test]
fn integer_keyframe_samples_round_ties_toward_positive_infinity() {
    for (from, to, t, expected) in [(1, 2, 0.75, 2), (-2, -1, 0.5, -1), (1, 2, 0.25, 1)] {
        let style = sample(
            &format!("from {{order:{from};z-index:{from}}} to {{order:{to};z-index:{to}}}"),
            t,
        );
        assert_eq!((style.order, style.z_index), (expected, Some(expected)));
    }
}

#[test]
fn translate_leaves_retain_physical_and_large_viewport_units() {
    for (source, control) in [("1in", "96px"), ("10lvw", "10vw"), ("10lvh", "10vh")] {
        for function in ["translateX", "translateY", "translateZ"] {
            let frames =
                |v| format!("from {{transform:{function}(0px)}} to {{transform:{function}({v})}}");
            assert_eq!(
                sample(&frames(source), 0.5).transform,
                sample(&frames(control), 0.5).transform
            );
        }
    }
}

#[test]
fn slotted_and_shadow_children_inherit_the_flat_tree_font() {
    use elidex_ecs::{Attributes, ShadowRootMode};
    let css = "@keyframes test {from {font-size:2em} to {font-size:2em}} div {font-size:10px} span {animation-name:test;animation-duration:1s}";
    let mut result = build_pipeline_interactive("<div><span></span></div>", css);
    let host = result.dom.query_by_tag("div")[0];
    let light = result.dom.query_by_tag("span")[0];
    let root = result
        .dom
        .attach_shadow(host, ShadowRootMode::Open)
        .unwrap();
    let slot = result
        .dom
        .create_element_ownerless_fixture("slot", Attributes::default());
    result.dom.set_attribute(slot, "style", "font-size:30px");
    assert!(result.dom.append_child(root, slot));
    let internal = result
        .dom
        .create_element_ownerless_fixture("p", Attributes::default());
    result.dom.set_attribute(
        internal,
        "style",
        "animation-name:test;animation-duration:1s",
    );
    assert!(result.dom.append_child(root, internal));
    re_render(&mut result);
    assert_eq!(
        result
            .dom
            .world()
            .get::<&ComputedStyle>(internal)
            .unwrap()
            .font_size,
        20.
    );
    assert_eq!(super::inheritance_parent(&result.dom, light), Some(slot));
    assert_eq!(super::inheritance_parent(&result.dom, slot), Some(host));
    assert_eq!(
        result
            .dom
            .world()
            .get::<&ComputedStyle>(light)
            .unwrap()
            .font_size,
        60.
    );
}

#[test]
fn stylesheet_versions_replace_only_changed_keyframe_indices() {
    let mut result = build_pipeline_interactive("<div></div>", "@keyframes keep {to {opacity:1}}");
    let doc = result.document;
    let style = result
        .dom
        .create_element_ownerless_fixture("style", elidex_ecs::Attributes::default());
    let text = result
        .dom
        .create_text_ownerless_fixture("@keyframes change {to {order:1}}");
    assert!(result.dom.append_child(style, text));
    assert!(result.dom.append_child(doc, style));
    re_render(&mut result);
    let keep = std::ptr::from_ref(result.animation_engine.get_keyframes("keep").unwrap());
    let change = std::ptr::from_ref(result.animation_engine.get_keyframes("change").unwrap());
    re_render(&mut result);
    assert_eq!(
        keep,
        std::ptr::from_ref(result.animation_engine.get_keyframes("keep").unwrap())
    );
    assert_eq!(
        change,
        std::ptr::from_ref(result.animation_engine.get_keyframes("change").unwrap())
    );
    assert!(result.dom.remove_child(style, text));
    let replacement = result
        .dom
        .create_text_ownerless_fixture("@keyframes change {to {order:2}}");
    assert!(result.dom.append_child(style, replacement));
    re_render(&mut result);
    assert_eq!(
        keep,
        std::ptr::from_ref(result.animation_engine.get_keyframes("keep").unwrap())
    );
    let changed = result.animation_engine.get_keyframes("change").unwrap();
    assert_ne!(change, std::ptr::from_ref(changed));
    assert_eq!(
        changed.keyframes.last().unwrap().declarations[0].value,
        elidex_plugin::CssValue::Number(2.)
    );
    assert!(result.dom.remove_child(doc, style));
    re_render(&mut result);
    assert!(result.animation_engine.get_keyframes("change").is_none());
}

#[test]
fn inherited_flex_keyframe_uses_the_parents_computed_factors() {
    let css = "@keyframes test {from {flex:inherit} to {flex:2 1 auto}} main {flex:6 3 auto} span {flex:8 5 auto;animation-name:test;animation-duration:1s;animation-timing-function:linear}";
    let mut result = build_pipeline_interactive("<main><span></span></main>", css);
    let _events = result.animation_engine.tick(0.5);
    re_render(&mut result);
    let span = result.dom.query_by_tag("span")[0];
    let style = result.dom.world().get::<&ComputedStyle>(span).unwrap();
    assert_eq!((style.flex_grow, style.flex_shrink), (4., 2.));
}

#[test]
fn initial_border_endpoints_use_target_style_and_color() {
    let style = sample_on(
        "from {border-top-width:initial} to {border-top-width:9px}",
        0.5,
        "border-top-style:solid",
    );
    assert_eq!(style.border_top.width, 6.);
    for border in ["none", "hidden", "solid"] {
        let style = sample_on("from {border-top-width:initial;border-top-color:initial} to {border-top-width:initial;border-top-color:initial}", 0.5, &format!("border-top-style:{border};border-top-width:12px;color:red;border-top-color:blue"));
        assert_eq!(
            style.border_top.width,
            if border == "solid" { 3. } else { 0. }
        );
        assert_eq!(style.border_top.color, style.color);
    }
}

#[test]
fn discrete_computed_endpoints_replace_nondefault_underlying_values() {
    for end in ["2", "initial"] {
        let frames = format!(
            "from {{z-index:initial;line-height:initial}} to {{z-index:{end};line-height:2}}"
        );
        let style = sample_on(&frames, 0.25, "position:relative;z-index:7;line-height:4");
        assert_eq!(style.z_index, None);
        assert_eq!(style.line_height, elidex_plugin::LineHeight::Normal);
        let later = sample_on(&frames, 0.75, "position:relative;z-index:7;line-height:4");
        assert_eq!(later.z_index, end.parse().ok());
        assert_eq!(later.line_height, elidex_plugin::LineHeight::Number(2.));
    }
}

#[test]
fn perspective_property_resolves_physical_and_viewport_literals() {
    for (unit, reference) in [("1in", "96px"), ("10lvw", "10vw")] {
        let frames = |v| format!("from {{perspective:96px}} to {{perspective:{v}}}");
        let style = sample(&frames(unit), 0.5);
        assert!(style.has_perspective);
        assert_eq!(
            style.perspective,
            sample(&frames(reference), 0.5).perspective
        );
        if unit == "1in" {
            assert_eq!(style.perspective, Some(96.));
        }
    }
}

#[test]
fn calculated_flex_basis_samples_in_the_targets_length_context() {
    for property in ["flex:1 1", "flex-basis:"] {
        for (from, to, midpoint) in [
            ("calc(10px)", "calc(50px)", 30.),
            ("calc(1em + 4px)", "calc(2em)", 32.),
            ("calc(-10px)", "calc(50px)", 25.),
        ] {
            let frames = format!("from {{{property} {from}}} to {{{property} {to}}}");
            let style = sample_on(&frames, 0.5, "font-size:20px;flex-basis:7px");
            assert_eq!(
                style.flex_basis,
                elidex_plugin::FlexBasis::Length(midpoint),
                "{frames}"
            );
        }
    }
}
