//! Writing-mode-aware flex layout tests.

use elidex_layout_block::LayoutInput;
use elidex_plugin::{Dimension, EdgeSizes, FlexWrap, Point, WritingMode};

use super::*;

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

#[test]
fn is_main_horizontal_horizontal_tb_row() {
    // horizontal-tb + row → main axis horizontal
    assert!(super::super::is_main_horizontal(
        FlexDirection::Row,
        elidex_plugin::WritingMode::HorizontalTb,
    ));
}

#[test]
fn is_main_horizontal_vertical_rl_row() {
    // vertical-rl + row → main axis vertical (not horizontal)
    assert!(!super::super::is_main_horizontal(
        FlexDirection::Row,
        elidex_plugin::WritingMode::VerticalRl,
    ));
}

#[test]
fn is_main_horizontal_vertical_lr_column() {
    // vertical-lr + column → main axis horizontal (perpendicular to inline)
    assert!(super::super::is_main_horizontal(
        FlexDirection::Column,
        elidex_plugin::WritingMode::VerticalLr,
    ));
}

#[test]
fn is_main_horizontal_vertical_rl_row_reverse() {
    // vertical-rl + row-reverse → main axis vertical (not horizontal)
    assert!(!super::super::is_main_horizontal(
        FlexDirection::RowReverse,
        elidex_plugin::WritingMode::VerticalRl,
    ));
}

#[test]
fn vertical_rl_row_items_stacked_vertically() {
    // In vertical-rl writing mode with flex-direction: row,
    // the main axis is vertical, so items should be stacked vertically.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 70.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // In vertical writing mode with row direction, main axis = vertical,
    // so item 1's y should be below item 0.
    assert!(
        lb1.content.origin.y > lb0.content.origin.y,
        "item1.y ({}) should be > item0.y ({})",
        lb1.content.origin.y,
        lb0.content.origin.y,
    );
}

#[test]
fn horizontal_tb_row_regression() {
    // Standard horizontal-tb + row: items laid out horizontally (regression).
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: elidex_plugin::WritingMode::HorizontalTb,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Horizontal row: item 1 should be to the right of item 0.
    assert!(
        lb1.content.origin.x > lb0.content.origin.x,
        "item1.x ({}) should be > item0.x ({})",
        lb1.content.origin.x,
        lb0.content.origin.x,
    );
}

