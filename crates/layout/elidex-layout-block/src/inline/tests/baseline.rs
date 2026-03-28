use super::*;

// --- Baseline tracking (inline) ---

#[test]
fn inline_baseline_from_text() {
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
    };
    let result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        result.first_baseline.is_some(),
        "text content should produce a first_baseline"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "baseline should be > 0, got {bl}");
    assert!(
        bl < css_line_height,
        "baseline ({bl}) should be within (0, line_height={css_line_height})"
    );
}

#[test]
fn inline_baseline_with_atomic_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(50.0),
        height: Dimension::Length(30.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    };
    let result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        result.first_baseline.is_some(),
        "atomic inline box should produce a first_baseline (fallback = content.height)"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "baseline from atomic box should be > 0, got {bl}");
}

#[test]
fn empty_inline_no_baseline() {
    let mut dom = EcsDom::new();
    let parent_entity = Entity::DANGLING;
    let font_db = FontDatabase::new();

    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    };
    let result = layout_inline_context(&mut dom, &[], 800.0, parent_entity, Point::ZERO, &env);

    assert!(
        result.first_baseline.is_none(),
        "empty children should produce no baseline"
    );
}

#[test]
fn vertical_writing_mode_no_baseline() {
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
    };
    let result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        result.first_baseline.is_none(),
        "vertical writing mode should skip baseline capture, got {:?}",
        result.first_baseline
    );
}

// --- Baseline tracking (block propagation) ---

#[test]
fn block_inline_text_baseline() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello world") else {
        return;
    };
    let _ = dom.world_mut().insert_one(parent, style.clone());

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
        result.first_baseline.is_some(),
        "parent with inline text children should have a first_baseline"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "baseline should be > 0, got {bl}");
}

#[test]
fn nested_block_baseline_propagation() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let block_child = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(40.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block_child, block_style);
    dom.append_child(parent, block_child);
    let text = dom.create_text("Nested text");
    dom.append_child(block_child, text);

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
        result.first_baseline.is_some(),
        "nested block child's baseline should propagate to parent"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "propagated baseline should be > 0, got {bl}");
}

#[test]
fn block_without_baseline_none() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let block_child = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(20.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block_child, block_style);
    dom.append_child(parent, block_child);

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
        result.first_baseline.is_none(),
        "empty block child with no text should produce no baseline, got {:?}",
        result.first_baseline
    );
}

#[test]
fn mixed_block_inline_baseline() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let text1 = dom.create_text("Hello");
    dom.append_child(parent, text1);

    let block_child = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(40.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block_child, block_style);
    dom.append_child(parent, block_child);

    let text2 = dom.create_text("World");
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
        result.first_baseline.is_some(),
        "first anonymous inline run should provide a baseline"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "baseline should be > 0, got {bl}");
}
