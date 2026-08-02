//! Runs collected across nested spans with differing styles, and the
//! line-height they resolve to.

use super::*;

#[test]
fn styled_runs_collect_from_nested_span() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        font_size: 24.0,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("World");
    dom.append_child(span, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 2, "should have 2 runs");
    assert_eq!(runs[0].text, "Hello ");
    assert!((runs[0].font_size - style.font_size).abs() < f32::EPSILON);
    assert_eq!(runs[1].text, "World");
    assert!((runs[1].font_size - 24.0).abs() < f32::EPSILON);
}

#[test]
fn multi_style_line_height_uses_max() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("A");
    dom.append_child(parent, text1);
    let span = dom.create_element("span", Attributes::default());
    let big_style = ComputedStyle {
        font_size: 32.0,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let big_line_height = big_style.line_height.resolve_px(big_style.font_size);
    let _ = dom.world_mut().insert_one(span, big_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("B");
    dom.append_child(span, text2);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - big_line_height).abs() < 1.0,
        "line height should be the bigger style's line-height ({big_line_height}), got {h}",
    );
}

#[test]
fn display_none_child_skipped_in_runs() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("visible");
    dom.append_child(parent, text1);
    let hidden = dom.create_element("span", Attributes::default());
    let hidden_style = ComputedStyle {
        display: Display::None,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);
    dom.append_child(parent, hidden);
    let text2 = dom.create_text("hidden");
    dom.append_child(hidden, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "visible");
}
