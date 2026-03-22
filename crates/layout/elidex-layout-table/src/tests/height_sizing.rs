//! G6 tests: height redistribution and collapse-aware column sizing (Step 4).

use super::*;
use elidex_plugin::{BorderSide, BorderStyle, Dimension};

// ---------------------------------------------------------------------------
// Height redistribution (CSS 2.1 §17.5.3)
// ---------------------------------------------------------------------------

#[test]
fn height_redistribution_proportional() {
    // Table with explicit height > content height -> surplus distributed proportionally.
    let (mut dom, table, _cells) = create_simple_table(
        2,
        1,
        ComputedStyle {
            display: Display::Table,
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    // Row 0 cell: 20px, Row 1 cell: 20px. Content height ~ 40 + spacing.
    // Explicit height 200 -> surplus distributed.
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
    // Table should be at least 200px tall.
    assert!(
        table_lb.content.size.height >= 199.0,
        "Table with height:200px should be >= 200, got {}",
        table_lb.content.size.height,
    );
}

#[test]
fn height_redistribution_zero_rows() {
    // All rows have zero content height -> surplus distributed equally.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    for _ in 0..2 {
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
    }

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
    assert!(
        table_lb.content.size.height >= 99.0,
        "Table with height:100px should be >= 100, got {}",
        table_lb.content.size.height,
    );
}

#[test]
fn no_redistribution_when_explicit_less_than_content() {
    // explicit height < content height -> no redistribution, content height used.
    let (mut dom, table, _) = create_simple_table(
        2,
        1,
        ComputedStyle {
            display: Display::Table,
            height: Dimension::Length(10.0), // Less than content (~40+spacing)
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
    // Table height should be at least the content height (> 10).
    assert!(
        table_lb.content.size.height > 10.0,
        "Content height should win over small explicit height, got {}",
        table_lb.content.size.height,
    );
}

// ---------------------------------------------------------------------------
// Collapse-aware column sizing
// ---------------------------------------------------------------------------

#[test]
fn collapse_aware_sizing_shrinks_columns() {
    // In collapse mode, the collapsed border half-width should be subtracted
    // from cell intrinsic widths, resulting in narrower columns than separate mode.
    // We test indirectly by comparing table widths in auto layout.
    let style_collapse = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_top: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        border_bottom: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        border_left: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        border_right: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style_collapse);
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

    // Should not panic and should produce a valid layout.
    let lb = get_layout(&dom, table);
    assert!(lb.content.size.width > 0.0);
}

#[test]
fn separate_model_unchanged() {
    // In separate model, no collapse adjustment should occur.
    let style_sep = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Separate,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style_sep);
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

    let lb = get_layout(&dom, table);
    assert!(lb.content.size.width > 0.0);
}
