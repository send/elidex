//! Tests for flex baseline alignment (Step 3) and cross-size definiteness (Step 4).

use super::*;
use elidex_plugin::AlignSelf;

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0
}

// ===========================================================================
// Step 3: Flex baseline alignment (CSS Flexbox §9.4, §9.6)
// ===========================================================================

#[test]
fn single_item_baseline() {
    // Row flex, 1 item with align-items: Baseline. Container's first_baseline should be Some.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let item = flex_item(100.0, 40.0);
    let (mut dom, c, _items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    let container_lb = do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!(
        container_lb.first_baseline.is_some(),
        "container first_baseline should be Some with baseline-aligned item"
    );
}

#[test]
fn multi_item_baseline_alignment() {
    // Row flex, 3 items with different heights, align-items: Baseline.
    // Items should be aligned by baseline — the tallest baseline-above item
    // defines the line baseline. Not all items should have the same y.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let items_styles = [
        flex_item(80.0, 20.0),
        flex_item(80.0, 40.0),
        flex_item(80.0, 30.0),
    ];
    let (mut dom, c, items) = make_flex_dom(container, &items_styles);
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    let lb2 = get_lb(&dom, items[2]);

    // With baseline alignment and different heights, items' y positions
    // reflect baseline offsets. The tallest item (40px) typically has its
    // baseline highest, so shorter items shift down. At minimum, layout
    // should succeed and items should be positioned.
    // The key assertion: not all items are at y=0 when heights differ and
    // baseline alignment is used — the fallback baseline is the margin-box
    // bottom, so items with smaller height get shifted down.
    let all_same_y = approx_eq(lb0.content.origin.y, lb1.content.origin.y)
        && approx_eq(lb1.content.origin.y, lb2.content.origin.y);
    // With fallback baseline = margin-box bottom edge, the tallest item
    // defines the line baseline, and shorter items are pushed down.
    assert!(
        !all_same_y,
        "baseline alignment with different heights should produce different y offsets: \
         y0={}, y1={}, y2={}",
        lb0.content.origin.y, lb1.content.origin.y, lb2.content.origin.y,
    );
}

#[test]
fn mixed_baseline_and_stretch() {
    // Row flex, 2 items: one Baseline (height:30), one Stretch (height:auto).
    // The stretch item stretches to the line cross size.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let baseline_item = flex_item(100.0, 30.0);
    let stretch_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        // height: Auto (default) — eligible for stretch via align_self override
        align_self: AlignSelf::Stretch,
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[baseline_item, stretch_item]);
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

    let lb_baseline = get_lb(&dom, items[0]);
    let lb_stretch = get_lb(&dom, items[1]);

    // The baseline item has height 30. The stretch item should stretch to
    // the line cross size (which is at least 30, accounting for baseline offsets).
    // The stretch item's height should be >= 30 (the line cross size).
    assert!(
        lb_stretch.content.size.height >= 30.0 - 1.0,
        "stretch item should stretch to line cross size: height={}, expected >= 30",
        lb_stretch.content.size.height,
    );
    // Baseline item should retain its explicit height.
    assert!(
        approx_eq(lb_baseline.content.size.height, 30.0),
        "baseline item should keep explicit height: height={}",
        lb_baseline.content.size.height,
    );
}

#[test]
fn wrap_with_baseline_per_line() {
    // Row flex-wrap, items that wrap into 2 lines, align-items: Baseline.
    // Container width 100, items with width 60 each → wraps.
    let container = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Baseline,
        width: Dimension::Length(100.0),
        ..Default::default()
    };
    let items_styles = [flex_item(60.0, 30.0), flex_item(60.0, 40.0)];
    let (mut dom, c, items) = make_flex_dom(container, &items_styles);
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);

    // Item 1 should be on the second line, so its y should be > first line height.
    assert!(
        lb1.content.origin.y > lb0.content.origin.y,
        "second line item should have y > first line: y0={}, y1={}",
        lb0.content.origin.y,
        lb1.content.origin.y,
    );
}

#[test]
fn column_direction_baseline_fallback() {
    // Column flex with align-items: Baseline. In horizontal writing mode,
    // baseline is not applicable for column direction — items should behave
    // like flex-start (y offset = 0 for first item).
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Baseline,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let items_styles = [flex_item(100.0, 40.0), flex_item(100.0, 50.0)];
    let (mut dom, c, items) = make_flex_dom(container, &items_styles);
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

    let lb0 = get_lb(&dom, items[0]);
    // Column flex in horizontal WM: baseline not applicable, items at cross-start (x=0).
    assert!(
        approx_eq(lb0.content.origin.x, 0.0),
        "column baseline fallback should act as flex-start: x={}",
        lb0.content.origin.x,
    );
}

