use super::*;

// ---------------------------------------------------------------------------
// colspan / rowspan tests
// ---------------------------------------------------------------------------

#[test]
fn table_colspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: one cell spanning 2 columns.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("colspan", "2");
    let td1 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Row 2: two cells.
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let cell_r2c1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_r2c1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, cell_r2c1);
    let cell_r2c2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_r2c2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, cell_r2c2);

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

    // The spanning cell should exist and have a LayoutBox.
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    // The second row cells should also have layout boxes.
    assert!(dom.world().get::<&LayoutBox>(cell_r2c1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(cell_r2c2).is_ok());
}

#[test]
fn table_rowspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: cell with rowspan=2 + normal cell.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("rowspan", "2");
    let td1 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td2);

    // Row 2: only one cell (col 0 occupied by rowspan).
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td3 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td3,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td3);

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

    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td3).is_ok());
}

#[test]
fn table_colspan_and_rowspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: cell(colspan=2, rowspan=2) + cell.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("colspan", "2");
    attrs.set("rowspan", "2");
    let td_span = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td_span,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(40.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td_span);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Row 2: only one cell at col 2.
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2);

    let font_db = FontDatabase::new();
    layout_table(
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

    assert!(dom.world().get::<&LayoutBox>(td_span).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

// ---------------------------------------------------------------------------
// Caption tests
// ---------------------------------------------------------------------------

#[test]
fn table_caption_top() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            height: Dimension::Length(30.0),
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

    let caption_lb = get_layout(&dom, caption);
    let td_lb = get_layout(&dom, td);
    // Caption should be above the table rows.
    assert!(caption_lb.content.y < td_lb.content.y);
}

#[test]
fn table_caption_bottom() {
    use elidex_plugin::CaptionSide;

    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::Bottom,
            height: Dimension::Length(30.0),
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

    let caption_lb = get_layout(&dom, caption);
    let td_lb = get_layout(&dom, td);
    // Caption with caption-side: bottom should be below the table rows.
    assert!(
        caption_lb.content.y > td_lb.content.y,
        "caption y={} should be below td y={}",
        caption_lb.content.y,
        td_lb.content.y
    );
}

// ---------------------------------------------------------------------------
// thead / tbody / tfoot ordering
// ---------------------------------------------------------------------------

#[test]
fn table_header_body_footer() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // tbody
    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tbody);

    let tr_body = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_body,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr_body);

    let cell_body = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_body,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_body, cell_body);

    // thead (added after tbody in DOM, but should be rendered first).
    let thead = dom.create_element("thead", Attributes::default());
    dom.world_mut().insert_one(
        thead,
        ComputedStyle {
            display: Display::TableHeaderGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, thead);

    let tr_head = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_head,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(thead, tr_head);

    let cell_head = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_head,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_head, cell_head);

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

    let head_lb = get_layout(&dom, cell_head);
    let body_lb = get_layout(&dom, cell_body);
    // Header row should come before body row.
    assert!(
        head_lb.content.y < body_lb.content.y,
        "thead should be before tbody: head_y={} body_y={}",
        head_lb.content.y,
        body_lb.content.y
    );
}

#[test]
fn table_tfoot_after_tbody() {
    // tfoot rows should appear after tbody rows even if tfoot is first in DOM.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // tfoot (first in DOM)
    let tfoot = dom.create_element("tfoot", Attributes::default());
    dom.world_mut().insert_one(
        tfoot,
        ComputedStyle {
            display: Display::TableFooterGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tfoot);
    let tr_foot = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_foot,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tfoot, tr_foot);
    let cell_foot = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_foot,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_foot, cell_foot);

    // tbody (second in DOM)
    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tbody);
    let tr_body = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_body,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr_body);
    let cell_body = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_body,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_body, cell_body);

    let font_db = FontDatabase::new();
    layout_table(
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

    let body_lb = get_layout(&dom, cell_body);
    let foot_lb = get_layout(&dom, cell_foot);
    // tbody rows should come before tfoot rows.
    assert!(
        body_lb.content.y < foot_lb.content.y,
        "tbody cell y ({}) should be < tfoot cell y ({})",
        body_lb.content.y,
        foot_lb.content.y,
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn table_empty_cells() {
    let (mut dom, table, _) = create_simple_table(1, 2, default_table_style());
    // Give cells zero height to simulate empty cells.
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
    // Should not panic.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_single_cell() {
    let (mut dom, table, cells) = create_simple_table(1, 1, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 300.0));
    assert!(dom.world().get::<&LayoutBox>(cells[0]).is_ok());
}

#[test]
fn table_direct_cells_anonymous_row() {
    // Direct table-cell children of a table (without a tr) should be
    // wrapped in an anonymous row per CSS 2.1 §17.2.1.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Add cells directly to the table (no <tr>).
    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td2);

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

    // Both cells should have been laid out (not silently dropped).
    assert!(
        dom.world().get::<&LayoutBox>(td1).is_ok(),
        "direct cell td1 should have LayoutBox"
    );
    assert!(
        dom.world().get::<&LayoutBox>(td2).is_ok(),
        "direct cell td2 should have LayoutBox"
    );
}

