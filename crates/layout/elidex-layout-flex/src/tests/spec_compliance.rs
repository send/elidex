//! Tests for flex spec compliance: blockification (§4.2), auto margins (§8.1),
//! and visibility:collapse (§4.4).

use super::*;

// ---------------------------------------------------------------------------
// Step 3: Flex item blockification (Flex §4.2)
// ---------------------------------------------------------------------------

#[test]
fn flex_item_inline_blockified() {
    let container = flex_container();
    let inline_item = ComputedStyle {
        display: Display::Inline,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[inline_item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(items[0])
        .map(|s| s.display)
        .unwrap();
    assert_eq!(
        style,
        Display::Block,
        "inline should be blockified to block"
    );
}

#[test]
fn flex_item_inline_flex_to_flex() {
    let container = flex_container();
    let inline_flex = ComputedStyle {
        display: Display::InlineFlex,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[inline_flex]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(items[0])
        .map(|s| s.display)
        .unwrap();
    assert_eq!(
        style,
        Display::Flex,
        "inline-flex should be blockified to flex"
    );
}

#[test]
fn flex_item_inline_table_to_table() {
    let container = flex_container();
    let inline_table = ComputedStyle {
        display: Display::InlineTable,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[inline_table]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(items[0])
        .map(|s| s.display)
        .unwrap();
    assert_eq!(
        style,
        Display::Table,
        "inline-table should be blockified to table"
    );
}

#[test]
fn flex_item_block_unchanged() {
    let container = flex_container();
    let block_item = flex_item(100.0, 50.0); // display: Block
    let (mut dom, c, items) = make_flex_dom(container, &[block_item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(items[0])
        .map(|s| s.display)
        .unwrap();
    assert_eq!(style, Display::Block, "block should remain unchanged");
}

// ---------------------------------------------------------------------------
// Step 4: Flex auto margins (Flex §8.1)
// ---------------------------------------------------------------------------

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0
}

#[test]
fn flex_auto_margin_main_both() {
    // Both margin-left and margin-right auto → centered in main axis.
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // Free space = 400 - 100 = 300. Both auto → 150 each. Item at x=150.
    assert!(
        approx_eq(lb.content.origin.x, 150.0),
        "both auto margins should center: x={}, expected 150",
        lb.content.origin.x,
    );
}

#[test]
fn flex_auto_margin_main_start() {
    // margin-left: auto → pushes item to the right (end).
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        margin_left: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // Free space = 300. Only start auto → all 300 goes to start. Item at x=300.
    assert!(
        approx_eq(lb.content.origin.x, 300.0),
        "start auto margin should push right: x={}, expected 300",
        lb.content.origin.x,
    );
}

#[test]
fn flex_auto_margin_main_end() {
    // margin-right: auto → item stays at start (auto absorbs free space at end).
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        margin_right: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // End auto only — item stays at x=0.
    assert!(
        approx_eq(lb.content.origin.x, 0.0),
        "end auto margin should keep at start: x={}, expected 0",
        lb.content.origin.x,
    );
}

#[test]
fn flex_auto_margin_cross_both() {
    // Both margin-top and margin-bottom auto → centered on cross axis.
    // Use flex-wrap:wrap so align-content:stretch expands the line to container_cross.
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(400.0),
        height: Dimension::Length(200.0),
        flex_wrap: FlexWrap::Wrap,
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        margin_top: Dimension::Auto,
        margin_bottom: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // With wrap and align-content:stretch, single line stretches to 200.
    // item cross = 50. Auto margins: (200-50)/2 = 75. Item at y=75.
    assert!(
        approx_eq(lb.content.origin.y, 75.0),
        "both cross auto margins should center: y={}, expected 75",
        lb.content.origin.y,
    );
}

#[test]
fn flex_auto_margin_overrides_justify() {
    // Auto margins override justify-content.
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(400.0),
        justify_content: JustifyContent::Center,
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        margin_left: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // justify-content:center would put item at x=150, but auto margin overrides.
    // margin-left:auto absorbs 300 → item at x=300.
    assert!(
        approx_eq(lb.content.origin.x, 300.0),
        "auto margin should override justify-content: x={}, expected 300",
        lb.content.origin.x,
    );
}

#[test]
fn flex_auto_margin_negative_space() {
    // When free space is negative, auto margins resolve to 0.
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(100.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        height: Dimension::Length(50.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        100.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // Negative free space → auto margins are 0. Item at x=0.
    assert!(
        approx_eq(lb.content.origin.x, 0.0),
        "negative space auto margin should be 0: x={}, expected 0",
        lb.content.origin.x,
    );
}

#[test]
fn flex_auto_margin_main_both_column() {
    // flex-direction:column — margin-top:auto + margin-bottom:auto → centered on main (vertical).
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        height: Dimension::Length(400.0),
        width: Dimension::Length(200.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(100.0),
        margin_top: Dimension::Auto,
        margin_bottom: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        200.0,
        Some(400.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // Container main (height) = 400, item main = 100. Free = 300. Both auto → 150 each.
    assert!(
        approx_eq(lb.content.origin.y, 150.0),
        "column both auto margins should center: y={}, expected 150",
        lb.content.origin.y,
    );
}

#[test]
fn flex_auto_margin_main_start_column() {
    // flex-direction:column — margin-top:auto → pushes item to bottom.
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        height: Dimension::Length(400.0),
        width: Dimension::Length(200.0),
        ..Default::default()
    };
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(100.0),
        margin_top: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        200.0,
        Some(400.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // Free = 300. Only start (top) auto → all 300 to top. Item at y=300.
    assert!(
        approx_eq(lb.content.origin.y, 300.0),
        "column start auto margin should push to bottom: y={}, expected 300",
        lb.content.origin.y,
    );
}

// ---------------------------------------------------------------------------
// Step 5: Flex visibility:collapse (Flex §4.4)
// ---------------------------------------------------------------------------

#[test]
fn flex_visibility_collapse_zero_main() {
    // Collapsed item should have zero main-axis size.
    let container = flex_container();
    let normal = flex_item(100.0, 50.0);
    let mut collapsed = flex_item(100.0, 50.0);
    collapsed.visibility = Visibility::Collapse;

    let (mut dom, c, items) = make_flex_dom(container, &[normal, collapsed.clone()]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[1]);
    assert!(
        lb.content.size.width < 1.0,
        "collapsed item main size should be ~0: width={}",
        lb.content.size.width,
    );
}

#[test]
fn flex_visibility_collapse_cross_contributes() {
    // Collapsed item's cross size should still contribute to the line cross size.
    let container = flex_container();
    let small = flex_item(100.0, 30.0);
    let mut tall_collapsed = flex_item(100.0, 80.0);
    tall_collapsed.visibility = Visibility::Collapse;

    let (mut dom, c, _items) = make_flex_dom(container, &[small, tall_collapsed]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    // The container height should reflect the taller (collapsed) item's cross size.
    let container_lb = get_lb(&dom, c);
    assert!(
        container_lb.content.size.height >= 80.0,
        "line cross should include collapsed item: container_height={}, expected >= 80",
        container_lb.content.size.height,
    );
}

#[test]
fn flex_visibility_collapse_siblings_fill() {
    // Non-collapsed siblings should expand to fill the freed main-axis space.
    let container = ComputedStyle {
        display: Display::Flex,
        width: Dimension::Length(300.0),
        ..Default::default()
    };
    let growing = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        flex_grow: 1.0,
        ..Default::default()
    };
    let collapsed = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        visibility: Visibility::Collapse,
        ..Default::default()
    };

    let (mut dom, c, items) = make_flex_dom(container, &[growing, collapsed]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        300.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // With collapsed item's final_main=0 and margin_main=0, free space increases.
    // grow item (flex-grow:1) should expand to fill more space.
    assert!(
        lb.content.size.width > 100.0,
        "growing sibling should fill freed space: width={}, expected > 100",
        lb.content.size.width,
    );
}

#[test]
fn flex_visibility_collapse_not_skipped() {
    // Collapsed items are NOT skipped like display:none — they still participate.
    let container = flex_container();
    let normal = flex_item(100.0, 50.0);
    let mut collapsed = flex_item(100.0, 50.0);
    collapsed.visibility = Visibility::Collapse;
    let normal2 = flex_item(100.0, 50.0);

    let (mut dom, c, items) = make_flex_dom(container, &[normal, collapsed, normal2]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        c,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    // All 3 items should have LayoutBoxes (display:none items don't get one).
    for (i, &e) in items.iter().enumerate() {
        assert!(
            dom.world().get::<&LayoutBox>(e).is_ok(),
            "item[{i}] should have LayoutBox (collapse ≠ display:none)",
        );
    }
}
