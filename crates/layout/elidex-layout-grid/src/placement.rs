//! Grid item placement: line resolution and the placement algorithm.
//!
//! Implements definite placement (explicit grid lines), semi-definite
//! placement (one axis specified), and fully automatic placement using
//! the CSS Grid auto-placement algorithm.

use elidex_plugin::GridLine;

use crate::occupancy::{
    find_free_col_in_row, find_free_row_in_col, find_free_slot_column_major,
    find_free_slot_row_major, initial_cursor_after_placed, OccupancyGrid,
};
use crate::GridItem;

/// Maximum grid track index (0-based). Matches typical browser limits (~10000 lines).
/// Prevents OOM from extreme CSS values like `grid-column-start: 1000000`.
pub(crate) const MAX_GRID_INDEX: usize = 10_000;

// ---------------------------------------------------------------------------
// Grid line resolution helpers
// ---------------------------------------------------------------------------

/// A map from line name to the list of 0-based line indices where it appears.
pub(crate) type LineNameMap = std::collections::HashMap<String, Vec<usize>>;

/// Build a `LineNameMap` from explicit line names and area-derived implicit names.
pub(crate) fn build_line_name_map(
    line_names: &[Vec<String>],
    areas: &elidex_plugin::GridTemplateAreas,
    is_column: bool,
) -> LineNameMap {
    let mut map = LineNameMap::new();

    // Explicit line names
    for (i, names) in line_names.iter().enumerate() {
        for name in names {
            map.entry(name.clone()).or_default().push(i);
        }
    }

    // Area-derived implicit lines
    if !areas.is_none() {
        let mut seen = std::collections::HashSet::new();
        for row in &areas.areas {
            for name in row {
                if name == "." || !seen.insert(name.clone()) {
                    continue;
                }
                // Find bounding box for this area
                let Some((min_r, min_c, max_r, max_c)) = area_bounds(areas, name) else {
                    continue;
                };
                if is_column {
                    map.entry(format!("{name}-start")).or_default().push(min_c);
                    map.entry(format!("{name}-end"))
                        .or_default()
                        .push(max_c + 1);
                } else {
                    map.entry(format!("{name}-start")).or_default().push(min_r);
                    map.entry(format!("{name}-end"))
                        .or_default()
                        .push(max_r + 1);
                }
            }
        }
    }

    map
}

/// Find the bounding box of an area name in the template.
///
/// Returns `None` if the name is not found in any cell.
fn area_bounds(
    areas: &elidex_plugin::GridTemplateAreas,
    name: &str,
) -> Option<(usize, usize, usize, usize)> {
    let mut min_r = usize::MAX;
    let mut min_c = usize::MAX;
    let mut max_r = 0;
    let mut max_c = 0;
    let mut found = false;
    for (r, row) in areas.areas.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell == name {
                found = true;
                min_r = min_r.min(r);
                min_c = min_c.min(c);
                max_r = max_r.max(r);
                max_c = max_c.max(c);
            }
        }
    }
    if found {
        Some((min_r, min_c, max_r, max_c))
    } else {
        None
    }
}

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
pub(crate) fn resolve_line(
    line: &GridLine,
    explicit_count: usize,
    name_map: &LineNameMap,
    is_end: bool,
) -> Option<usize> {
    match line {
        GridLine::Auto | GridLine::Span(_) | GridLine::SpanNamed(..) => None,
        GridLine::Line(n) => match n.cmp(&0) {
            std::cmp::Ordering::Greater => {
                Some((*n as usize).saturating_sub(1).min(MAX_GRID_INDEX))
            }
            std::cmp::Ordering::Less => {
                let pos = explicit_count as i32 + n + 1;
                if pos >= 0 {
                    Some((pos as usize).min(MAX_GRID_INDEX))
                } else {
                    Some(0)
                }
            }
            std::cmp::Ordering::Equal => None,
        },
        GridLine::Named(name) => {
            // Direct lookup
            if let Some(indices) = name_map.get(name.as_str()) {
                if !indices.is_empty() {
                    return Some(indices[0]);
                }
            }
            // Try area-derived suffix
            let suffix = if is_end {
                format!("{name}-end")
            } else {
                format!("{name}-start")
            };
            if let Some(indices) = name_map.get(&suffix) {
                if !indices.is_empty() {
                    return Some(indices[0]);
                }
            }
            None
        }
        GridLine::NamedWithIndex(name, n) => {
            let indices = name_map.get(name.as_str())?;
            if indices.is_empty() {
                return None;
            }
            if *n > 0 {
                // 1-based positive index
                indices.get((*n as usize).saturating_sub(1)).copied()
            } else {
                // Negative: count from end
                let abs_n = n.unsigned_abs() as usize;
                if abs_n <= indices.len() {
                    Some(indices[indices.len() - abs_n])
                } else {
                    Some(indices[0])
                }
            }
        }
    }
}