#[test]
fn table_rowspan_height_extension() {
    // When a rowspan cell is taller than the combined spanned rows,
    // the last spanned row should be extended.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: td1 (rowspan=2, height=100) + td2 (height=20)
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut td1_attrs = Attributes::default();
    td1_attrs.set("rowspan", "2");
    let td1 = dom.create_element("td", td1_attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td2);

    // Row 2: td3 (height=20)
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td3 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td3,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td3);

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

    let td1_lb = get_layout(&dom, td1);
    // The rowspan cell should span the full height of both rows.
    // Row 1 base height = 20, row 2 base height = 20 → total = 40.
    // But td1 needs 100, so row 2 should be extended by 80 → total = 100.
    assert!(
        td1_lb.content.height >= 100.0 - 1.0,
        "rowspan cell height should be at least 100, got {}",
        td1_lb.content.height
    );
}

#[test]
fn table_empty_zero_children() {
    // Table with no children at all should produce a valid LayoutBox.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let font_db = FontDatabase::new();
    layout_table(
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
    assert!(lb.content.width >= 0.0);
    assert!(lb.content.height >= 0.0);
}

#[test]
fn table_display_none_cell_skipped() {
    // A cell with display:none should not occupy a grid position.
    // Row: [visible_cell] [display:none] [visible_cell]
    // Expected: 2 columns (not 3).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            width: Dimension::Length(300.0),
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

    let td_hidden = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td_hidden,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    dom.append_child(tr, td_hidden);

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
    layout_table(
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

    let lb1 = get_layout(&dom, td1);
    let lb2 = get_layout(&dom, td2);
    // Two visible cells in a 300px table should each get ~150px (minus spacing).
    // The hidden cell should not create a third column.
    assert!(
        lb1.content.width > 100.0,
        "td1 width {} should be > 100px (2-col, not 3-col)",
        lb1.content.width
    );
    assert!(
        lb2.content.width > 100.0,
        "td2 width {} should be > 100px (2-col, not 3-col)",
        lb2.content.width
    );
    // td_hidden should not have a LayoutBox.
    assert!(
        dom.world().get::<&LayoutBox>(td_hidden).is_err(),
        "display:none cell should not have a LayoutBox"
    );
}

// ---------------------------------------------------------------------------
// M3.5-4: RTL direction support
// ---------------------------------------------------------------------------

#[test]
fn table_rtl_reverses_column_order() {
    // direction: rtl → columns placed right-to-left
    let table_style = ComputedStyle {
        display: Display::Table,
        direction: elidex_plugin::Direction::Rtl,
        ..Default::default()
    };
    let (mut dom, table, cells) = create_simple_table(1, 3, table_style);

    let font_db = FontDatabase::new();
    layout_table(
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

    // RTL: first column should be rightmost, last column leftmost.
    assert!(
        lb0.content.x > lb1.content.x,
        "RTL table: col 0 (x={}) should be right of col 1 (x={})",
        lb0.content.x,
        lb1.content.x,
    );
    assert!(
        lb1.content.x > lb2.content.x,
        "RTL table: col 1 (x={}) should be right of col 2 (x={})",
        lb1.content.x,
        lb2.content.x,
    );
}