#[test]
fn reversed_direction_baseline() {
    // RowReverse flex with align-items: Baseline, 2 items.
    // Items should still have baseline alignment applied (reverse order on main axis).
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::RowReverse,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let items_styles = [flex_item(100.0, 20.0), flex_item(100.0, 40.0)];
    let (mut dom, c, items) = make_flex_dom(container, &items_styles);
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);

    // RowReverse: item 0 should be to the right of item 1.
    assert!(
        lb0.content.origin.x > lb1.content.origin.x,
        "row-reverse: item 0 (x={}) should be right of item 1 (x={})",
        lb0.content.origin.x,
        lb1.content.origin.x,
    );

    // Baseline alignment should still produce different y positions for
    // items with different heights (fallback baseline = margin-box bottom).
    let y_differ = !approx_eq(lb0.content.origin.y, lb1.content.origin.y);
    assert!(
        y_differ,
        "row-reverse baseline: different heights should give different y: y0={}, y1={}",
        lb0.content.origin.y, lb1.content.origin.y,
    );
}

#[test]
fn fallback_no_baseline_item() {
    // Row flex, align-items: Baseline, items with explicit heights (no inline text).
    // Block layout children without text produce first_baseline=None in LayoutBox.
    // Fallback: margin-box bottom used as baseline. Layout should succeed.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let items_styles = [flex_item(100.0, 30.0), flex_item(100.0, 60.0)];
    let (mut dom, c, items) = make_flex_dom(container, &items_styles);
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

    // Layout should complete without panic.
    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Both items should have valid dimensions.
    assert!(
        approx_eq(lb0.content.size.width, 100.0),
        "item 0 width={}",
        lb0.content.size.width,
    );
    assert!(
        approx_eq(lb1.content.size.width, 100.0),
        "item 1 width={}",
        lb1.content.size.width,
    );
    // Fallback baseline = margin-box bottom. Taller item (60) has higher
    // baseline, so shorter item (30) is shifted down.
    assert!(
        lb0.content.origin.y > lb1.content.origin.y
            || approx_eq(lb0.content.origin.y, lb1.content.origin.y),
        "shorter item should be at or below taller: y0={}, y1={}",
        lb0.content.origin.y,
        lb1.content.origin.y,
    );
}

#[test]
fn container_baseline_propagation() {
    // Row flex, items with height 30 and 50, align-items: Baseline.
    // The flex container's own first_baseline should be Some.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let items_styles = [flex_item(100.0, 30.0), flex_item(100.0, 50.0)];
    let (mut dom, c, _items) = make_flex_dom(container, &items_styles);
    let font_db = FontDatabase::new();
    let container_lb = do_layout_flex(
        &mut dom,
        c,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!(
        container_lb.first_baseline.is_some(),
        "flex container should propagate first_baseline from baseline-aligned items"
    );
}

#[test]
fn auto_cross_margin_excludes_baseline() {
    // Row flex, align-items: Baseline, 2 items.
    // First item has margin_top: Auto — auto cross margin excludes the item
    // from baseline participation (CSS Flexbox §9.6).
    // The line baseline should only consider the second item.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Baseline,
        ..Default::default()
    };
    let auto_margin_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(20.0),
        margin_top: Dimension::Auto,
        ..Default::default()
    };
    let normal_item = flex_item(100.0, 40.0);
    let (mut dom, c, items) = make_flex_dom(container, &[auto_margin_item, normal_item]);
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);

    // The auto-margin item is excluded from baseline computation.
    // The second item (height 40) defines the line baseline alone.
    // The auto-margin item should be pushed by its auto margin (cross-start).
    // Line cross size is at least 40 (from the second item).
    // With margin_top: Auto on item 0 (height 20), auto margin absorbs
    // the difference: item 0 pushed down by (line_cross - 20).
    assert!(
        lb0.content.origin.y > 0.0 || lb1.content.origin.y == 0.0,
        "auto cross margin item should be pushed down or normal item at top: y0={}, y1={}",
        lb0.content.origin.y,
        lb1.content.origin.y,
    );
    // The normal item (no auto margin) participates in baseline, so it
    // should be at a stable position.
    assert!(
        lb1.content.size.height >= 39.0,
        "normal item should retain its height: {}",
        lb1.content.size.height,
    );
}

