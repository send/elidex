//! Which inline participants receive a `LayoutBox`, and the fragment bounds
//! a multi-line inline box unions.

use super::*;
use elidex_plugin::WhiteSpace;

#[test]
fn inline_span_gets_layout_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
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
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("World");
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
    let _result = layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::new(10.0, 20.0),
        &env,
    );

    let lb = dom.world().get::<&LayoutBox>(span);
    assert!(
        lb.is_ok(),
        "inline span should have a LayoutBox after layout"
    );
    let lb = lb.unwrap();
    assert!(
        lb.content.origin.x >= 10.0,
        "span x should be >= content_origin.x"
    );
    assert!(
        (lb.content.origin.y - 20.0).abs() < f32::EPSILON,
        "span y should be content_origin.y"
    );
    assert!(
        lb.content.size.width > 0.0,
        "span should have positive width"
    );
    assert!(
        lb.content.size.height > 0.0,
        "span should have positive height"
    );
}

#[test]
fn whitespace_only_inline_span_gets_no_layout_box() {
    // A span whose only content is collapsible whitespace generates no box — its
    // line is suppressed (CSS 2 §9.4.2 zero-height line boxes; NOT §9.2.2.1, whose
    // suppression sentence is scoped to ANONYMOUS inline boxes and this text is
    // inside a real `<span>`) — so it must NOT get a phantom LayoutBox /
    // getClientRects geometry. The per-line rects are discarded on suppression.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let ws = dom.create_text("   ");
    dom.append_child(span, ws);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        dom.world().get::<&LayoutBox>(span).is_err(),
        "whitespace-only inline span must not get a phantom LayoutBox",
    );
}

#[test]
fn multi_line_inline_box_unions_fragment_bounds() {
    // A `<span>` spanning two lines must get a LayoutBox whose width encloses the
    // WIDER line, not just the last (narrow) fragment — `getBoundingClientRect` is
    // the union of the fragment rects. `white-space: pre` forces the break.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        white_space: WhiteSpace::Pre,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text = dom.create_text("WIDE\nx");
    dom.append_child(span, text);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env);

    let lb = dom.world().get::<&LayoutBox>(span);
    assert!(lb.is_ok(), "multi-line span should get a LayoutBox");
    let span_width = lb.unwrap().content.size.width;

    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: style.font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    let narrow = measure_text(&font_db, &params, "x").map_or(0.0, |m| m.width);
    // Union ⇒ the box width reflects the wide first line, so it is strictly wider
    // than the narrow last line alone (the overwrite-bug result).
    assert!(
        span_width > narrow + 1.0,
        "multi-line span box width {span_width} must enclose the wider line, not just the narrow last line ({narrow})",
    );
}

#[test]
fn parent_entity_does_not_get_inline_layout_box() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hello") else {
        return;
    };

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        dom.world().get::<&LayoutBox>(parent).is_err(),
        "parent entity should not get LayoutBox from inline layout"
    );
}