#[test]
fn vertical_lr_column_items_stacked_horizontally() {
    // vertical-lr + column: main axis is horizontal (block direction = left-to-right).
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        writing_mode: WritingMode::VerticalLr,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(80.0, 50.0), flex_item(60.0, 70.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Column in vertical-lr: main axis horizontal, items stack left-to-right.
    assert!(
        lb1.content.origin.x > lb0.content.origin.x,
        "item1.x ({}) should be > item0.x ({})",
        lb1.content.origin.x,
        lb0.content.origin.x,
    );
}

#[test]
fn vertical_rl_row_reverse_items_stacked_vertically_reversed() {
    // vertical-rl + row-reverse: main axis vertical, reversed order.
    // Container needs explicit height (= main size) for reverse positioning.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::RowReverse,
        writing_mode: WritingMode::VerticalRl,
        height: Dimension::Length(400.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 70.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // row-reverse in vertical mode: item 0 at bottom, item 1 above.
    assert!(
        lb0.content.origin.y > lb1.content.origin.y,
        "item0.y ({}) should be > item1.y ({}) in row-reverse",
        lb0.content.origin.y,
        lb1.content.origin.y,
    );
}

#[test]
fn vertical_rl_padding_percent_resolves_against_inline_size() {
    // Flex container padding % should resolve against containing_inline_size.
    // In vertical-rl, inline size = physical height.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalRl,
        padding: EdgeSizes {
            top: Dimension::Percentage(10.0),
            right: Dimension::ZERO,
            bottom: Dimension::ZERO,
            left: Dimension::ZERO,
        },
        ..Default::default()
    };
    let (mut dom, container, _items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 500.0),
        containing_inline_size: 500.0,
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
    let outcome = crate::layout_flex(&mut dom, container, &input, layout_block_only);
    // 10% of inline size 500 = 50.
    assert!(
        approx_eq(outcome.layout_box.padding.top, 50.0),
        "padding-top should be 50 (10% of 500), got {}",
        outcome.layout_box.padding.top,
    );
}

#[test]
fn vertical_rl_margin_percent_resolves_against_inline_size() {
    // Flex container margin % should resolve against containing_inline_size.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalRl,
        margin_top: Dimension::Percentage(10.0),
        ..Default::default()
    };
    let (mut dom, container, _items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    let input = LayoutInput {
        containing: CssSize::definite(800.0, 500.0),
        containing_inline_size: 500.0,
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
    let outcome = crate::layout_flex(&mut dom, container, &input, layout_block_only);
    // 10% of inline size 500 = 50.
    assert!(
        approx_eq(outcome.layout_box.margin.top, 50.0),
        "margin-top should be 50 (10% of 500), got {}",
        outcome.layout_box.margin.top,
    );
}

#[test]
fn vertical_rl_flex_items_non_zero_size() {
    // Flex items in vertical-rl should have non-zero dimensions.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalRl,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(
        style,
        &[
            flex_item(100.0, 50.0),
            flex_item(200.0, 70.0),
            flex_item(150.0, 60.0),
        ],
    );
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    for &item in &items {
        let lb = get_lb(&dom, item);
        assert!(
            lb.content.size.width > 0.0 || lb.content.size.height > 0.0,
            "flex item should have non-zero size",
        );
    }
}

#[test]
fn vertical_lr_row_items_stacked_vertically() {
    // vertical-lr + row: main axis vertical (inline direction = top-to-bottom).
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalLr,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 70.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Row in vertical-lr: main axis = vertical (Y), items stack top-to-bottom.
    assert!(
        lb1.content.origin.y > lb0.content.origin.y,
        "item1.y ({}) should be > item0.y ({})",
        lb1.content.origin.y,
        lb0.content.origin.y,
    );
}

#[test]
fn vertical_rl_flex_wrap_wraps_on_cross_axis() {
    // In vertical-rl + row + wrap, main=Y, cross=X.
    // When items overflow the main axis (height), they wrap onto the cross axis.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        writing_mode: WritingMode::VerticalRl,
        height: Dimension::Length(100.0), // main size = 100
        ..Default::default()
    };
    // Three items each 50px tall = 150px total > 100px main size.
    let (mut dom, container, items) = make_flex_dom(
        style,
        &[
            flex_item(80.0, 50.0),
            flex_item(80.0, 50.0),
            flex_item(80.0, 50.0),
        ],
    );
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let _lb1 = get_lb(&dom, items[1]);
    let lb2 = get_lb(&dom, items[2]);
    // Items 0 and 1 fit on first line (50+50=100), item 2 wraps.
    // Wrapped item should be on a different cross position (X axis).
    assert!(
        approx_eq(lb0.content.origin.y, lb2.content.origin.y)
            || lb2.content.origin.x != lb0.content.origin.x,
        "item2 should wrap to a new cross-axis line: lb0=({},{}) lb2=({},{})",
        lb0.content.origin.x,
        lb0.content.origin.y,
        lb2.content.origin.x,
        lb2.content.origin.y,
    );
}

#[test]
fn vertical_rl_flex_grow_distributes_on_main_axis() {
    // In vertical-rl + row, main=Y. Flex-grow should distribute extra main-axis space.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalRl,
        height: Dimension::Length(300.0), // main size = 300
        ..Default::default()
    };
    let item1 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(80.0),
        height: Dimension::Length(50.0),
        flex_grow: 1.0,
        ..Default::default()
    };
    let item2 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(80.0),
        height: Dimension::Length(50.0),
        flex_grow: 1.0,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[item1, item2]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Each item should grow to fill half of 300px main (Y) = 150px each.
    assert!(
        approx_eq(lb0.content.size.height, 150.0),
        "item0 height should be 150 (grown), got {}",
        lb0.content.size.height,
    );
    assert!(
        approx_eq(lb1.content.size.height, 150.0),
        "item1 height should be 150 (grown), got {}",
        lb1.content.size.height,
    );
}

#[test]
fn vertical_rl_flex_shrink_reduces_on_main_axis() {
    // In vertical-rl + row, main=Y. Flex-shrink should reduce items when they overflow.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        writing_mode: WritingMode::VerticalRl,
        height: Dimension::Length(200.0), // main size = 200
        ..Default::default()
    };
    let item1 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(80.0),
        height: Dimension::Length(150.0),
        flex_shrink: 1.0,
        ..Default::default()
    };
    let item2 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(80.0),
        height: Dimension::Length(150.0),
        flex_shrink: 1.0,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[item1, item2]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(600.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Each item should shrink to fit 200px main = 100px each.
    assert!(
        approx_eq(lb0.content.size.height, 100.0),
        "item0 height should be 100 (shrunk), got {}",
        lb0.content.size.height,
    );
    assert!(
        approx_eq(lb1.content.size.height, 100.0),
        "item1 height should be 100 (shrunk), got {}",
        lb1.content.size.height,
    );
}
