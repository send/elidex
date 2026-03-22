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
//! Current simplifications:
//! - `<col>` span attribute read from HTML (clamped to 1–1000 per WHATWG §4.9.11)
//! - Cells are laid out twice (height probe + final positioning); could cache first pass

mod algo;
mod grid;
mod helpers;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    horizontal_pb, sanitize_border, vertical_pb, ChildLayoutFn, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    BorderCollapse, CaptionSide, ComputedStyle, CssSize, Direction, Display, EdgeSizes, LayoutBox,
    Point, VerticalAlign,
};

use elidex_layout_block::block::resolve_margin;

use algo::resolve_collapsed_borders;
pub(crate) use algo::CollapsedBorders;
pub(crate) use grid::{
    build_cell_grid, cell_available_width, collect_all_rows, span_end_col, span_end_row, CellInfo,
    RowInfo,
};
pub use grid::{collect_anonymous_pool, inherit_for_anonymous};
use helpers::{
    box_total_height, build_table_layout_box, collapse_adjusted_width, collect_col_info,
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
        cell_lb.padding.top + cell_lb.border.top + cell_lb.content.size.height,
        |b| b + cell_lb.padding.top + cell_lb.border.top,
    )
}

// ---------------------------------------------------------------------------
// Anonymous table object generation (CSS 2.1 §17.2.1)
// ---------------------------------------------------------------------------

/// Wrap consecutive non-table-cell children in rows with anonymous table-cell boxes.
///
/// CSS 2.1 §17.2.1 Stage 2 Rule 3: any child of a table-row that is not a
/// table-cell generates an anonymous table-cell wrapper.
fn wrap_non_cell_content_in_rows(dom: &mut EcsDom, all_rows: &[RowInfo]) {
    for row_info in all_rows {
        grid::wrap_non_matching_children(
            dom,
            row_info.entity,
            Display::TableCell,
            Display::TableCell,
            "td",
        );
    }
}

// ---------------------------------------------------------------------------
// Row group info for border collapse
// ---------------------------------------------------------------------------

/// Row-group metadata for border-collapse resolution.
pub(crate) struct RowGroupInfo {
    pub(crate) style: ComputedStyle,
    pub(crate) start_row: usize,
    /// Exclusive end row index.
    pub(crate) end_row: usize,
}

/// Column-group metadata for border-collapse resolution (CSS 2.1 §17.6.2.1).
pub(crate) struct ColGroupBorderInfo {
    pub(crate) style: ComputedStyle,
    pub(crate) start_col: usize,
    /// Exclusive end column index.
    pub(crate) end_col: usize,
}

/// Collect row group style and row-range info from the row collection.
///
/// Derives row ranges directly from [`RowInfo::group_end_row`] boundaries,
/// avoiding a redundant DOM re-walk.  Each distinct `(start, group_end_row)`
/// range with an associated row-group entity produces one `RowGroupInfo`.
fn collect_row_group_infos(
    dom: &EcsDom,
    row_groups: &[Entity],
    all_rows: &[RowInfo],
) -> Vec<RowGroupInfo> {
    // Build a set of row-group entities for O(1) lookup.
    let rg_set: std::collections::HashSet<Entity> = row_groups.iter().copied().collect();

    let mut infos = Vec::new();
    let mut i = 0;
    while i < all_rows.len() {
        let group_end = all_rows[i].group_end_row;
        let start = i;

        // Find the row-group entity for this range by checking the parent of
        // the first row.  If the parent is a row-group, use it; otherwise this
        // range consists of direct rows (no RowGroupInfo needed).
        let parent = dom.get_parent(all_rows[i].entity);
        let is_rg = parent.is_some_and(|p| rg_set.contains(&p));

        // Advance past all rows that share the same group_end_row.
        while i < all_rows.len() && all_rows[i].group_end_row == group_end {
            i += 1;
        }

        if is_rg {
            let rg_entity = parent.unwrap(); // safe: is_rg guarantees Some
            infos.push(RowGroupInfo {
                style: elidex_layout_block::get_style(dom, rg_entity),
                start_row: start,
                end_row: i,
            });
        }
    }
    infos
}

