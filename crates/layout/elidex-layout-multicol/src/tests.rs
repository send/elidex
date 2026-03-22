//! Tests for CSS Multi-column Layout.

#![allow(unused_must_use)]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{
    is_multicol, BoxSizing, ColumnFill, ColumnSpan, ComputedStyle, CssSize, Dimension, Display,
    EdgeSizes, Float, LayoutBox, MulticolInfo, Point, Position, Size, WritingMode,
};
use elidex_text::FontDatabase;

use crate::layout_multicol;
use elidex_layout_block::LayoutInput;

fn make_font_db() -> FontDatabase {
    FontDatabase::new()
}

fn make_input(font_db: &FontDatabase) -> LayoutInput<'_> {
    LayoutInput {
        containing: CssSize::definite(600.0, 800.0),
        containing_inline_size: 600.0,
        offset: Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(600.0, 800.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    }
}

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn add_block_child(dom: &mut EcsDom, parent: Entity, height: f32) -> Entity {
    let child = elem(dom, "div");
    dom.append_child(parent, child);
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(height),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(child, style);
    child
}

fn layout_child_fn(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    elidex_layout_block::block::layout_block_inner(dom, entity, input, layout_child_fn)
}

// --- is_multicol tests ---

#[test]
fn is_multicol_block_with_count() {
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(3),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}

#[test]
fn is_multicol_block_with_width() {
    let style = ComputedStyle {
        display: Display::Block,
        column_width: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}

#[test]
fn is_multicol_block_without_columns() {
    let style = ComputedStyle::default(); // display: block, no column props
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_flex_false() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_count: Some(3),
        ..ComputedStyle::default()
    };
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_grid_false() {
    let style = ComputedStyle {
        display: Display::Grid,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_inline_block_true() {
    let style = ComputedStyle {
        display: Display::InlineBlock,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}

// --- layout_multicol tests ---

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
fn column_span_all_splits_segments() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Block A
    add_block_child(&mut dom, container, 40.0);

    // Spanner
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    // Block B
    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // Total: segment1(40px balanced) + spanner(20px) + segment2(40px balanced)
    assert!(lb.content.size.height > 20.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Two normal segments (spanner excluded).
    assert_eq!(info.segments.len(), 2);
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
fn auto_vs_balance_fill() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With auto fill and definite height=200, container uses 200.
    assert_eq!(lb.content.size.height, 200.0);
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
fn spanner_first_child() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Spanner as first child
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(30.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert!(lb.content.size.height >= 30.0);
}

#[test]
fn spanner_last_child() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // Spanner as last child
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(30.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert!(lb.content.size.height >= 30.0);
}

#[test]
fn multiple_spanners() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 30.0);

    // Spanner 1
    let s1 = elem(&mut dom, "div");
    dom.append_child(container, s1);
    let sp1 = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(s1, sp1);

    add_block_child(&mut dom, container, 30.0);

    // Spanner 2
    let s2 = elem(&mut dom, "div");
    dom.append_child(container, s2);
    let sp2 = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(10.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(s2, sp2);

    add_block_child(&mut dom, container, 30.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Three normal segments (two spanners excluded).
    assert_eq!(info.segments.len(), 3);
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

// --- Overflow columns ---

#[test]
fn overflow_columns_beyond_count() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Three children that won't fit in 2 columns at 50px each
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With definite height + auto fill, container height = definite height
    assert_eq!(lb.content.size.height, 50.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Should have overflow columns (>2)
    assert!(!info.segments.is_empty());
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

// --- Spanner edge cases ---

#[test]
fn inline_spanner_not_treated_as_spanner() {
    // column-span: all on inline-level element should NOT create a spanner
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // Inline element with column-span: all — should be ignored
    let inline_span = elem(&mut dom, "span");
    dom.append_child(container, inline_span);
    let span_style = ComputedStyle {
        display: Display::Inline,
        column_span: ColumnSpan::All,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(inline_span, span_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Should have only 1 normal segment (no spanner split)
    assert_eq!(info.segments.len(), 1);
}

#[test]
fn floated_spanner_not_treated_as_spanner() {
    // column-span: all on floated element should NOT create a spanner
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    let float_span = elem(&mut dom, "div");
    dom.append_child(container, float_span);
    let fs_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        float: Float::Left,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(float_span, fs_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Floated element not treated as spanner → 1 segment
    assert_eq!(info.segments.len(), 1);
}

#[test]
fn display_none_excluded_from_segments() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // display:none child — should be excluded from segments entirely
    let hidden = elem(&mut dom, "div");
    dom.append_child(container, hidden);
    let hidden_style = ComputedStyle {
        display: Display::None,
        column_span: ColumnSpan::All,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // display:none with column-span:all should NOT split into segments
    assert_eq!(info.segments.len(), 1);
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
