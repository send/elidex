use super::*;

// --- Writing-mode-aware positioned layout tests ---

/// Regression: horizontal-tb absolute positioning is unchanged.
#[test]
fn horizontal_tb_absolute_regression() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::HorizontalTb,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 20.0));
    assert!(approx_eq(lb.content.origin.y, 10.0));
    assert!(approx_eq(lb.content.size.width, 100.0));
    assert!(approx_eq(lb.content.size.height, 50.0));
}

/// In vertical-rl mode, percentage margins/padding resolve against cb.height
/// (the containing block's inline size in vertical modes).
#[test]
fn vertical_rl_absolute_percentage_margin_resolves_against_cb_height() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, abs_child);

    // Root: 800x600, relative positioned
    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );

    // Abs child: vertical-rl, margin-left: 10% (should resolve against cb.height=600)
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            margin_left: Dimension::Percentage(10.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // In vertical-rl, inline_containing = cb.height = 600.
    // margin-left: 10% of 600 = 60.
    // x = cb.x(0) + left(0) + margin_left(60) + border(0) + padding(0) = 60
    assert!(approx_eq(lb.content.origin.x, 60.0));
    assert!(approx_eq(lb.content.origin.y, 0.0));
}

/// vertical-rl: absolute child with top/left offsets should position correctly.
#[test]
fn vertical_rl_absolute_basic_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 20.0));
    assert!(approx_eq(lb.content.origin.y, 10.0));
    assert!(approx_eq(lb.content.size.width, 100.0));
    assert!(approx_eq(lb.content.size.height, 50.0));
}

/// vertical-rl: auto margins on an axis should center the element.
#[test]
fn vertical_rl_absolute_auto_margins_center() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(50.0),
            margin_left: Dimension::Auto,
            margin_right: Dimension::Auto,
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Centered horizontally: (800 - 200) / 2 = 300 per auto margin.
    assert!(approx_eq(lb.content.size.width, 200.0));
    assert!(approx_eq(lb.margin.left, 300.0));
    assert!(approx_eq(lb.margin.right, 300.0));
}

/// vertical-rl: stretch with left+right and auto width.
#[test]
fn vertical_rl_absolute_stretch() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(10.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            height: Dimension::Length(40.0),
            // width: auto → stretch
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // width = cb(800) - left(50) - right(50) - h_pb(0) - margin(0) = 700
    assert!(approx_eq(lb.content.size.width, 700.0));
    assert!(approx_eq(lb.content.origin.x, 50.0));
}

/// vertical-lr: basic absolute positioning.
#[test]
fn vertical_lr_absolute_basic_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalLr,
            top: Dimension::Length(15.0),
            left: Dimension::Length(25.0),
            width: Dimension::Length(80.0),
            height: Dimension::Length(60.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 25.0));
    assert!(approx_eq(lb.content.origin.y, 15.0));
    assert!(approx_eq(lb.content.size.width, 80.0));
    assert!(approx_eq(lb.content.size.height, 60.0));
}

/// vertical-rl: percentage padding should resolve against cb.height (inline size).
#[test]
fn vertical_rl_absolute_padding_pct_resolves_against_inline_size() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, abs_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            padding: elidex_plugin::EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-rl: inline_containing = cb.height = 600. 10% of 600 = 60.
    assert!(
        approx_eq(lb.padding.top, 60.0),
        "padding-top should be 60 (10% of 600), got {}",
        lb.padding.top,
    );
}

/// vertical-rl: percentage height should resolve against cb.height.
#[test]
fn vertical_rl_absolute_percentage_height() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Percentage(50.0), // 50% of cb.height=600 = 300
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.size.height, 300.0),
        "height should be 300 (50% of 600), got {}",
        lb.content.size.height,
    );
}

