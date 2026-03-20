//! Occupancy grid and auto-placement helper functions for CSS Grid.
//!
//! Tracks which cells are occupied and provides search functions
//! for the auto-placement algorithm.

use super::placement::MAX_GRID_INDEX;

// ---------------------------------------------------------------------------
// OccupancyGrid
// ---------------------------------------------------------------------------

/// A 2D boolean grid tracking which cells are occupied by placed items.
pub(crate) struct OccupancyGrid {
    pub(crate) cols: usize,
    pub(crate) rows: usize,
    /// Row-major cells: `cells[row * cols + col]`.
    cells: Vec<bool>,
}

impl OccupancyGrid {
    /// Create an empty grid with the given dimensions.
    pub(crate) fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![false; cols * rows],
        }
    }

    /// Ensure the grid has at least `min_rows` rows, extending if necessary.
    ///
    /// Capped at `MAX_GRID_INDEX + 1` to prevent excessive memory allocation.
    pub(crate) fn ensure_rows(&mut self, min_rows: usize) {
        let capped = min_rows.min(MAX_GRID_INDEX + 1);
        if capped > self.rows {
            self.cells.resize(self.cols * capped, false);
            self.rows = capped;
        }
    }

    /// Ensure the grid has at least `min_cols` columns, extending if necessary.
    ///
    /// Capped at `MAX_GRID_INDEX + 1` to prevent excessive memory allocation.
    pub(crate) fn ensure_cols(&mut self, min_cols: usize) {
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
    pub(crate) fn is_region_occupied(
        &self,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
    ) -> bool {
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
    pub(crate) fn occupy(&mut self, row: usize, col: usize, row_span: usize, col_span: usize) {
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
// Auto-placement helpers
// ---------------------------------------------------------------------------

/// Compute the initial auto-placement cursor position after Phases 1-2.
///
/// CSS Grid §8.5: In sparse mode, auto-placed items must not backfill
/// gaps before items that were placed with definite or semi-definite
/// positions. The cursor starts just past the last placed item in flow order.
pub(crate) fn initial_cursor_after_placed(
    items: &[crate::GridItem],
    column_flow: bool,
    max_cols: usize,
    max_rows: usize,
) -> (usize, usize) {
    use super::placement::is_definite;

    let mut best_row: usize = 0;
    let mut best_col: usize = 0;

    for item in items {
        let row_def = is_definite(&item.grid_row_start, &item.grid_row_end);
        let col_def = is_definite(&item.grid_column_start, &item.grid_column_end);
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
pub(crate) fn find_free_col_in_row(
    grid: &OccupancyGrid,
    row: usize,
    rspan: usize,
    cspan: usize,
) -> usize {
    let max_col = grid.cols + cspan;
    for c in 0..max_col {
        if !grid.is_region_occupied(row, c, rspan, cspan) {
            return c;
        }
    }
    grid.cols
}

/// Find the first row where `rspan` consecutive cells are free in cols [col..col+cspan].
pub(crate) fn find_free_row_in_col(
    grid: &mut OccupancyGrid,
    col: usize,
    cspan: usize,
    rspan: usize,
) -> usize {
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
pub(crate) fn find_free_slot_row_major(
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
pub(crate) fn find_free_slot_column_major(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occupancy_grid_basic() {
        let mut grid = OccupancyGrid::new(3, 3);
        assert!(!grid.is_region_occupied(0, 0, 1, 1));
        grid.occupy(0, 0, 1, 2);
        assert!(grid.is_region_occupied(0, 0, 1, 1));
        assert!(grid.is_region_occupied(0, 1, 1, 1));
        assert!(!grid.is_region_occupied(0, 2, 1, 1));
    }

    #[test]
    fn occupancy_grid_ensure_rows() {
        let mut grid = OccupancyGrid::new(3, 2);
        assert_eq!(grid.rows, 2);
        grid.ensure_rows(5);
        assert_eq!(grid.rows, 5);
        // Should not shrink.
        grid.ensure_rows(3);
        assert_eq!(grid.rows, 5);
    }

    #[test]
    fn occupancy_grid_ensure_cols() {
        let mut grid = OccupancyGrid::new(2, 3);
        grid.occupy(0, 0, 1, 1);
        grid.ensure_cols(4);
        assert_eq!(grid.cols, 4);
        // Previous data should be preserved.
        assert!(grid.is_region_occupied(0, 0, 1, 1));
        assert!(!grid.is_region_occupied(0, 3, 1, 1));
    }

    #[test]
    fn find_free_col_basic() {
        let mut grid = OccupancyGrid::new(4, 2);
        grid.occupy(0, 0, 1, 2);
        let col = find_free_col_in_row(&grid, 0, 1, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn find_free_row_basic() {
        let mut grid = OccupancyGrid::new(3, 4);
        grid.occupy(0, 0, 2, 1);
        let row = find_free_row_in_col(&mut grid, 0, 1, 1);
        assert_eq!(row, 2);
    }

    #[test]
    fn find_free_slot_row_major_basic() {
        let mut grid = OccupancyGrid::new(3, 3);
        grid.occupy(0, 0, 1, 2);
        let (r, c) = find_free_slot_row_major(&mut grid, 1, 1, 0, 0, 3);
        assert_eq!((r, c), (0, 2));
    }

    #[test]
    fn find_free_slot_column_major_basic() {
        let mut grid = OccupancyGrid::new(3, 3);
        grid.occupy(0, 0, 2, 1);
        let (r, c) = find_free_slot_column_major(&mut grid, 1, 1, 0, 0, 3);
        assert_eq!((r, c), (2, 0));
    }
}
