//! Vertical writing modes: line advance comes from the font size, not the
//! horizontal line-height.

use super::*;

#[test]
fn vertical_mode_uses_font_size_line_advance() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let block_dim =
        layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (block_dim - style.font_size).abs() < f32::EPSILON,
        "vertical single line should be font_size ({}), got {}",
        style.font_size,
        block_dim,
    );
}

#[test]
fn vertical_lr_same_as_vertical_rl_for_height() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalLr;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let block_dim =
        layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (block_dim - style.font_size).abs() < f32::EPSILON,
        "vertical-lr single line should be font_size ({}), got {}",
        style.font_size,
        block_dim,
    );
}

#[test]
fn horizontal_tb_uses_line_height() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
        return;
    };

    let css_line_height = style.line_height.resolve_px(style.font_size);
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
        (h - css_line_height).abs() < f32::EPSILON,
        "horizontal-tb single line should be line-height ({css_line_height}), got {h}",
    );
}
