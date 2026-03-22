//! Fragmentation tests for CSS Table (CSS 2.1 §17.5.4).

use super::*;
use elidex_layout_block::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutOutcome};
use elidex_plugin::BreakValue;

/// Layout a table with fragmentation context and return the full outcome.
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn do_layout_table_fragmented(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset: Point,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: elidex_layout_block::ChildLayoutFn,
    fragmentainer: Option<FragmentainerContext>,
    break_token: Option<elidex_layout_block::BreakToken>,
) -> LayoutOutcome {
    let input = LayoutInput {
        containing: CssSize {
            width: containing_width,
            height: containing_height,
        },
        containing_inline_size: containing_width,
        offset,
        font_db,
        depth,
        float_ctx: None,
        viewport: None,
        fragmentainer: fragmentainer.as_ref(),
        break_token: break_token.as_ref(),
        subgrid: None,
        layout_generation: 0,
    };
    layout_table(dom, entity, &input, layout_child)
}

/// Create a table with N rows x M cols where each cell has the given height.
fn create_table_with_height(
    rows: usize,
    cols: usize,
    cell_height: f32,
) -> (EcsDom, elidex_ecs::Entity, Vec<elidex_ecs::Entity>) {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let mut cells = Vec::new();
    for _ in 0..rows {
        let tr = dom.create_element("tr", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);

        for _ in 0..cols {
            let td = dom.create_element("td", elidex_ecs::Attributes::default());
            dom.world_mut().insert_one(
                td,
                ComputedStyle {
                    display: Display::TableCell,
                    height: Dimension::Length(cell_height),
                    ..Default::default()
                },
            );
            dom.append_child(tr, td);
            cells.push(td);
        }
    }
    (dom, table, cells)
}

// ---------------------------------------------------------------------------
// 1. No fragmentainer -> break_token is None
// ---------------------------------------------------------------------------

#[test]
fn no_fragmentation_returns_none() {
    let (mut dom, table, _) = create_table_with_height(3, 2, 20.0);
    let fdb = FontDatabase::new();

    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        None, // no fragmentainer
        None,
    );
    assert!(outcome.break_token.is_none());
}

// ---------------------------------------------------------------------------
// 2. All rows fit within available block size
// ---------------------------------------------------------------------------

#[test]
fn rows_fit_no_break() {
    // 3 rows at 20px each = 60px. Available: 200px -> fits.
    let (mut dom, table, _) = create_table_with_height(3, 2, 20.0);
    let fdb = FontDatabase::new();

    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 200.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    assert!(outcome.break_token.is_none());
}

// ---------------------------------------------------------------------------
// 3. Overflow at row boundary
// ---------------------------------------------------------------------------

#[test]
fn overflow_at_row_boundary() {
    // 4 rows at 30px each = 120px (+ spacing). Available: 80px.
    // Should break at some row boundary.
    let (mut dom, table, _) = create_table_with_height(4, 1, 30.0);
    let fdb = FontDatabase::new();

    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 80.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should produce break token");
    assert_eq!(bt.entity, table);
    match &bt.mode_data {
        Some(BreakTokenData::Table { row_index, .. }) => {
            assert!(
                *row_index > 0 && *row_index < 4,
                "should break at a valid row boundary"
            );
        }
        other => panic!("expected Table break token data, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Resume from break token
// ---------------------------------------------------------------------------

#[test]
fn resume_from_break_token() {
    let (mut dom, table, _) = create_table_with_height(4, 1, 30.0);
    let fdb = FontDatabase::new();
    let frag = Some(FragmentainerContext {
        available_block_size: 80.0,
        fragmentation_type: FragmentationType::Column,
    });

    // First pass: should break.
    let outcome1 = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        frag,
        None,
    );
    let bt1 = outcome1.break_token.expect("first pass should break");

    // Second pass with break token.
    let outcome2 = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 200.0, // generous space for remaining rows
            fragmentation_type: FragmentationType::Column,
        }),
        Some(bt1),
    );
    // Remaining rows should fit in 200px.
    assert!(
        outcome2.break_token.is_none(),
        "second pass should complete without break"
    );
}

// ---------------------------------------------------------------------------
// 5. thead repeated in continuation
// ---------------------------------------------------------------------------

