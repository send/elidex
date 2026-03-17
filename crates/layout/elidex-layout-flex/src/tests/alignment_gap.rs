use super::*;

type JustifyContentTestCase = (JustifyContent, &'static [(f32, f32)], f32, &'static [f32]);

#[test]
fn justify_content_variants() {
    // (JustifyContent, item_sizes, container_width, expected_x_positions)
    let cases: &[JustifyContentTestCase] = &[
        // FlexStart: 2 items at x=0, x=100
        (
            JustifyContent::FlexStart,
            &[(100.0, 50.0), (100.0, 50.0)],
            800.0,
            &[0.0, 100.0],
        ),
        // Center: 1 item 100px in 800px -> x=350
        (JustifyContent::Center, &[(100.0, 50.0)], 800.0, &[350.0]),
        // FlexEnd: 1 item 100px in 800px -> x=700
        (JustifyContent::FlexEnd, &[(100.0, 50.0)], 800.0, &[700.0]),
        // SpaceBetween: 2 items 100px in 800px -> x=0, x=700
        (
            JustifyContent::SpaceBetween,
            &[(100.0, 50.0), (100.0, 50.0)],
            800.0,
            &[0.0, 700.0],
        ),
        // SpaceAround: 2 items 100px in 800px -> x=150, x=550
        (
            JustifyContent::SpaceAround,
            &[(100.0, 50.0), (100.0, 50.0)],
            800.0,
            &[150.0, 550.0],
        ),
        // SpaceEvenly: 2 items 100px in 800px -> x=200, x=500
        (
            JustifyContent::SpaceEvenly,
            &[(100.0, 50.0), (100.0, 50.0)],
            800.0,
            &[200.0, 500.0],
        ),
    ];

    for (jc, item_sizes, container_width, expected_positions) in cases {
        let style = ComputedStyle {
            display: Display::Flex,
            justify_content: *jc,
            ..Default::default()
        };
        let items: Vec<ComputedStyle> = item_sizes.iter().map(|(w, h)| flex_item(*w, *h)).collect();
        let (mut dom, container, item_entities) = make_flex_dom(style, &items);
        let font_db = FontDatabase::new();
        do_layout_flex(
            &mut dom,
            container,
            *container_width,
            None,
            0.0,
            0.0,
            &font_db,
            0,
            layout_block_only,
        );

        for (i, expected_x) in expected_positions.iter().enumerate() {
            let lb = get_lb(&dom, item_entities[i]);
            assert!(
                (lb.content.x - expected_x).abs() < 1.0,
                "justify-content:{jc:?} item[{i}] x={} expected {expected_x}",
                lb.content.x,
            );
        }
    }
}

#[test]
fn align_items_non_stretch() {
    // (AlignItems, expected_y_of_shorter_item, expected_y_of_taller_item)
    // Two items: 100x30 and 100x60. Line cross size = 60.
    // Center: shorter y = (60-30)/2 = 15, taller y = 0
    // FlexStart: shorter y = 0, taller y = 0
    // FlexEnd: shorter y = 60-30 = 30, taller y = 0
    for (ai, expected_short_y, expected_tall_y) in [
        (AlignItems::Center, 15.0, 0.0),
        (AlignItems::FlexStart, 0.0, 0.0),
        (AlignItems::FlexEnd, 30.0, 0.0),
    ] {
        let style = ComputedStyle {
            display: Display::Flex,
            align_items: ai,
            ..Default::default()
        };
        let (mut dom, container, items) =
            make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
        let font_db = FontDatabase::new();
        do_layout_flex(
            &mut dom,
            container,
            800.0,
            None,
            0.0,
            0.0,
            &font_db,
            0,
            layout_block_only,
        );

        let lb0 = get_lb(&dom, items[0]);
        assert!(
            (lb0.content.y - expected_short_y).abs() < 1.0,
            "align-items:{ai:?} shorter item y={} expected {expected_short_y}",
            lb0.content.y,
        );
        let lb1 = get_lb(&dom, items[1]);
        assert!(
            (lb1.content.y - expected_tall_y).abs() < 1.0,
            "align-items:{ai:?} taller item y={} expected {expected_tall_y}",
            lb1.content.y,
        );
    }
}

#[test]
fn align_items_stretch() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Stretch,
        ..Default::default()
    };
    // First item has auto height (should stretch), second has explicit 60px height.
    let auto_height_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        // height: Auto (default) -- eligible for stretch
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[auto_height_item, flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    // Auto-height item should stretch to line cross size (60).
    let lb0 = get_lb(&dom, items[0]);
    assert!((lb0.content.height - 60.0).abs() < 1.0);

    // Explicit-height item should NOT stretch (remains 60).
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb1.content.height - 60.0).abs() < 1.0);
}

// --- M3-5: Flexbox gap ---

#[test]
fn column_gap_row_direction() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: Dimension::Length(20.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Item 0 at x=0, width=100. Gap=20. Item 1 at x=120.
    assert!((lb0.content.x).abs() < f32::EPSILON);
    assert!((lb1.content.x - 120.0).abs() < 1.0);
}

