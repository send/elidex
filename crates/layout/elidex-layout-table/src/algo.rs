//! Table layout sizing algorithms and border-collapse logic.
//!
//! Implements CSS 2.1 §17.5 (table-layout: auto and fixed) and
//! §17.6 (border-collapse: collapse).

use elidex_plugin::{BorderStyle, ComputedStyle};

use crate::{span_end_col, span_end_row, CellInfo, ColGroupBorderInfo, RowGroupInfo};

/// Maximum cells in the collapsed border grid (~8 MB budget).
/// Tables exceeding this threshold fall back to separate borders.
const MAX_COLLAPSE_GRID_CELLS: usize = 1_000_000;

/// Sanitize a float for layout: replace non-finite or negative values with 0.0.
#[inline]
fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// table-layout: auto (CSS 2.1 §17.5.2.2)
// ---------------------------------------------------------------------------

/// Distribute available width among columns using the auto table layout algorithm.
///
/// Returns a vector of column widths (one per column).
///
/// Algorithm:
/// 1. Each column gets `min_width` = max(cell minimum content widths in column)
/// 2. Each column gets `max_width` = max(cell maximum content widths in column)
/// 3. If total min >= available, each column gets `min_width`
/// 4. If total max <= available, each column gets `max_width`, remainder distributed equally
/// 5. Otherwise, proportionally distribute between min and max
#[must_use]
#[allow(clippy::cast_precision_loss)] // column/span counts are small integers
pub fn auto_column_widths(
    num_cols: usize,
    cells: &[CellInfo],
    cell_content_widths: &[(f32, f32)],
    available: f32,
) -> Vec<f32> {
    if num_cols == 0 {
        return Vec::new();
    }

    let available = sanitize_f32(available);
    let mut min_widths = vec![0.0_f32; num_cols];
    let mut max_widths = vec![0.0_f32; num_cols];

    for (cell, &(raw_min, raw_max)) in cells.iter().zip(cell_content_widths.iter()) {
        let min_w = sanitize_f32(raw_min);
        let max_w = sanitize_f32(raw_max).max(min_w);
        if cell.colspan == 1 {
            min_widths[cell.col] = min_widths[cell.col].max(min_w);
            max_widths[cell.col] = max_widths[cell.col].max(max_w);
        } else {
            // Spanning cells: distribute proportionally across spanned columns
            // based on existing column widths (CSS 2.1 §17.5.2.2).
            let col_end = span_end_col(cell, num_cols);
            let existing_sum: f32 = (cell.col..col_end)
                .map(|c| max_widths[c].max(min_widths[c]).max(1.0))
                .sum();
            for c in cell.col..col_end {
                let weight = max_widths[c].max(min_widths[c]).max(1.0) / existing_sum;
                min_widths[c] = min_widths[c].max(min_w * weight);
                max_widths[c] = max_widths[c].max(max_w * weight);
            }
        }
    }

    // Ensure max >= min for each column.
    for (max_w, min_w) in max_widths.iter_mut().zip(min_widths.iter()) {
        *max_w = (*max_w).max(*min_w);
    }

    let total_min: f32 = min_widths.iter().sum();
    let total_max: f32 = max_widths.iter().sum();

    if total_min >= available {
        // Case: not enough space — use minimum widths.
        min_widths
    } else if total_max <= available {
        // Case: plenty of space — use max widths, distribute remainder.
        let remainder = available - total_max;
        let extra = remainder / num_cols as f32;
        max_widths.iter().map(|w| w + extra).collect()
    } else {
        // Case: between min and max — proportional distribution.
        let range = total_max - total_min;
        if range < f32::EPSILON {
            min_widths
        } else {
            let fraction = (available - total_min) / range;
            min_widths
                .iter()
                .zip(max_widths.iter())
                .map(|(mn, mx)| mn + (mx - mn) * fraction)
                .collect()
        }
    }
}

// ---------------------------------------------------------------------------
// table-layout: fixed (CSS 2.1 §17.5.2.1)
// ---------------------------------------------------------------------------

