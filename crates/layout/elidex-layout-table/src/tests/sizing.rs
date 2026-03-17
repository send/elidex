use super::*;

// ---------------------------------------------------------------------------
// Basic table tests
// ---------------------------------------------------------------------------

#[test]
fn table_basic_two_columns() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
    // Both cells should exist and share the width equally.
    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    assert!(
        approx_eq(lb0.content.width, 200.0),
        "cell 0 width: {} expected 200",
        lb0.content.width
    );
    assert!(
        approx_eq(lb1.content.width, 200.0),
        "cell 1 width: {} expected 200",
        lb1.content.width
    );
}

#[test]
fn table_three_rows() {
    let (mut dom, table, _cells) = create_simple_table(3, 2, default_table_style());
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    // Table should have some height from 3 rows.
    assert!(lb.content.height > 0.0);
}

#[test]
fn table_explicit_width() {
    let style = ComputedStyle {
        display: Display::Table,
        width: Dimension::Length(500.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 500.0));
}

#[test]
fn table_auto_width() {
    let (mut dom, table, _) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 600.0));
}

#[test]
fn table_border_spacing() {
    let style = ComputedStyle {
        display: Display::Table,
        border_spacing_h: 10.0,
        border_spacing_v: 10.0,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
}

#[test]
fn table_border_spacing_two_values() {
    let style = ComputedStyle {
        display: Display::Table,
        border_spacing_h: 5.0,
        border_spacing_v: 10.0,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(2, 2, style);
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    // Height should include vertical spacing.
    assert!(lb.content.height > 0.0);
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn table_auto_column_sizing() {
    // Verify the auto column sizing produces reasonable widths.
    let (mut dom, table, cells) = create_simple_table(1, 3, default_table_style());
    // Give different heights to verify cells are properly laid out.
    for (i, &cell) in cells.iter().enumerate() {
        dom.world_mut().insert_one(
            cell,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(20.0 + i as f32 * 10.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // All cells should have LayoutBoxes.
    for &cell in &cells {
        assert!(dom.world().get::<&LayoutBox>(cell).is_ok());
    }
}

#[test]
fn table_column_distribution() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    // In auto layout with equal content, columns should get roughly equal widths.
    assert!(approx_eq(c0.content.width, c1.content.width));
}

// ---------------------------------------------------------------------------
// table-layout: fixed tests
// ---------------------------------------------------------------------------

#[test]
fn table_fixed_layout_basic() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
}

#[test]
fn table_fixed_layout_explicit_widths() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, style);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            width: Dimension::Length(100.0),
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn table_fixed_layout_equal_distribution() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let (mut dom, table, cells) = create_simple_table(1, 2, style);
    // No explicit widths on cells — should distribute equally.
    for &cell in &cells {
        dom.world_mut().insert_one(
            cell,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(20.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    assert!(approx_eq(c0.content.width, c1.content.width));
}

#[test]
fn table_fixed_overflow() {
    // Fixed layout where content exceeds column width — should not panic.
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(100.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

// ---------------------------------------------------------------------------
// Safety / limits tests (R4-8)
// ---------------------------------------------------------------------------

#[test]
fn table_colspan_clamped_to_max() {
    // colspan="5000" should be clamped to remaining columns (max 1000 total).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    let _ = dom.append_child(table, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tbody, tr);

    // One cell with colspan=5000 (exceeds MAX_TABLE_COLS=1000)
    let mut td_attrs = Attributes::default();
    td_attrs.set("colspan", "5000");
    let td = dom.create_element("td", td_attrs);
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; layout succeeds with clamped colspan.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_column_limit_overflow() {
    // 10 cells each with colspan=200 → 2000 columns requested, capped at 1000.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    let _ = dom.append_child(table, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tbody, tr);

    for _ in 0..10 {
        let mut td_attrs = Attributes::default();
        td_attrs.set("colspan", "200");
        let td = dom.create_element("td", td_attrs);
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td);
    }

    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; columns capped at MAX_TABLE_COLS.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn auto_column_widths_nan_infinity_sanitized() {
    // NaN, Infinity, and negative values in cell content widths should be sanitized.
    let cells = vec![
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 0,
            row: 0,
            colspan: 1,
            rowspan: 1,
        },
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 1,
            row: 0,
            colspan: 1,
            rowspan: 1,
        },
    ];
    let content_widths = vec![
        (f32::NAN, f32::INFINITY),  // col 0: both non-finite
        (-10.0, f32::NEG_INFINITY), // col 1: negative + neg-infinity
    ];
    let widths = algo::auto_column_widths(2, &cells, &content_widths, 200.0);
    assert_eq!(widths.len(), 2);
    for w in &widths {
        assert!(w.is_finite(), "Width should be finite, got {w}");
        assert!(*w >= 0.0, "Width should be non-negative, got {w}");
    }
    // Total should equal available since all intrinsic sizes are zero.
    let total: f32 = widths.iter().sum();
    assert!(approx_eq(total, 200.0));
}

// ---------------------------------------------------------------------------
// T-1: auto_column_widths branch coverage (min overflow / max+remainder / proportional)
// ---------------------------------------------------------------------------

#[test]
fn auto_col_widths_branches() {
    // Each case: (description, content_widths_per_col, available_width, expected_widths)
    for (desc, content_widths, available, expected) in [
        (
            "min overflow: total_min (60+80=140) >= available (100) returns min widths",
            vec![(60.0, 200.0), (80.0, 300.0)],
            100.0,
            vec![60.0, 80.0],
        ),
        (
            "max with remainder: total_max (200) <= available (400), each gets 100 + (400-200)/2 = 200",
            vec![(50.0, 100.0), (50.0, 100.0)],
            400.0,
            vec![200.0, 200.0],
        ),
        (
            "proportional: total_min=60 < available=180 < total_max=300, fraction=0.5",
            vec![(20.0, 100.0), (40.0, 200.0)],
            180.0,
            vec![60.0, 120.0],
        ),
    ] {
        let num_cols = content_widths.len();
        let cells: Vec<_> = (0..num_cols).map(|c| make_cell(c, 0)).collect();
        let widths = algo::auto_column_widths(num_cols, &cells, &content_widths, available);
        assert_eq!(
            widths.len(),
            expected.len(),
            "{desc}: column count mismatch"
        );
        for (col, (got, want)) in widths.iter().zip(expected.iter()).enumerate() {
            assert!(
                approx_eq(*got, *want),
                "{desc}: col {col} width: got {got}, expected {want}"
            );
        }
    }
}

#[test]
fn auto_col_widths_spanning_cell() {
    // colspan=2 cell distributes evenly across 2 columns.
    let cells = vec![CellInfo {
        entity: elidex_ecs::Entity::DANGLING,
        col: 0,
        row: 0,
        colspan: 2,
        rowspan: 1,
    }];
    let content_widths = vec![(100.0, 200.0)];
    let widths = algo::auto_column_widths(2, &cells, &content_widths, 300.0);
    // min per col = 50, max per col = 100. total_max=200 <= 300.
    // Each gets 100 + (300-200)/2 = 150.
    assert_eq!(widths.len(), 2);
    assert!(approx_eq(widths[0], 150.0));
    assert!(approx_eq(widths[1], 150.0));
}

// ---------------------------------------------------------------------------
// <col>/<colgroup> width tests
// ---------------------------------------------------------------------------

#[test]
fn col_width_overrides_cell_width_in_fixed_layout() {
    // Col width (150px) should override first-row cell width (200px).
    let cells = vec![make_cell(0, 0), make_cell(1, 0)];
    let cell_widths = vec![Some(200.0), None];
    let col_widths: Vec<Option<f32>> = vec![Some(150.0), None];
    let widths = algo::fixed_column_widths(2, &cells, &cell_widths, &col_widths, 400.0);
    // Col 0: 150 (from col element), Col 1: 400-150 = 250 (auto).
    assert!(approx_eq(widths[0], 150.0));
    assert!(approx_eq(widths[1], 250.0));
}

#[test]
fn col_width_in_fixed_layout_no_col() {
    // No col widths: falls back to cell widths.
    let cells = vec![make_cell(0, 0), make_cell(1, 0)];
    let cell_widths = vec![Some(100.0), Some(200.0)];
    let col_widths: Vec<Option<f32>> = vec![None, None];
    let widths = algo::fixed_column_widths(2, &cells, &cell_widths, &col_widths, 400.0);
    assert!(approx_eq(widths[0], 100.0));
    assert!(approx_eq(widths[1], 200.0));
}

#[test]
fn col_element_width_integrated() {
    // Full integration: table with <col> elements specifying widths.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            table_layout: TableLayout::Fixed,
            width: Dimension::Length(400.0),
            ..Default::default()
        },
    );

    // <col style="width: 150px">
    let col1 = dom.create_element("col", Attributes::default());
    dom.world_mut().insert_one(
        col1,
        ComputedStyle {
            display: Display::TableColumn,
            width: Dimension::Length(150.0),
            ..Default::default()
        },
    );
    dom.append_child(table, col1);

    // <col> (no explicit width)
    let col2 = dom.create_element("col", Attributes::default());
    dom.world_mut().insert_one(
        col2,
        ComputedStyle {
            display: Display::TableColumn,
            ..Default::default()
        },
    );
    dom.append_child(table, col2);

    // <tr> with 2 cells
    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let lb1 = get_layout(&dom, td1);
    let lb2 = get_layout(&dom, td2);
    // Col 0: 150px from <col>, Col 1: remaining space.
    assert!(
        approx_eq(lb1.content.width, 150.0),
        "td1 width {} expected 150",
        lb1.content.width
    );
    assert!(
        lb2.content.width > 100.0,
        "td2 width {} should be > 100 (auto from remaining space)",
        lb2.content.width
    );
}