#[test]
fn row_gap_column_direction() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        row_gap: Dimension::Length(10.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 40.0), flex_item(100.0, 40.0)]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Item 0 at y=0, height=40. Gap=10. Item 1 at y=50.
    assert!((lb0.content.y).abs() < f32::EPSILON);
    assert!((lb1.content.y - 50.0).abs() < 1.0);
    // Container height = 40 + 10 + 40 = 90.
    assert!((lb.content.height - 90.0).abs() < 1.0);
}

#[test]
fn gap_affects_flex_grow() {
    let items_styles = [
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            flex_grow: 1.0,
            ..Default::default()
        },
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            flex_grow: 1.0,
            ..Default::default()
        },
    ];
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: Dimension::Length(100.0),
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &items_styles);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Available = 600 - 100 (gap) = 500. Each grows to 250.
    assert!((lb0.content.width - 250.0).abs() < 1.0);
    assert!((lb1.content.width - 250.0).abs() < 1.0);
}

#[test]
fn gap_zero_default_unchanged() {
    // Default gap is 0 -- layout should be identical to pre-gap behavior.
    let (mut dom, container, items) = make_flex_dom(
        flex_container(),
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // No gap: item1 starts right after item0.
    assert!((lb1.content.x - 100.0).abs() < f32::EPSILON);
    assert!((lb0.content.x).abs() < f32::EPSILON);
}

#[test]
fn gap_with_wrap_cross_axis() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Wrap,
        width: Dimension::Length(300.0),
        column_gap: Dimension::Length(10.0),
        row_gap: Dimension::Length(20.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Items wrap: line 0 has item0 (height 50), gap_cross=20, line 1 has item1.
    assert!((lb1.content.y - 70.0).abs() < 1.0);
    // Container height = 50 + 20 + 50 = 120.
    assert!((lb.content.height - 120.0).abs() < 1.0);
    // Both items at x=0 (different lines).
    assert!((lb0.content.x).abs() < f32::EPSILON);
    assert!((lb1.content.x).abs() < f32::EPSILON);
}

// L5: single item + gap (gap should not affect layout with only one item)
#[test]
fn gap_single_item_no_effect() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: Dimension::Length(20.0),
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    assert!((lb.content.x).abs() < f32::EPSILON);
    assert!((lb.content.width - 100.0).abs() < 1.0);
}

// L6: gap + justify-content: space-between
#[test]
fn gap_with_justify_space_between() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: Dimension::Length(10.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // space-between distributes free space between items, gap adds on top.
    // Effective gap = max(justify_gap, column_gap) -> items should be well-separated.
    assert!((lb0.content.x).abs() < f32::EPSILON);
    // Item 1 should be at right edge: 800 - 100 = 700.
    assert!((lb1.content.x - 700.0).abs() < 1.0);
}

// L7: gap + flex-direction: row-reverse
#[test]
fn gap_with_row_reverse() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::RowReverse,
        column_gap: Dimension::Length(20.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Row-reverse: item 0 at right, item 1 to its left with gap.
    // Item 0 at x = 800 - 100 = 700.
    assert!((lb0.content.x - 700.0).abs() < 1.0);
    // Item 1 at x = 700 - 20 (gap) - 100 = 580.
    assert!((lb1.content.x - 580.0).abs() < 1.0);
}

// L8: gap + flex-shrink (items shrink, gap is preserved)
#[test]
fn gap_with_flex_shrink() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: Dimension::Length(20.0),
        ..Default::default()
    };
    // Two items each 200px wide + 20px gap = 420px. Container = 400px.
    // Items should shrink to fit, gap preserved.
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Both items shrink equally. Total available = 400 - 20 (gap) = 380. Each gets 190.
    assert!((lb0.content.width - 190.0).abs() < 1.0);
    assert!((lb1.content.width - 190.0).abs() < 1.0);
    // Gap between items is maintained.
    let gap = lb1.content.x - (lb0.content.x + lb0.content.width);
    assert!((gap - 20.0).abs() < 1.0);
}

// ---------------------------------------------------------------------------
// M3.5-4: RTL direction support
// ---------------------------------------------------------------------------

#[test]
fn row_rtl_reverses_item_order() {
    // direction: rtl + flex-direction: row -> items placed right-to-left
    let (mut dom, container, items) = make_flex_dom(
        ComputedStyle {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            direction: Direction::Rtl,
            ..Default::default()
        },
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // RTL: first item should be to the right of the second item.
    assert!(
        lb0.content.x > lb1.content.x,
        "RTL row: item 0 (x={}) should be right of item 1 (x={})",
        lb0.content.x,
        lb1.content.x,
    );
}

#[test]
fn row_reverse_rtl_restores_ltr_order() {
    // direction: rtl + flex-direction: row-reverse -> double reversal = LTR order
    let (mut dom, container, items) = make_flex_dom(
        ComputedStyle {
            display: Display::Flex,
            flex_direction: FlexDirection::RowReverse,
            direction: Direction::Rtl,
            ..Default::default()
        },
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Double reversal: item 0 should be left of item 1 (same as normal LTR row).
    assert!(
        lb0.content.x < lb1.content.x,
        "RTL row-reverse: item 0 (x={}) should be left of item 1 (x={})",
        lb0.content.x,
        lb1.content.x,
    );
}
