//! CSS Table layout algorithm (CSS 2.1 §17, simplified).
//!
//! Implements the core table layout: row/column sizing, item placement,
//! border-collapse/separate models, colspan/rowspan, and captions.
//!
//! **Dimension limits:** Tables are capped at `MAX_TABLE_COLS` (1,000) columns and
//! `MAX_TABLE_ROWS` (65,534) rows per WHATWG §4.9.11.  The collapsed border
//! resolution grid has a cell-count budget and falls back to separate borders
//! when exceeded.
//!
//! **Invariant:** After `build_cell_grid`, all `CellInfo` satisfy
//! `cell.col < num_cols` and `cell.row < num_rows`.  Parallel vectors
//! `cells`, `cell_styles`, `cell_layout_boxes`, and `collapsed_borders`
//! always have identical length.
//!
//! Current simplifications (Phase 4 deferred):
//! - `vertical-align` treated as `top`
//! - `InlineTable` treated as block-level
//! - `TableColumn`/`TableColumnGroup` recognized but width propagation skipped
//! - `empty-cells` always `show`
//! - `baseline` alignment treated as `start`
//! - Anonymous row DOM mutation is not idempotent across re-layouts (needs layout caching)
//! - Cells are laid out twice (height probe + final positioning); could cache first pass

mod algo;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_layout_block::{
    horizontal_pb, sanitize_border, sanitize_padding, vertical_pb, ChildLayoutFn, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    BorderCollapse, CaptionSide, ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox, Rect,
    TableLayout,
};
use elidex_text::FontDatabase;

use elidex_layout_block::block::resolve_margin;

use algo::{auto_column_widths, fixed_column_widths, resolve_collapsed_borders, CollapsedBorders};

// ---------------------------------------------------------------------------
// Table dimension limits (WHATWG §4.9.11)
// ---------------------------------------------------------------------------

/// Maximum number of columns in a table (WHATWG colspan max).
const MAX_TABLE_COLS: usize = 1000;

/// Maximum number of rows in a table (WHATWG rowspan max).
const MAX_TABLE_ROWS: usize = 65534;

/// Maximum cells in the occupancy grid (~4 MB budget for `Vec<bool>`).
/// Tables exceeding this threshold are row-truncated.
const MAX_OCCUPANCY_CELLS: usize = 4_000_000;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Total outer height of a layout box (margin + border + padding + content).
#[inline]
#[must_use]
fn box_total_height(lb: &LayoutBox) -> f32 {
    lb.margin.top
        + lb.border.top
        + lb.padding.top
        + lb.content.height
        + lb.padding.bottom
        + lb.border.bottom
        + lb.margin.bottom
}

/// Cell width adjusted for the collapsed border half-width model.
#[inline]
#[must_use]
fn collapse_adjusted_width(cell_width: f32, is_collapse: bool, cb: &CollapsedBorders) -> f32 {
    if is_collapse {
        (cell_width - cb.left / 2.0 - cb.right / 2.0).max(0.0)
    } else {
        cell_width
    }
}

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
// Entry point
// ---------------------------------------------------------------------------

