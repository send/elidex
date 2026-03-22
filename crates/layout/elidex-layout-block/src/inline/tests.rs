use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{Dimension, Point, WritingMode};

/// Collect only text runs from inline items (for tests that don't need atomics).
fn collect_styled_runs(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> Vec<StyledRun> {
    collect_inline_items(dom, children, parent_style, parent_entity)
        .into_iter()
        .filter_map(|item| match item {
            InlineItem::Text(run) => Some(run),
            InlineItem::Atomic { .. } | InlineItem::Placeholder(_) => None,
        })
        .collect()
}

const TEST_FAMILIES: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];

/// Setup a DOM with a `<p>` parent and a text child, a default `ComputedStyle`
/// with test font families, and a `FontDatabase`. Returns `None` if no font is available.
fn setup_inline_test(text_content: &str) -> Option<(EcsDom, Entity, ComputedStyle, FontDatabase)> {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let text = dom.create_text(text_content);
    dom.append_child(parent, text);

    let style = ComputedStyle {
        font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
    };
    let font_db = FontDatabase::new();
    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: style.font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    measure_text(&font_db, &params, "x")?;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    Some((dom, parent, style, font_db))
}

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
    // Wide container: should still produce 2 lines due to \n
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
    // Use a very narrow width to force wrapping
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

// --- M3.5-4: Vertical writing mode ---

#[test]
fn vertical_mode_uses_font_size_line_advance() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    // In vertical mode, the block-axis advance per line is font_size, not line-height.
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
    // Single line: block dimension should be font_size.
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
    // Default writing_mode is HorizontalTb, no modification needed.

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

// --- Step 1: Multi-style inline layout ---

#[test]
fn styled_runs_collect_from_nested_span() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    // Remove the empty text child and build: <p>Hello <span>World</span></p>
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
    // Build: <p>A<span style="font-size:32px">B</span></p>
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
    // Line height should be max(parent line height, span line height) = big_line_height
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

// --- Step 2: Inline elements get LayoutBox ---

#[test]
fn inline_span_gets_layout_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    // Build: <p>Hello <span>World</span></p>
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

    // The span should now have a LayoutBox.
    let lb = dom.world().get::<&LayoutBox>(span);
    assert!(
        lb.is_ok(),
        "inline span should have a LayoutBox after layout"
    );
    let lb = lb.unwrap();
    // LayoutBox x should start after "Hello " at content_origin.x + offset.
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

    // Parent should NOT get a LayoutBox from inline layout.
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    };
    let _result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    // Parent (the <p>) should not get LayoutBox from inline layout
    // (it's the parent_entity, excluded from inline LayoutBox assignment).
    assert!(
        dom.world().get::<&LayoutBox>(parent).is_err(),
        "parent entity should not get LayoutBox from inline layout"
    );
}

// --- Step 3: Atomic inline boxes (InlineBlock) ---

#[test]
fn inline_block_participates_in_ifc() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    // Build: <p>Hello <span style="display:inline-block; width:50px; height:30px">X</span> World</p>
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

    // The inline-block should get a LayoutBox from dispatch.
    let ib_lb = dom.world().get::<&LayoutBox>(ib);
    assert!(ib_lb.is_ok(), "inline-block should have a LayoutBox");
    let ib_lb = ib_lb.unwrap();
    assert!(
        (ib_lb.content.size.width - 50.0).abs() < f32::EPSILON,
        "inline-block width should be 50px, got {}",
        ib_lb.content.size.width
    );

    // Line height should be at least 30px (the inline-block's height).
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

// --- Step 4: Anonymous block boxes (CSS 2.1 §9.2.1.1) ---

#[test]
fn mixed_block_inline_anonymous_box() {
    // <div>text <p style="display:block;height:40px">block</p> more text</div>
    // The text before and after the <p> should be wrapped in anonymous
    // block boxes and contribute height.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    // Insert parent's ComputedStyle so stack_block_children can look it up.
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

    // The block child has height 40. The text runs add line height.
    assert!(
        result.height >= 40.0,
        "height should be at least block child height (40), got {}",
        result.height
    );
    // With text content, anonymous boxes add inline layout height.
    let line_h = style.line_height.resolve_px(style.font_size);
    // Two anonymous blocks (before + after) + one block child = total.
    let expected_min = 40.0 + line_h; // at least one anonymous box contributes
    assert!(
        result.height >= expected_min,
        "height should include anonymous box height ({expected_min}), got {}",
        result.height
    );
}

#[test]
fn block_only_children_no_anonymous_boxes() {
    // All children are block-level: no anonymous block boxes created.
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

    // Two blocks at 20px each = 40px.
    assert!(
        (result.height - 40.0).abs() < f32::EPSILON,
        "height should be 40.0 (2 × 20), got {}",
        result.height
    );
}

#[test]
fn display_none_skipped_in_block_context() {
    // display:none children should not appear in inline runs.
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

    // Only the block child contributes height; hidden span skipped.
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

    // Build: <p>Hello <span style="display:inline-block">IB</span> World</p>
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
    // collect_styled_runs should NOT include the InlineBlock's text.
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 2, "should have 2 text runs (Hello + World)");
    assert_eq!(runs[0].text, "Hello ");
    assert_eq!(runs[1].text, " World");
}

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

    // Single inline-block child with explicit height, no preceding text.
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

    // Create a block child with height=40 that contains inline text.
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

    // Single empty block child with explicit height, no text children.
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

    // Build: text "Hello" + block child (height=40, no text) + text "World"
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

    // The first anonymous inline run ("Hello") provides the baseline
    // before the block child.
    assert!(
        result.first_baseline.is_some(),
        "first anonymous inline run should provide a baseline"
    );
    let bl = result.first_baseline.unwrap();
    assert!(bl > 0.0, "baseline should be > 0, got {bl}");
}
