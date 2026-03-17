//! Grid item placement: occupancy grid and auto-placement algorithm.
//!
//! Implements definite placement (explicit grid lines), semi-definite
//! placement (one axis specified), and fully automatic placement using
//! the CSS Grid auto-placement algorithm.

use elidex_plugin::GridLine;

use crate::GridItem;

/// Maximum grid track index (0-based). Matches typical browser limits (~10000 lines).
/// Prevents OOM from extreme CSS values like `grid-column-start: 1000000`.
const MAX_GRID_INDEX: usize = 10_000;

// ---------------------------------------------------------------------------
// OccupancyGrid
// ---------------------------------------------------------------------------

/// A 2D boolean grid tracking which cells are occupied by placed items.
struct OccupancyGrid {
    cols: usize,
    rows: usize,
    /// Row-major cells: `cells[row * cols + col]`.
    cells: Vec<bool>,
}

impl OccupancyGrid {
    /// Create an empty grid with the given dimensions.
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![false; cols * rows],
        }
    }

    /// Ensure the grid has at least `min_rows` rows, extending if necessary.
    ///
    /// Capped at `MAX_GRID_INDEX + 1` to prevent excessive memory allocation.
    fn ensure_rows(&mut self, min_rows: usize) {
        let capped = min_rows.min(MAX_GRID_INDEX + 1);
        if capped > self.rows {
            self.cells.resize(self.cols * capped, false);
            self.rows = capped;
        }
    }

    /// Ensure the grid has at least `min_cols` columns, extending if necessary.
    ///
    /// Capped at `MAX_GRID_INDEX + 1` to prevent excessive memory allocation.
    fn ensure_cols(&mut self, min_cols: usize) {
        let capped = min_cols.min(MAX_GRID_INDEX + 1);
        if capped <= self.cols {
            return;
        }
        let mut new_cells = vec![false; capped * self.rows];
        for r in 0..self.rows {
            for c in 0..self.cols {
                new_cells[r * capped + c] = self.cells[r * self.cols + c];
            }
        }
        self.cols = capped;
        self.cells = new_cells;
    }

    /// Check if any cell in the rectangular region is occupied.
    fn is_region_occupied(&self, row: usize, col: usize, row_span: usize, col_span: usize) -> bool {
        for r in row..row + row_span {
            for c in col..col + col_span {
                if r < self.rows && c < self.cols && self.cells[r * self.cols + c] {
                    return true;
                }
            }
        }
        false
    }

    /// Mark a rectangular region as occupied.
    ///
    /// Cells beyond the capped grid dimensions are silently skipped.
    fn occupy(&mut self, row: usize, col: usize, row_span: usize, col_span: usize) {
        self.ensure_rows(row + row_span);
        self.ensure_cols(col + col_span);
        for r in row..row + row_span {
            for c in col..col + col_span {
                if r < self.rows && c < self.cols {
                    self.cells[r * self.cols + c] = true;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Grid line resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a `GridLine` value to a 0-based track index.
///
/// `explicit_count` is the number of explicit tracks on that axis.
/// Negative line numbers count from the end of the explicit grid.
/// Returns `None` for `Auto`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn resolve_line(line: GridLine, explicit_count: usize) -> Option<usize> {
    match line {
        GridLine::Auto | GridLine::Span(_) => None,
        GridLine::Line(n) => match n.cmp(&0) {
            std::cmp::Ordering::Greater => Some((n as usize).saturating_sub(1).min(MAX_GRID_INDEX)),
            std::cmp::Ordering::Less => {
                let pos = explicit_count as i32 + n + 1;
                if pos >= 0 {
                    Some((pos as usize).min(MAX_GRID_INDEX))
                } else {
                    Some(0)
                }
            }
            std::cmp::Ordering::Equal => None, // Line 0 is invalid.
        },
    }
}

/// Get the span count from a `GridLine::Span(n)`, defaulting to 1.
fn get_span(line: GridLine) -> usize {
    match line {
        GridLine::Span(n) if n >= 1 => (n as usize).min(MAX_GRID_INDEX),
        _ => 1,
    }
}

/// Check if a start/end pair has a definite position (not fully auto).
fn is_definite(start: GridLine, end: GridLine) -> bool {
    matches!(start, GridLine::Line(_)) || matches!(end, GridLine::Line(_))
}

/// Compute the span for a single axis from start and end line values.
fn compute_span(start: GridLine, end: GridLine, explicit_count: usize) -> usize {
    if let GridLine::Span(n) = start {
        return (n as usize).clamp(1, MAX_GRID_INDEX);
    }
    if let GridLine::Span(n) = end {
        return (n as usize).clamp(1, MAX_GRID_INDEX);
    }
    if let (Some(s), Some(e)) = (
        resolve_line(start, explicit_count),
        resolve_line(end, explicit_count),
    ) {
        if e > s {
            return e - s;
        }
    }
    1
}

/// Resolve a definite start/end pair to (`start_index`, span).
fn resolve_definite_range(
    start: GridLine,
    end: GridLine,
    current_span: usize,
    explicit_count: usize,
) -> (usize, usize) {
    let s = resolve_line(start, explicit_count);
    let e = resolve_line(end, explicit_count);

    match (s, e) {
        (Some(si), Some(ei)) => match ei.cmp(&si) {
            std::cmp::Ordering::Greater => (si, ei - si),
            std::cmp::Ordering::Less => (ei, si - ei),
            std::cmp::Ordering::Equal => (si, 1),
        },
        (Some(si), None) => {
            let span = get_span(end).max(current_span);
            (si, span)
        }
        (None, Some(ei)) => {
            let span = get_span(start).max(current_span);
            let si = ei.saturating_sub(span);
            (si, span)
        }
        (None, None) => (0, current_span),
    }
}

// ---------------------------------------------------------------------------
// Placement
// ---------------------------------------------------------------------------

/// Resolve definite and semi-definite placements, then auto-place the rest.
///
/// After this function, every item in `items` has valid `row_start`, `col_start`,
/// `row_span`, and `col_span` set (all `0`-based).
pub(crate) fn place_items(
    items: &mut [GridItem],
    explicit_cols: usize,
    explicit_rows: usize,
    column_flow: bool,
    dense: bool,
) {
    let initial_cols = explicit_cols.max(1);
    let initial_rows = explicit_rows.max(1);
    let mut grid = OccupancyGrid::new(initial_cols, initial_rows);

    // Resolve spans for all items first.
    for item in items.iter_mut() {
        item.row_span = compute_span(item.grid_row_start, item.grid_row_end, explicit_rows);
        item.col_span = compute_span(item.grid_column_start, item.grid_column_end, explicit_cols);
    }

    // Phase 1: Place items with definite positions on both axes.
    for item in items.iter_mut() {
        let row_def = is_definite(item.grid_row_start, item.grid_row_end);
        let col_def = is_definite(item.grid_column_start, item.grid_column_end);

        if row_def && col_def {
            let (rs, rspan) = resolve_definite_range(
                item.grid_row_start,
                item.grid_row_end,
                item.row_span,
                explicit_rows,
            );
            let (cs, cspan) = resolve_definite_range(
                item.grid_column_start,
                item.grid_column_end,
                item.col_span,
                explicit_cols,
            );
            item.row_start = rs;
            item.col_start = cs;
            item.row_span = rspan;
            item.col_span = cspan;
            grid.occupy(rs, cs, rspan, cspan);
        }
    }

    // Phase 2: Place items with one definite axis.
    for item in items.iter_mut() {
        let row_def = is_definite(item.grid_row_start, item.grid_row_end);
        let col_def = is_definite(item.grid_column_start, item.grid_column_end);

        if row_def && !col_def {
            let (rs, rspan) = resolve_definite_range(
                item.grid_row_start,
                item.grid_row_end,
                item.row_span,
                explicit_rows,
            );
            item.row_start = rs;
            item.row_span = rspan;
            grid.ensure_rows(rs + rspan);
            let col = find_free_col_in_row(&grid, rs, rspan, item.col_span);
            item.col_start = col;
            grid.occupy(rs, col, rspan, item.col_span);
        } else if !row_def && col_def {
            let (cs, cspan) = resolve_definite_range(
                item.grid_column_start,
                item.grid_column_end,
                item.col_span,
                explicit_cols,
            );
            item.col_start = cs;
            item.col_span = cspan;
            let row = find_free_row_in_col(&mut grid, cs, cspan, item.row_span);
            item.row_start = row;
            grid.occupy(row, cs, item.row_span, cspan);
        }
    }

    // Determine the fixed dimension for auto-placement.
    // Row-major: columns are fixed, rows grow. Column-major: rows are fixed, columns grow.
    let grid_cols = grid.cols;
    let grid_rows = grid.rows;

    // Phase 3: Auto-place remaining items.
    // CSS Grid §8.5: In sparse mode, the auto-placement cursor must advance
    // past items placed in Phases 1-2 so auto-placed items don't backfill
    // gaps before semi-definite items.
    let (mut cursor_row, mut cursor_col) = if dense {
        (0, 0)
    } else {
        initial_cursor_after_placed(items, column_flow, grid_cols, grid_rows)
    };

    for item in items.iter_mut() {
        if is_definite(item.grid_row_start, item.grid_row_end)
            || is_definite(item.grid_column_start, item.grid_column_end)
        {
            continue;
        }

        let rspan = item.row_span;
        let cspan = item.col_span;

        if dense {
            cursor_row = 0;
            cursor_col = 0;
        }

        if column_flow {
            // Column-major: rows are fixed, columns grow.
            let (r, c) = find_free_slot_column_major(
                &mut grid, rspan, cspan, cursor_row, cursor_col, grid_rows,
            );
            item.row_start = r;
            item.col_start = c;
            grid.occupy(r, c, rspan, cspan);
            cursor_row = r;
            cursor_col = c;
        } else {
            // Row-major: columns are fixed, rows grow.
            let (r, c) = find_free_slot_row_major(
                &mut grid, rspan, cspan, cursor_row, cursor_col, grid_cols,
            );
            item.row_start = r;
            item.col_start = c;
            grid.occupy(r, c, rspan, cspan);
            cursor_row = r;
            cursor_col = c;
        }
    }
}

/// Compute the initial auto-placement cursor position after Phases 1-2.
///
/// CSS Grid §8.5: In sparse mode, auto-placed items must not backfill
/// gaps before items that were placed with definite or semi-definite
/// positions. The cursor starts just past the last placed item in flow order.
fn initial_cursor_after_placed(
    items: &[GridItem],
    column_flow: bool,
    max_cols: usize,
    max_rows: usize,
) -> (usize, usize) {
    let mut best_row: usize = 0;
    let mut best_col: usize = 0;

    for item in items {
        let row_def = is_definite(item.grid_row_start, item.grid_row_end);
        let col_def = is_definite(item.grid_column_start, item.grid_column_end);
        if !row_def && !col_def {
            continue; // Skip fully auto items (not yet placed).
        }
        // Advance cursor past the end of each placed item in flow order.
        if column_flow {
            // Column-major: rows are the minor axis, columns are the major axis.
            let end_row = item.row_start + item.row_span;
            let end_col = item.col_start;
            if (end_col, end_row) > (best_col, best_row) {
                best_col = end_col;
                best_row = end_row;
            }
        } else {
            // Row-major: columns are the minor axis, rows are the major axis.
            let end_col = item.col_start + item.col_span;
            let end_row = item.row_start;
            if (end_row, end_col) > (best_row, best_col) {
                best_row = end_row;
                best_col = end_col;
            }
        }
    }

    // Wrap cursor if it exceeds the fixed axis.
    if column_flow {
        if best_row >= max_rows && max_rows > 0 {
            best_row = 0;
            best_col += 1;
        }
    } else if best_col >= max_cols && max_cols > 0 {
        best_col = 0;
        best_row += 1;
    }

    (best_row, best_col)
}

/// Find the first column where `cspan` consecutive cells are free in rows [row..row+rspan].
fn find_free_col_in_row(grid: &OccupancyGrid, row: usize, rspan: usize, cspan: usize) -> usize {
    let max_col = grid.cols + cspan;
    for c in 0..max_col {
        if !grid.is_region_occupied(row, c, rspan, cspan) {
            return c;
        }
    }
    grid.cols
}

/// Find the first row where `rspan` consecutive cells are free in cols [col..col+cspan].
fn find_free_row_in_col(grid: &mut OccupancyGrid, col: usize, cspan: usize, rspan: usize) -> usize {
    let limit = MAX_GRID_INDEX;
    for r in 0..limit {
        grid.ensure_rows(r + rspan);
        if !grid.is_region_occupied(r, col, rspan, cspan) {
            return r;
        }
    }
    grid.rows
}

/// Row-major auto-placement: scan columns within each row, then advance rows.
///
/// `max_cols` is the fixed column count — items wrap to the next row when
/// they exceed this. Rows grow implicitly.
/// If the item's column span exceeds `max_cols`, the grid is extended to fit.
fn find_free_slot_row_major(
    grid: &mut OccupancyGrid,
    rspan: usize,
    cspan: usize,
    start_row: usize,
    start_col: usize,
    max_cols: usize,
) -> (usize, usize) {
    // If the item is wider than the grid, extend the grid to fit.
    let effective_cols = max_cols.max(cspan);
    let mut r = start_row;
    let mut c = start_col;
    let limit = MAX_GRID_INDEX;
    let mut iterations = 0;

    loop {
        grid.ensure_rows(r + rspan);

        if c + cspan <= effective_cols && !grid.is_region_occupied(r, c, rspan, cspan) {
            return (r, c);
        }

        c += 1;
        if c + cspan > effective_cols {
            c = 0;
            r += 1;
        }

        iterations += 1;
        if iterations > limit {
            return (r, 0);
        }
    }
}

/// Column-major auto-placement: scan rows within each column, then advance columns.
///
/// `max_rows` is the fixed row count — items wrap to the next column when
/// they exceed this. Columns grow implicitly.
/// If the item's row span exceeds `max_rows`, the grid is extended to fit.
fn find_free_slot_column_major(
    grid: &mut OccupancyGrid,
    rspan: usize,
    cspan: usize,
    start_row: usize,
    start_col: usize,
    max_rows: usize,
) -> (usize, usize) {
    // If the item is taller than the grid, extend the grid to fit.
    let effective_rows = max_rows.max(rspan);
    let mut r = start_row;
    let mut c = start_col;
    let limit = MAX_GRID_INDEX;
    let mut iterations = 0;

    loop {
        grid.ensure_rows(r + rspan);
        grid.ensure_cols(c + cspan);

        if r + rspan <= effective_rows && !grid.is_region_occupied(r, c, rspan, cspan) {
            return (r, c);
        }

        r += 1;
        if r + rspan > effective_rows {
            r = 0;
            c += 1;
        }

        iterations += 1;
        if iterations > limit {
            return (0, c);
        }
    }
}
