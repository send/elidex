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
//! - `<col>` span attribute not read (always treated as span=1)
//! - `empty-cells` always `show`
//! - `baseline` alignment treated as `start`
//! - Anonymous row DOM mutation is not idempotent across re-layouts (needs layout caching)
//! - Cells are laid out twice (height probe + final positioning); could cache first pass

mod algo;
mod grid;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_layout_block::{
    horizontal_pb, sanitize_border, vertical_pb, ChildLayoutFn, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    BorderCollapse, CaptionSide, ComputedStyle, Dimension, Direction, Display, EdgeSizes,
    LayoutBox, Rect, TableLayout,
};
use elidex_text::FontDatabase;

use elidex_layout_block::block::resolve_margin;

use algo::{auto_column_widths, fixed_column_widths, resolve_collapsed_borders, CollapsedBorders};
pub(crate) use grid::{
    build_cell_grid, cell_available_width, collect_all_rows, span_end_col, span_end_row, CellInfo,
};

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

/// Collect per-column widths from `<col>` and `<colgroup>` elements.
///
/// Walks the table's children, expanding `<colgroup>` into its child `<col>`
/// elements. Returns a vec of `Option<f32>` indexed by column, where `None`
/// means no col-specified width for that column.
#[must_use]
fn collect_col_widths(
    dom: &EcsDom,
    children: &[Entity],
    num_cols: usize,
    available_for_cols: f32,
) -> Vec<Option<f32>> {
    let mut result = vec![None::<f32>; num_cols];
    let mut col_idx = 0;

    for &child in children {
        if col_idx >= num_cols {
            break;
        }
        let child_style = elidex_layout_block::get_style(dom, child);
        match child_style.display {
            Display::TableColumnGroup => {
                // Expand colgroup: walk its <col> children.
                let mut col_child = dom.get_first_child(child);
                while let Some(cc) = col_child {
                    if col_idx >= num_cols {
                        break;
                    }
                    let cc_style = elidex_layout_block::get_style(dom, cc);
                    if cc_style.display == Display::TableColumn {
                        let span = col_span_count(&cc_style);
                        let w = resolve_col_width(&cc_style, available_for_cols)
                            .or_else(|| resolve_col_width(&child_style, available_for_cols));
                        for _ in 0..span {
                            if col_idx < num_cols {
                                if result[col_idx].is_none() {
                                    result[col_idx] = w;
                                }
                                col_idx += 1;
                            }
                        }
                    }
                    col_child = dom.get_next_sibling(cc);
                }
            }
            Display::TableColumn => {
                let span = col_span_count(&child_style);
                let w = resolve_col_width(&child_style, available_for_cols);
                for _ in 0..span {
                    if col_idx < num_cols {
                        if result[col_idx].is_none() {
                            result[col_idx] = w;
                        }
                        col_idx += 1;
                    }
                }
            }
            _ => {}
        }
    }
    result
}

/// Resolve a `<col>` or `<colgroup>` element's width to pixels.
fn resolve_col_width(style: &ComputedStyle, available: f32) -> Option<f32> {
    match style.width {
        Dimension::Length(px) if px.is_finite() && px > 0.0 => Some(px),
        Dimension::Percentage(pct) if pct > 0.0 => Some(available * pct / 100.0),
        _ => None,
    }
}

/// Get the `span` count for a `<col>` element (defaults to 1).
fn col_span_count(_style: &ComputedStyle) -> usize {
    // CSS does not have a dedicated ComputedStyle field for col span;
    // the HTML `span` attribute is typically 1. Future: read from Attributes.
    1
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
        };
        let cap_lb = layout_child(dom, cap, &cap_input);
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
        };
        let cell_lb = layout_child(dom, cell.entity, &cell_input);

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
        let cell_y = cursor_y + row_y_offsets[cell.row];

        let cell_width = cell_available_width(&col_widths, cell, spacing_h, num_cols);
        let effective_width =
            collapse_adjusted_width(cell_width, is_collapse, &collapsed_borders[i]);

        // Calculate cell height (sum of spanned rows + internal spacing).
        let span_end = span_end_row(cell, num_rows);
        let cell_height: f32 = row_heights[cell.row..span_end].iter().sum::<f32>()
            + spacing_v * (cell.rowspan as f32 - 1.0).max(0.0);

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
        };
        let cell_lb = layout_child(dom, cell.entity, &cell_relayout_input);
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
        };
        let cap_lb = layout_child(dom, cap, &cap_input);
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

    // Layout positioned descendants owned by this containing block.
    // CSS 2.1 §17.2: the table establishes a CB for absolute children
    // when it is itself positioned (or is the root).
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root;
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

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

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