/// Layout a `display: table` or `display: inline-table` element.
///
/// # Arguments
///
/// * `dom` — ECS DOM to read styles from and write layout boxes to
/// * `entity` — the table element entity
/// * `containing_width` — the width of the containing block
/// * `containing_height` — the height of the containing block (if known)
/// * `offset_x`, `offset_y` — top-left offset from the containing block
/// * `font_db` — font database for text measurement
/// * `depth` — recursion depth guard
/// * `layout_child` — callback for laying out child elements
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]
pub fn layout_table(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
) -> LayoutBox {
    if depth >= MAX_LAYOUT_DEPTH {
        return LayoutBox::default();
    }

    let style = elidex_layout_block::get_style(dom, entity);
    let is_collapse = style.border_collapse == BorderCollapse::Collapse;
    // CSS 2.1 §17.6.2: in the collapsing border model, the table has no padding.
    let padding = if is_collapse {
        EdgeSizes::default()
    } else {
        sanitize_padding(&style)
    };
    // CSS 2.1 §17.6.2: in the collapsing border model, the table's own border
    // is not rendered — borders exist only as collapsed edges between cells and
    // the table edge.  We still need the *raw* border for collapse resolution.
    let raw_border = sanitize_border(&style);
    let border = if is_collapse {
        EdgeSizes::default()
    } else {
        raw_border
    };
    let h_pb = horizontal_pb(&padding, &border);
    let v_pb = vertical_pb(&padding, &border);

    // Resolve margins (before width, since auto width depends on margins).
    let margin = EdgeSizes {
        top: resolve_margin(style.margin_top, containing_width),
        right: resolve_margin(style.margin_right, containing_width),
        bottom: resolve_margin(style.margin_bottom, containing_width),
        left: resolve_margin(style.margin_left, containing_width),
    };
    let h_margin = margin.left + margin.right;

    // Resolve table width.
    let content_width = resolve_table_width(&style, containing_width, h_pb, h_margin);

    let content_x = offset_x + margin.left + border.left + padding.left;
    let mut cursor_y = offset_y + margin.top + border.top + padding.top;

    // Collect children by role.
    let children = dom.children(entity);
    let mut captions: Vec<Entity> = Vec::new();
    let mut row_groups: Vec<Entity> = Vec::new();
    let mut direct_rows: Vec<Entity> = Vec::new();
    let mut direct_cells: Vec<Entity> = Vec::new();

    for &child in &children {
        let child_style = elidex_layout_block::get_style(dom, child);
        match child_style.display {
            Display::TableCaption => captions.push(child),
            Display::TableHeaderGroup | Display::TableRowGroup | Display::TableFooterGroup => {
                row_groups.push(child);
            }
            Display::TableRow => direct_rows.push(child),
            Display::TableCell => direct_cells.push(child),
            Display::TableColumn | Display::TableColumnGroup | Display::None => {}
            _ => {
                // Treat unknown children as if they were in an anonymous cell.
                direct_cells.push(child);
            }
        }
    }

    // Partition captions by caption-side.
    let (captions_top, captions_bottom): (Vec<Entity>, Vec<Entity>) =
        captions.into_iter().partition(|&cap| {
            let cs = elidex_layout_block::get_style(dom, cap);
            cs.caption_side != CaptionSide::Bottom
        });

    // Layout top captions.
    let mut caption_top_height = 0.0;
    for &cap in &captions_top {
        let cap_lb = layout_child(
            dom,
            cap,
            content_width,
            None,
            content_x,
            cursor_y,
            font_db,
            depth + 1,
        );
        caption_top_height += box_total_height(&cap_lb);
        let _ = dom.world_mut().insert_one(cap, cap_lb);
    }
    cursor_y += caption_top_height;

    // Wrap direct cells in an anonymous row (CSS 2.1 §17.2.1).
    if !direct_cells.is_empty() {
        let anon_row = dom.create_element("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            anon_row,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(entity, anon_row);
        for &cell in &direct_cells {
            let _ = dom.append_child(anon_row, cell);
        }
        direct_rows.push(anon_row);
    }

    // Collect all rows (from row groups + direct rows).
    let all_rows = collect_all_rows(dom, &row_groups, &direct_rows);

    // Build cell grid from rows.
    let (cells, num_cols, num_rows) = build_cell_grid(dom, &all_rows);

    // Invariant: build_cell_grid guarantees cell indices are within bounds.
    debug_assert!(cells
        .iter()
        .all(|c| c.col < num_cols.max(1) && c.row < num_rows.max(1)));

    if num_cols == 0 || num_rows == 0 {
        // Empty table.
        let content_height =
            resolve_table_height(&style, containing_height, v_pb, caption_top_height);
        let lb = build_table_layout_box(
            &padding,
            &border,
            &margin,
            content_x,
            offset_y + margin.top + border.top + padding.top,
            content_width,
            content_height,
        );
        let _ = dom.world_mut().insert_one(entity, lb.clone());
        return lb;
    }

    let spacing_h = if is_collapse {
        0.0
    } else {
        style.border_spacing_h
    };
    let spacing_v = if is_collapse {
        0.0
    } else {
        style.border_spacing_v
    };

    // Calculate spacing overhead.
    let spacing_overhead_h = spacing_h * (num_cols + 1) as f32;
    let available_for_cols = (content_width - spacing_overhead_h).max(0.0);

    // Collect cell styles for collapse resolution.
    let cell_styles: Vec<ComputedStyle> = cells
        .iter()
        .map(|c| elidex_layout_block::get_style(dom, c.entity))
        .collect();

    // Resolve collapsed borders if needed.
    let collapsed_borders: Vec<CollapsedBorders> = if is_collapse {
        resolve_collapsed_borders(&cells, &cell_styles, &style, num_cols, num_rows)
    } else {
        vec![CollapsedBorders::default(); cells.len()]
    };

    // Determine column widths.
    let col_widths = compute_column_widths(
        dom,
        &style,
        &cells,
        &cell_styles,
        &collapsed_borders,
        num_cols,
        available_for_cols,
        content_width,
        font_db,
        depth,
        layout_child,
        is_collapse,
    );

    // Compute row heights by laying out each cell with its resolved column width.
    let mut row_heights = vec![0.0_f32; num_rows];
    let mut cell_layout_boxes: Vec<LayoutBox> = Vec::with_capacity(cells.len());

    for (i, cell) in cells.iter().enumerate() {
        // Calculate cell available width (sum of spanned columns + spacing between).
        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);

        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Layout the cell content (using block layout).
        let cell_lb = layout_child(
            dom,
            cell.entity,
            effective_width,
            None,
            0.0, // temporary x, will be positioned later
            0.0, // temporary y
            font_db,
            depth + 1,
        );

        let cell_total_height = box_total_height(&cell_lb);

        // Only single-row cells contribute to row height directly.
        if cell.rowspan == 1 {
            row_heights[cell.row] = row_heights[cell.row].max(cell_total_height);
        }

        cell_layout_boxes.push(cell_lb);
    }

    // Handle rowspan: if spanning rows don't have enough combined height,
    // distribute the deficit evenly across all spanned rows (CSS 2.1 §17.5.3).
    for (i, cell) in cells.iter().enumerate() {
        if cell.rowspan > 1 {
            let cell_total_height = box_total_height(&cell_layout_boxes[i]);
            let span_end = span_end_row(cell, num_rows);
            let spanned_height: f32 = row_heights[cell.row..span_end].iter().sum::<f32>()
                + spacing_v * (cell.rowspan as f32 - 1.0);
            if cell_total_height > spanned_height {
                let deficit = cell_total_height - spanned_height;
                let span_count = (span_end - cell.row) as f32;
                let per_row = deficit / span_count;
                for h in &mut row_heights[cell.row..span_end] {
                    *h += per_row;
                }
            }
        }
    }

    // Position cells.
    // Compute column x offsets.
    let mut col_x_offsets = vec![0.0_f32; num_cols];
    let mut x = spacing_h;
    for c in 0..num_cols {
        col_x_offsets[c] = x;
        x += col_widths[c] + spacing_h;
    }

    // Compute row y offsets.
    let mut row_y_offsets = vec![0.0_f32; num_rows];
    let mut y = spacing_v;
    for r in 0..num_rows {
        row_y_offsets[r] = y;
        y += row_heights[r] + spacing_v;
    }

    let table_content_height_from_rows = y;

    // Now position each cell with final coordinates.
    for (i, cell) in cells.iter().enumerate() {
        let cell_x = content_x + col_x_offsets[cell.col];
        let cell_y = cursor_y + row_y_offsets[cell.row];

        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);
        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Calculate cell height (sum of spanned rows + internal spacing).
        let span_end = span_end_row(cell, num_rows);
        let cell_height: f32 = row_heights[cell.row..span_end].iter().sum::<f32>()
            + spacing_v * (cell.rowspan as f32 - 1.0).max(0.0);

        // Re-layout cell at correct position.
        let cell_lb = layout_child(
            dom,
            cell.entity,
            effective_width,
            Some(cell_height),
            cell_x,
            cell_y,
            font_db,
            depth + 1,
        );
        let _ = dom.world_mut().insert_one(cell.entity, cell_lb);
    }

    // Advance cursor past the table rows.
    cursor_y += table_content_height_from_rows;

    // Layout bottom captions after table rows.
    let mut caption_bottom_height = 0.0;
    for &cap in &captions_bottom {
        let cap_lb = layout_child(
            dom,
            cap,
            content_width,
            None,
            content_x,
            cursor_y,
            font_db,
            depth + 1,
        );
        caption_bottom_height += box_total_height(&cap_lb);
        cursor_y += box_total_height(&cap_lb);
        let _ = dom.world_mut().insert_one(cap, cap_lb);
    }

    // Determine final table height.
    let total_caption_height = caption_top_height + caption_bottom_height;
    let min_content_height = total_caption_height + table_content_height_from_rows;
    let content_height = resolve_table_height(&style, containing_height, v_pb, min_content_height)
        .max(min_content_height);

    let lb = build_table_layout_box(
        &padding,
        &border,
        &margin,
        content_x,
        offset_y + margin.top + border.top + padding.top,
        content_width,
        content_height,
    );
    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Resolve the table's content width from its style.
