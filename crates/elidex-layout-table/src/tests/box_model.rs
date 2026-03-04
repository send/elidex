use super::*;

// ---------------------------------------------------------------------------
// Table padding / border tests
// ---------------------------------------------------------------------------

#[test]
fn table_padding_border() {
    let style = ComputedStyle {
        display: Display::Table,
        padding_top: 5.0,
        padding_right: 5.0,
        padding_bottom: 5.0,
        padding_left: 5.0,
        border_top_width: 2.0,
        border_right_width: 2.0,
        border_bottom_width: 2.0,
        border_left_width: 2.0,
        border_top_style: BorderStyle::Solid,
        border_right_style: BorderStyle::Solid,
        border_bottom_style: BorderStyle::Solid,
        border_left_style: BorderStyle::Solid,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
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
    assert!(approx_eq(lb.padding.top, 5.0));
    assert!(approx_eq(lb.border.top, 2.0));
    // Content width = 400 - 2*5 - 2*2 = 386
    assert!(approx_eq(lb.content.width, 386.0));
}

#[test]
fn table_cell_padding() {
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
            padding_top: 5.0,
            padding_right: 5.0,
            padding_bottom: 5.0,
            padding_left: 5.0,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
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
    let td_lb = get_layout(&dom, td);
    assert!(approx_eq(td_lb.padding.top, 5.0));
}

#[test]
fn table_nested_content() {
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
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    // Put a block element inside the cell.
    let inner_div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        inner_div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            ..Default::default()
        },
    );
    dom.append_child(td, inner_div);

    let font_db = FontDatabase::new();
    layout_table(
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

    let td_lb = get_layout(&dom, td);
    // Cell height should accommodate the inner div.
    assert!(td_lb.content.height >= 40.0);
}

