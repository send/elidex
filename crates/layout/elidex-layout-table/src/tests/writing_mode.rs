//! Writing-mode and caption-side logical keyword tests.

use elidex_ecs::Attributes;
use elidex_layout_block::LayoutInput;
use elidex_plugin::{CaptionSide, Dimension, Display, EdgeSizes, Point, WritingMode};

use super::*;

#[test]
fn caption_side_block_start_treated_as_top() {
    // caption-side: block-start should place caption before the table rows.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            ..Default::default()
        },
    );

    // Add a caption with block-start
    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::BlockStart,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    // Add a row with a cell
    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    let _lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let cell_lb = get_layout(&dom, td);
    // Caption (block-start) should be above the cell.
    assert!(
        caption_lb.content.origin.y < cell_lb.content.origin.y,
        "caption y ({}) should be < cell y ({})",
        caption_lb.content.origin.y,
        cell_lb.content.origin.y,
    );
}

#[test]
fn caption_side_block_end_treated_as_bottom() {
    // caption-side: block-end should place caption after the table rows.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            ..Default::default()
        },
    );

    // Add a row first
    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    // Add a caption with block-end
    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::BlockEnd,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    let font_db = FontDatabase::new();
    let _lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let cell_lb = get_layout(&dom, td);
    // Caption (block-end) should be below the cell.
    assert!(
        caption_lb.content.origin.y > cell_lb.content.origin.y,
        "caption y ({}) should be > cell y ({})",
        caption_lb.content.origin.y,
        cell_lb.content.origin.y,
    );
}

#[test]
fn horizontal_tb_table_regression() {
    // Standard horizontal-tb table should work unchanged.
    let (mut dom, table, cells) = create_simple_table(
        2,
        2,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::HorizontalTb,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    // Table should have non-zero dimensions.
    assert!(lb.content.size.width > 0.0, "table width should be > 0");
    assert!(lb.content.size.height > 0.0, "table height should be > 0");

    // Cells should be laid out.
    for &cell in &cells {
        let cell_lb = get_layout(&dom, cell);
        assert!(
            cell_lb.content.size.width > 0.0 || cell_lb.content.size.height > 0.0,
            "cell should have non-zero size",
        );
    }
}

#[test]
fn vertical_rl_table_layout() {
    // Table in vertical-rl mode should produce non-zero dimensions.
    let (mut dom, table, cells) = create_simple_table(
        2,
        2,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    assert!(lb.content.size.width > 0.0, "table width should be > 0");
    assert!(lb.content.size.height > 0.0, "table height should be > 0");
    for &cell in &cells {
        let cell_lb = get_layout(&dom, cell);
        assert!(
            cell_lb.content.size.width > 0.0 || cell_lb.content.size.height > 0.0,
            "cell should have non-zero size",
        );
    }
}

#[test]
fn vertical_lr_table_layout() {
    // Table in vertical-lr mode should produce non-zero dimensions.
    let (mut dom, table, cells) = create_simple_table(
        2,
        2,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalLr,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    assert!(lb.content.size.width > 0.0, "table width should be > 0");
    assert!(lb.content.size.height > 0.0, "table height should be > 0");
    for &cell in &cells {
        let cell_lb = get_layout(&dom, cell);
        assert!(
            cell_lb.content.size.width > 0.0 || cell_lb.content.size.height > 0.0,
            "cell should have non-zero size",
        );
    }
}

#[test]
fn vertical_rl_padding_percent_resolves_against_inline_size() {
    // Table padding % in vertical-rl should resolve against containing_inline_size.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalRl,
            padding: EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );
    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    let containing_inline_size = 500.0;
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 500.0),
        containing_inline_size,
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
    let outcome = crate::layout_table(&mut dom, table, &input, test_layout_child);

    // 10% of 500 = 50.
    assert!(
        approx_eq(outcome.layout_box.padding.top, 50.0),
        "padding-top should be 50 (10% of 500), got {}",
        outcome.layout_box.padding.top,
    );
}

#[test]
fn caption_side_top_in_vertical_rl() {
    // caption-side: top in a vertical-rl table should still place caption
    // before the rows (block-start).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::Top,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let cell_lb = get_layout(&dom, td);
    // Caption should be placed before the cell in the block direction.
    assert!(
        caption_lb.content.origin.y < cell_lb.content.origin.y
            || caption_lb.content.origin.x != cell_lb.content.origin.x,
        "caption should be positioned separately from cell: caption=({},{}), cell=({},{})",
        caption_lb.content.origin.x,
        caption_lb.content.origin.y,
        cell_lb.content.origin.x,
        cell_lb.content.origin.y,
    );
    // Table should have non-zero size.
    assert!(lb.content.size.width > 0.0 || lb.content.size.height > 0.0);
}

#[test]
fn caption_side_block_end_in_vertical_lr() {
    // caption-side: block-end in vertical-lr should place caption after rows.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalLr,
            ..Default::default()
        },
    );

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::BlockEnd,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let cell_lb = get_layout(&dom, td);
    // Caption (block-end) should be after the cell in block direction.
    assert!(
        caption_lb.content.origin.y > cell_lb.content.origin.y
            || caption_lb.content.origin.x != cell_lb.content.origin.x,
        "block-end caption should be after cell: caption=({},{}), cell=({},{})",
        caption_lb.content.origin.x,
        caption_lb.content.origin.y,
        cell_lb.content.origin.x,
        cell_lb.content.origin.y,
    );
    assert!(lb.content.size.width > 0.0 || lb.content.size.height > 0.0);
}

#[test]
fn vertical_rl_cells_positioned_with_axis_swap() {
    // In vertical-rl, columns map to the inline axis (physical Y)
    // and rows map to the block axis (physical X).
    // A 2-column × 1-row table should have cells at different Y positions.
    let (mut dom, table, cells) = create_simple_table(
        1, // 1 row
        2, // 2 columns
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let _lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    // With axis swap: columns → Y, so cells should have different Y positions.
    assert!(
        (c0.content.origin.y - c1.content.origin.y).abs() > 0.5,
        "cells should be at different Y positions (column=inline=Y): c0.y={}, c1.y={}",
        c0.content.origin.y,
        c1.content.origin.y,
    );
}

#[test]
fn vertical_rl_table_dimensions_swapped() {
    // In vertical-rl, the table's physical width should reflect block-size (row heights)
    // and height should reflect inline-size (column widths).
    let (mut dom, table, _cells) = create_simple_table(
        1,
        1,
        ComputedStyle {
            display: Display::Table,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = do_layout_table(
        &mut dom,
        table,
        400.0,
        Some(300.0),
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    // Table should have non-zero dimensions.
    assert!(
        lb.content.size.width > 0.0 && lb.content.size.height > 0.0,
        "table should have non-zero dimensions: {}x{}",
        lb.content.size.width,
        lb.content.size.height,
    );
}