/// Get the span count from a `GridLine::Span(n)` or `SpanNamed(_, n)`, defaulting to 1.
fn get_span(line: &GridLine) -> usize {
    match line {
        GridLine::Span(n) | GridLine::SpanNamed(_, n) if *n >= 1 => {
            (*n as usize).min(MAX_GRID_INDEX)
        }
        _ => 1,
    }
}

/// Check if a start/end pair has a definite position (not fully auto).
pub(crate) fn is_definite(start: &GridLine, end: &GridLine) -> bool {
    matches!(
        start,
        GridLine::Line(_) | GridLine::Named(_) | GridLine::NamedWithIndex(..)
    ) || matches!(
        end,
        GridLine::Line(_) | GridLine::Named(_) | GridLine::NamedWithIndex(..)
    )
}

/// Compute the span for a single axis from start and end line values.
fn compute_span(
    start: &GridLine,
    end: &GridLine,
    explicit_count: usize,
    name_map: &LineNameMap,
) -> usize {
    if let GridLine::Span(n) = start {
        return (*n as usize).clamp(1, MAX_GRID_INDEX);
    }
    if let GridLine::Span(n) = end {
        return (*n as usize).clamp(1, MAX_GRID_INDEX);
    }
    // CSS Grid §8.3.1: SpanNamed searches for the nth occurrence of a named
    // line from the resolved definite position. If the other side resolves,
    // perform the search; otherwise fall back to the count.
    if let GridLine::SpanNamed(name, n) = start {
        // Start is span-named → end must be definite. Search backward from end.
        if let Some(ei) = resolve_line(end, explicit_count, name_map, true) {
            if let Some(span) = resolve_span_named_backward(name, *n, ei, name_map) {
                return span;
            }
        }
        return (*n as usize).clamp(1, MAX_GRID_INDEX);
    }
    if let GridLine::SpanNamed(name, n) = end {
        // End is span-named → start must be definite. Search forward from start.
        if let Some(si) = resolve_line(start, explicit_count, name_map, false) {
            if let Some(span) = resolve_span_named_forward(name, *n, si, name_map) {
                return span;
            }
        }
        return (*n as usize).clamp(1, MAX_GRID_INDEX);
    }
    if let (Some(s), Some(e)) = (
        resolve_line(start, explicit_count, name_map, false),
        resolve_line(end, explicit_count, name_map, true),
    ) {
        if e > s {
            return e - s;
        }
    }
    1
}

/// Search forward from `start_index` for the nth occurrence of `name` in the
/// name map. Returns the span (distance) if found.
///
/// CSS Grid §8.3.1: When `span <custom-ident> <integer>` is on the end side,
/// count named lines forward from the definite start position.
fn resolve_span_named_forward(
    name: &str,
    n: u32,
    start_index: usize,
    name_map: &LineNameMap,
) -> Option<usize> {
    let indices = name_map.get(name)?;
    // Count occurrences strictly after start_index
    let mut count = 0u32;
    for &idx in indices {
        if idx > start_index {
            count += 1;
            if count == n {
                return Some(idx - start_index);
            }
        }
    }
    None
}

/// Search backward from `end_index` for the nth occurrence of `name` in the
/// name map. Returns the span (distance) if found.
///
/// CSS Grid §8.3.1: When `span <custom-ident> <integer>` is on the start side,
/// count named lines backward from the definite end position.
fn resolve_span_named_backward(
    name: &str,
    n: u32,
    end_index: usize,
    name_map: &LineNameMap,
) -> Option<usize> {
    let indices = name_map.get(name)?;
    // Count occurrences strictly before end_index, searching from the end
    let mut count = 0u32;
    for &idx in indices.iter().rev() {
        if idx < end_index {
            count += 1;
            if count == n {
                return Some(end_index - idx);
            }
        }
    }
    None
}

