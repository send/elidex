//! Table intrinsic sizing (CSS 2.1 §17.5.2).

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    composed_children_flat, get_style, total_gap, IntrinsicSizes, LayoutEnv,
};
use elidex_plugin::{Display, WritingModeContext};

use super::compute_intrinsic_sizes;

/// Table intrinsic sizing (CSS 2.1 §17.5.2).
///
/// Walks the table structure (rows → cells) to build per-column min/max widths,
/// then sums columns.
pub(super) fn compute_table_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
) -> IntrinsicSizes {
    let mut col_min: Vec<f32> = Vec::new();
    let mut col_max: Vec<f32> = Vec::new();

    // Walk rows (direct or inside row-groups) → cells.
    let mut rows: Vec<Entity> = Vec::new();
    for &child in children {
        let child_style = elidex_layout_block::try_get_style(dom, child);
        match child_style.as_ref().map(|s| s.display) {
            Some(Display::TableRow) => rows.push(child),
            Some(
                Display::TableHeaderGroup | Display::TableRowGroup | Display::TableFooterGroup,
            ) => {
                // Row group: collect its row children.
                let group_children = composed_children_flat(dom, child);
                for &gc in &group_children {
                    if elidex_layout_block::try_get_style(dom, gc)
                        .is_some_and(|s| s.display == Display::TableRow)
                    {
                        rows.push(gc);
                    }
                }
            }
            Some(Display::TableCell) => {
                // Direct cell (anonymous row): treat as column 0.
                let cell_sizes = compute_intrinsic_sizes(dom, child, &env.deeper());
                if col_min.is_empty() {
                    col_min.push(0.0);
                    col_max.push(0.0);
                }
                col_min[0] = col_min[0].max(cell_sizes.min_content);
                col_max[0] = col_max[0].max(cell_sizes.max_content);
            }
            _ => {}
        }
    }

    // Colspan-aware intrinsic sizing: spanning cells distribute their intrinsic
    // width equally across spanned columns.
    for &row in &rows {
        let cells = composed_children_flat(dom, row);
        let mut col_idx = 0;
        for &cell in &cells {
            // Skip display:none and non-cell children.
            let cell_style = elidex_layout_block::try_get_style(dom, cell);
            if cell_style
                .as_ref()
                .is_some_and(|s| s.display == Display::None)
            {
                continue;
            }
            // Read colspan from HTML attribute.
            let colspan = dom
                .world()
                .get::<&elidex_ecs::Attributes>(cell)
                .ok()
                .and_then(|attrs| attrs.get("colspan").and_then(|s| s.parse::<usize>().ok()))
                .unwrap_or(1)
                .clamp(1, 1000);

            let cell_sizes = compute_intrinsic_sizes(dom, cell, &env.deeper());

            // Grow column vectors as needed.
            while col_min.len() < col_idx + colspan {
                col_min.push(0.0);
                col_max.push(0.0);
            }

            if colspan == 1 {
                col_min[col_idx] = col_min[col_idx].max(cell_sizes.min_content);
                col_max[col_idx] = col_max[col_idx].max(cell_sizes.max_content);
            } else {
                // Distribute spanning cell's intrinsic width equally across columns.
                #[allow(clippy::cast_precision_loss)] // colspan clamped to 1..=1000
                let col_f = colspan as f32;
                let per_min = cell_sizes.min_content / col_f;
                let per_max = cell_sizes.max_content / col_f;
                for c in col_idx..col_idx + colspan {
                    col_min[c] = col_min[c].max(per_min);
                    col_max[c] = col_max[c].max(per_max);
                }
            }
            col_idx += colspan;
        }
    }

    let style = get_style(dom, entity);
    let inline_horizontal =
        WritingModeContext::new(style.writing_mode, style.direction).is_horizontal();
    // CSS 2.1 §17.6.2: in the collapsing border model, border-spacing is ignored.
    // Use inline-axis border-spacing (horizontal in horizontal-tb, vertical in vertical modes).
    let gap = if style.border_collapse == elidex_plugin::BorderCollapse::Collapse {
        0.0
    } else if inline_horizontal {
        style.border_spacing_h.max(0.0)
    } else {
        style.border_spacing_v.max(0.0)
    };
    let gap_total = total_gap(col_min.len(), gap);

    IntrinsicSizes {
        min_content: col_min.iter().sum::<f32>() + gap_total,
        max_content: col_max.iter().sum::<f32>() + gap_total,
    }
}