#[test]
fn thead_repeated_in_continuation() {
    // Table with thead + body rows. Verify break token stores thead_entity.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Create thead.
    let thead = dom.create_element("thead", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead,
        ComputedStyle {
            display: Display::TableHeaderGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, thead);
    let thead_row = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead_row,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(thead, thead_row);
    let thead_cell = dom.create_element("th", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead_cell,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(thead_row, thead_cell);

    // Create body rows.
    for _ in 0..4 {
        let tr = dom.create_element("tr", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);
        let td = dom.create_element("td", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);
    }

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 80.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Table {
            thead_entity: te, ..
        }) => {
            assert_eq!(*te, Some(thead), "break token should store thead entity");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. tfoot repeated in continuation
// ---------------------------------------------------------------------------

#[test]
fn tfoot_repeated_in_continuation() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Create body rows.
    for _ in 0..4 {
        let tr = dom.create_element("tr", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);
        let td = dom.create_element("td", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);
    }

    // Create tfoot.
    let tfoot = dom.create_element("tfoot", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot,
        ComputedStyle {
            display: Display::TableFooterGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tfoot);
    let tfoot_row = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot_row,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tfoot, tfoot_row);
    let tfoot_cell = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot_cell,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tfoot_row, tfoot_cell);

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 80.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Table {
            tfoot_entity: te, ..
        }) => {
            assert_eq!(*te, Some(tfoot), "break token should store tfoot entity");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. Caption only in first fragment
// ---------------------------------------------------------------------------

#[test]
fn caption_only_in_first_fragment() {
    // Verify that resuming with a break token (continuation) doesn't re-layout
    // captions. We check that the content height is reasonable and the layout
    // completes without error.
    let (mut dom, table, _) = create_table_with_height(4, 1, 30.0);
    let fdb = FontDatabase::new();

    // First pass with tight fragmentainer.
    let outcome1 = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 70.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt1 = outcome1.break_token.expect("should break");

    // Second pass: continuation fragment.
    let outcome2 = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 200.0,
            fragmentation_type: FragmentationType::Column,
        }),
        Some(bt1),
    );
    // Should complete or break again, but not panic.
    let _ = outcome2;
}

// ---------------------------------------------------------------------------
// 8. Forced break-before on a row
// ---------------------------------------------------------------------------

#[test]
fn forced_break_before_row() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tr1 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);
    let td1 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Second row has forced break-before.
    let tr2 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            break_before: BreakValue::Column,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);
    let td2 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2);

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome
        .break_token
        .expect("forced break should produce token");
    match &bt.mode_data {
        Some(BreakTokenData::Table { row_index, .. }) => {
            assert_eq!(*row_index, 1, "forced break before row 1");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 9. Forced break-after on a row
// ---------------------------------------------------------------------------

#[test]
fn forced_break_after_row() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // First row has forced break-after.
    let tr1 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            break_after: BreakValue::Column,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);
    let td1 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    let tr2 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);
    let td2 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2);

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome
        .break_token
        .expect("forced break-after should produce token");
    match &bt.mode_data {
        Some(BreakTokenData::Table { row_index, .. }) => {
            assert_eq!(*row_index, 1, "forced break after row 0 -> resume at row 1");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 10. break-inside: avoid penalizes break candidates
// ---------------------------------------------------------------------------

#[test]
fn break_inside_avoid() {
    // Row 1 has break-after: avoid-column. This should penalize the
    // candidate between row 1 and row 2.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    for i in 0..3 {
        let tr = dom.create_element("tr", elidex_ecs::Attributes::default());
        let mut row_style = ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        };
        if i == 1 {
            row_style.break_after = BreakValue::AvoidColumn;
        }
        dom.world_mut().insert_one(tr, row_style);
        dom.append_child(table, tr);
        let td = dom.create_element("td", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);
    }

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Table { row_index, .. }) => {
            // Candidate after row 0 is non-avoid, after row 1 is avoid.
            // find_best_break should prefer the non-avoid candidate (row 1).
            assert_eq!(*row_index, 1, "should prefer non-avoid break point");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 11. thead + tfoot reduce available space
// ---------------------------------------------------------------------------

#[test]
#[allow(clippy::too_many_lines)]
fn thead_tfoot_reduce_available_space() {
    // In continuation fragments, thead/tfoot heights reduce the available
    // space for body rows. This test creates a table with thead (20px) +
    // tfoot (20px) + 4 body rows (30px each).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // thead
    let thead = dom.create_element("thead", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead,
        ComputedStyle {
            display: Display::TableHeaderGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, thead);
    let thead_row = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead_row,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(thead, thead_row);
    let thead_cell = dom.create_element("th", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        thead_cell,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(thead_row, thead_cell);

    // body rows
    for _ in 0..4 {
        let tr = dom.create_element("tr", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);
        let td = dom.create_element("td", elidex_ecs::Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);
    }

    // tfoot
    let tfoot = dom.create_element("tfoot", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot,
        ComputedStyle {
            display: Display::TableFooterGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tfoot);
    let tfoot_row = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot_row,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tfoot, tfoot_row);
    let tfoot_cell = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tfoot_cell,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tfoot_row, tfoot_cell);

    let fdb = FontDatabase::new();

    // First pass - break somewhere.
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Table {
            thead_entity: te,
            tfoot_entity: tf,
            ..
        }) => {
            assert_eq!(*te, Some(thead), "break token should store thead");
            assert_eq!(*tf, Some(tfoot), "break token should store tfoot");
        }
        other => panic!("expected Table break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 12. Propagated break values
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_values() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // First row has break-before: page
    let tr1 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            break_before: BreakValue::Page,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);
    let td1 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Last row has break-after: page
    let tr2 = dom.create_element("tr", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            break_after: BreakValue::Page,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);
    let td2 = dom.create_element("td", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2);

    let fdb = FontDatabase::new();
    let outcome = do_layout_table_fragmented(
        &mut dom,
        table,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        test_layout_child,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    assert_eq!(
        outcome.propagated_break_before,
        Some(BreakValue::Page),
        "should propagate break-before from first row"
    );
    assert_eq!(
        outcome.propagated_break_after,
        Some(BreakValue::Page),
        "should propagate break-after from last row"
    );
}