// ===========================================================================
// Step 4: Flex cross-size definiteness (CSS Flexbox §9.9)
// ===========================================================================

#[test]
fn percentage_height_in_stretched_row_item() {
    // Row flex container (height:200, wrap), child with align:Stretch, height:auto.
    // Child contains a grandchild with height:50%.
    // Stretch makes cross-size definite → grandchild resolves to 50% of 200 = 100.
    // flex-wrap:Wrap is needed so align-content:stretch expands the single line
    // to the container cross size.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Stretch,
        flex_wrap: FlexWrap::Wrap,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let stretch_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        // height: Auto → stretches to container cross size (200)
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[stretch_item]);

    // Add a grandchild with height: 50%
    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(items[0], grandchild);
    let grandchild_style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Percentage(50.0),
        ..Default::default()
    };
    dom.world_mut().insert_one(grandchild, grandchild_style);

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

    let gc_lb = get_lb(&dom, grandchild);
    // Grandchild height should be 50% of stretched cross size (200) = 100.
    assert!(
        approx_eq(gc_lb.content.size.height, 100.0),
        "grandchild 50% height in stretched item should be 100: height={}",
        gc_lb.content.size.height,
    );
}

#[test]
fn percentage_width_in_stretched_column_item() {
    // Column flex container (width via containing_width), child with align:Stretch, width:auto.
    // Child contains a grandchild with width:50%.
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        height: Dimension::Length(400.0),
        ..Default::default()
    };
    let stretch_item = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(100.0),
        // width: Auto → stretches to container cross size (containing_width)
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[stretch_item]);

    // Add a grandchild with width: 50%
    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(items[0], grandchild);
    let grandchild_style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Percentage(50.0),
        height: Dimension::Length(50.0),
        ..Default::default()
    };
    dom.world_mut().insert_one(grandchild, grandchild_style);

    let font_db = FontDatabase::new();
    let containing_width = 400.0;
    do_layout_flex(
        &mut dom,
        c,
        containing_width,
        Some(400.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let gc_lb = get_lb(&dom, grandchild);
    // Column flex: cross axis is width. Stretched item gets containing_width.
    // Grandchild 50% of that = 200.
    assert!(
        approx_eq(gc_lb.content.size.width, 200.0),
        "grandchild 50% width in stretched column item should be 200: width={}",
        gc_lb.content.size.width,
    );
}

#[test]
fn non_stretch_remains_indefinite() {
    // Row flex (height:200), child with align:FlexStart, height:auto.
    // Child has grandchild with height:50%. Since not stretched, cross-size
    // is NOT definite → percentage resolves against 0 or treated as auto.
    let container = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::FlexStart,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let non_stretch_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        // height: Auto, but align is FlexStart (not Stretch)
        ..Default::default()
    };
    let (mut dom, c, items) = make_flex_dom(container, &[non_stretch_item]);

    // Add a grandchild with height: 50%
    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(items[0], grandchild);
    let grandchild_style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Percentage(50.0),
        ..Default::default()
    };
    dom.world_mut().insert_one(grandchild, grandchild_style);

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

    let gc_lb = get_lb(&dom, grandchild);
    // Non-stretched item: cross-size is indefinite for percentage children.
    // The grandchild's 50% should NOT resolve to 100 (which would be 50% of 200).
    // It should resolve against 0 or the resolved_height, but NOT
    // against the stretched cross size (since there is no stretch).
    assert!(
        gc_lb.content.size.height < 100.0 - 1.0,
        "non-stretch item grandchild 50% height should not resolve to 100: height={}",
        gc_lb.content.size.height,
    );
}

#[test]
fn explicit_cross_size_already_definite() {
    // Row flex, child with explicit height:100. Grandchild with height:50%
    // should resolve to 50.
    let container = ComputedStyle {
        display: Display::Flex,
        ..Default::default()
    };
    let explicit_item = flex_item(200.0, 100.0);
    let (mut dom, c, items) = make_flex_dom(container, &[explicit_item]);

    // Add a grandchild with height: 50%
    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(items[0], grandchild);
    let grandchild_style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Percentage(50.0),
        ..Default::default()
    };
    dom.world_mut().insert_one(grandchild, grandchild_style);

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

    let gc_lb = get_lb(&dom, grandchild);
    // Explicit height: 100 on the flex item makes it definite.
    // Grandchild 50% of 100 = 50.
    assert!(
        approx_eq(gc_lb.content.size.height, 50.0),
        "grandchild 50% of explicit 100px should be 50: height={}",
        gc_lb.content.size.height,
    );
}