/// Distribute available width among columns using the fixed table layout algorithm.
///
/// Per CSS 2.1 §17.5.2.1, `<col>`/`<colgroup>` widths are applied first
/// (highest priority), then first-row cell widths fill in any remaining columns.
/// Columns without an explicit width share the remaining space equally.
#[must_use]
#[allow(clippy::cast_precision_loss)] // column/span counts are small integers
pub fn fixed_column_widths(
    num_cols: usize,
    first_row_cells: &[CellInfo],
    first_row_explicit_widths: &[Option<f32>],
    col_element_widths: &[Option<f32>],
    available: f32,
) -> Vec<f32> {
    if num_cols == 0 {
        return Vec::new();
    }

    let available = sanitize_f32(available);
    let mut widths = vec![None::<f32>; num_cols];

    // Step 1: Apply <col>/<colgroup> widths (highest priority, CSS 2.1 §17.5.2.1).
    for (col, col_w) in col_element_widths.iter().enumerate() {
        if col < num_cols {
            if let Some(w) = col_w {
                widths[col] = Some(*w);
            }
        }
    }

    // Step 2: Apply first-row cell widths for columns not already set by col elements.
    for (cell, explicit) in first_row_cells.iter().zip(first_row_explicit_widths.iter()) {
        if let Some(w) = explicit {
            if cell.colspan == 1 {
                // Only apply cell width if no col element width was set.
                if widths[cell.col].is_none() {
                    widths[cell.col] = Some(*w);
                }
            } else {
                // Distribute explicit width across spanned columns.
                let per_col = w / cell.colspan as f32;
                for w in &mut widths[cell.col..span_end_col(cell, num_cols)] {
                    if w.is_none() {
                        *w = Some(per_col);
                    }
                }
            }
        }
    }

    // Calculate remaining space for auto columns.
    let assigned: f32 = widths.iter().filter_map(|w| *w).sum();
    let auto_count = widths.iter().filter(|w| w.is_none()).count();
    let auto_width = if auto_count > 0 {
        ((available - assigned) / auto_count as f32).max(0.0)
    } else {
        0.0
    };

    widths.iter().map(|w| w.unwrap_or(auto_width)).collect()
}

// ---------------------------------------------------------------------------
// border-collapse: collapse (CSS 2.1 §17.6)
// ---------------------------------------------------------------------------

/// Collapsed border widths for a single cell (4 sides).
#[derive(Clone, Debug, Default)]
pub struct CollapsedBorders {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Priority order for border conflict resolution (CSS 2.1 §17.6.2.1).
///
/// Lower value = lower priority. Full spec priority:
/// cell > row > row-group > column > column-group > table.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BorderOrigin {
    Table = 0,
    ColumnGroup = 1,
    Column = 2,
    RowGroup = 3,
    Row = 4,
    Cell = 5,
}

/// A candidate border for collapse resolution.
#[derive(Clone)]
struct BorderCandidate {
    width: f32,
    style: BorderStyle,
    origin: BorderOrigin,
}

impl BorderCandidate {
    /// Returns true if this border beats `other` in CSS 2.1 §17.6.2.1 conflict resolution.
    ///
    /// Priority: hidden always wins → none always loses → wider wins →
    /// style priority (double > solid > dashed > dotted > ridge > outset > groove > inset)
    /// → higher origin wins (cell > row > row-group > column > column-group > table).
    fn beats(&self, other: &Self) -> bool {
        // 1. `hidden` always wins.
        if self.style == BorderStyle::Hidden && other.style != BorderStyle::Hidden {
            return true;
        }
        if other.style == BorderStyle::Hidden && self.style != BorderStyle::Hidden {
            return false;
        }
        // 2. `none` always loses.
        if self.style == BorderStyle::None && other.style != BorderStyle::None {
            return false;
        }
        if other.style == BorderStyle::None && self.style != BorderStyle::None {
            return true;
        }
        // 3. Wider wins.
        if (self.width - other.width).abs() > f32::EPSILON {
            return self.width > other.width;
        }
        // 4. Equal width: style priority (CSS 2.1 §17.6.2.1).
        let sp_self = style_priority(self.style);
        let sp_other = style_priority(other.style);
        if sp_self != sp_other {
            return sp_self > sp_other;
        }
        // 5. Same style: higher origin wins (cell > row > row-group > table).
        self.origin > other.origin
    }
}

/// Border style priority for collapse resolution (CSS 2.1 §17.6.2.1).
fn style_priority(style: BorderStyle) -> u8 {
    match style {
        BorderStyle::Double => 8,
        BorderStyle::Solid => 7,
        BorderStyle::Dashed => 6,
        BorderStyle::Dotted => 5,
        BorderStyle::Ridge => 4,
        BorderStyle::Outset => 3,
        BorderStyle::Groove => 2,
        BorderStyle::Inset => 1,
        BorderStyle::Hidden | BorderStyle::None => 0,
    }
}

/// Check if a column border candidate beats `best` and update if so.
fn check_col_border(
    best: &mut BorderCandidate,
    col_styles: &[Option<ComputedStyle>],
    col: usize,
    side: fn(&ComputedStyle) -> (f32, BorderStyle),
) {
    if let Some(Some(cs)) = col_styles.get(col) {
        let (w, s) = side(cs);
        let cand = BorderCandidate {
            width: w,
            style: s,
            origin: BorderOrigin::Column,
        };
        if cand.beats(best) {
            *best = cand;
        }
    }
}

