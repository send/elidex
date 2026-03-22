//! G6 tests: anonymous table object generation (Steps 1-2).
//!
//! Step 1: Idempotent anonymous row wrapping.
//! Step 2: Full anonymous table object generation (CSS 2.1 §17.2.1).

use super::*;
use elidex_ecs::AnonymousTableMarker;
use elidex_plugin::Dimension;

// ---------------------------------------------------------------------------
// Step 1: Idempotent anonymous row wrapping
// ---------------------------------------------------------------------------

#[test]
fn idempotent_anonymous_row_relayout() {
    // Direct cells should be wrapped in an anonymous row.
    // Calling layout_table twice should NOT create a second anonymous row.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td);

    let font_db = FontDatabase::new();
    // First layout.
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

    // Count anonymous rows.
    let count1 = dom
        .composed_children(table)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();

    // Second layout.
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

    let count2 = dom
        .composed_children(table)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();

    assert_eq!(
        count1, 1,
        "Should have exactly 1 anonymous row after first layout"
    );
    assert_eq!(
        count2, 1,
        "Should still have exactly 1 anonymous row after re-layout"
    );
}

#[test]
fn anonymous_row_marker_preserved() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td);

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

    // The anonymous row should have AnonymousTableMarker.
    let children = dom.composed_children(table);
    let anon_row = children
        .iter()
        .find(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok());
    assert!(
        anon_row.is_some(),
        "Anonymous row should have AnonymousTableMarker"
    );
}

#[test]
fn no_anonymous_row_without_direct_cells() {
    // A table with only <tr> children should not generate anonymous rows.
    let (mut dom, table, _) = create_simple_table(2, 2, default_table_style());
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

    let anon_count = dom
        .composed_children(table)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();
    assert_eq!(
        anon_count, 0,
        "No anonymous rows needed when rows have proper <tr> parents"
    );
}

// ---------------------------------------------------------------------------
// Step 2: Full anonymous table object generation
// ---------------------------------------------------------------------------

#[test]
fn non_cell_content_wrapped_in_anonymous_cell() {
    // A <span> inside a <tr> should be wrapped in an anonymous table-cell.
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

    // Normal cell.
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

    // Non-cell content (will be wrapped in anonymous td).
    let span = dom.create_element("span", Attributes::default());
    dom.world_mut().insert_one(
        span,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(15.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, span);

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

    // The span should have been laid out (via anonymous cell wrapping).
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
    assert!(dom.world().get::<&LayoutBox>(span).is_ok());
}

#[test]
fn mixed_cell_and_non_cell_in_row() {
    // <tr> with [td, div, td] — the div should be wrapped in anonymous cell.
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

    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, div);

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
        Point::ZERO,
        &font_db,
        0,
        test_layout_child,
    );

    // All three elements should have layout boxes.
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(div).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn idempotent_non_cell_wrapping() {
    // Re-layout should not create duplicate anonymous cells.
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

    let span = dom.create_element("span", Attributes::default());
    dom.world_mut().insert_one(
        span,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, span);

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
    let count1 = dom
        .composed_children(tr)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();

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
    let count2 = dom
        .composed_children(tr)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();

    assert_eq!(count1, 1);
    assert_eq!(
        count2, 1,
        "Re-layout should not create duplicate anonymous cells"
    );
}

#[test]
fn row_group_non_row_children_wrapped() {
    // Non-row children inside a <tbody> should be wrapped in anonymous rows.
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

    // A cell directly in tbody (should be wrapped in anonymous row).
    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tbody, td);

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

    // The cell should have been laid out successfully.
    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Cell in rowgroup via anonymous row"
    );
}

#[test]
fn row_group_mixed_row_and_non_row() {
    // <tbody> with [tr, div, tr] — the div should be wrapped in anonymous row.
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

    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr1);
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

    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(15.0),
            ..Default::default()
        },
    );
    dom.append_child(tbody, div);

    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr2);
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

    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(div).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

// ---------------------------------------------------------------------------
// F-1: Pool-based anonymous entity creation (multiple runs)
// ---------------------------------------------------------------------------

#[test]
fn multiple_non_cell_runs_get_separate_anonymous_cells() {
    // <tr> with [span1, td, span2] — two separate non-cell runs.
    // Each should get a distinct anonymous cell, not reuse the same one.
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

    let span1 = dom.create_element("span", Attributes::default());
    dom.world_mut().insert_one(
        span1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, span1);

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

    let span2 = dom.create_element("span", Attributes::default());
    dom.world_mut().insert_one(
        span2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(15.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, span2);

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

    // Should have exactly 2 anonymous cells (one per non-cell run).
    let anon_count = dom
        .composed_children(tr)
        .iter()
        .filter(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .count();
    assert_eq!(
        anon_count, 2,
        "Two separate non-cell runs should produce 2 anonymous cells, got {anon_count}",
    );

    // All content should be laid out.
    assert!(dom.world().get::<&LayoutBox>(span1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
    assert!(dom.world().get::<&LayoutBox>(span2).is_ok());
}

// ---------------------------------------------------------------------------
// F-2: Empty row group handling
// ---------------------------------------------------------------------------

#[test]
fn empty_row_group_does_not_break_layout() {
    // A <tbody> with no children should produce empty RowGroupInfo (start_row == end_row).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Empty tbody.
    let empty_tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        empty_tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, empty_tbody);

    // Non-empty tbody.
    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr);

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
    // Should not panic even with empty row group.
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
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

// ---------------------------------------------------------------------------
// F-8: Style inheritance for anonymous table entities
// ---------------------------------------------------------------------------

#[test]
fn anonymous_row_inherits_parent_style() {
    // Anonymous rows should inherit color, font-size, etc. from parent.
    use elidex_plugin::CssColor;

    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            color: CssColor::new(255, 0, 0, 255),
            font_size: 24.0,
            ..Default::default()
        },
    );

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td);

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

    // Find the anonymous row.
    let children = dom.composed_children(table);
    let anon_row = children
        .iter()
        .find(|&&c| dom.world().get::<&AnonymousTableMarker>(c).is_ok())
        .copied();
    assert!(anon_row.is_some(), "Should have anonymous row");

    let anon_style = dom
        .world()
        .get::<&ComputedStyle>(anon_row.unwrap())
        .map(|s| (*s).clone())
        .unwrap();
    assert_eq!(anon_style.color, CssColor::new(255, 0, 0, 255));
    assert!((anon_style.font_size - 24.0).abs() < f32::EPSILON);
    // Non-inherited properties should be default.
    assert_eq!(anon_style.display, Display::TableRow);
}