/// Redistribute surplus table height proportionally across rows.
///
/// CSS 2.1 §17.5.3 does not define how extra space is distributed
/// ("CSS 2.1 does not define how extra space is distributed").
/// We use proportional distribution based on content height,
/// which matches Chromium behavior.
#[allow(clippy::cast_precision_loss)] // row count is small
fn redistribute_surplus(row_heights: &mut [f32], surplus: f32) {
    let total: f32 = row_heights.iter().sum();
    if total > f32::EPSILON {
        for h in row_heights.iter_mut() {
            let ratio = *h / total;
            if ratio.is_finite() {
                *h += surplus * ratio;
            }
        }
    } else {
        // All rows are zero height: distribute equally.
        let per_row = surplus / row_heights.len().max(1) as f32;
        for h in row_heights.iter_mut() {
            *h += per_row;
        }
    }
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
    let containing_width = input.containing.width;
    let containing_height = input.containing.height;
    let offset_x = input.offset.x;
    let offset_y = input.offset.y;
    let font_db = input.font_db;
    let depth = input.depth;
    if depth >= MAX_LAYOUT_DEPTH {
        return LayoutBox::default();
    }

    let style = elidex_layout_block::get_style(dom, entity);
    let is_rtl = style.direction == Direction::Rtl;
    let is_horizontal_wm = style.writing_mode.is_horizontal();
    let is_collapse = style.border_collapse == BorderCollapse::Collapse;
    // CSS 2.1 §17.6.2: in the collapsing border model, the table has no padding.
    let padding = if is_collapse {
        EdgeSizes::default()
    } else {
        elidex_layout_block::resolve_padding(&style, input.containing_inline_size)
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
        top: resolve_margin(style.margin_top, input.containing_inline_size),
        right: resolve_margin(style.margin_right, input.containing_inline_size),
        bottom: resolve_margin(style.margin_bottom, input.containing_inline_size),
        left: resolve_margin(style.margin_left, input.containing_inline_size),
    };
    let h_margin = margin.left + margin.right;

    // Resolve table width.
    let content_width =
        elidex_layout_block::resolve_content_width(&style, containing_width, h_pb, h_margin);

    // Child inline containing size: in vertical modes, inline size = physical height.
    let child_inline_containing = elidex_layout_block::compute_inline_containing(
        style.writing_mode,
        content_width,
        containing_height,
    );

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

    // Partition captions by position relative to the table grid.
    // CSS Tables L3 §4.1: `top`/`bottom` are physical; `block-start`/`block-end`
    // are logical (mapped through writing mode).
    // In horizontal-tb: top = before, bottom = after.
    // In vertical modes: top ≠ block-start; `top` stays physical (rendered
    // before the table in the block flow if it coincides with block-start).
    // `block-start`/`block-end` always map correctly regardless of writing mode.
    let (captions_top, captions_bottom): (Vec<Entity>, Vec<Entity>) =
        captions.into_iter().partition(|&cap| {
            let cs = elidex_layout_block::get_style(dom, cap);
            // CSS Tables L3 §4.1: top/block-start = before, bottom/block-end = after.
            // In vertical modes, CSS 2.1 §17.4.1 treats top/bottom as
            // block-start/block-end equivalents for layout flow purposes.
            matches!(cs.caption_side, CaptionSide::Top | CaptionSide::BlockStart)
        });

    // Layout top captions.
    let mut caption_top_height = 0.0;
    for &cap in &captions_top {
        let cap_input = LayoutInput {
            containing: CssSize::width_only(content_width),
            containing_inline_size: child_inline_containing,
            offset: Point::new(content_x, cursor_y),
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
        };
        let cap_lb = layout_child(dom, cap, &cap_input).layout_box;
        caption_top_height += box_total_height(&cap_lb);
        let _ = dom.world_mut().insert_one(cap, cap_lb);
    }
    cursor_y += caption_top_height;

    // Wrap direct cells in an anonymous row (CSS 2.1 §17.2.1).
    // Uses find_or_create_anonymous for idempotent re-layout.
    if !direct_cells.is_empty() {
        let anon_row = grid::find_or_create_anonymous(dom, entity, Display::TableRow, "tr");
        for &cell in &direct_cells {
            let _ = dom.append_child(anon_row, cell);
        }
        direct_rows.push(anon_row);
    }

    // Collect all rows (from row groups + direct rows).
    let all_rows = collect_all_rows(dom, &row_groups, &direct_rows);

    // Wrap non-cell content in rows (CSS 2.1 §17.2.1 Stage 2 Rule 3).
    wrap_non_cell_content_in_rows(dom, &all_rows);

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
            Point::new(content_x, offset_y + margin.top + border.top + padding.top),
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

    // Collect row and row-group styles for collapsed border resolution.
    let row_styles: Vec<ComputedStyle> = all_rows
        .iter()
        .map(|ri| elidex_layout_block::get_style(dom, ri.entity))
        .collect();
    let row_group_infos = collect_row_group_infos(dom, &row_groups, &all_rows);

    // Extract <col>/<colgroup> widths and border styles in a single walk
    // (CSS 2.1 §17.5.2.1 + §17.6.2.1).
    let (col_element_widths, col_border_styles, col_group_border_infos) =
        collect_col_info(dom, &children, num_cols, available_for_cols);

    // Resolve collapsed borders if needed.
    let collapsed_borders: Vec<CollapsedBorders> = if is_collapse {
        resolve_collapsed_borders(
            &cells,
            &cell_styles,
            &style,
            &row_styles,
            &row_group_infos,
            &col_border_styles,
            &col_group_border_infos,
            num_cols,
            num_rows,
        )
    } else {
        vec![CollapsedBorders::default(); cells.len()]
    };

    // Determine column widths.
    let col_input = TableColumnInput {
        style: &style,
        cells: &cells,
        cell_styles: &cell_styles,
        collapsed_borders: &collapsed_borders,
        col_element_widths: &col_element_widths,
        num_cols,
        available_for_cols,
        is_collapse,
    };
    let table_env = elidex_layout_block::LayoutEnv {
        font_db,
        layout_child,
        depth,
        viewport: input.viewport,
    };
    let col_widths = compute_column_widths(dom, &col_input, &table_env);

    // Pre-compute the table's explicit content height for cell percentage-height
    // resolution (CSS 2.1 §17.5.3). If the table has an explicit height, cell
    // percentage heights resolve against the content-box height; otherwise auto.
    let table_explicit_height = match style.height {
        elidex_plugin::Dimension::Length(px) if px.is_finite() && px > 0.0 => {
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                Some((px - v_pb).max(0.0))
            } else {
                Some(px)
            }
        }
        elidex_plugin::Dimension::Percentage(pct) if pct > 0.0 => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (resolved - v_pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    };

    // Compute row heights by laying out each cell with its resolved column width.
    let mut row_heights = vec![0.0_f32; num_rows];
    let mut cell_layout_boxes: Vec<LayoutBox> = Vec::with_capacity(cells.len());

    for (i, cell) in cells.iter().enumerate() {
        // Calculate cell available width (sum of spanned columns + spacing between).
        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);

        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Layout the cell content (using block layout).
        // Pass table explicit height as containing_height so cell percentage
        // heights resolve correctly (CSS 2.1 §17.5.3).
        let cell_input = LayoutInput {
            containing: CssSize {
                width: effective_width,
                height: table_explicit_height,
            },
            containing_inline_size: effective_width,
            offset: Point::ZERO, // temporary position, will be set later
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
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
                if span_count <= 0.0 {
                    continue; // Safety: avoid div/0 on degenerate span
                }
                let per_row = deficit / span_count;
                for h in &mut row_heights[cell.row..span_end] {
                    *h += per_row;
                }
            }
        }
    }

    // Height redistribution (CSS 2.1 §17.5.3): if the table has an explicit
    // height larger than the sum of row heights, distribute the surplus
    // proportionally across rows.
    let pre_redistribution_row_height: f32 =
        row_heights.iter().sum::<f32>() + spacing_v * (num_rows as f32 + 1.0);
    let min_for_redistribution = caption_top_height + pre_redistribution_row_height;
    let explicit_height =
        resolve_table_height(&style, containing_height, v_pb, min_for_redistribution);
    if explicit_height > min_for_redistribution {
        let surplus = explicit_height - min_for_redistribution;
        redistribute_surplus(&mut row_heights, surplus);
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
    // CSS 2.1 §17.1: in horizontal-tb, columns = inline (X), rows = block (Y).
    // In vertical modes, columns = inline (Y), rows = block (X).
    for (i, cell) in cells.iter().enumerate() {
        let col_offset = col_x_offsets[cell.col];
        let row_offset = row_y_offsets[cell.row];

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

        // Map logical table coordinates to physical coordinates.
        let (cell_x, cell_y) = if is_horizontal_wm {
            (content_x + col_offset, cursor_y + row_offset + va_offset)
        } else {
            // Vertical: columns → Y (inline), rows → X (block).
            (cursor_y + row_offset + va_offset, content_x + col_offset)
        };

        // In vertical modes, the cell's physical width/height are swapped.
        let (cell_phys_w, cell_phys_h) = if is_horizontal_wm {
            (effective_width, cell_height)
        } else {
            (cell_height, effective_width)
        };

        // Re-layout cell at correct position.
        let cell_relayout_input = LayoutInput {
            containing: CssSize::definite(cell_phys_w, cell_phys_h),
            containing_inline_size: child_inline_containing,
            offset: Point::new(cell_x, cell_y),
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
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
            containing: CssSize::width_only(content_width),
            containing_inline_size: child_inline_containing,
            offset: Point::new(content_x, cursor_y),
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
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

    // In vertical modes, the table's physical dimensions are swapped:
    // content_width (column widths, inline axis) becomes the physical height,
    // content_height (row heights, block axis) becomes the physical width.
    let (final_content_w, final_content_h) = if is_horizontal_wm {
        (content_width, content_height)
    } else {
        (content_height, content_width)
    };
    let lb = build_table_layout_box(
        &padding,
        &border,
        &margin,
        Point::new(content_x, offset_y + margin.top + border.top + padding.top),
        final_content_w,
        final_content_h,
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
            dom,
            &children,
            elidex_plugin::Point::new(content_x, cursor_y),
        );
        let pb = lb.padding_box();
        let pos_env = elidex_layout_block::LayoutEnv {
            font_db,
            layout_child,
            depth,
            viewport: input.viewport,
        };
        elidex_layout_block::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            &static_positions,
            &pos_env,
        );
    }

    lb
}