/// Check if a colgroup border candidate beats `best` and update if so.
fn check_colgroup_border(
    best: &mut BorderCandidate,
    col_group_infos: &[ColGroupBorderInfo],
    col_to_group: &[Option<usize>],
    col: usize,
    boundary_col: usize,
    is_start: bool,
    side: fn(&ComputedStyle) -> (f32, BorderStyle),
) {
    if let Some(gi) = col_to_group.get(col).copied().flatten() {
        let cg = &col_group_infos[gi];
        let at_boundary = if is_start {
            boundary_col == cg.start_col
        } else {
            boundary_col == cg.end_col
        };
        if at_boundary {
            let (w, s) = side(&cg.style);
            let cand = BorderCandidate {
                width: w,
                style: s,
                origin: BorderOrigin::ColumnGroup,
            };
            if cand.beats(best) {
                *best = cand;
            }
        }
    }
}

/// Resolve collapsed borders for all cells in the table.
///
/// Returns a vector parallel to `cells` with the resolved border widths for each cell.
/// Includes row, row-group, column, and column-group borders in the conflict resolution
/// chain (CSS 2.1 §17.6.2.1: cell > row > row-group > column > column-group > table).
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
pub fn resolve_collapsed_borders(
    cells: &[CellInfo],
    cell_styles: &[ComputedStyle],
    table_style: &ComputedStyle,
    row_styles: &[ComputedStyle],
    row_group_infos: &[RowGroupInfo],
    col_styles: &[Option<ComputedStyle>],
    col_group_infos: &[ColGroupBorderInfo],
    num_cols: usize,
    num_rows: usize,
) -> Vec<CollapsedBorders> {
    let make_border = |style: &ComputedStyle,
                       side: fn(&ComputedStyle) -> (f32, BorderStyle),
                       origin: BorderOrigin|
     -> BorderCandidate {
        let (w, s) = side(style);
        BorderCandidate {
            width: w,
            style: s,
            origin,
        }
    };

    // Budget check: fall back to default borders for very large tables
    // to avoid excessive memory allocation (grid would be rows × cols × 8 bytes).
    if num_rows.saturating_mul(num_cols) > MAX_COLLAPSE_GRID_CELLS {
        return vec![CollapsedBorders::default(); cells.len()];
    }

    // Build a grid lookup: (row, col) → cell index.
    let mut grid: Vec<Vec<Option<usize>>> = vec![vec![None; num_cols]; num_rows];
    for (i, cell) in cells.iter().enumerate() {
        for grid_row in &mut grid[cell.row..span_end_row(cell, num_rows)] {
            for slot in &mut grid_row[cell.col..span_end_col(cell, num_cols)] {
                *slot = Some(i);
            }
        }
    }

    // Build row → row-group index lookup.
    let mut row_to_group: Vec<Option<usize>> = vec![None; num_rows];
    for (gi, rg) in row_group_infos.iter().enumerate() {
        for slot in &mut row_to_group[rg.start_row..rg.end_row.min(num_rows)] {
            *slot = Some(gi);
        }
    }

    // Build col → col-group index lookup.
    let mut col_to_group: Vec<Option<usize>> = vec![None; num_cols];
    for (gi, cg) in col_group_infos.iter().enumerate() {
        for slot in &mut col_to_group[cg.start_col..cg.end_col.min(num_cols)] {
            *slot = Some(gi);
        }
    }

    let mut result = vec![CollapsedBorders::default(); cells.len()];

    for (i, cell) in cells.iter().enumerate() {
        let style = &cell_styles[i];
        let this_top = make_border(style, border_top, BorderOrigin::Cell);
        let this_right = make_border(style, border_right, BorderOrigin::Cell);
        let this_bottom = make_border(style, border_bottom, BorderOrigin::Cell);
        let this_left = make_border(style, border_left, BorderOrigin::Cell);

        let col_end = span_end_col(cell, num_cols);
        let row_end = span_end_row(cell, num_rows);

        // --- Top edge ---
        {
            let mut best = this_top.clone();

            // Row border (top of this cell's row).
            if cell.row < row_styles.len() {
                let row_b = make_border(&row_styles[cell.row], border_top, BorderOrigin::Row);
                if row_b.beats(&best) {
                    best = row_b;
                }
            }

            // Row-group border (top, if cell is in the first row of the group).
            if let Some(gi) = row_to_group.get(cell.row).copied().flatten() {
                if cell.row == row_group_infos[gi].start_row {
                    let rg_b = make_border(
                        &row_group_infos[gi].style,
                        border_top,
                        BorderOrigin::RowGroup,
                    );
                    if rg_b.beats(&best) {
                        best = rg_b;
                    }
                }
            }

            // Column/colgroup top borders only at table top edge.
            if cell.row == 0 {
                for c in cell.col..col_end {
                    check_col_border(&mut best, col_styles, c, border_top);
                    check_colgroup_border(
                        &mut best,
                        col_group_infos,
                        &col_to_group,
                        c,
                        c,
                        true,
                        border_top,
                    );
                }
            }

            if cell.row == 0 {
                // Table border.
                let tb = make_border(table_style, border_top, BorderOrigin::Table);
                if tb.beats(&best) {
                    best = tb;
                }
            } else {
                // Neighbor cell bottom borders.
                let neighbor_row = &grid[cell.row - 1];
                for slot in &neighbor_row[cell.col..col_end] {
                    if let Some(ni) = *slot {
                        let cand = make_border(&cell_styles[ni], border_bottom, BorderOrigin::Cell);
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
                // Neighbor row bottom border.
                if cell.row - 1 < row_styles.len() {
                    let cand =
                        make_border(&row_styles[cell.row - 1], border_bottom, BorderOrigin::Row);
                    if cand.beats(&best) {
                        best = cand;
                    }
                }
                // Neighbor rowgroup bottom border (R-1: internal boundary).
                if let Some(gi) = row_to_group.get(cell.row - 1).copied().flatten() {
                    if cell.row == row_group_infos[gi].end_row {
                        let cand = make_border(
                            &row_group_infos[gi].style,
                            border_bottom,
                            BorderOrigin::RowGroup,
                        );
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
            }
            result[i].top = best.width;
        }

        // --- Bottom edge ---
        {
            let bottom_row = row_end.saturating_sub(1);
            let mut best = this_bottom.clone();

            // Row border (bottom of last spanned row).
            if bottom_row < row_styles.len() {
                let row_b = make_border(&row_styles[bottom_row], border_bottom, BorderOrigin::Row);
                if row_b.beats(&best) {
                    best = row_b;
                }
            }

            // Row-group border (bottom, if cell's last row is the last in the group).
            if let Some(gi) = row_to_group.get(bottom_row).copied().flatten() {
                if bottom_row + 1 == row_group_infos[gi].end_row {
                    let rg_b = make_border(
                        &row_group_infos[gi].style,
                        border_bottom,
                        BorderOrigin::RowGroup,
                    );
                    if rg_b.beats(&best) {
                        best = rg_b;
                    }
                }
            }

            // Column/colgroup bottom borders only at table bottom edge.
            if bottom_row + 1 >= num_rows {
                for c in cell.col..col_end {
                    check_col_border(&mut best, col_styles, c, border_bottom);
                    check_colgroup_border(
                        &mut best,
                        col_group_infos,
                        &col_to_group,
                        c,
                        c + 1,
                        false,
                        border_bottom,
                    );
                }
            }

            if bottom_row + 1 >= num_rows {
                let tb = make_border(table_style, border_bottom, BorderOrigin::Table);
                if tb.beats(&best) {
                    best = tb;
                }
            } else {
                let neighbor_row = &grid[bottom_row + 1];
                for slot in &neighbor_row[cell.col..col_end] {
                    if let Some(ni) = *slot {
                        let cand = make_border(&cell_styles[ni], border_top, BorderOrigin::Cell);
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
                if bottom_row + 1 < row_styles.len() {
                    let cand =
                        make_border(&row_styles[bottom_row + 1], border_top, BorderOrigin::Row);
                    if cand.beats(&best) {
                        best = cand;
                    }
                }
                // Neighbor rowgroup top border (R-1: internal boundary).
                if let Some(gi) = row_to_group.get(bottom_row + 1).copied().flatten() {
                    if bottom_row + 1 == row_group_infos[gi].start_row {
                        let cand = make_border(
                            &row_group_infos[gi].style,
                            border_top,
                            BorderOrigin::RowGroup,
                        );
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
            }
            result[i].bottom = best.width;
        }

        // --- Left edge ---
        {
            let mut best = this_left.clone();

            // Row/row-group borders for ALL spanned rows (F-5 fix: was only cell.row).
            for r in cell.row..row_end {
                if r < row_styles.len() {
                    let row_b = make_border(&row_styles[r], border_left, BorderOrigin::Row);
                    if row_b.beats(&best) {
                        best = row_b;
                    }
                }
                if let Some(gi) = row_to_group.get(r).copied().flatten() {
                    let rg_b = make_border(
                        &row_group_infos[gi].style,
                        border_left,
                        BorderOrigin::RowGroup,
                    );
                    if rg_b.beats(&best) {
                        best = rg_b;
                    }
                }
            }

            // Column border (left of this cell's first column).
            check_col_border(&mut best, col_styles, cell.col, border_left);
            // Colgroup border (left, if cell.col is at group start).
            check_colgroup_border(
                &mut best,
                col_group_infos,
                &col_to_group,
                cell.col,
                cell.col,
                true,
                border_left,
            );

            if cell.col == 0 {
                let tb = make_border(table_style, border_left, BorderOrigin::Table);
                if tb.beats(&best) {
                    best = tb;
                }
            } else {
                let left_col = cell.col - 1;
                for grid_row in &grid[cell.row..row_end] {
                    if let Some(ni) = grid_row[left_col] {
                        let cand = make_border(&cell_styles[ni], border_right, BorderOrigin::Cell);
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
                // Neighbor column's right border.
                if let Some(Some(cs)) = col_styles.get(left_col) {
                    let cand = make_border(cs, border_right, BorderOrigin::Column);
                    if cand.beats(&best) {
                        best = cand;
                    }
                }
                // Neighbor colgroup's right border (if left_col is at group end).
                if let Some(gi) = col_to_group.get(left_col).copied().flatten() {
                    if left_col + 1 == col_group_infos[gi].end_col {
                        let cand = make_border(
                            &col_group_infos[gi].style,
                            border_right,
                            BorderOrigin::ColumnGroup,
                        );
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
            }
            result[i].left = best.width;
        }

        // --- Right edge ---
        {
            let right_col = col_end.saturating_sub(1);
            let mut best = this_right.clone();

            // Row/row-group borders for ALL spanned rows (F-5 fix: was only cell.row).
            for r in cell.row..row_end {
                if r < row_styles.len() {
                    let row_b = make_border(&row_styles[r], border_right, BorderOrigin::Row);
                    if row_b.beats(&best) {
                        best = row_b;
                    }
                }
                if let Some(gi) = row_to_group.get(r).copied().flatten() {
                    let rg_b = make_border(
                        &row_group_infos[gi].style,
                        border_right,
                        BorderOrigin::RowGroup,
                    );
                    if rg_b.beats(&best) {
                        best = rg_b;
                    }
                }
            }

            // Column border (right of this cell's last column).
            check_col_border(&mut best, col_styles, right_col, border_right);
            // Colgroup border (right, if col_end is at group end).
            check_colgroup_border(
                &mut best,
                col_group_infos,
                &col_to_group,
                right_col,
                col_end,
                false,
                border_right,
            );

            if right_col + 1 >= num_cols {
                let tb = make_border(table_style, border_right, BorderOrigin::Table);
                if tb.beats(&best) {
                    best = tb;
                }
            } else {
                let check_col = right_col + 1;
                for grid_row in &grid[cell.row..row_end] {
                    if let Some(ni) = grid_row[check_col] {
                        let cand = make_border(&cell_styles[ni], border_left, BorderOrigin::Cell);
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
                // Neighbor column's left border.
                if let Some(Some(cs)) = col_styles.get(check_col) {
                    let cand = make_border(cs, border_left, BorderOrigin::Column);
                    if cand.beats(&best) {
                        best = cand;
                    }
                }
                // Neighbor colgroup's left border (if check_col is at group start).
                if let Some(gi) = col_to_group.get(check_col).copied().flatten() {
                    if check_col == col_group_infos[gi].start_col {
                        let cand = make_border(
                            &col_group_infos[gi].style,
                            border_left,
                            BorderOrigin::ColumnGroup,
                        );
                        if cand.beats(&best) {
                            best = cand;
                        }
                    }
                }
            }
            result[i].right = best.width;
        }
    }

    result
}

/// Extract (width, style) for the top border from a computed style.
fn border_top(s: &ComputedStyle) -> (f32, BorderStyle) {
    (s.border_top.width, s.border_top.style)
}

/// Extract (width, style) for the right border from a computed style.
fn border_right(s: &ComputedStyle) -> (f32, BorderStyle) {
    (s.border_right.width, s.border_right.style)
}

/// Extract (width, style) for the bottom border from a computed style.
fn border_bottom(s: &ComputedStyle) -> (f32, BorderStyle) {
    (s.border_bottom.width, s.border_bottom.style)
}

/// Extract (width, style) for the left border from a computed style.
fn border_left(s: &ComputedStyle) -> (f32, BorderStyle) {
    (s.border_left.width, s.border_left.style)
}