/// `apply_relative_offset` works correctly in vertical-rl mode.
/// In vertical-rl: inline axis = top/bottom (top is inline-start for LTR),
/// block axis = left/right (right is block-start for rl).
#[test]
fn vertical_rl_relative_offset() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
        layout_generation: 0,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        top: Dimension::Length(10.0),  // inline-start offset → dy = +10
        left: Dimension::Length(20.0), // block-end offset (vertical-rl: block-start=right)
        ..Default::default()
    };
    // vertical-rl LTR:
    //   inline axis (top/bottom): top(inline-start) wins → dy = +10
    //   block axis (left/right): left only specified → dx = +20
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.origin.x, 120.0)); // 100 + 20
    assert!(approx_eq(lb.content.origin.y, 210.0)); // 200 + 10
}

/// vertical-rl: top+bottom stretch (auto height).
#[test]
fn vertical_rl_absolute_top_bottom_stretch_height() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(100.0),
            bottom: Dimension::Length(100.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            // height: auto → stretch between top and bottom
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // height = cb(600) - top(100) - bottom(100) - v_pb(0) - margins(0) = 400
    assert!(
        approx_eq(lb.content.size.height, 400.0),
        "height should be 400 (stretched), got {}",
        lb.content.size.height,
    );
}

/// vertical-rl: over-constrained with all three specified (left+width+right).
/// In vertical-rl: block axis = horizontal, block-start = right, block-end = left.
/// Over-constrained block axis: block-end (left) is ignored, block-start (right) wins.
#[test]
fn vertical_rl_absolute_over_constrained() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            width: Dimension::Length(600.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-rl block axis: block-start = right(50), block-end = left(50) (ignored).
    // x = cb_width(800) - right(50) - width(600) = 150
    assert!(
        approx_eq(lb.content.origin.x, 150.0),
        "expected x=150 (right wins in vertical-rl), got {}",
        lb.content.origin.x,
    );
    assert!(approx_eq(lb.content.size.width, 600.0));
}

/// Horizontal-tb relative offset regression.
#[test]
fn horizontal_tb_relative_offset_regression() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(0.0, 0.0, 100.0, 50.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
        layout_generation: 0,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::HorizontalTb,
        top: Dimension::Length(10.0),
        left: Dimension::Length(20.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.origin.x, 20.0));
    assert!(approx_eq(lb.content.origin.y, 10.0));
    assert!(approx_eq(lb.content.size.width, 100.0));
    assert!(approx_eq(lb.content.size.height, 50.0));
}

/// vertical-rl: min/max constraints on block-size (physical width).
#[test]
fn vertical_rl_absolute_min_max_block_size() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            // width (block-size): auto → stretch between right(0) and left(auto)
            // but min-width = 500 should clamp
            min_width: Dimension::Length(500.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.size.width, 500.0),
        "min-width should clamp to 500, got {}",
        lb.content.size.width,
    );
}

/// vertical-rl: min/max constraints on inline-size (physical height).
#[test]
fn vertical_rl_absolute_min_max_inline_size() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            max_height: Dimension::Length(30.0), // max-height clamps inline-size
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.size.height, 30.0),
        "max-height should clamp inline-size to 30, got {}",
        lb.content.size.height,
    );
}

/// RTL + vertical-rl: inline direction is reversed (inline-start = bottom).
/// bottom offset should be treated as inline-start.
#[test]
fn vertical_rl_rtl_absolute_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            direction: Direction::Rtl,
            // In vertical-rl RTL: inline-start = bottom, inline-end = top.
            // block-start = right, block-end = left.
            bottom: Dimension::Length(20.0), // inline-start offset
            right: Dimension::Length(30.0),  // block-start offset
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Physical y = cb.height(600) - bottom(20) - height(50) = 530
    assert!(
        approx_eq(lb.content.origin.y, 530.0),
        "vertical-rl RTL: y should be 530 (bottom=20 from bottom edge), got {}",
        lb.content.origin.y,
    );
    // Physical x: block-start = right(30). x = cb_width(800) - right(30) - width(100) = 670
    assert!(
        approx_eq(lb.content.origin.x, 670.0),
        "vertical-rl RTL: x should be 670 (right=30), got {}",
        lb.content.origin.x,
    );
}

