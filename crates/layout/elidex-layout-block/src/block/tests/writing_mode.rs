//! Writing mode tests for block layout (Steps 4-5 of G7 plan).

use super::*;
use elidex_plugin::WritingMode;

/// Helper: create a dom with a parent div using the given writing mode and
/// two block children of specified dimensions.
fn make_vertical_dom(
    wm: WritingMode,
    child_widths: &[f32],
    child_heights: &[f32],
) -> (EcsDom, Entity, Vec<Entity>) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: wm,
            ..Default::default()
        },
    );
    let mut children = Vec::new();
    for (w, h) in child_widths.iter().zip(child_heights.iter()) {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                writing_mode: wm, // Inherited from parent
                width: Dimension::Length(*w),
                height: Dimension::Length(*h),
                ..Default::default()
            },
        );
        children.push(child);
    }
    (dom, parent, children)
}

// -----------------------------------------------------------------------
// Step 4 regression: horizontal-tb uses width as inline containing size
// -----------------------------------------------------------------------

#[test]
fn horizontal_tb_containing_inline_size_equals_width() {
    // In horizontal-tb, containing_inline_size == containing_width.
    // Padding % should resolve against width.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        padding: EdgeSizes {
            top: Dimension::Percentage(10.0), // 10% of inline size
            right: Dimension::ZERO,
            bottom: Dimension::ZERO,
            left: Dimension::ZERO,
        },
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, Point::ZERO, &font_db);
    // In horizontal-tb, containing_inline_size = containing_width = 800.
    // padding-top = 10% of 800 = 80.
    assert!((lb.padding.top - 80.0).abs() < f32::EPSILON);
}

#[test]
fn horizontal_tb_margin_collapse_still_works() {
    // Verify that margin collapse still functions correctly in horizontal-tb
    // after switching to containing_inline_size.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let child1 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            margin_bottom: Dimension::Length(20.0),
            ..Default::default()
        },
    );

    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            margin_top: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Collapsed margin = max(20, 30) = 30. Total = 50 + 30 + 50 = 130.
    assert!((parent_box.content.size.height - 130.0).abs() < f32::EPSILON);
}

#[test]
fn horizontal_tb_basic_block_children() {
    // Basic regression: two fixed-size children stack vertically.
    let (mut dom, parent, children) =
        make_vertical_dom(WritingMode::HorizontalTb, &[100.0, 100.0], &[50.0, 60.0]);
    let font_db = FontDatabase::new();

    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);
    // Total height = 50 + 60 = 110
    assert!((parent_box.content.size.height - 110.0).abs() < f32::EPSILON);

    let child1_box = dom.world().get::<&LayoutBox>(children[0]).unwrap();
    assert!((child1_box.content.origin.y - 0.0).abs() < f32::EPSILON);

    let child2_box = dom.world().get::<&LayoutBox>(children[1]).unwrap();
    assert!((child2_box.content.origin.y - 50.0).abs() < f32::EPSILON);
}

// -----------------------------------------------------------------------
// Step 5: Block layout in vertical writing modes
// -----------------------------------------------------------------------

#[test]
fn vertical_rl_children_stack_in_block_direction() {
    // In vertical-rl, block direction is horizontal (right to left).
    // Children stack vertically in physical terms because the current
    // implementation stacks on cursor_y. The key verification is that
    // containing_inline_size is propagated correctly.
    let (mut dom, parent, children) =
        make_vertical_dom(WritingMode::VerticalRl, &[100.0, 100.0], &[50.0, 60.0]);
    let font_db = FontDatabase::new();

    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);
    // In vertical-rl, block direction is X (right to left).
    // Children have width=100 (block-size) each, stacking horizontally.
    // Parent: phys_width = block_size = content-determined = 100 + 100 = 200.
    // Parent: phys_height = inline-size = fills available = 800.
    assert!(
        (parent_box.content.size.width - 200.0).abs() < f32::EPSILON,
        "expected width=200 (block-size), got {}",
        parent_box.content.size.width
    );
    assert!(
        (parent_box.content.size.height - 800.0).abs() < f32::EPSILON,
        "expected height=800 (inline-size), got {}",
        parent_box.content.size.height
    );

    // Child 1 at block-start (x=0), child 2 at x=100 (after child 1's block-size).
    let c1 = dom.world().get::<&LayoutBox>(children[0]).unwrap();
    assert!(
        (c1.content.origin.x - 0.0).abs() < f32::EPSILON,
        "child1 x={}",
        c1.content.origin.x
    );
    let c2 = dom.world().get::<&LayoutBox>(children[1]).unwrap();
    assert!(
        (c2.content.origin.x - 100.0).abs() < f32::EPSILON,
        "child2 x should be 100, got {}",
        c2.content.origin.x
    );
}

