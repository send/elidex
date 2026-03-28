use super::*;

#[test]
fn empty_text_zero_height() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let text = dom.create_text("");
    dom.append_child(parent, text);

    let font_db = FontDatabase::new();
    let children = dom.composed_children(parent);

    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(h.abs() < f32::EPSILON);
}

#[test]
fn no_children_zero_height() {
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
    let h = layout_inline_context(&mut dom, &[], 800.0, parent_entity, Point::ZERO, &env).height;
    assert!(h.abs() < f32::EPSILON);
}

#[test]
fn single_line_text() {
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
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!((h - css_line_height).abs() < f32::EPSILON);
}

#[test]
fn mandatory_newline_break() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("line1\nline2") else {
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
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!((h - css_line_height * 2.0).abs() < f32::EPSILON);
}

#[test]
fn text_wrapping_increases_height() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("hello world foo bar baz")
    else {
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
    let h = layout_inline_context(&mut dom, &children, 1.0, parent, Point::ZERO, &env).height;
    assert!(h > css_line_height);
}

// --- Vertical writing mode ---

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
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height).abs() < f32::EPSILON,
        "horizontal-tb single line should be line-height ({css_line_height}), got {h}",
    );
}

// --- Multi-style inline layout ---

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

// --- Inline elements get LayoutBox ---

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
    };
    let _result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        dom.world().get::<&LayoutBox>(parent).is_err(),
        "parent entity should not get LayoutBox from inline layout"
    );
}

// --- Atomic inline boxes (InlineBlock) ---

#[test]
fn inline_block_participates_in_ifc() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
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
        width: Dimension::Length(50.0),
        height: Dimension::Length(30.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);
    let ib_text = dom.create_text("X");
    dom.append_child(ib, ib_text);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;

    let ib_lb = dom.world().get::<&LayoutBox>(ib);
    assert!(ib_lb.is_ok(), "inline-block should have a LayoutBox");
    let ib_lb = ib_lb.unwrap();
    assert!(
        (ib_lb.content.size.width - 50.0).abs() < f32::EPSILON,
        "inline-block width should be 50px, got {}",
        ib_lb.content.size.width
    );

    assert!(
        h >= 30.0,
        "line height should be >= inline-block height (30px), got {h}"
    );
}

#[test]
fn inline_block_not_block_level() {
    assert!(
        !crate::block::is_block_level(Display::InlineBlock),
        "InlineBlock should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineFlex),
        "InlineFlex should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineGrid),
        "InlineGrid should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineTable),
        "InlineTable should not be block-level"
    );
}

// --- Anonymous block boxes (CSS 2.1 §9.2.1.1) ---

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
