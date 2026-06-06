//! Basic column layout: geometry, width, gap, and child positioning.

use super::*;

#[test]
fn basic_two_columns() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // Container width = 600
    assert_eq!(lb.content.size.width, 600.0);
    // Height should be balanced: total 100 / 2 cols = 50
    assert!(lb.content.size.height <= 100.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    assert_eq!(info.column_gap, 0.0);
}

#[test]
fn basic_three_columns() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(3),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    for _ in 0..6 {
        add_block_child(&mut dom, container, 30.0);
    }

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.width, 600.0);
    // 6 children × 30px = 180 total, balanced across 3 = 60px height
    assert!(lb.content.size.height <= 180.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    assert!(!info.segments.is_empty());
}

#[test]
fn empty_container() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.height, 0.0);
}

#[test]
fn padding_border_on_container() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        padding: EdgeSizes {
            top: Dimension::Length(10.0),
            right: Dimension::Length(10.0),
            bottom: Dimension::Length(10.0),
            left: Dimension::Length(10.0),
        },
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.padding.top, 10.0);
    assert_eq!(lb.padding.left, 10.0);
    // Content width = 600 - 20 (padding) = 580
    assert_eq!(lb.content.size.width, 580.0);
}

#[test]
fn column_count_one_degenerate() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(1),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 100.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.width, 600.0);
    // Single column: height = content height
    assert!(lb.content.size.height >= 100.0);
}

#[test]
fn float_context_resets_per_column() {
    // Float in column 1 should not affect column 2.
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        height: Dimension::Length(100.0),
        column_fill: ColumnFill::Auto,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Floated child in first column
    let float_child = elem(&mut dom, "div");
    dom.append_child(container, float_child);
    let float_style = ComputedStyle {
        display: Display::Block,
        float: Float::Left,
        width: Dimension::Length(50.0),
        height: Dimension::Length(50.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(float_child, float_style);

    // More content
    add_block_child(&mut dom, container, 80.0);
    add_block_child(&mut dom, container, 80.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    // Should not panic or produce invalid layout.
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.height, 100.0);
}

#[test]
fn nested_content() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Nested block
    let outer = elem(&mut dom, "div");
    dom.append_child(container, outer);
    let outer_style = ComputedStyle {
        display: Display::Block,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(outer, outer_style);

    // Inner children
    let inner = elem(&mut dom, "div");
    dom.append_child(outer, inner);
    let inner_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(60.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(inner, inner_style);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert!(lb.content.size.height > 0.0);
}

#[test]
fn vertical_rl_columns() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        writing_mode: WritingMode::VerticalRl,
        width: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let mut input = make_input(&font_db);
    input.containing.height = Some(600.0);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let _lb = dom.world().get::<&LayoutBox>(container).unwrap();
    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    assert_eq!(info.writing_mode, WritingMode::VerticalRl);
}

#[test]
fn vertical_lr_columns() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        writing_mode: WritingMode::VerticalLr,
        width: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let mut input = make_input(&font_db);
    input.containing.height = Some(600.0);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    assert_eq!(info.writing_mode, WritingMode::VerticalLr);
}

// --- Explicit width ---
#[test]
fn explicit_width_content_box() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        width: Dimension::Length(400.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.width, 400.0);
}

#[test]
fn explicit_width_border_box() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        width: Dimension::Length(400.0),
        box_sizing: BoxSizing::BorderBox,
        padding: EdgeSizes {
            top: Dimension::Length(0.0),
            right: Dimension::Length(20.0),
            bottom: Dimension::Length(0.0),
            left: Dimension::Length(20.0),
        },
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // border-box: 400 - 20 - 20 (padding) = 360 content width
    assert_eq!(lb.content.size.width, 360.0);
}

#[test]
fn min_max_inline_size() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        max_width: Dimension::Length(300.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.width, 300.0);
}

// --- Positioned children ---
#[test]
fn absolute_children_excluded_from_columns() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        position: Position::Relative,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Normal child
    add_block_child(&mut dom, container, 50.0);

    // Absolutely positioned child — should not participate in column layout
    let abs_child = elem(&mut dom, "div");
    dom.append_child(container, abs_child);
    let abs_style = ComputedStyle {
        display: Display::Block,
        position: Position::Absolute,
        height: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(abs_child, abs_style);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // Container height should be based on the in-flow child (50px), not the absolute child (200px)
    assert!(lb.content.size.height <= 50.0);
}

// --- column-gap ---
#[test]
fn column_gap_affects_geometry() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_gap: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    assert_eq!(info.column_gap, 20.0);
    // Column width = (600 - 20) / 2 = 290
    assert_eq!(info.column_width, 290.0);
}

// --- column-width only ---
#[test]
fn column_width_only_auto_count() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_width: Dimension::Length(150.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    for _ in 0..4 {
        add_block_child(&mut dom, container, 30.0);
    }

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert_eq!(lb.content.size.width, 600.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // 600 / 150 = 4 columns max, actual width may differ
    assert!(info.column_width >= 150.0);
}

// --- Child position verification ---
#[test]
fn children_positioned_at_correct_column_offsets() {
    // Verify that children in column 1+ are at exactly 1× inline offset,
    // not 2× (regression test for double-shift bug).
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(100.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Two children: each 100px tall → col1 gets child_a, col2 gets child_b.
    let child_a = add_block_child(&mut dom, container, 100.0);
    let child_b = add_block_child(&mut dom, container, 100.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb_a = dom.world().get::<&LayoutBox>(child_a).unwrap();
    let lb_b = dom.world().get::<&LayoutBox>(child_b).unwrap();

    // Column width = 600 / 2 = 300, gap = 0.
    // Child A in column 0: x = 0
    // Child B in column 1: x = 300 (NOT 600)
    assert!(
        lb_a.content.origin.x < 1.0,
        "child_a should be near x=0, got {}",
        lb_a.content.origin.x
    );
    assert!(
        (lb_b.content.origin.x - 300.0).abs() < 1.0,
        "child_b should be near x=300, got {}",
        lb_b.content.origin.x
    );
}

#[test]
fn children_positioned_with_gap() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(3),
        column_gap: Dimension::Length(30.0),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(100.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    let child_a = add_block_child(&mut dom, container, 100.0);
    let child_b = add_block_child(&mut dom, container, 100.0);
    let child_c = add_block_child(&mut dom, container, 100.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb_a = dom.world().get::<&LayoutBox>(child_a).unwrap();
    let lb_b = dom.world().get::<&LayoutBox>(child_b).unwrap();
    let lb_c = dom.world().get::<&LayoutBox>(child_c).unwrap();

    // Column width = (600 - 2*30) / 3 = 180, gap = 30.
    // Col 0: x = 0, Col 1: x = 210, Col 2: x = 420
    assert!(
        lb_a.content.origin.x < 1.0,
        "child_a x={}, expected ~0",
        lb_a.content.origin.x
    );
    assert!(
        (lb_b.content.origin.x - 210.0).abs() < 1.0,
        "child_b x={}, expected ~210",
        lb_b.content.origin.x
    );
    assert!(
        (lb_c.content.origin.x - 420.0).abs() < 1.0,
        "child_c x={}, expected ~420",
        lb_c.content.origin.x
    );
}