/// vertical-rl: auto margin centering on inline axis (top/bottom).
#[test]
fn vertical_rl_absolute_auto_margins_center_inline_axis() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(200.0),
            margin_top: Dimension::Auto,
            margin_bottom: Dimension::Auto,
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Inline axis centering: (600 - 200) / 2 = 200 per auto margin.
    // In vertical-rl: inline-axis margins are physical top/bottom.
    // Negative centering absorbs into inline-end (bottom in LTR).
    assert!(
        approx_eq(lb.content.size.height, 200.0),
        "height should be 200, got {}",
        lb.content.size.height,
    );
    assert!(
        approx_eq(lb.margin.top, 200.0),
        "margin-top should be 200 (centered), got {}",
        lb.margin.top,
    );
    assert!(
        approx_eq(lb.margin.bottom, 200.0),
        "margin-bottom should be 200 (centered), got {}",
        lb.margin.bottom,
    );
}

/// vertical-lr: block-start = left (not reversed). left+right+width over-constrained
/// should ignore block-end (right).
#[test]
fn vertical_lr_absolute_over_constrained() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalLr,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            width: Dimension::Length(600.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-lr: block-start = left(50), block-end = right(50) (ignored).
    // x = left(50)
    assert!(
        approx_eq(lb.content.origin.x, 50.0),
        "vertical-lr over-constrained: x should be 50 (left wins), got {}",
        lb.content.origin.x,
    );
    assert!(approx_eq(lb.content.size.width, 600.0));
}

/// vertical-rl: relative offset with both top+bottom specified.
/// In vertical-rl LTR: inline axis = top/bottom, top (inline-start) wins.
#[test]
fn vertical_rl_relative_offset_both_top_bottom() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
        layout_generation: 0,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        top: Dimension::Length(10.0),
        bottom: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-rl LTR: top (inline-start) wins over bottom (inline-end).
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.origin.y, 210.0), // 200 + 10 (top wins)
        "top (inline-start) should win, got y={}",
        lb.content.origin.y,
    );
}

/// vertical-rl: relative offset with both left+right specified.
/// In vertical-rl: block axis = left/right, block-start = right. Right wins.
#[test]
fn vertical_rl_relative_offset_both_left_right() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
        layout_generation: 0,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        left: Dimension::Length(10.0),
        right: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-rl: block-start = right. When both specified, block-start (right) wins.
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.origin.x, 80.0), // 100 + (-20) = 80, right wins
        "right (block-start) should win in vertical-rl, got x={}",
        lb.content.origin.x,
    );
}

/// vertical-lr: relative offset with both left+right specified.
/// In vertical-lr: block-start = left. Left wins.
#[test]
fn vertical_lr_relative_offset_both_left_right() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
        layout_generation: 0,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalLr,
        left: Dimension::Length(10.0),
        right: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-lr: block-start = left. Left wins.
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.origin.x, 110.0), // 100 + 10 (left wins)
        "left (block-start) should win in vertical-lr, got x={}",
        lb.content.origin.x,
    );
}

/// Fixed positioning in vertical-rl should work correctly.
#[test]
fn vertical_rl_fixed_positioning() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            right: Dimension::Length(10.0), // block-start
            top: Dimension::Length(20.0),   // inline-start
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    // Use viewport 1024x768.
    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(1024.0),
        containing_inline_size: 1024.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(1024.0, 768.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // vertical-rl: block-start = right(10). x = viewport_w(1024) - right(10) - width(100) = 914
    assert!(
        approx_eq(lb.content.origin.x, 914.0),
        "fixed vertical-rl: x should be 914, got {}",
        lb.content.origin.x,
    );
    // inline-start = top(20). y = 20
    assert!(
        approx_eq(lb.content.origin.y, 20.0),
        "fixed vertical-rl: y should be 20, got {}",
        lb.content.origin.y,
    );
}

/// vertical-rl: percentage width (block-size) resolves against cb.width.
#[test]
fn vertical_rl_absolute_percentage_width() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Percentage(50.0), // 50% of cb.width=800 = 400
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.size.width, 400.0),
        "width should be 400 (50% of 800), got {}",
        lb.content.size.width,
    );
}
