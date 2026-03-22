//! Helper functions for table layout (extracted from `lib.rs`).
//!
//! Contains column width computation, cell positioning helpers,
//! and the `TableColumnInput` parameter struct.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_layout_block::{horizontal_pb, sanitize_border, LayoutEnv, LayoutInput};
use elidex_plugin::{
    BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox, Point, Rect, TableLayout,
};

use crate::algo::{auto_column_widths, fixed_column_widths, CollapsedBorders};
use crate::grid::CellInfo;
use crate::ColGroupBorderInfo;

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
        + lb.content.size.height
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
pub(crate) fn build_table_layout_box(
    padding: &EdgeSizes,
    border: &EdgeSizes,
    margin: &EdgeSizes,
    content_origin: Point,
    content_width: f32,
    content_height: f32,
    first_baseline: Option<f32>,
) -> LayoutBox {
    LayoutBox {
        content: Rect::new(
            content_origin.x,
            content_origin.y,
            content_width,
            content_height,
        ),
        padding: *padding,
        border: *border,
        margin: *margin,
        first_baseline,
        layout_generation: 0,
    }
}

// ---------------------------------------------------------------------------
// Column width helpers
// ---------------------------------------------------------------------------

/// Collect per-column widths, styles, and column-group ranges in a single walk.
///
/// Combines width resolution and border-style collection for `<col>`/`<colgroup>`
/// elements (CSS 2.1 §17.5.2.1 + §17.6.2.1).
///
/// Returns `(col_widths, col_styles, col_group_infos)` where:
/// - `col_widths[c]` is the resolved pixel width from the `<col>` covering column `c` (if any)
/// - `col_styles[c]` is the `ComputedStyle` of the `<col>` covering column `c` (if any)
/// - `col_group_infos` records each `<colgroup>` range and style
#[must_use]
pub(crate) fn collect_col_info(
    dom: &EcsDom,
    children: &[Entity],
    num_cols: usize,
    available_for_cols: f32,
) -> (
    Vec<Option<f32>>,
    Vec<Option<ComputedStyle>>,
    Vec<ColGroupBorderInfo>,
) {
    let mut widths = vec![None::<f32>; num_cols];
    let mut styles: Vec<Option<ComputedStyle>> = vec![None; num_cols];
    let mut col_groups: Vec<ColGroupBorderInfo> = Vec::new();
    let mut col_idx = 0;

    for &child in children {
        if col_idx >= num_cols {
            break;
        }
        let child_style = elidex_layout_block::get_style(dom, child);
        match child_style.display {
            Display::TableColumnGroup => {
                let group_start = col_idx;
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
                                if widths[col_idx].is_none() {
                                    widths[col_idx] = w;
                                }
                                styles[col_idx] = Some(cc_style.clone());
                                col_idx += 1;
                            }
                        }
                    }
                    col_child = dom.get_next_sibling(cc);
                }
                // If the colgroup has no col children, apply its span attribute.
                if col_idx == group_start {
                    let span = col_span_count(dom, child);
                    for _ in 0..span {
                        if col_idx < num_cols {
                            col_idx += 1;
                        }
                    }
                }
                col_groups.push(ColGroupBorderInfo {
                    style: child_style,
                    start_col: group_start,
                    end_col: col_idx,
                });
            }
            Display::TableColumn => {
                let span = col_span_count(dom, child);
                let w = resolve_col_width(&child_style, available_for_cols);
                for _ in 0..span {
                    if col_idx < num_cols {
                        if widths[col_idx].is_none() {
                            widths[col_idx] = w;
                        }
                        styles[col_idx] = Some(child_style.clone());
                        col_idx += 1;
                    }
                }
            }
            _ => {}
        }
    }
    (widths, styles, col_groups)
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
    pub(crate) is_collapse: bool,
}

/// Compute column widths based on table-layout algorithm.
///
/// In the collapsing border model, cell intrinsic widths are adjusted by
/// subtracting the collapsed border half-widths (CSS 2.1 §17.6.2).
#[must_use]
pub(crate) fn compute_column_widths(
    dom: &mut EcsDom,
    params: &TableColumnInput<'_>,
    env: &LayoutEnv<'_>,
) -> Vec<f32> {
    let style = params.style;
    let cells = params.cells;
    let cell_styles = params.cell_styles;
    let collapsed_borders = params.collapsed_borders;
    let is_collapse = params.is_collapse;
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
                let raw = match cs.width {
                    Dimension::Length(px) if px.is_finite() && px > 0.0 => Some(px),
                    Dimension::Percentage(pct) if pct > 0.0 => {
                        Some(available_for_cols * pct / 100.0)
                    }
                    _ => None,
                };
                // In the collapsing border model, subtract collapsed border half-widths
                // from explicit cell widths (CSS 2.1 §17.6.2, F-4 fix).
                if is_collapse {
                    raw.map(|w| {
                        let cb = &collapsed_borders[i];
                        (w - f32::midpoint(cb.left, cb.right)).max(0.0)
                    })
                } else {
                    raw
                }
            })
            .collect();
        let first_row_cell_infos: Vec<CellInfo> =
            first_row.iter().map(|&(_, c)| c.clone()).collect();
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
            .enumerate()
            .map(|(i, cell)| {
                // Layout cell with a very small width to get min-content,
                // and with a very large width to get max-content.
                let min_input = LayoutInput::probe(env, 1.0);
                let min_lb = (env.layout_child)(dom, cell.entity, &min_input).layout_box;
                let max_input = LayoutInput::probe(env, f32::MAX / 4.0);
                let max_lb = (env.layout_child)(dom, cell.entity, &max_input).layout_box;
                let cs = elidex_layout_block::get_style(dom, cell.entity);
                let p = elidex_layout_block::resolve_padding(&cs, available_for_cols);
                let b = sanitize_border(&cs);
                let cell_h_pb = horizontal_pb(&p, &b);
                // min-content = content width from narrow probe + cell pb
                let mut min_w = min_lb.content.size.width + cell_h_pb;
                // max-content = content width from wide probe + cell pb
                let mut max_w = max_lb.content.size.width + cell_h_pb;
                // In the collapsing border model, subtract collapsed border half-widths
                // from intrinsic sizes (CSS 2.1 §17.6.2).
                if is_collapse {
                    let cb = &collapsed_borders[i];
                    let border_adj = f32::midpoint(cb.left, cb.right);
                    min_w = (min_w - border_adj).max(0.0);
                    max_w = (max_w - border_adj).max(0.0);
                }
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
