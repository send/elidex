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
//! - `InlineTable` treated as block-level
//! - `<col>` span attribute read from HTML (clamped to 1–1000 per WHATWG §4.9.11)
//! - `empty-cells` always `show`
//! - Anonymous row DOM mutation is not idempotent across re-layouts (needs layout caching)
//! - Cells are laid out twice (height probe + final positioning); could cache first pass

mod algo;
mod grid;
mod helpers;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_layout_block::{
    horizontal_pb, sanitize_border, vertical_pb, ChildLayoutFn, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    BorderCollapse, CaptionSide, ComputedStyle, Direction, Display, EdgeSizes, LayoutBox,
    VerticalAlign,
};

use elidex_layout_block::block::resolve_margin;

use algo::{resolve_collapsed_borders, CollapsedBorders};
pub(crate) use grid::{
    build_cell_grid, cell_available_width, collect_all_rows, span_end_col, span_end_row, CellInfo,
};
use helpers::{
    box_total_height, build_table_layout_box, collapse_adjusted_width, collect_col_widths,
    compute_column_widths, resolve_table_height, TableColumnInput,
};
// Re-exported for tests (col_span.rs uses `crate::col_span_count`).
#[cfg(test)]
pub(crate) use helpers::col_span_count;

/// Compute a table cell's baseline from its border-box top edge (CSS 2.1 §17.5.1).
///
/// If the cell has a `first_baseline`, returns `padding.top + border.top + baseline`.
/// Otherwise, returns the content-edge bottom: `padding.top + border.top + content.height`
/// (CSS 2.1 §17.5.1: "the baseline is the bottom of the content edge of the cell box").
fn cell_baseline(cell_lb: &LayoutBox) -> f32 {
    cell_lb.first_baseline.map_or(
        cell_lb.padding.top + cell_lb.border.top + cell_lb.content.height,
        |b| b + cell_lb.padding.top + cell_lb.border.top,
    )
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
/// * `input` — contextual layout parameters (containing block size, offsets, `font_db`, depth)
/// * `layout_child` — callback for laying out child elements
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn layout_table(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutBox {
    let containing_width = input.containing_width;
    let containing_height = input.containing_height;
    let offset_x = input.offset_x;
    let offset_y = input.offset_y;
    let font_db = input.font_db;
    let depth = input.depth;
    if depth >= MAX_LAYOUT_DEPTH {
        return LayoutBox::default();
    }

    let style = elidex_layout_block::get_style(dom, entity);
    let is_rtl = style.direction == Direction::Rtl;
    let is_collapse = style.border_collapse == BorderCollapse::Collapse;
    // CSS 2.1 §17.6.2: in the collapsing border model, the table has no padding.
    let padding = if is_collapse {
        EdgeSizes::default()
    } else {
        elidex_layout_block::resolve_padding(&style, containing_width)
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
    let content_width =
        elidex_layout_block::resolve_content_width(&style, containing_width, h_pb, h_margin);

    let content_x = offset_x + margin.left + border.left + padding.left;
    let mut cursor_y = offset_y + margin.top + border.top + padding.top;

    // Collect children by role.
    let children = elidex_layout_block::composed_children_flat(dom, entity);
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
            Display::TableColumn | Display::TableColumnGroup | Display::None => {
                // Col/colgroup widths are extracted separately below.
            }
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
        let cap_input = LayoutInput {
            containing_width: content_width,
            containing_height: None,
            offset_x: content_x,
            offset_y: cursor_y,
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let cap_lb = layout_child(dom, cap, &cap_input).layout_box;
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
            None,
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

    // Extract <col>/<colgroup> widths (CSS 2.1 §17.5.2.1).
    let col_element_widths = collect_col_widths(dom, &children, num_cols, available_for_cols);

    // Determine column widths.
    let col_input = TableColumnInput {
        style: &style,
        cells: &cells,
        cell_styles: &cell_styles,
        collapsed_borders: &collapsed_borders,
        col_element_widths: &col_element_widths,
        num_cols,
        available_for_cols,
        content_width,
        is_collapse,
    };
    let col_widths = compute_column_widths(dom, &col_input, font_db, depth, layout_child);

    // Compute row heights by laying out each cell with its resolved column width.
    let mut row_heights = vec![0.0_f32; num_rows];
    let mut cell_layout_boxes: Vec<LayoutBox> = Vec::with_capacity(cells.len());

    for (i, cell) in cells.iter().enumerate() {
        // Calculate cell available width (sum of spanned columns + spacing between).
        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);

        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Layout the cell content (using block layout).
        let cell_input = LayoutInput {
            containing_width: effective_width,
            containing_height: None,
            offset_x: 0.0, // temporary x, will be positioned later
            offset_y: 0.0, // temporary y
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let cell_lb = layout_child(dom, cell.entity, &cell_input).layout_box;

        let cell_total_height = box_total_height(&cell_lb);

        // Only single-row cells contribute to row height directly.
        if cell.rowspan == 1 {
            row_heights[cell.row] = row_heights[cell.row].max(cell_total_height);
        }

        cell_layout_boxes.push(cell_lb);
    }

    // Pre-compute per-cell baselines to avoid repeated computation.
    let cell_baselines: Vec<f32> = cell_layout_boxes.iter().map(cell_baseline).collect();

    // Per-row baseline for baseline-aligned cells (CSS 2.1 §17.5.1).
    // Table cells have zero margins (CSS 2.1 §17.5.4), so padding+border only.
    let mut row_baselines = vec![0.0_f32; num_rows];
    let mut row_has_baseline = vec![false; num_rows];
    for (i, cell) in cells.iter().enumerate() {
        if cell.rowspan == 1 {
            let cell_style = &cell_styles[i];
            if cell_style.vertical_align == VerticalAlign::Baseline {
                row_baselines[cell.row] = row_baselines[cell.row].max(cell_baselines[i]);
                row_has_baseline[cell.row] = true;
            }
        }
    }

    // Ensure rows are tall enough for baseline alignment.
    // Row height >= max(baseline_above) + max(baseline_below).
    for r in 0..num_rows {
        if row_has_baseline[r] {
            let baseline_below: f32 = cells
                .iter()
                .enumerate()
                .filter(|(_, c)| c.row == r && c.rowspan == 1)
                .filter(|(i, _)| cell_styles[*i].vertical_align == VerticalAlign::Baseline)
                .map(|(i, _)| box_total_height(&cell_layout_boxes[i]) - cell_baselines[i])
                .fold(0.0_f32, f32::max);
            let min_row_h = row_baselines[r] + baseline_below;
            row_heights[r] = row_heights[r].max(min_row_h);
        }
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
    // RTL: columns flow right-to-left, so we mirror the LTR offsets.
    let total_table_width: f32 =
        col_widths.iter().sum::<f32>() + spacing_h * (num_cols as f32 + 1.0);
    let mut col_x_offsets = vec![0.0_f32; num_cols];
    let mut x = spacing_h;
    for c in 0..num_cols {
        if is_rtl {
            // Mirror: place rightmost column first.
            col_x_offsets[c] = total_table_width - x - col_widths[c];
        } else {
            col_x_offsets[c] = x;
        }
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

        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);
        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Calculate cell height (sum of spanned rows + internal spacing).
        let span_end = span_end_row(cell, num_rows);
        let cell_height: f32 = row_heights[cell.row..span_end].iter().sum::<f32>()
            + spacing_v * (cell.rowspan as f32 - 1.0).max(0.0);

        // Compute vertical-align offset within the cell's row slot.
        let cell_total_height = box_total_height(&cell_layout_boxes[i]);
        let free = (cell_height - cell_total_height).max(0.0);
        let va_offset = match cell_styles[i].vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => free / 2.0,
            VerticalAlign::Bottom => free,
            // CSS 2.1 §17.5.1: sub/super/text-top/text-bottom/length/percentage
            // are treated as baseline alignment in table cell context.
            _ => (row_baselines[cell.row] - cell_baselines[i]).max(0.0),
        };

        let cell_y = cursor_y + row_y_offsets[cell.row] + va_offset;

        // Re-layout cell at correct position.
        let cell_relayout_input = LayoutInput {
            containing_width: effective_width,
            containing_height: Some(cell_height),
            offset_x: cell_x,
            offset_y: cell_y,
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let cell_lb = layout_child(dom, cell.entity, &cell_relayout_input).layout_box;
        let _ = dom.world_mut().insert_one(cell.entity, cell_lb);
    }

    // Advance cursor past the table rows.
    cursor_y += table_content_height_from_rows;

    // Layout bottom captions after table rows.
    let mut caption_bottom_height = 0.0;
    for &cap in &captions_bottom {
        let cap_input = LayoutInput {
            containing_width: content_width,
            containing_height: None,
            offset_x: content_x,
            offset_y: cursor_y,
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let cap_lb = layout_child(dom, cap, &cap_input).layout_box;
        caption_bottom_height += box_total_height(&cap_lb);
        cursor_y += box_total_height(&cap_lb);
        let _ = dom.world_mut().insert_one(cap, cap_lb);
    }

    // Determine final table height.
    let total_caption_height = caption_top_height + caption_bottom_height;
    let min_content_height = total_caption_height + table_content_height_from_rows;
    let content_height = resolve_table_height(&style, containing_height, v_pb, min_content_height)
        .max(min_content_height);

    // Table container baseline = first row's baseline (CSS 2.1 §17.5.1).
    let table_baseline = if !row_has_baseline.is_empty() && row_has_baseline[0] {
        // Row 0 has baseline-aligned cells — use the shared row baseline.
        Some(caption_top_height + spacing_v + row_baselines[0])
    } else if !row_heights.is_empty() {
        // No baseline-aligned cells in row 0 — use row height as fallback.
        Some(caption_top_height + spacing_v + row_heights[0])
    } else {
        None
    };

    let lb = build_table_layout_box(
        &padding,
        &border,
        &margin,
        content_x,
        offset_y + margin.top + border.top + padding.top,
        content_width,
        content_height,
        table_baseline,
    );
    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // Layout positioned descendants owned by this containing block.
    // CSS 2.1 §17.2: the table establishes a CB for absolute children
    // when it is itself positioned (or is the root).
    // CSS Transforms L1 §2: transform establishes CB for all descendants.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root || style.has_transform;
    if is_cb {
        let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
            dom, &children, content_x, cursor_y,
        );
        let pb = lb.padding_box();
        elidex_layout_block::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            input.viewport,
            &static_positions,
            font_db,
            layout_child,
            depth,
        );
    }

    lb
}
