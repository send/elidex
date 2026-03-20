//! G6 tests: rowspan=0 and cell percentage height (Step 5).

use super::*;
use elidex_plugin::Dimension;

// ---------------------------------------------------------------------------
// rowspan=0 (WHATWG §4.9.11)
// ---------------------------------------------------------------------------

#[test]
fn rowspan_zero_spans_remaining_rows_in_group() {
    // rowspan=0 should span remaining rows in the row group, not the entire table.
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
    dom.append_child(table, tbody);

    // 3 rows in tbody.
    let mut tds = Vec::new();
    for i in 0..3 {
        let tr = dom.create_element("tr", Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(tbody, tr);

        if i == 0 {
            // First row: cell with rowspan=0 (span all remaining rows in group).
            let mut attrs = Attributes::default();
            attrs.set("rowspan", "0");
            let td = dom.create_element("td", attrs);
            dom.world_mut().insert_one(
                td,
                ComputedStyle {
                    display: Display::TableCell,
                    ..Default::default()
                },
            );
            dom.append_child(tr, td);
            tds.push(td);
        }

        // Second column cell.
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
        tds.push(td2);
    }

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

    // tds = [td_span(r0c0), td_r0_c1, td_r1_c1, td_r2_c1]
    // The rowspan=0 cell should span all 3 rows.
    // Verify: row 2's cell is below row 0's cell, confirming 3 rows exist.
    let lb_row0_col1 = get_layout(&dom, tds[1]);
    let lb_row2_col1 = get_layout(&dom, tds[3]);
    assert!(
        lb_row2_col1.content.y > lb_row0_col1.content.y + 10.0,
        "Row 2 cell (y={}) should be well below row 0 cell (y={}), confirming 3 rows exist",
        lb_row2_col1.content.y,
        lb_row0_col1.content.y,
    );
    // The rowspan=0 cell should be laid out (not dropped).
    assert!(dom.world().get::<&LayoutBox>(tds[0]).is_ok());
}

#[test]
fn rowspan_zero_with_other_cells() {
    // rowspan=0 in first row, other cells in subsequent rows at same column position
    // should still work (occupancy grid handles it).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // 2 rows, 2 columns. Cell(0,0) has rowspan=0.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs0 = Attributes::default();
    attrs0.set("rowspan", "0");
    let td_span = dom.create_element("td", attrs0);
    dom.world_mut().insert_one(
        td_span,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td_span);

    let td1b = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1b,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1b);

    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td2b = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2b,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2b);

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

    // Should layout without panic.
    assert!(dom.world().get::<&LayoutBox>(td_span).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td1b).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2b).is_ok());
}

// ---------------------------------------------------------------------------
// Cell percentage height
// ---------------------------------------------------------------------------

#[test]
fn cell_pct_height_resolved_against_explicit_table_height() {
    // Cell with height: 50% should resolve against explicit table height.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            height: Dimension::Length(200.0),
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
            height: Dimension::Percentage(50.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

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

    // Cell height should be influenced by table's explicit height.
    let table_lb = get_layout(&dom, table);
    assert!(
        table_lb.content.height >= 199.0,
        "Table should respect explicit height, got {}",
        table_lb.content.height,
    );

    // Cell's containing_height was set to table explicit height (200px),
    // so cell with height:50% should resolve to 100px content height.
    let cell_lb = get_layout(&dom, td);
    assert!(
        cell_lb.content.height >= 99.0,
        "Cell with height:50% against table height:200px should be ~100px, got {}",
        cell_lb.content.height,
    );
}

#[test]
fn cell_pct_height_ignored_without_explicit_table_height() {
    // Without explicit table height, cell % height should fall back to auto.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

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
            height: Dimension::Percentage(50.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

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

    // Should not crash; cell height will be 0 or small (auto).
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}
