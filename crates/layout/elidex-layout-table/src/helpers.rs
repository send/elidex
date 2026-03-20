//! Helper functions for table layout (extracted from `lib.rs`).
//!
//! Contains column width computation, cell positioning helpers,
//! and the `TableColumnInput` parameter struct.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_layout_block::{horizontal_pb, sanitize_border, ChildLayoutFn, LayoutInput};
use elidex_plugin::{
    BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox, Rect, TableLayout,
};
use elidex_text::FontDatabase;

use crate::algo::{auto_column_widths, fixed_column_widths, CollapsedBorders};
use crate::grid::CellInfo;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Total outer height of a layout box (margin + border + padding + content).
#[inline]
#[must_use]
pub(crate) fn box_total_height(lb: &LayoutBox) -> f32 {
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
pub(crate) fn collapse_adjusted_width(
    cell_width: f32,
    is_collapse: bool,
    cb: &CollapsedBorders,
) -> f32 {
    if is_collapse {
        (cell_width - cb.left / 2.0 - cb.right / 2.0).max(0.0)
    } else {
        cell_width
    }
}

/// Build the final `LayoutBox` for the table element.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_table_layout_box(
    padding: &EdgeSizes,
    border: &EdgeSizes,
    margin: &EdgeSizes,
    content_x: f32,
    content_y: f32,
    content_width: f32,
    content_height: f32,
    first_baseline: Option<f32>,
) -> LayoutBox {
    LayoutBox {
        content: Rect::new(content_x, content_y, content_width, content_height),
        padding: *padding,
        border: *border,
        margin: *margin,
        first_baseline,
    }
}

// ---------------------------------------------------------------------------
// Column width helpers
// ---------------------------------------------------------------------------

/// Collect per-column widths from `<col>` and `<colgroup>` elements.
///
/// Walks the table's children, expanding `<colgroup>` into its child `<col>`
/// elements. Returns a vec of `Option<f32>` indexed by column, where `None`
/// means no col-specified width for that column.
#[must_use]
pub(crate) fn collect_col_widths(
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
                        let span = col_span_count(dom, cc);
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
                let span = col_span_count(dom, child);
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

/// Get the `span` count for a `<col>` element from its HTML `span` attribute.
///
/// Defaults to 1 if the attribute is absent or invalid. Clamped to 1000 max
/// to prevent degenerate tables (WHATWG §4.9.11).
pub(crate) fn col_span_count(dom: &EcsDom, entity: Entity) -> usize {
    let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
        return 1;
    };
    attrs
        .get("span")
        .and_then(|val| val.parse::<u32>().ok())
        .filter(|&n| n >= 1)
        .map_or(1, |n| n.min(1000) as usize)
}

// ---------------------------------------------------------------------------
// Table height
// ---------------------------------------------------------------------------

/// Resolve the table's content height.
#[must_use]
pub(crate) fn resolve_table_height(
    style: &ComputedStyle,
    containing_height: Option<f32>,
    v_pb: f32,
    min_height: f32,
) -> f32 {
    let explicit = match style.height {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - v_pb).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                (resolved - v_pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    };
    explicit.unwrap_or(min_height).max(min_height)
}

// ---------------------------------------------------------------------------
// Column width computation
// ---------------------------------------------------------------------------

/// Parameters for column width computation.
///
/// `collapsed_borders`, `content_width`, and `is_collapse` are reserved
/// for Phase 4 (border-collapse-aware column sizing).
#[allow(dead_code)]
pub(crate) struct TableColumnInput<'a> {
    pub(crate) style: &'a ComputedStyle,
    pub(crate) cells: &'a [CellInfo],
    pub(crate) cell_styles: &'a [ComputedStyle],
    pub(crate) collapsed_borders: &'a [CollapsedBorders],
    /// Per-column widths from `<col>`/`<colgroup>` elements (CSS 2.1 §17.5.2.1).
    /// `None` means the column has no col-specified width.
    pub(crate) col_element_widths: &'a [Option<f32>],
    pub(crate) num_cols: usize,
    pub(crate) available_for_cols: f32,
    pub(crate) content_width: f32,
    pub(crate) is_collapse: bool,
}

/// Compute column widths based on table-layout algorithm.
///
/// `collapsed_borders`, `content_width`, and `is_collapse` are reserved
/// for Phase 4 (border-collapse-aware column sizing) and currently unused.
#[must_use]
pub(crate) fn compute_column_widths(
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
                    fragmentainer: None,
                    break_token: None,
                };
                let min_lb = layout_child(dom, cell.entity, &min_input).layout_box;
                let max_input = LayoutInput {
                    containing_width: f32::MAX / 4.0, // max-content probe
                    containing_height: None,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    font_db,
                    depth: depth + 1,
                    float_ctx: None,
                    viewport: None,
                    fragmentainer: None,
                    break_token: None,
                };
                let max_lb = layout_child(dom, cell.entity, &max_input).layout_box;
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
