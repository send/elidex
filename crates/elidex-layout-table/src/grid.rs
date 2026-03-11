//! Cell grid construction and occupancy tracking.
//!
//! Builds the (row, col) grid of table cells from the DOM, handling
//! colspan/rowspan, anonymous row wrapping, and WHATWG dimension limits.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::Display;

// ---------------------------------------------------------------------------
// Table dimension limits (WHATWG §4.9.11)
// ---------------------------------------------------------------------------

/// Maximum number of columns in a table (WHATWG colspan max).
pub(crate) const MAX_TABLE_COLS: usize = 1000;

/// Maximum number of rows in a table (WHATWG rowspan max).
pub(crate) const MAX_TABLE_ROWS: usize = 65534;

/// Maximum cells in the occupancy grid (~4 MB budget for `Vec<bool>`).
/// Tables exceeding this threshold are row-truncated.
pub(crate) const MAX_OCCUPANCY_CELLS: usize = 4_000_000;

// ---------------------------------------------------------------------------
// CellInfo
// ---------------------------------------------------------------------------

/// Metadata for a single table cell.
pub(crate) struct CellInfo {
    pub(crate) entity: Entity,
    pub(crate) col: usize,
    pub(crate) row: usize,
    pub(crate) colspan: u32,
    pub(crate) rowspan: u32,
}

// ---------------------------------------------------------------------------
// Span helpers
// ---------------------------------------------------------------------------

/// End row (exclusive) of a cell's row span, clamped to table bounds.
#[inline]
#[must_use]
pub(crate) fn span_end_row(cell: &CellInfo, num_rows: usize) -> usize {
    cell.row.saturating_add(cell.rowspan as usize).min(num_rows)
}

/// End column (exclusive) of a cell's column span, clamped to table bounds.
#[inline]
#[must_use]
pub(crate) fn span_end_col(cell: &CellInfo, num_cols: usize) -> usize {
    cell.col.saturating_add(cell.colspan as usize).min(num_cols)
}

// ---------------------------------------------------------------------------
// Row collection
// ---------------------------------------------------------------------------

/// Collect all rows from row groups and direct rows.
///
/// Row groups are ordered: thead first, then tbody, then tfoot.
/// Direct rows follow after all row-group rows.
#[must_use]
pub(crate) fn collect_all_rows(
    dom: &EcsDom,
    row_groups: &[Entity],
    direct_rows: &[Entity],
) -> Vec<Entity> {
    let mut rows = Vec::new();

    // Sort row groups by header/body/footer order.
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    let mut footers = Vec::new();
    for &rg in row_groups {
        let rg_style = elidex_layout_block::get_style(dom, rg);
        match rg_style.display {
            Display::TableHeaderGroup => headers.push(rg),
            Display::TableFooterGroup => footers.push(rg),
            _ => bodies.push(rg),
        }
    }

    // Headers first, then bodies, then footers.
    for groups in [&headers, &bodies, &footers] {
        for &rg in groups {
            let rg_children = dom.composed_children(rg);
            for &child in &rg_children {
                let child_style = elidex_layout_block::get_style(dom, child);
                if child_style.display == Display::TableRow {
                    rows.push(child);
                }
            }
        }
    }

    // Add direct rows (including any anonymous row wrapping direct cells).
    rows.extend(direct_rows);

    rows
}

// ---------------------------------------------------------------------------
// Cell grid construction
// ---------------------------------------------------------------------------

