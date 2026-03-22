//! G6 tests: Row/RowGroup borders in collapsed border model (Step 3).

use super::*;
use elidex_plugin::{BorderSide, BorderStyle};

#[test]
fn row_border_beats_table_in_collapse() {
    // Row border (4px solid) should beat table border (2px solid).
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle::default()]; // no cell border
    let table_style = ComputedStyle {
        border_top: BorderSide {
            width: 2.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let row_styles = vec![ComputedStyle {
        border_top: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &[],
        &[],
        &[],
        1,
        1,
    );
    assert!(
        approx_eq(result[0].top, 4.0),
        "Row border (4) should beat table border (2), got {}",
        result[0].top
    );
}

#[test]
fn rowgroup_border_beats_table_in_collapse() {
    // RowGroup border should beat table border.
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle::default()];
    let table_style = ComputedStyle {
        border_top: BorderSide {
            width: 2.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let row_group_infos = vec![crate::RowGroupInfo {
        style: ComputedStyle {
            border_top: BorderSide {
                width: 5.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_row: 0,
        end_row: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &[],
        &row_group_infos,
        &[],
        &[],
        1,
        1,
    );
    assert!(
        approx_eq(result[0].top, 5.0),
        "RowGroup border (5) should beat table (2), got {}",
        result[0].top
    );
}

#[test]
fn cell_beats_row_beats_rowgroup_beats_table() {
    // Full priority chain: cell(8) > row(6) > rowgroup(4) > table(2).
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle {
        border_top: BorderSide {
            width: 8.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let table_style = ComputedStyle {
        border_top: BorderSide {
            width: 2.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let row_styles = vec![ComputedStyle {
        border_top: BorderSide {
            width: 6.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let row_group_infos = vec![crate::RowGroupInfo {
        style: ComputedStyle {
            border_top: BorderSide {
                width: 4.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_row: 0,
        end_row: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &row_group_infos,
        &[],
        &[],
        1,
        1,
    );
    assert!(
        approx_eq(result[0].top, 8.0),
        "Cell (8) should win, got {}",
        result[0].top
    );
}

#[test]
fn hidden_row_border_wins() {
    // A hidden row border should suppress cell border (CSS 2.1 §17.6.2.1).
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle {
        border_top: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let table_style = ComputedStyle::default();
    let row_styles = vec![ComputedStyle {
        border_top: BorderSide {
            width: 0.0,
            style: BorderStyle::Hidden,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &[],
        &[],
        &[],
        1,
        1,
    );
    // Hidden wins (width = 0).
    assert!(
        approx_eq(result[0].top, 0.0),
        "Hidden row border should win, got {}",
        result[0].top
    );
}

#[test]
fn rowgroup_bottom_edge() {
    // Two rows: row 0 in a rowgroup, row 1 direct.
    // RowGroup bottom border should apply at the bottom of row 0.
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let cell_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let table_style = ComputedStyle::default();
    let row_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let row_group_infos = vec![crate::RowGroupInfo {
        style: ComputedStyle {
            border_bottom: BorderSide {
                width: 6.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_row: 0,
        end_row: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &row_group_infos,
        &[],
        &[],
        1,
        2,
    );
    assert!(
        approx_eq(result[0].bottom, 6.0),
        "RowGroup bottom should be 6, got {}",
        result[0].bottom
    );
}

// ---------------------------------------------------------------------------
// Column/ColumnGroup border tests (CSS 2.1 §17.6.2.1)
// ---------------------------------------------------------------------------

#[test]
fn col_border_beats_table_in_collapse() {
    // Column border (5px solid left) should beat table border (2px solid left).
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle::default()];
    let table_style = ComputedStyle {
        border_left: BorderSide {
            width: 2.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let col_styles: Vec<Option<ComputedStyle>> = vec![Some(ComputedStyle {
        border_left: BorderSide {
            width: 5.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    })];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &[],
        &[],
        &col_styles,
        &[],
        1,
        1,
    );
    assert!(
        approx_eq(result[0].left, 5.0),
        "Col border (5) should beat table border (2), got {}",
        result[0].left
    );
}

#[test]
fn colgroup_border_beats_table_in_collapse() {
    // Colgroup border (6px solid right) should beat table border (3px solid right).
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle::default()];
    let table_style = ComputedStyle {
        border_right: BorderSide {
            width: 3.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let col_group_infos = vec![crate::ColGroupBorderInfo {
        style: ComputedStyle {
            border_right: BorderSide {
                width: 6.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_col: 0,
        end_col: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &[],
        &[],
        &[],
        &col_group_infos,
        1,
        1,
    );
    assert!(
        approx_eq(result[0].right, 6.0),
        "Colgroup border (6) should beat table border (3), got {}",
        result[0].right
    );
}

#[test]
fn full_priority_chain_with_col_colgroup() {
    // Full priority chain: cell(10) > row(8) > rowgroup(6) > col(4) > colgroup(2) > table(1).
    // Left edge: cell.col == 0, so table border is the neighbor.
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle {
        border_left: BorderSide {
            width: 10.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let table_style = ComputedStyle {
        border_left: BorderSide {
            width: 1.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let row_styles = vec![ComputedStyle {
        border_left: BorderSide {
            width: 8.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    }];
    let row_group_infos = vec![crate::RowGroupInfo {
        style: ComputedStyle {
            border_left: BorderSide {
                width: 6.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_row: 0,
        end_row: 1,
    }];
    let col_styles: Vec<Option<ComputedStyle>> = vec![Some(ComputedStyle {
        border_left: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    })];
    let col_group_infos = vec![crate::ColGroupBorderInfo {
        style: ComputedStyle {
            border_left: BorderSide {
                width: 2.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_col: 0,
        end_col: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &row_group_infos,
        &col_styles,
        &col_group_infos,
        1,
        1,
    );
    assert!(
        approx_eq(result[0].left, 10.0),
        "Cell (10) should win full priority chain, got {}",
        result[0].left
    );
}

#[test]
fn col_border_wins_over_colgroup_and_table() {
    // Column (4) > colgroup (3) > table (2), no cell/row/rowgroup border.
    let cells = vec![make_cell(0, 0)];
    let cell_styles = vec![ComputedStyle::default()];
    let table_style = ComputedStyle {
        border_left: BorderSide {
            width: 2.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    };
    let col_styles: Vec<Option<ComputedStyle>> = vec![Some(ComputedStyle {
        border_left: BorderSide {
            width: 4.0,
            style: BorderStyle::Solid,
            ..BorderSide::NONE
        },
        ..Default::default()
    })];
    let col_group_infos = vec![crate::ColGroupBorderInfo {
        style: ComputedStyle {
            border_left: BorderSide {
                width: 3.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        start_col: 0,
        end_col: 1,
    }];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &[],
        &[],
        &col_styles,
        &col_group_infos,
        1,
        1,
    );
    assert!(
        approx_eq(result[0].left, 4.0),
        "Col (4) > colgroup (3) > table (2), got {}",
        result[0].left
    );
}

// ---------------------------------------------------------------------------
// F-5: Left/right borders with rowspan > 1
// ---------------------------------------------------------------------------

#[test]
fn rowspan_left_right_checks_all_spanned_rows() {
    // Cell with rowspan=2. Row 0 has no left border, row 1 has left border (7px).
    // The cell's left edge should reflect row 1's border.
    //
    // | cell0 (rowspan=2) | cell1 (row 0) |
    // |                   | cell2 (row 1) |
    let cells = vec![
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 0,
            row: 0,
            colspan: 1,
            rowspan: 2,
        },
        make_cell(1, 0),
        make_cell(1, 1),
    ];
    let cell_styles = vec![
        ComputedStyle::default(), // cell0: no border
        ComputedStyle::default(),
        ComputedStyle::default(),
    ];
    let table_style = ComputedStyle::default();
    let row_styles = vec![
        ComputedStyle::default(), // row 0: no left border
        ComputedStyle {
            border_left: BorderSide {
                width: 7.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        }, // row 1: left border 7px
    ];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &[],
        &[],
        &[],
        2,
        2,
    );
    // cell0 left: should check both row 0 (0) and row 1 (7), picking 7.
    // At col 0, neighbor is table border (0). Best = 7 from row 1.
    assert!(
        approx_eq(result[0].left, 7.0),
        "Rowspan cell should check all spanned rows for left border, got {}",
        result[0].left
    );
}

// ---------------------------------------------------------------------------
// R-1: Neighbor rowgroup border at internal boundaries
// ---------------------------------------------------------------------------

#[test]
fn neighbor_rowgroup_bottom_border_at_internal_boundary() {
    // Two rowgroups: group A (row 0) with bottom border 6px,
    // group B (row 1) with no borders. Cell in row 1 should see
    // neighbor rowgroup A's bottom border on its top edge.
    //
    // | cell0 (row 0, group A) |
    // | cell1 (row 1, group B) |
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let cell_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let table_style = ComputedStyle::default();
    let row_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let row_group_infos = vec![
        crate::RowGroupInfo {
            style: ComputedStyle {
                border_bottom: BorderSide {
                    width: 6.0,
                    style: BorderStyle::Solid,
                    ..BorderSide::NONE
                },
                ..Default::default()
            },
            start_row: 0,
            end_row: 1,
        },
        crate::RowGroupInfo {
            style: ComputedStyle::default(),
            start_row: 1,
            end_row: 2,
        },
    ];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &row_group_infos,
        &[],
        &[],
        1,
        2,
    );
    // cell1 top: neighbor rowgroup A bottom border (6) should be considered.
    assert!(
        approx_eq(result[1].top, 6.0),
        "Neighbor rowgroup bottom border (6) should be seen at internal boundary, got {}",
        result[1].top
    );
    // Also: cell0 bottom should see the same border.
    assert!(
        approx_eq(result[0].bottom, 6.0),
        "Cell0 bottom should reflect rowgroup A bottom border (6), got {}",
        result[0].bottom
    );
}

#[test]
fn neighbor_rowgroup_top_border_at_internal_boundary() {
    // Two rowgroups: group A (row 0) with no borders,
    // group B (row 1) with top border 7px. Cell in row 0 should see
    // neighbor rowgroup B's top border on its bottom edge.
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let cell_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let table_style = ComputedStyle::default();
    let row_styles = vec![ComputedStyle::default(), ComputedStyle::default()];
    let row_group_infos = vec![
        crate::RowGroupInfo {
            style: ComputedStyle::default(),
            start_row: 0,
            end_row: 1,
        },
        crate::RowGroupInfo {
            style: ComputedStyle {
                border_top: BorderSide {
                    width: 7.0,
                    style: BorderStyle::Solid,
                    ..BorderSide::NONE
                },
                ..Default::default()
            },
            start_row: 1,
            end_row: 2,
        },
    ];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &row_group_infos,
        &[],
        &[],
        1,
        2,
    );
    // cell0 bottom: neighbor rowgroup B top border (7) should be considered.
    assert!(
        approx_eq(result[0].bottom, 7.0),
        "Neighbor rowgroup top border (7) should be seen at internal boundary, got {}",
        result[0].bottom
    );
}

// ---------------------------------------------------------------------------
// R-2: Hidden neighbor border suppresses visible border
// ---------------------------------------------------------------------------

#[test]
fn hidden_neighbor_cell_border_suppresses_visible() {
    // Two cells side by side. Cell 0 has solid right border (5px).
    // Cell 1 has hidden left border. Hidden should suppress cell 0's right.
    //
    // | cell0 | cell1 |
    let cells = vec![make_cell(0, 0), make_cell(1, 0)];
    let cell_styles = vec![
        ComputedStyle {
            border_right: BorderSide {
                width: 5.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        ComputedStyle {
            border_left: BorderSide {
                width: 0.0,
                style: BorderStyle::Hidden,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle::default();
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &[],
        &[],
        &[],
        &[],
        2,
        1,
    );
    // Cell 0's right edge: hidden from cell 1 should win → 0.
    assert!(
        approx_eq(result[0].right, 0.0),
        "Hidden neighbor cell border should suppress visible (5), got {}",
        result[0].right
    );
    // Cell 1's left edge: same boundary, hidden should win → 0.
    assert!(
        approx_eq(result[1].left, 0.0),
        "Hidden cell border should produce 0, got {}",
        result[1].left
    );
}

#[test]
fn hidden_neighbor_row_border_suppresses_visible_cell() {
    // Cell in row 0 has solid bottom border (4px).
    // Row 1 has hidden top border. Hidden should suppress.
    //
    // | cell0 (row 0) |
    // | cell1 (row 1) |
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let cell_styles = vec![
        ComputedStyle {
            border_bottom: BorderSide {
                width: 4.0,
                style: BorderStyle::Solid,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
        ComputedStyle::default(),
    ];
    let table_style = ComputedStyle::default();
    let row_styles = vec![
        ComputedStyle::default(),
        ComputedStyle {
            border_top: BorderSide {
                width: 0.0,
                style: BorderStyle::Hidden,
                ..BorderSide::NONE
            },
            ..Default::default()
        },
    ];
    let result = algo::resolve_collapsed_borders(
        &cells,
        &cell_styles,
        &table_style,
        &row_styles,
        &[],
        &[],
        &[],
        1,
        2,
    );
    // Cell 0 bottom: hidden neighbor row border should suppress cell's 4px solid.
    assert!(
        approx_eq(result[0].bottom, 0.0),
        "Hidden neighbor row border should suppress cell bottom (4), got {}",
        result[0].bottom
    );
}