#[test]
fn table_cell_align() {
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
            text_align: TextAlign::Center,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
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
    // Should not panic; text-align is inherited by cell content.
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

#[test]
fn table_row_background() {
    // Just ensure row background doesn't break layout.
    let (mut dom, table, _) = create_simple_table(2, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
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
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

// ---------------------------------------------------------------------------
// border-collapse tests
// ---------------------------------------------------------------------------

#[test]
fn table_collapse_basic() {
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_spacing_h: 10.0, // should be ignored in collapse mode
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
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
fn table_collapse_no_spacing() {
    // With border-collapse: collapse, border-spacing should be ignored.
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_spacing_h: 20.0,
        border_spacing_v: 20.0,
        ..Default::default()
    };
    let (mut dom, table, cells) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
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
    // In collapse mode, spacing should be 0, so cells should be adjacent.
    // c1 should start right after c0 (no spacing gap).
    let gap = c1.content.x - (c0.content.x + c0.content.width + c0.padding.right + c0.border.right);
    assert!(
        gap < 2.0,
        "gap between cells should be minimal in collapse mode, got {gap}"
    );
}

#[test]
fn table_collapse_border_merge() {
    // Test that wider border wins in collapse mode.
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_right_width: 4.0,
        border_right_style: BorderStyle::Solid,
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
            height: Dimension::Length(20.0),
            border_right_width: 2.0,
            border_right_style: BorderStyle::Solid,
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
            border_left_width: 6.0,
            border_left_style: BorderStyle::Solid,
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    layout_table(
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
    // Should not panic; border merge logic should work.
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn table_collapse_outer_border() {
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_top_width: 3.0,
        border_top_style: BorderStyle::Solid,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 1, style);
    let font_db = FontDatabase::new();
    layout_table(
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
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_collapse_grid_budget_exceeded() {
    // 1500 cols × 1500 rows exceeds MAX_COLLAPSE_GRID_CELLS (1M).
    // resolve_collapsed_borders should return default (zero) borders.
    let cells: Vec<CellInfo> = (0..4)
        .map(|i| CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: i % 2,
            row: i / 2,
            colspan: 1,
            rowspan: 1,
        })
        .collect();
    let styles: Vec<ComputedStyle> = vec![ComputedStyle::default(); 4];
    let table_style = ComputedStyle::default();

    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 1500, 1500);
    // Falls back to default borders (all zeros).
    assert_eq!(result.len(), 4);
    for cb in &result {
        assert!(
            cb.top == 0.0 && cb.right == 0.0 && cb.bottom == 0.0 && cb.left == 0.0,
            "Expected zero borders for budget-exceeded fallback"
        );
    }
}

// ---------------------------------------------------------------------------
// T-2: resolve_collapsed_borders numeric validation
// ---------------------------------------------------------------------------

#[test]
fn collapse_border_numeric_top_bottom() {
    // 2-row, 1-col table. Top cell border-bottom=2, bottom cell border-top=6.
    // Top edge of cell0 resolves against table border-top (4px).
    // Bottom edge of cell0 resolves against cell1's border-top (6px > 2px).
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let styles = vec![
        ComputedStyle {
            border_bottom_width: 2.0,
            border_bottom_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 6.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle {
        border_top_width: 4.0,
        border_top_style: BorderStyle::Solid,
        border_bottom_width: 1.0,
        border_bottom_style: BorderStyle::Solid,
        ..Default::default()
    };
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 1, 2);
    assert_eq!(result.len(), 2);
    // cell0 top: max(cell0-top=0 none, table-top=4 solid) → 4.
    assert!(approx_eq(result[0].top, 4.0));
    // cell0 bottom: resolve(cell0-bottom=2 solid, cell1-top=6 solid) → 6 (wider wins).
    assert!(approx_eq(result[0].bottom, 6.0));
    // cell1 top: resolve(cell1-top=6 solid, cell0-bottom=2 solid) → 6.
    assert!(approx_eq(result[1].top, 6.0));
    // cell1 bottom: resolve(cell1-bottom=0 none, table-bottom=1 solid) → 1.
    assert!(approx_eq(result[1].bottom, 1.0));
}

#[test]
fn collapse_border_numeric_left_right() {
    // 1-row, 2-col table. Cell0 border-right=3, Cell1 border-left=5.
    let cells = vec![make_cell(0, 0), make_cell(1, 0)];
    let styles = vec![
        ComputedStyle {
            border_right_width: 3.0,
            border_right_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_left_width: 5.0,
            border_left_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle {
        border_left_width: 2.0,
        border_left_style: BorderStyle::Solid,
        border_right_width: 7.0,
        border_right_style: BorderStyle::Solid,
        ..Default::default()
    };
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 2, 1);
    // cell0 left: resolve(cell0-left=0 none, table-left=2 solid) → 2.
    assert!(approx_eq(result[0].left, 2.0));
    // cell0 right: resolve(cell0-right=3, cell1-left=5) → 5 (wider wins).
    assert!(approx_eq(result[0].right, 5.0));
    // cell1 left: resolve(cell1-left=5, cell0-right=3) → 5.
    assert!(approx_eq(result[1].left, 5.0));
    // cell1 right: resolve(cell1-right=0 none, table-right=7 solid) → 7.
    assert!(approx_eq(result[1].right, 7.0));
}

#[test]
fn collapse_border_spanning_cell_checks_all_neighbors() {
    // 2-row, 2-col table. Row 0: one colspan=2 cell. Row 1: two cells with
    // different border-top widths. The spanning cell's bottom edge should
    // reflect the maximum across both neighbors.
    //
    // | cell0 (colspan=2)     |
    // | cell1 (top=3) | cell2 (top=8) |
    let cells = vec![
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 0,
            row: 0,
            colspan: 2,
            rowspan: 1,
        },
        make_cell(0, 1),
        make_cell(1, 1),
    ];
    let styles = vec![
        ComputedStyle {
            border_bottom_width: 1.0,
            border_bottom_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 3.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 8.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle::default();
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 2, 2);
    // cell0 bottom should resolve against both cell1 (top=3) and cell2 (top=8),
    // taking the max: resolve(1,3)=3, resolve(1,8)=8 → max=8.
    assert!(approx_eq(result[0].bottom, 8.0));
}