#[must_use]
fn resolve_table_width(
    style: &ComputedStyle,
    containing_width: f32,
    h_pb: f32,
    h_margin: f32,
) -> f32 {
    match style.width {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (px - h_pb).max(0.0)
            } else {
                px
            }
        }
        Dimension::Percentage(pct) => {
            let resolved = containing_width * pct / 100.0;
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (resolved - h_pb).max(0.0)
            } else {
                resolved
            }
        }
        _ => {
            // auto: use containing width minus margins, padding, and border.
            (containing_width - h_pb - h_margin).max(0.0)
        }
    }
}

/// Resolve the table's content height.
#[must_use]
fn resolve_table_height(
    style: &ComputedStyle,
    containing_height: Option<f32>,
    v_pb: f32,
    min_height: f32,
) -> f32 {
    let explicit = match style.height {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                Some((px - v_pb).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (resolved - v_pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    };
    explicit.unwrap_or(min_height).max(min_height)
}

/// Collect all rows from row groups and direct rows.
///
/// Row groups are ordered: thead first, then tbody, then tfoot.
/// Direct rows follow after all row-group rows.
#[must_use]
fn collect_all_rows(dom: &EcsDom, row_groups: &[Entity], direct_rows: &[Entity]) -> Vec<Entity> {
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
            let rg_children = dom.children(rg);
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

/// Build the cell grid from collected rows.
///
/// Returns (cells, `num_cols`, `num_rows`).
///
/// Rows beyond [`MAX_TABLE_ROWS`] are silently truncated; column positions
/// beyond [`MAX_TABLE_COLS`] cause remaining cells in that row to be skipped.
#[must_use]
fn build_cell_grid(dom: &EcsDom, rows: &[Entity]) -> (Vec<CellInfo>, usize, usize) {
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
        let row_children = dom.children(row_entity);
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

/// Calculate the available width for a cell (sum of spanned columns + inter-column spacing).
#[must_use]
#[allow(clippy::cast_precision_loss)] // span counts are small integers
fn cell_available_width(
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

/// Compute column widths based on table-layout algorithm.
///
/// `collapsed_borders`, `content_width`, and `is_collapse` are reserved
/// for Phase 4 (border-collapse-aware column sizing) and currently unused.
#[must_use]
#[allow(clippy::too_many_arguments)]
fn compute_column_widths(
    dom: &mut EcsDom,
    style: &ComputedStyle,
    cells: &[CellInfo],
    cell_styles: &[ComputedStyle],
    _collapsed_borders: &[CollapsedBorders],
    num_cols: usize,
    available_for_cols: f32,
    _content_width: f32,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
    _is_collapse: bool,
) -> Vec<f32> {
    if style.table_layout == TableLayout::Fixed && !matches!(style.width, Dimension::Auto) {
        // Fixed table layout: use first row cell widths.
        // Collect (index, &CellInfo) pairs to avoid re-searching for indices.
        let first_row: Vec<(usize, &CellInfo)> = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.row == 0)
            .collect();
        let first_row_explicit: Vec<Option<f32>> = first_row
            .iter()
            .map(|&(i, _)| {
                let cs = &cell_styles[i];
                match cs.width {
                    Dimension::Length(px) if px.is_finite() && px > 0.0 => Some(px),
                    Dimension::Percentage(pct) if pct > 0.0 => {
                        Some(available_for_cols * pct / 100.0)
                    }
                    _ => None,
                }
            })
            .collect();
        let first_row_cell_infos: Vec<CellInfo> = first_row
            .iter()
            .map(|&(_, c)| CellInfo {
                entity: c.entity,
                col: c.col,
                row: c.row,
                colspan: c.colspan,
                rowspan: c.rowspan,
            })
            .collect();
        fixed_column_widths(
            num_cols,
            &first_row_cell_infos,
            &first_row_explicit,
            available_for_cols,
        )
    } else {
        // Auto table layout: measure cell intrinsic widths.
        let cell_widths: Vec<(f32, f32)> = cells
            .iter()
            .map(|cell| {
                // Layout cell with a very small width to get min-content,
                // and with a very large width to get max-content.
                let min_lb = layout_child(
                    dom,
                    cell.entity,
                    1.0, // min-content probe
                    None,
                    0.0,
                    0.0,
                    font_db,
                    depth + 1,
                );
                let max_lb = layout_child(
                    dom,
                    cell.entity,
                    f32::MAX / 4.0, // max-content probe
                    None,
                    0.0,
                    0.0,
                    font_db,
                    depth + 1,
                );
                let cs = elidex_layout_block::get_style(dom, cell.entity);
                let p = sanitize_padding(&cs);
                let b = sanitize_border(&cs);
                let cell_h_pb = horizontal_pb(&p, &b);
                // min-content = content width from narrow probe + cell pb
                let min_w = min_lb.content.width + cell_h_pb;
                // max-content = content width from wide probe + cell pb
                let max_w = max_lb.content.width + cell_h_pb;
                (min_w.max(0.0), max_w.max(min_w))
            })
            .collect();
        auto_column_widths(num_cols, cells, &cell_widths, available_for_cols)
    }
}

/// Build the final `LayoutBox` for the table element.
#[must_use]
fn build_table_layout_box(
    padding: &EdgeSizes,
    border: &EdgeSizes,
    margin: &EdgeSizes,
    content_x: f32,
    content_y: f32,
    content_width: f32,
    content_height: f32,
) -> LayoutBox {
    LayoutBox {
        content: Rect {
            x: content_x,
            y: content_y,
            width: content_width,
            height: content_height,
        },
        padding: *padding,
        border: *border,
        margin: *margin,
    }
}
