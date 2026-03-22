//! G6 tests: inline-table layout and baseline (Step 7).

use super::*;
use elidex_plugin::Dimension;

#[test]
fn inline_table_layout() {
    // Inline-table should produce a valid layout when used within table layout.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
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
            width: Dimension::Length(100.0),
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let lb = get_layout(&dom, table);
    assert!(lb.content.size.width > 0.0);
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

#[test]
fn inline_table_baseline() {
    // Inline-table baseline should come from the first row.
    let (mut dom, table, _cells) = create_simple_table(
        2,
        1,
        ComputedStyle {
            display: Display::Table,
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    do_layout_table(
        &mut dom,
        table,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    let table_lb = get_layout(&dom, table);
    // Table baseline should be set (first row).
    assert!(
        table_lb.first_baseline.is_some(),
        "Table should have a first_baseline"
    );
}
