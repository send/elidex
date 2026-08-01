//! Anonymous block boxes (CSS 2 §9.2.1.1) — inline runs among block
//! siblings, and the cases that generate no anonymous box.

use super::*;

#[test]
fn mixed_block_inline_anonymous_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let _ = dom.world_mut().insert_one(parent, style.clone());

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let block = dom.create_element("p", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(40.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block, block_style);
    dom.append_child(parent, block);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        result.height >= 40.0,
        "height should be at least block child height (40), got {}",
        result.height
    );
    let line_h = style.line_height.resolve_px(style.font_size);
    let expected_min = 40.0 + line_h;
    assert!(
        result.height >= expected_min,
        "height should include anonymous box height ({expected_min}), got {}",
        result.height
    );
}

#[test]
fn block_only_children_no_anonymous_boxes() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let block1 = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(20.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block1, block_style.clone());
    dom.append_child(parent, block1);

    let block2 = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(block2, block_style);
    dom.append_child(parent, block2);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        (result.height - 40.0).abs() < f32::EPSILON,
        "height should be 40.0 (2 x 20), got {}",
        result.height
    );
}

#[test]
fn display_none_skipped_in_block_context() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let hidden = dom.create_element("span", Attributes::default());
    let hidden_style = ComputedStyle {
        display: Display::None,
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);
    dom.append_child(parent, hidden);
    let hidden_text = dom.create_text("invisible");
    dom.append_child(hidden, hidden_text);

    let block = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(30.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block, block_style);
    dom.append_child(parent, block);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        (result.height - 30.0).abs() < f32::EPSILON,
        "height should be 30.0 (block only), got {}",
        result.height
    );
}

#[test]
fn atomic_inline_skipped_in_styled_runs() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);
    let ib_text = dom.create_text("IB");
    dom.append_child(ib, ib_text);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 2, "should have 2 text runs (Hello + World)");
    assert_eq!(runs[0].text, "Hello ");
    assert_eq!(runs[1].text, " World");
}
