//! Table layout sizing algorithms and border-collapse logic.
//!
//! Implements CSS 2.1 §17.5 (table-layout: auto and fixed) and
//! §17.6 (border-collapse: collapse).

use elidex_plugin::{BorderStyle, ComputedStyle};

use crate::{span_end_col, span_end_row, CellInfo};

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
/// Lower value = lower priority. Currently only `Table` and `Cell` origins
/// are used; `RowGroup` and `Row` are defined for spec completeness and
/// will be utilized when row/row-group border collection is implemented
/// (Phase 4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
enum BorderOrigin {
    Table = 0,
    RowGroup = 1,
    Row = 2,
    Cell = 3,
}

/// A candidate border for collapse resolution.
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
    /// → higher origin wins (cell > row > row-group > table).
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

/// Resolve the winning border width between two candidates.
fn resolve_border(a: &BorderCandidate, b: &BorderCandidate) -> f32 {
    if a.beats(b) {
        a.width
    } else {
        b.width
    }
}

/// Resolve collapsed borders for all cells in the table.
///
/// Returns a vector parallel to `cells` with the resolved border widths for each cell.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub fn resolve_collapsed_borders(
    cells: &[CellInfo],
    cell_styles: &[ComputedStyle],
    table_style: &ComputedStyle,
    num_cols: usize,
    num_rows: usize,
) -> Vec<CollapsedBorders> {
    let table_border = |side: fn(&ComputedStyle) -> (f32, BorderStyle)| -> BorderCandidate {
        let (w, s) = side(table_style);
        BorderCandidate {
            width: w,
            style: s,
            origin: BorderOrigin::Table,
        }
    };

    let cell_border = |style: &ComputedStyle,
                       side: fn(&ComputedStyle) -> (f32, BorderStyle)|
     -> BorderCandidate {
        let (w, s) = side(style);
        BorderCandidate {
            width: w,
            style: s,
            origin: BorderOrigin::Cell,
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

    let mut result = vec![CollapsedBorders::default(); cells.len()];

    for (i, cell) in cells.iter().enumerate() {
        let style = &cell_styles[i];
        let this_top = cell_border(style, border_top);
        let this_right = cell_border(style, border_right);
        let this_bottom = cell_border(style, border_bottom);
        let this_left = cell_border(style, border_left);

        let col_end = span_end_col(cell, num_cols);
        let row_end = span_end_row(cell, num_rows);

        // Top edge: resolve against all neighbors across the column span.
        result[i].top = if cell.row == 0 {
            resolve_border(&this_top, &table_border(border_top))
        } else {
            let neighbor_row = &grid[cell.row - 1];
            let mut best = this_top.width;
            for slot in &neighbor_row[cell.col..col_end] {
                if let Some(ni) = *slot {
                    best = best.max(resolve_border(
                        &this_top,
                        &cell_border(&cell_styles[ni], border_bottom),
                    ));
                }
            }
            best
        };

        // Bottom edge: resolve against all neighbors across the column span.
        let bottom_row = row_end.saturating_sub(1);
        result[i].bottom = if bottom_row + 1 >= num_rows {
            resolve_border(&this_bottom, &table_border(border_bottom))
        } else {
            let neighbor_row = &grid[bottom_row + 1];
            let mut best = this_bottom.width;
            for slot in &neighbor_row[cell.col..col_end] {
                if let Some(ni) = *slot {
                    best = best.max(resolve_border(
                        &this_bottom,
                        &cell_border(&cell_styles[ni], border_top),
                    ));
                }
            }
            best
        };

        // Left edge: resolve against all neighbors across the row span.
        result[i].left = if cell.col == 0 {
            resolve_border(&this_left, &table_border(border_left))
        } else {
            let left_col = cell.col - 1;
            let mut best = this_left.width;
            for grid_row in &grid[cell.row..row_end] {
                if let Some(ni) = grid_row[left_col] {
                    best = best.max(resolve_border(
                        &this_left,
                        &cell_border(&cell_styles[ni], border_right),
                    ));
                }
            }
            best
        };

        // Right edge: resolve against all neighbors across the row span.
        let right_col = col_end.saturating_sub(1);
        result[i].right = if right_col + 1 >= num_cols {
            resolve_border(&this_right, &table_border(border_right))
        } else {
            let check_col = right_col + 1;
            let mut best = this_right.width;
            for grid_row in &grid[cell.row..row_end] {
                if let Some(ni) = grid_row[check_col] {
                    best = best.max(resolve_border(
                        &this_right,
                        &cell_border(&cell_styles[ni], border_left),
                    ));
                }
            }
            best
        };
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
