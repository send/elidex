//! Baseline alignment helpers for flex layout (CSS Flexbox §9.4).

use elidex_ecs::EcsDom;
use elidex_plugin::AlignItems;

use super::FlexContext;
use super::FlexItem;

/// Read each item's baseline from the ECS `LayoutBox` after cross-size layout.
///
/// The baseline is stored as an offset from the item's margin-box cross-start edge,
/// so it can be compared directly across items in the same line.
pub(crate) fn read_item_baselines(dom: &mut EcsDom, items: &mut [FlexItem], ctx: &FlexContext) {
    for item in items.iter_mut() {
        if item.align != AlignItems::Baseline {
            continue;
        }
        if let Ok(child_lb) = dom.world().get::<&elidex_plugin::LayoutBox>(item.entity) {
            let cross_start_pb = if ctx.horizontal {
                child_lb.padding.top + child_lb.border.top
            } else {
                child_lb.padding.left + child_lb.border.left
            };
            item.first_baseline = child_lb
                .first_baseline
                .map(|bl| item.margin_cross_start + cross_start_pb + bl);
        }
    }
}

/// Compute per-line maximum baseline for baseline alignment.
///
/// For each line, returns the maximum baseline offset (from the margin-box
/// cross-start edge) among items with `align: Baseline` and no auto cross margins.
/// Column flex in horizontal writing mode: baseline not applicable, returns 0.
pub(crate) fn compute_line_baselines(
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    horizontal: bool,
) -> Vec<f32> {
    line_ranges
        .iter()
        .map(|&(s, e)| {
            // Column flex: baseline alignment requires inline axis parallel to cross axis.
            // In horizontal writing mode with column direction, use 0 (not applicable).
            if !horizontal {
                return 0.0;
            }
            items[s..e]
                .iter()
                .filter(|i| {
                    i.align == AlignItems::Baseline
                        && !i.margin_cross_start_auto
                        && !i.margin_cross_end_auto
                })
                // CSS Flexbox §9.4: synthesized baseline = border-box bottom
                // from margin-box cross-start = margin_cross_start + final_cross.
                .map(FlexItem::baseline_or_synthesized)
                .fold(0.0_f32, f32::max)
        })
        .collect()
}
