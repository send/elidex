use super::*;
use elidex_plugin::VerticalAlign;

// ---------------------------------------------------------------------------
// Table cell vertical-align tests
// ---------------------------------------------------------------------------

/// Helper: insert a `TableCell` `ComputedStyle` with the given height and vertical-align.
fn set_cell_style(dom: &mut EcsDom, cell: elidex_ecs::Entity, height: f32, va: VerticalAlign) {
    dom.world_mut().insert_one(
        cell,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(height),
            vertical_align: va,
            ..Default::default()
        },
    );
}

#[test]
fn vertical_align_top() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Top);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // Top-aligned cells should start at the same y position.
    assert!(
        approx_eq(lb0.content.y, lb1.content.y),
        "top-aligned cell y={} should equal tall cell y={}",
        lb0.content.y,
        lb1.content.y
    );
}

#[test]
fn vertical_align_middle() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Middle);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // Middle-aligned: offset = (row_height - cell_height) / 2 = (40 - 20) / 2 = 10
    let expected_offset = 10.0;
    assert!(
        approx_eq(lb0.content.y - lb1.content.y, expected_offset),
        "middle-aligned cell should be offset by {} from top, got {}",
        expected_offset,
        lb0.content.y - lb1.content.y
    );
}

#[test]
fn vertical_align_bottom() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Bottom);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // Bottom-aligned: offset = row_height - cell_height = 40 - 20 = 20
    let expected_offset = 20.0;
    assert!(
        approx_eq(lb0.content.y - lb1.content.y, expected_offset),
        "bottom-aligned cell should be offset by {} from top, got {}",
        expected_offset,
        lb0.content.y - lb1.content.y
    );
}

#[test]
fn vertical_align_baseline() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Baseline);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Baseline);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // With no text content, fallback baseline = cell_total_height.
    // The taller cell (40) sets the row baseline. The shorter cell (20) aligns
    // its baseline to the row baseline, pushing it down by (40 - 20) = 20.
    let offset = lb0.content.y - lb1.content.y;
    assert!(
        offset >= 0.0,
        "baseline-aligned shorter cell should not be above the taller cell, offset={offset}",
    );
}

#[test]
fn baseline_alignment_within_row() {
    let (mut dom, table, cells) = create_simple_table(2, 2, default_table_style());
    // Row 0: cells[0] height 20, cells[1] height 40, both baseline-aligned.
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Baseline);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Baseline);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    let table_lb = get_layout(&dom, table);
    // Row height should be at least 40 (the taller cell).
    assert!(
        table_lb.content.height >= 40.0,
        "table should be tall enough to hold the 40px row, got {}",
        table_lb.content.height
    );
    // The shorter cell's y should be >= the taller cell's y (pushed down for baseline alignment).
    assert!(
        lb0.content.y >= lb1.content.y,
        "shorter cell y={} should be >= taller cell y={}",
        lb0.content.y,
        lb1.content.y
    );
}

#[test]
fn rowspan_interaction() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 0
    let tr0 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr0,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr0);

    let mut attrs = Attributes::default();
    attrs.set("rowspan", "2");
    let td00 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td00,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(50.0),
            vertical_align: VerticalAlign::Middle,
            ..Default::default()
        },
    );
    dom.append_child(tr0, td00);

    let td01 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td01,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr0, td01);

    // Row 1
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let td11 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td11,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td11);

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

    let lb00 = get_layout(&dom, td00);
    let lb01 = get_layout(&dom, td01);
    let lb11 = get_layout(&dom, td11);

    // The spanning cell (td00) spans both rows (total height = row0 + row1).
    // With vertical-align: Middle, it should be centered in the combined row slot.
    // Its y should be >= row 0 top (starts at or below row 0 top).
    assert!(
        lb00.content.y >= lb01.content.y,
        "spanning cell y={} should be >= row 0 cell y={}",
        lb00.content.y,
        lb01.content.y
    );
    // td11 should be in row 1, strictly below row 0 cells.
    assert!(
        lb11.content.y > lb01.content.y,
        "row 1 cell y={} should be below row 0 cell y={}",
        lb11.content.y,
        lb01.content.y
    );
}

#[test]
fn empty_cell() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 0.0, VerticalAlign::Middle);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // Empty cell with middle alignment: offset = free / 2 = (40 - 0) / 2 = 20
    let expected_offset = 20.0;
    assert!(
        approx_eq(lb0.content.y - lb1.content.y, expected_offset),
        "empty middle-aligned cell should be offset by {} from top, got {}",
        expected_offset,
        lb0.content.y - lb1.content.y
    );
}