#[test]
fn vertical_lr_children_stack_in_block_direction() {
    // vertical-lr: block direction is left-to-right.
    // Children stack horizontally (X axis), same as vertical-rl but left-to-right.
    let (mut dom, parent, _) =
        make_vertical_dom(WritingMode::VerticalLr, &[100.0, 100.0], &[50.0, 60.0]);
    let font_db = FontDatabase::new();

    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);
    // phys_width = block-size = 100 + 100 = 200
    // phys_height = inline-size = 800
    assert!(
        (parent_box.content.size.width - 200.0).abs() < f32::EPSILON,
        "expected width=200, got {}",
        parent_box.content.size.width
    );
    assert!(
        (parent_box.content.size.height - 800.0).abs() < f32::EPSILON,
        "expected height=800, got {}",
        parent_box.content.size.height
    );
}

#[test]
fn vertical_rl_auto_sizes() {
    // In vertical-rl:
    // - auto height (inline-size) fills available inline space
    // - auto width (block-size) = content-determined (0 with no children)
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 600.0, Point::ZERO, &font_db);
    // Auto inline-size (height) fills available = 600 (no containing_height, falls back)
    assert!(
        (lb.content.size.height - 600.0).abs() < f32::EPSILON,
        "expected height=600 (inline-size fills available), got {}",
        lb.content.size.height
    );
    // Auto block-size (width) = content-determined = 0 (no children)
    assert!(
        (lb.content.size.width - 0.0).abs() < f32::EPSILON,
        "expected width=0 (block-size, no children), got {}",
        lb.content.size.width
    );
}

#[test]
fn vertical_rl_padding_pct_resolves_against_inline_size() {
    // In vertical-rl, padding % should resolve against inline size
    // (physical height, i.e. containing_inline_size).
    // When containing_inline_size is set from the parent's writing mode,
    // the child's padding % resolves against it.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    // Parent: containing_width=800, containing_height=Some(600).
    // Parent is vertical-rl, auto sizes:
    //   available_inline = containing_height = 600
    //   content_inline = 600 (auto inline-size fills available)
    //   child_phys_height = Some(content_inline) = Some(600)
    //   child containing_inline_size = compute_inline_containing(VRL, 800, Some(600)) = 600
    // Child padding-top = 10% of 600 = 60.
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    assert!(
        (child_box.padding.top - 60.0).abs() < f32::EPSILON,
        "expected padding.top=60.0, got {}",
        child_box.padding.top
    );
}

#[test]
fn vertical_rl_with_definite_parent_height_inline_size() {
    // When a vertical parent has a definite height, child's containing_inline_size
    // should be the parent's definite height (since inline axis = vertical).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(500.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(100.0),
            height: Dimension::Length(100.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0), // should be 10% of inline size
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // Parent: vertical-rl, content_width=800, child_containing_height = Some(500)
    // (from resolve_explicit_height: parent has height=500px).
    // child containing_inline_size = compute_inline_containing(VerticalRl, 800, Some(500)) = 500
    // child padding-top = 10% of 500 = 50
    assert!(
        (child_box.padding.top - 50.0).abs() < f32::EPSILON,
        "expected padding.top=50.0, got {}",
        child_box.padding.top
    );
}

