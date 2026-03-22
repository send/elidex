//! Writing-mode-aware grid layout tests.

use elidex_ecs::Attributes;
use elidex_layout_block::LayoutInput;
use elidex_plugin::{
    Dimension, Display, EdgeSizes, GridTrackList, Point, TrackSection, TrackSize, WritingMode,
};

use super::*;

#[test]
fn horizontal_tb_regression() {
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::HorizontalTb,
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(200.0),
            ])),
            ..Default::default()
        },
    );
    let child = make_grid_child(&mut dom, grid, 30.0);

    let font_db = elidex_text::FontDatabase::new();
    let _lb = do_layout_grid(
        &mut dom,
        grid,
        600.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let child_lb = get_layout(&dom, child);
    assert!(
        child_lb.content.size.width <= 200.0 + 0.5,
        "child width {} should be <= 200",
        child_lb.content.size.width,
    );
}

#[test]
fn vertical_rl_grid_child_layout() {
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalRl,
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(200.0),
            ])),
            ..Default::default()
        },
    );
    let child = make_grid_child(&mut dom, grid, 30.0);

    let font_db = elidex_text::FontDatabase::new();
    let _lb = do_layout_grid(
        &mut dom,
        grid,
        600.0,
        Some(400.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let child_lb = get_layout(&dom, child);
    assert!(
        child_lb.content.size.width > 0.0 || child_lb.content.size.height > 0.0,
        "child should have non-zero size",
    );
}

/// In vertical-rl writing mode, percentage padding/margin should resolve
/// against the containing block's inline size (physical height), not width.
#[test]
fn vertical_rl_padding_percent_resolves_against_inline_size() {
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalRl,
            // 10% padding on all sides
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::Percentage(10.0),
                bottom: Dimension::Percentage(10.0),
                left: Dimension::Percentage(10.0),
            },
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(100.0),
            ])),
            ..Default::default()
        },
    );
    let _child = make_grid_child(&mut dom, grid, 30.0);

    let font_db = elidex_text::FontDatabase::new();
    // containing_width = 800, containing_height = 400
    // For vertical-rl, containing_inline_size = 400 (the physical height).
    let containing_inline_size = 400.0;
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 400.0),
        containing_inline_size,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let outcome = crate::layout_grid(&mut dom, grid, &input, layout_block_only);

    // 10% of 400 = 40 per side.
    // If the bug existed (resolving against containing_width=800), padding would be 80.
    assert!(
        approx_eq(outcome.layout_box.padding.top, 40.0),
        "padding-top should be 40 (10% of inline size 400), got {}",
        outcome.layout_box.padding.top,
    );
    assert!(
        approx_eq(outcome.layout_box.padding.left, 40.0),
        "padding-left should be 40 (10% of inline size 400), got {}",
        outcome.layout_box.padding.left,
    );
}

#[test]
fn vertical_lr_grid_child_layout() {
    // vertical-lr: same as vertical-rl but block direction is left-to-right.
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalLr,
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(150.0),
            ])),
            ..Default::default()
        },
    );
    let child = make_grid_child(&mut dom, grid, 40.0);

    let font_db = elidex_text::FontDatabase::new();
    let _lb = do_layout_grid(
        &mut dom,
        grid,
        600.0,
        Some(400.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let child_lb = get_layout(&dom, child);
    assert!(
        child_lb.content.size.width > 0.0 || child_lb.content.size.height > 0.0,
        "vertical-lr grid child should have non-zero size",
    );
}

#[test]
fn vertical_rl_two_column_tracks() {
    // Two column tracks in vertical-rl: columns = inline axis = physical Y.
    // Children should be placed in separate column tracks (non-overlapping on Y axis).
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalRl,
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(100.0),
                TrackSize::Length(150.0),
            ])),
            ..Default::default()
        },
    );
    let child1 = make_grid_child(&mut dom, grid, 20.0);
    let child2 = make_grid_child(&mut dom, grid, 30.0);

    let font_db = elidex_text::FontDatabase::new();
    let _lb = do_layout_grid(
        &mut dom,
        grid,
        800.0,
        Some(500.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let c1 = get_layout(&dom, child1);
    let c2 = get_layout(&dom, child2);
    // Both children should be placed and sized.
    assert!(
        c1.content.size.width > 0.0 || c1.content.size.height > 0.0,
        "child1 non-zero"
    );
    assert!(
        c2.content.size.width > 0.0 || c2.content.size.height > 0.0,
        "child2 non-zero"
    );
    // Column tracks map to physical Y in vertical-rl.
    // Children should not overlap on the Y axis.
    let c1_end_y = c1.content.bottom();
    assert!(
        c2.content.origin.y >= c1_end_y - 0.5 || c1.content.origin.y >= c2.content.bottom() - 0.5,
        "children should not overlap on Y: c1 y={} h={}, c2 y={} h={}",
        c1.content.origin.y,
        c1.content.size.height,
        c2.content.origin.y,
        c2.content.size.height,
    );
}

#[test]
fn vertical_rl_margin_pct_resolves_against_inline_size() {
    // Grid child's margin % should resolve against the grid area's inline size.
    // In vertical-rl, inline axis = physical Y.
    // CSS Grid §7.1: column tracks = inline axis, so the area's inline size = column track.
    // Use explicit column track of 500px so the area inline dimension is deterministic.
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalRl,
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(500.0),
            ])),
            grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(200.0),
            ])),
            ..Default::default()
        },
    );
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(grid, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                margin_top: Dimension::Percentage(10.0), // inline-start margin in VRL
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = elidex_text::FontDatabase::new();
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size: 600.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let _outcome = crate::layout_grid(&mut dom, grid, &input, layout_block_only);

    let child_lb = get_layout(&dom, child);
    // margin-top in vertical-rl is inline-start margin.
    // Grid area inline size = column track = 500px (columns = inline axis in §7.1).
    // In vertical mode, column offsets map to physical Y, so area height = column track.
    // 10% of 500 = 50.
    assert!(
        approx_eq(child_lb.margin.top, 50.0),
        "margin-top should be 50 (10% of 500), got {}",
        child_lb.margin.top,
    );
}

#[test]
fn vertical_lr_padding_percent_resolves_against_inline_size() {
    // Same as vertical-rl padding test but for vertical-lr.
    let mut dom = elidex_ecs::EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalLr,
            padding: EdgeSizes {
                top: Dimension::Percentage(5.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(100.0),
            ])),
            ..Default::default()
        },
    );
    let _child = make_grid_child(&mut dom, grid, 30.0);

    let font_db = elidex_text::FontDatabase::new();
    let containing_inline_size = 600.0;
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let outcome = crate::layout_grid(&mut dom, grid, &input, layout_block_only);

    // 5% of 600 = 30.
    assert!(
        approx_eq(outcome.layout_box.padding.top, 30.0),
        "padding-top should be 30 (5% of inline size 600), got {}",
        outcome.layout_box.padding.top,
    );
}