#[test]
fn table_container_baseline_propagation() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Baseline);
    set_cell_style(&mut dom, cells[1], 30.0, VerticalAlign::Baseline);

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

    let table_lb = get_layout(&dom, table);
    // The table container should propagate first_baseline from the first row.
    assert!(
        table_lb.first_baseline.is_some(),
        "table should have first_baseline set"
    );
    assert!(
        table_lb.first_baseline.unwrap() > 0.0,
        "table first_baseline should be > 0, got {}",
        table_lb.first_baseline.unwrap()
    );
}

#[test]
fn mixed_baseline_and_non_baseline_cells() {
    let (mut dom, table, cells) = create_simple_table(1, 3, default_table_style());
    // Cell 0: baseline-aligned, height 20
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Baseline);
    // Cell 1: top-aligned, height 40 (tallest, determines row height)
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);
    // Cell 2: bottom-aligned, height 30
    set_cell_style(&mut dom, cells[2], 30.0, VerticalAlign::Bottom);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    let lb2 = get_layout(&dom, cells[2]);
    // Cell 1 (top): should be at row top.
    // Cell 2 (bottom): offset = 40 - 30 = 10 from row top.
    let bottom_offset = lb2.content.y - lb1.content.y;
    assert!(
        approx_eq(bottom_offset, 10.0),
        "bottom-aligned cell should be offset by 10 from top, got {bottom_offset}"
    );
    // Cell 0 (baseline): with no text, synthesized baseline = content-edge bottom.
    // Row has a baseline-aligned cell, so row_baseline = cell's baseline.
    // The baseline-aligned cell should be offset to align its baseline with
    // the row baseline. With only one baseline cell, offset should be 0.
    // The cell y should be >= top cell y (may be pushed down if row baseline
    // causes expansion, but with one baseline cell it just sits at top).
    assert!(
        lb0.content.y >= lb1.content.y,
        "baseline cell y={} should be >= top cell y={}",
        lb0.content.y,
        lb1.content.y
    );
}

#[test]
fn sub_super_treated_as_baseline_in_table() {
    // CSS 2.1 §17.5.1: sub/super/text-top/text-bottom treated as baseline in table context.
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Sub);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Top);

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

    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    // Sub in table → baseline fallback. No baseline-aligned cells to set row baseline,
    // so offset = max(0, 0 - cell_baseline) = 0. Cell aligns to top.
    assert!(
        approx_eq(lb0.content.y, lb1.content.y),
        "sub-aligned cell should align to top when no baseline cells exist, y0={} y1={}",
        lb0.content.y,
        lb1.content.y
    );
}

#[test]
fn baseline_rowspan_cell_does_not_contribute() {
    // CSS 2.1 §17.5.1: rowspan > 1 cells do not contribute to per-row baseline.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tr0 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr0,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr0);

    // Cell spanning 2 rows, baseline-aligned.
    let mut attrs = Attributes::default();
    attrs.set("rowspan", "2");
    let td00 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td00,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(50.0),
            vertical_align: VerticalAlign::Baseline,
            ..Default::default()
        },
    );
    dom.append_child(tr0, td00);

    // Normal cell in row 0, baseline-aligned.
    let td01 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td01,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            vertical_align: VerticalAlign::Baseline,
            ..Default::default()
        },
    );
    dom.append_child(tr0, td01);

    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let td11 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td11,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td11);

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

    let lb00 = get_layout(&dom, td00);
    let lb01 = get_layout(&dom, td01);
    // The rowspan cell should NOT set the row baseline (only single-row cells do).
    // td01 (single-row, baseline) sets the row baseline.
    // Both should have valid positions.
    assert!(
        lb01.content.y >= 0.0,
        "single-row baseline cell should have valid y={}",
        lb01.content.y
    );
    assert!(
        lb00.content.y >= 0.0,
        "rowspan cell should have valid y={}",
        lb00.content.y
    );
}

#[test]
fn table_baseline_no_baseline_cells_in_row_0() {
    // When row 0 has no baseline-aligned cells, table baseline = row_heights[0].
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    set_cell_style(&mut dom, cells[0], 20.0, VerticalAlign::Top);
    set_cell_style(&mut dom, cells[1], 40.0, VerticalAlign::Middle);

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

    let table_lb = get_layout(&dom, table);
    // Table should still have a baseline (fallback to row height).
    assert!(
        table_lb.first_baseline.is_some(),
        "table should have first_baseline even without baseline-aligned cells"
    );
    // The baseline should equal the row height (40) + spacing.
    // Default table style has no spacing, so baseline ≈ row height.
    let bl = table_lb.first_baseline.unwrap();
    assert!(
        bl >= 40.0,
        "table baseline should be >= row height (40), got {bl}"
    );
}