#[test]
fn vertical_lr_padding_pct_resolves_against_inline_size() {
    // Same as vertical-rl test but with vertical-lr.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalLr,
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalLr,
            width: Dimension::Length(100.0),
            height: Dimension::Length(100.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(20.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // Parent: vertical-lr, height=400 => child_containing_height = Some(400)
    // child containing_inline_size = compute_inline_containing(VerticalLr, 800, Some(400)) = 400
    // child padding-top = 20% of 400 = 80
    assert!(
        (child_box.padding.top - 80.0).abs() < f32::EPSILON,
        "expected padding.top=80.0, got {}",
        child_box.padding.top
    );
}

#[test]
fn vertical_margin_pct_resolves_against_inline_size() {
    // Margin % should also resolve against inline size in vertical modes.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(500.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            margin_top: Dimension::Percentage(10.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // Parent: vertical-rl, height=500 => child_containing_height = Some(500)
    // child containing_inline_size = 500
    // child margin-top = 10% of 500 = 50
    assert!(
        (child_box.margin.top - 50.0).abs() < f32::EPSILON,
        "expected margin.top=50.0, got {}",
        child_box.margin.top
    );
}

#[test]
fn horizontal_tb_padding_pct_resolves_against_width() {
    // Regression: horizontal-tb padding % still resolves against width.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::HorizontalTb,
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block(&mut dom, parent, 600.0, Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // horizontal-tb: containing_inline_size = containing_width = 600
    // child padding-top = 10% of 600 = 60
    assert!(
        (child_box.padding.top - 60.0).abs() < f32::EPSILON,
        "expected padding.top=60.0, got {}",
        child_box.padding.top
    );
}

#[test]
fn vertical_mode_child_inherits_writing_mode_for_inline_size() {
    // When parent is vertical-rl and child is also vertical-rl (inherited),
    // the child's containing_inline_size should be set from the parent's
    // writing mode (compute_inline_containing uses parent's wm).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(300.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(child, grandchild);
    dom.world_mut().insert_one(
        grandchild,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(50.0),
            height: Dimension::Length(30.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let grandchild_box = dom.world().get::<&LayoutBox>(grandchild).unwrap();
    // Parent: vertical-rl, height=300 => child_containing_height = Some(300)
    // Child's containing_inline_size = compute_inline_containing(VerticalRl, 800, Some(300)) = 300
    // But child itself has height=100 and is vertical-rl.
    // child_containing_height for grandchild = resolve_explicit_height(child_style, Some(300)) = Some(100)
    // grandchild containing_inline_size = compute_inline_containing(VerticalRl, 200, Some(100)) = 100
    // grandchild padding-top = 10% of 100 = 10
    assert!(
        (grandchild_box.padding.top - 10.0).abs() < f32::EPSILON,
        "expected padding.top=10.0, got {}",
        grandchild_box.padding.top
    );
}

#[test]
fn mixed_writing_mode_horizontal_parent_vertical_child() {
    // horizontal-tb parent with vertical-rl child.
    // The parent computes child's containing_inline_size using the PARENT's
    // writing mode (horizontal-tb), so it equals containing_width.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::HorizontalTb,
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            padding: EdgeSizes {
                top: Dimension::Percentage(5.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block(&mut dom, parent, 1000.0, Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // Parent is horizontal-tb, so child's containing_inline_size =
    // compute_inline_containing(HorizontalTb, 1000, None) = 1000.
    // But wait - the parent itself resolves its own padding/margin against
    // its containing_inline_size. The child's containing_inline_size is set
    // by the parent when building child_input.
    // Parent is htb => containing_inline_size for child =
    // compute_inline_containing(HorizontalTb, content_width, child_containing_height)
    // = content_width = 1000
    // child padding-top = 5% of 1000 = 50
    assert!(
        (child_box.padding.top - 50.0).abs() < f32::EPSILON,
        "expected padding.top=50.0, got {}",
        child_box.padding.top
    );
}

#[test]
fn float_in_vertical_mode_uses_inline_size_for_margins() {
    // Floated child in a vertical-rl parent should use containing_inline_size
    // for margin % resolution.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(500.0),
            ..Default::default()
        },
    );

    let floated = dom.create_element("div", Attributes::default());
    dom.append_child(parent, floated);
    dom.world_mut().insert_one(
        floated,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            float: Float::Left,
            width: Dimension::Length(100.0),
            height: Dimension::Length(80.0),
            margin_left: Dimension::Percentage(10.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Parent: vertical-rl, height=500 => child_containing_height = Some(500)
    // child containing_inline_size = compute_inline_containing(VerticalRl, 800, Some(500)) = 500
    // float margin-left = 10% of 500 = 50
    assert!(
        (float_box.margin.left - 50.0).abs() < f32::EPSILON,
        "expected margin.left=50.0, got {}",
        float_box.margin.left
    );
}

#[test]
fn block_pb_and_inline_pb_used_correctly() {
    // Verify that the inline_pb and block_pb helpers produce correct results
    // for a vertical-rl context with asymmetric padding/border.
    use elidex_plugin::{Direction, WritingModeContext};

    let wm_h = WritingModeContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let wm_v = WritingModeContext::new(WritingMode::VerticalRl, Direction::Ltr);

    let padding = EdgeSizes::new(5.0, 10.0, 15.0, 20.0);
    let border = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);

    // horizontal: inline_pb = left+right = (20+10)+(4+2) = 36
    assert_eq!(crate::inline_pb(&wm_h, &padding, &border), 36.0);
    // horizontal: block_pb = top+bottom = (5+15)+(1+3) = 24
    assert_eq!(crate::block_pb(&wm_h, &padding, &border), 24.0);

    // vertical: inline_pb = top+bottom = (5+15)+(1+3) = 24
    assert_eq!(crate::inline_pb(&wm_v, &padding, &border), 24.0);
    // vertical: block_pb = left+right = (20+10)+(4+2) = 36
    assert_eq!(crate::block_pb(&wm_v, &padding, &border), 36.0);
}

#[test]
fn vertical_rl_margin_collapse_between_siblings() {
    // Margin collapse operates on block-axis margins.
    // In vertical-rl: block-start = right, block-end = left.
    // So we use margin_left (block-end) on child1 and margin_right (block-start) on child2.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    let child1 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(40.0),
            width: Dimension::Length(100.0),
            margin_left: Dimension::Length(15.0), // block-end margin (VRL)
            ..Default::default()
        },
    );

    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(40.0),
            width: Dimension::Length(100.0),
            margin_right: Dimension::Length(25.0), // block-start margin (VRL)
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Collapsed block-axis margin = max(15, 25) = 25.
    // Total block extent (width) = 100 + 25 + 100 = 225.
    assert!(
        (parent_box.content.size.width - 225.0).abs() < f32::EPSILON,
        "expected width=225.0, got {}",
        parent_box.content.size.width
    );
}

#[test]
fn sideways_rl_treated_as_vertical() {
    // SidewaysRl should behave like a vertical mode for inline size computation.
    let inline_size = crate::compute_inline_containing(WritingMode::SidewaysRl, 800.0, Some(500.0));
    assert_eq!(inline_size, 500.0);
}

#[test]
fn vertical_rl_min_max_block_size() {
    // In vertical-rl: block-size = physical width.
    // min-width and max-width constrain the block-size.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(500.0), // block-size = 500
            height: Dimension::Length(100.0),
            min_width: Dimension::Length(200.0), // min block-size
            max_width: Dimension::Length(300.0), // max block-size — clamps to 300
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // block-size requested 500, clamped by max-width=300.
    assert!(
        (child_box.content.size.width - 300.0).abs() < f32::EPSILON,
        "expected width=300 (max block-size), got {}",
        child_box.content.size.width
    );
}

#[test]
fn vertical_rl_min_inline_size() {
    // In vertical-rl: inline-size = physical height.
    // min-height constrains the inline-size.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),      // inline-size = 50
            min_height: Dimension::Length(200.0), // min inline-size — expands to 200
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let child_box = dom.world().get::<&LayoutBox>(child).unwrap();
    // inline-size requested 50, expanded by min-height=200.
    assert!(
        (child_box.content.size.height - 200.0).abs() < f32::EPSILON,
        "expected height=200 (min inline-size), got {}",
        child_box.content.size.height
    );
}

#[test]
fn vertical_rl_float_clearance() {
    // Float + clear in vertical-rl mode.
    // Currently, FloatContext operates on physical axes only (inline=X, block=Y),
    // so float:left in vertical-rl still places on the physical left, and clear:left
    // clears on the physical Y axis. Full writing-mode-aware float placement
    // (Step 5d: FloatContext logical coordinate system) is a larger refactor.
    //
    // This test verifies that float + clear at least produce non-overlapping layout
    // in vertical mode (the cleared element is placed after the float in block direction).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            height: Dimension::Length(500.0),
            ..Default::default()
        },
    );

    let floated = dom.create_element("div", Attributes::default());
    dom.append_child(parent, floated);
    dom.world_mut().insert_one(
        floated,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            float: Float::Left,
            width: Dimension::Length(100.0),
            height: Dimension::Length(80.0),
            ..Default::default()
        },
    );

    let cleared = dom.create_element("div", Attributes::default());
    dom.append_child(parent, cleared);
    dom.world_mut().insert_one(
        cleared,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            clear: Clear::Left,
            width: Dimension::Length(50.0),
            height: Dimension::Length(40.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    let cleared_box = dom.world().get::<&LayoutBox>(cleared).unwrap();

    // Float should be placed and have non-zero dimensions.
    assert!(
        float_box.content.size.width > 0.0 && float_box.content.size.height > 0.0,
        "float should have non-zero size: {}x{}",
        float_box.content.size.width,
        float_box.content.size.height,
    );
    // Cleared element should not overlap with float on block axis (X in VRL).
    // Since FloatContext is physical, clear:left works on physical Y, and the
    // cleared element is placed after the float in block direction (X).
    assert!(
        cleared_box.content.origin.x != float_box.content.origin.x
            || cleared_box.content.origin.y >= float_box.content.bottom() - 0.5,
        "cleared element should not overlap float: float=({},{}) cleared=({},{})",
        float_box.content.origin.x,
        float_box.content.origin.y,
        cleared_box.content.origin.x,
        cleared_box.content.origin.y,
    );
}