/// Build the cell grid from collected rows.
///
/// Returns (cells, `num_cols`, `num_rows`).
///
/// Rows beyond [`MAX_TABLE_ROWS`] are silently truncated; column positions
/// beyond [`MAX_TABLE_COLS`] cause remaining cells in that row to be skipped.
#[must_use]
pub(crate) fn build_cell_grid(dom: &EcsDom, rows: &[Entity]) -> (Vec<CellInfo>, usize, usize) {
    let mut cells = Vec::new();
    let mut num_rows = rows.len().min(MAX_TABLE_ROWS);
    // Budget check: limit rows so occupancy grid stays within memory budget.
    // Worst case each row could have MAX_TABLE_COLS columns of bool.
    if num_rows.saturating_mul(MAX_TABLE_COLS) > MAX_OCCUPANCY_CELLS {
        num_rows = MAX_OCCUPANCY_CELLS / MAX_TABLE_COLS;
    }
    if num_rows == 0 {
        return (cells, 0, 0);
    }

    // Occupancy grid: tracks which (row, col) positions are occupied by spanning cells.
    // Rows are pre-allocated; columns expand via `resize()`.
    let mut occupancy: Vec<Vec<bool>> = vec![Vec::new(); num_rows];
    let mut max_col = 0_usize;

    for (r, &row_entity) in rows.iter().enumerate().take(num_rows) {
        let row_children = dom.composed_children(row_entity);
        let mut col = 0_usize;

        for &child in &row_children {
            let child_style = elidex_layout_block::get_style(dom, child);
            if child_style.display == Display::None {
                continue;
            }

            // Read colspan/rowspan from HTML attributes.
            let (colspan, rowspan) = read_span_attributes(dom, child);

            // Find next unoccupied column.
            while col < occupancy[r].len() && occupancy[r][col] {
                col += 1;
            }

            // Cap column position to avoid unbounded growth.
            if col >= MAX_TABLE_COLS {
                break;
            }

            // Clamp colspan so it doesn't push past the column limit.
            let max_span = MAX_TABLE_COLS - col; // safe: col < MAX_TABLE_COLS
            let clamped_colspan = (colspan as usize).min(max_span);
            let span_end_col = col + clamped_colspan; // safe: <= MAX_TABLE_COLS
            let clamped_rowspan = (rowspan as usize).min(num_rows - r);
            let span_end_row = r + clamped_rowspan; // safe: <= num_rows

            // Mark occupancy for this cell's span.
            for occ_row in &mut occupancy[r..span_end_row] {
                if occ_row.len() < span_end_col {
                    occ_row.resize(span_end_col, false);
                }
                for slot in &mut occ_row[col..span_end_col] {
                    *slot = true;
                }
            }

            #[allow(clippy::cast_possible_truncation)] // clamped values fit in u32
            cells.push(CellInfo {
                entity: child,
                col,
                row: r,
                colspan: clamped_colspan as u32,
                rowspan: clamped_rowspan as u32,
            });

            max_col = max_col.max(span_end_col);
            col = span_end_col;
        }
    }

    (cells, max_col.min(MAX_TABLE_COLS), num_rows)
}

/// Read colspan and rowspan attributes from an entity.
///
/// Per WHATWG §4.9.11: colspan is clamped to `[1, 1000]`, rowspan to `[0, 65534]`.
/// `rowspan=0` (span all remaining rows) is not yet supported and is treated as 1.
#[must_use]
fn read_span_attributes(dom: &EcsDom, entity: Entity) -> (u32, u32) {
    const MAX_COLSPAN: u32 = 1000;
    const MAX_ROWSPAN: u32 = 65534;

    let (mut colspan, mut rowspan) = (1u32, 1u32);
    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
        if let Some(val) = attrs.get("colspan") {
            if let Ok(n) = val.parse::<u32>() {
                if n >= 1 {
                    colspan = n.min(MAX_COLSPAN);
                }
            }
        }
        if let Some(val) = attrs.get("rowspan") {
            if let Ok(n) = val.parse::<u32>() {
                // rowspan=0 means "span all remaining rows" (Phase 4);
                // for now treat as 1.
                if n >= 1 {
                    rowspan = n.min(MAX_ROWSPAN);
                }
            }
        }
    }
    (colspan, rowspan)
}

// ---------------------------------------------------------------------------
// Cell width calculation
// ---------------------------------------------------------------------------

/// Calculate the available width for a cell (sum of spanned columns + inter-column spacing).
#[must_use]
#[allow(clippy::cast_precision_loss)] // span counts are small integers
pub(crate) fn cell_available_width(
    col_widths: &[f32],
    cell: &CellInfo,
    spacing_h: f32,
    num_cols: usize,
) -> f32 {
    let w: f32 = col_widths[cell.col..span_end_col(cell, num_cols)]
        .iter()
        .sum();
    // Add spacing between spanned columns.
    let extra_spacing = spacing_h * (cell.colspan as f32 - 1.0).max(0.0);
    w + extra_spacing
}