/// Resolve a definite start/end pair to (`start_index`, span).
fn resolve_definite_range(
    start: &GridLine,
    end: &GridLine,
    current_span: usize,
    explicit_count: usize,
    name_map: &LineNameMap,
) -> (usize, usize) {
    let s = resolve_line(start, explicit_count, name_map, false);
    let e = resolve_line(end, explicit_count, name_map, true);

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
#[allow(clippy::too_many_lines)]
pub(crate) fn place_items(
    items: &mut [GridItem],
    explicit_cols: usize,
    explicit_rows: usize,
    column_flow: bool,
    dense: bool,
    col_name_map: &LineNameMap,
    row_name_map: &LineNameMap,
) {
    let initial_cols = explicit_cols.max(1);
    let initial_rows = explicit_rows.max(1);
    let mut grid = OccupancyGrid::new(initial_cols, initial_rows);

    // Resolve spans for all items first.
    for item in items.iter_mut() {
        item.row_span = compute_span(
            &item.grid_row_start,
            &item.grid_row_end,
            explicit_rows,
            row_name_map,
        );
        item.col_span = compute_span(
            &item.grid_column_start,
            &item.grid_column_end,
            explicit_cols,
            col_name_map,
        );
    }

    // Phase 1: Place items with definite positions on both axes.
    for item in items.iter_mut() {
        let row_def = is_definite(&item.grid_row_start, &item.grid_row_end);
        let col_def = is_definite(&item.grid_column_start, &item.grid_column_end);

        if row_def && col_def {
            let (rs, rspan) = resolve_definite_range(
                &item.grid_row_start,
                &item.grid_row_end,
                item.row_span,
                explicit_rows,
                row_name_map,
            );
            let (cs, cspan) = resolve_definite_range(
                &item.grid_column_start,
                &item.grid_column_end,
                item.col_span,
                explicit_cols,
                col_name_map,
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
        let row_def = is_definite(&item.grid_row_start, &item.grid_row_end);
        let col_def = is_definite(&item.grid_column_start, &item.grid_column_end);

        if row_def && !col_def {
            let (rs, rspan) = resolve_definite_range(
                &item.grid_row_start,
                &item.grid_row_end,
                item.row_span,
                explicit_rows,
                row_name_map,
            );
            item.row_start = rs;
            item.row_span = rspan;
            grid.ensure_rows(rs + rspan);
            let col = find_free_col_in_row(&grid, rs, rspan, item.col_span);
            item.col_start = col;
            grid.occupy(rs, col, rspan, item.col_span);
        } else if !row_def && col_def {
            let (cs, cspan) = resolve_definite_range(
                &item.grid_column_start,
                &item.grid_column_end,
                item.col_span,
                explicit_cols,
                col_name_map,
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
        if is_definite(&item.grid_row_start, &item.grid_row_end)
            || is_definite(&item.grid_column_start, &item.grid_column_end)
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

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::GridTemplateAreas;

    #[test]
    fn build_line_name_map_explicit_names() {
        let line_names = vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        ];
        let areas = GridTemplateAreas::default();
        let map = build_line_name_map(&line_names, &areas, true);
        assert_eq!(map.get("a"), Some(&vec![0]));
        assert_eq!(map.get("b"), Some(&vec![1]));
        assert_eq!(map.get("c"), Some(&vec![2]));
    }

    #[test]
    fn build_line_name_map_area_derived() {
        let areas = GridTemplateAreas {
            areas: vec![
                vec!["header".to_string(), "header".to_string()],
                vec!["main".to_string(), "sidebar".to_string()],
            ],
        };
        // 2 tracks → 3 line name slots
        let line_names = vec![vec![], vec![], vec![]];
        let col_map = build_line_name_map(&line_names, &areas, true);
        assert_eq!(col_map.get("header-start"), Some(&vec![0]));
        assert_eq!(col_map.get("header-end"), Some(&vec![2]));
        assert_eq!(col_map.get("main-start"), Some(&vec![0]));
        assert_eq!(col_map.get("sidebar-start"), Some(&vec![1]));

        let row_map = build_line_name_map(&line_names, &areas, false);
        assert_eq!(row_map.get("header-start"), Some(&vec![0]));
        assert_eq!(row_map.get("header-end"), Some(&vec![1]));
    }

    #[test]
    fn resolve_named_line() {
        let mut map = LineNameMap::new();
        map.insert("header".to_string(), vec![0]);
        map.insert("main".to_string(), vec![2]);

        assert_eq!(
            resolve_line(&GridLine::Named("header".into()), 3, &map, false),
            Some(0)
        );
        assert_eq!(
            resolve_line(&GridLine::Named("main".into()), 3, &map, false),
            Some(2)
        );
        assert_eq!(
            resolve_line(&GridLine::Named("unknown".into()), 3, &map, false),
            None
        );
    }

    #[test]
    fn resolve_named_with_index() {
        let mut map = LineNameMap::new();
        map.insert("a".to_string(), vec![0, 3, 5]);

        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("a".into(), 1), 6, &map, false),
            Some(0)
        );
        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("a".into(), 2), 6, &map, false),
            Some(3)
        );
        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("a".into(), 3), 6, &map, false),
            Some(5)
        );
        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("a".into(), -1), 6, &map, false),
            Some(5)
        );
        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("a".into(), -2), 6, &map, false),
            Some(3)
        );
    }

    #[test]
    fn resolve_named_with_area_suffix_fallback() {
        let mut map = LineNameMap::new();
        map.insert("header-start".to_string(), vec![0]);
        map.insert("header-end".to_string(), vec![2]);

        // Named("header") with is_end=false → tries "header" first (not found), then "header-start"
        assert_eq!(
            resolve_line(&GridLine::Named("header".into()), 3, &map, false),
            Some(0)
        );
        // Named("header") with is_end=true → tries "header" first (not found), then "header-end"
        assert_eq!(
            resolve_line(&GridLine::Named("header".into()), 3, &map, true),
            Some(2)
        );
    }

    #[test]
    fn empty_name_map_returns_none() {
        let map = LineNameMap::new();
        assert_eq!(
            resolve_line(&GridLine::Named("x".into()), 3, &map, false),
            None
        );
        assert_eq!(
            resolve_line(&GridLine::NamedWithIndex("x".into(), 1), 3, &map, false),
            None
        );
    }

    #[test]
    fn span_named_forward_search() {
        // Name "a" at indices 0, 2, 4
        let mut map = LineNameMap::new();
        map.insert("a".to_string(), vec![0, 2, 4]);

        // span 1 a from start=0 → first "a" after 0 is at 2, span = 2
        assert_eq!(resolve_span_named_forward("a", 1, 0, &map), Some(2));
        // span 2 a from start=0 → second "a" after 0 is at 4, span = 4
        assert_eq!(resolve_span_named_forward("a", 2, 0, &map), Some(4));
        // span 1 a from start=2 → first "a" after 2 is at 4, span = 2
        assert_eq!(resolve_span_named_forward("a", 1, 2, &map), Some(2));
        // span 2 a from start=2 → only 1 "a" after 2 → None
        assert_eq!(resolve_span_named_forward("a", 2, 2, &map), None);
        // Unknown name → None
        assert_eq!(resolve_span_named_forward("b", 1, 0, &map), None);
    }

    #[test]
    fn span_named_backward_search() {
        let mut map = LineNameMap::new();
        map.insert("a".to_string(), vec![0, 2, 4]);

        // span 1 a backward from end=4 → first "a" before 4 is at 2, span = 2
        assert_eq!(resolve_span_named_backward("a", 1, 4, &map), Some(2));
        // span 2 a backward from end=4 → second "a" before 4 is at 0, span = 4
        assert_eq!(resolve_span_named_backward("a", 2, 4, &map), Some(4));
        // span 1 a backward from end=2 → first "a" before 2 is at 0, span = 2
        assert_eq!(resolve_span_named_backward("a", 1, 2, &map), Some(2));
        // span 2 a backward from end=2 → only 1 "a" before 2 → None
        assert_eq!(resolve_span_named_backward("a", 2, 2, &map), None);
    }

    #[test]
    fn compute_span_with_span_named() {
        let mut map = LineNameMap::new();
        map.insert("a".to_string(), vec![0, 2, 4]);

        // End is SpanNamed, start is definite at line 1 (index 0)
        let span = compute_span(
            &GridLine::Line(1),
            &GridLine::SpanNamed("a".into(), 1),
            5,
            &map,
        );
        // From index 0, first "a" after 0 is at index 2 → span = 2
        assert_eq!(span, 2);

        // Start is SpanNamed, end is definite at line 5 (index 4)
        let span = compute_span(
            &GridLine::SpanNamed("a".into(), 1),
            &GridLine::Line(5),
            5,
            &map,
        );
        // From index 4, first "a" before 4 is at index 2 → span = 2
        assert_eq!(span, 2);

        // Unknown name → falls back to count
        let span = compute_span(
            &GridLine::Line(1),
            &GridLine::SpanNamed("unknown".into(), 3),
            5,
            &map,
        );
        assert_eq!(span, 3);
    }
}