/// Parameters for column width computation.
///
/// `collapsed_borders`, `content_width`, and `is_collapse` are reserved
/// for Phase 4 (border-collapse-aware column sizing).
#[allow(dead_code)]
struct TableColumnInput<'a> {
    style: &'a ComputedStyle,
    cells: &'a [CellInfo],
    cell_styles: &'a [ComputedStyle],
    collapsed_borders: &'a [CollapsedBorders],
    /// Per-column widths from `<col>`/`<colgroup>` elements (CSS 2.1 §17.5.2.1).
    /// `None` means the column has no col-specified width.
    col_element_widths: &'a [Option<f32>],
    num_cols: usize,
    available_for_cols: f32,
    content_width: f32,
    is_collapse: bool,
}

/// Compute column widths based on table-layout algorithm.
///
/// `collapsed_borders`, `content_width`, and `is_collapse` are reserved
/// for Phase 4 (border-collapse-aware column sizing) and currently unused.
#[must_use]
fn compute_column_widths(
    dom: &mut EcsDom,
    params: &TableColumnInput<'_>,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
) -> Vec<f32> {
    let style = params.style;
    let cells = params.cells;
    let cell_styles = params.cell_styles;
    // params.collapsed_borders, params.content_width, params.is_collapse reserved for Phase 4.
    let num_cols = params.num_cols;
    let available_for_cols = params.available_for_cols;
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
        let mut widths = fixed_column_widths(
            num_cols,
            &first_row_cell_infos,
            &first_row_explicit,
            params.col_element_widths,
            available_for_cols,
        );
        // Col element widths are already applied inside fixed_column_widths.
        // Clamp negative widths.
        for w in &mut widths {
            *w = w.max(0.0);
        }
        widths
    } else {
        // Auto table layout: measure cell intrinsic widths.
        let cell_widths: Vec<(f32, f32)> = cells
            .iter()
            .map(|cell| {
                // Layout cell with a very small width to get min-content,
                // and with a very large width to get max-content.
                let min_input = LayoutInput {
                    containing_width: 1.0, // min-content probe
                    containing_height: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    font_db,
                    depth: depth + 1,
                    float_ctx: None,
                    viewport: None,
                };
                let min_lb = layout_child(dom, cell.entity, &min_input);
                let max_input = LayoutInput {
                    containing_width: f32::MAX / 4.0, // max-content probe
                    containing_height: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    font_db,
                    depth: depth + 1,
                    float_ctx: None,
                    viewport: None,
                };
                let max_lb = layout_child(dom, cell.entity, &max_input);
                let cs = elidex_layout_block::get_style(dom, cell.entity);
                let p = elidex_layout_block::resolve_padding(&cs, available_for_cols);
                let b = sanitize_border(&cs);
                let cell_h_pb = horizontal_pb(&p, &b);
                // min-content = content width from narrow probe + cell pb
                let min_w = min_lb.content.width + cell_h_pb;
                // max-content = content width from wide probe + cell pb
                let max_w = max_lb.content.width + cell_h_pb;
                (min_w.max(0.0), max_w.max(min_w))
            })
            .collect();
        let mut widths = auto_column_widths(num_cols, cells, &cell_widths, available_for_cols);
        // Apply col element widths as minimum constraints (CSS 2.1 §17.5.2.2).
        for (col, w) in widths.iter_mut().enumerate() {
            if let Some(Some(col_w)) = params.col_element_widths.get(col) {
                *w = w.max(*col_w);
            }
        }
        widths
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
        content: Rect::new(content_x, content_y, content_width, content_height),
        padding: *padding,
        border: *border,
        margin: *margin,
    }
}
