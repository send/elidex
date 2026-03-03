//! Block child stacking, shifting, and height resolution.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, EdgeSizes, LayoutBox};
use elidex_text::FontDatabase;

use crate::{adjust_min_max_for_border_box, clamp_min_max, resolve_min_max, vertical_pb};

use super::is_block_level;
use super::margin::{collapse_margins, resolve_margin};

/// Result of stacking block children, including margin info for parent-child collapse.
pub struct StackResult {
    /// Total content height consumed by stacked children.
    pub height: f32,
    /// Top margin of the first block child (for parent-child collapse).
    pub first_child_margin_top: Option<f32>,
    /// Bottom margin of the last block child (for parent-child collapse).
    pub last_child_margin_bottom: Option<f32>,
}

/// Stack block-level children with vertical margin collapse.
///
/// Shared by block children layout and document-root layout. Returns
/// the total height consumed and first/last child margin info for
/// parent-child collapse (CSS 2.1 §8.3.1).
#[allow(clippy::too_many_arguments)]
pub fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: crate::ChildLayoutFn,
) -> StackResult {
    let mut cursor_y = offset_y;
    let mut prev_margin_bottom: Option<f32> = None;
    let mut first_child_margin_top: Option<f32> = None;
    let mut last_child_margin_bottom: Option<f32> = None;

    for &child in children {
        let Some(child_style) = crate::try_get_style(dom, child) else {
            continue; // text node in block context: skip
        };
        let (child_display, child_margin_top_dim) = (child_style.display, child_style.margin_top);

        if !is_block_level(child_display) {
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        // Both positive -> max, both negative -> min, mixed -> sum.
        let child_margin_top = resolve_margin(child_margin_top_dim, containing_width);
        if first_child_margin_top.is_none() {
            first_child_margin_top = Some(child_margin_top);
        }
        if let Some(prev_mb) = prev_margin_bottom {
            let collapsed = collapse_margins(prev_mb, child_margin_top);
            cursor_y -= prev_mb + child_margin_top - collapsed;
        }

        // Dispatch child layout via callback (routes to block/flex/grid
        // based on the child's display type).
        let child_box = layout_child(
            dom,
            child,
            containing_width,
            containing_height,
            offset_x,
            cursor_y,
            font_db,
            depth,
        );
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
        last_child_margin_bottom = Some(child_box.margin.bottom);
    }

    StackResult {
        height: cursor_y - offset_y,
        first_child_margin_top,
        last_child_margin_bottom,
    }
}

/// Shift all block-level children's `LayoutBox.content.y` by `delta`,
/// recursively including descendants.
///
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
pub(super) fn shift_block_children(dom: &mut EcsDom, children: &[Entity], delta: f32) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    for &child in children {
        let is_block = crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display));
        if !is_block {
            continue;
        }
        if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(child) {
            lb.content.y += delta;
        }
        // Recurse into descendants so nested layout positions stay consistent.
        let grandchildren = dom.children(child);
        if !grandchildren.is_empty() {
            shift_block_children(dom, &grandchildren, delta);
        }
    }
}

/// Resolve the final height for a block element.
///
/// Handles CSS height property (Length/Percentage/Auto), border-box adjustment,
/// and min-height/max-height constraints. `content_height` is used when the
/// height is auto.
pub fn resolve_block_height(
    style: &ComputedStyle,
    content_height: f32,
    containing_height: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    is_replaced: bool,
) -> f32 {
    let mut height = if is_replaced {
        content_height
    } else {
        match style.height {
            Dimension::Length(px) if px.is_finite() => {
                if style.box_sizing == BoxSizing::BorderBox {
                    (px - vertical_pb(padding, border)).max(0.0)
                } else {
                    px
                }
            }
            Dimension::Percentage(pct) => containing_height.map_or(content_height, |ch| {
                let resolved = ch * pct / 100.0;
                if style.box_sizing == BoxSizing::BorderBox {
                    (resolved - vertical_pb(padding, border)).max(0.0)
                } else {
                    resolved
                }
            }),
            _ => content_height,
        }
    };

    // Apply min-height / max-height constraints.
    let ch = containing_height.unwrap_or(0.0);
    let mut min_h = resolve_min_max(style.min_height, ch, 0.0);
    let mut max_h = resolve_min_max(style.max_height, ch, f32::INFINITY);
    if style.box_sizing == BoxSizing::BorderBox && !is_replaced {
        adjust_min_max_for_border_box(&mut min_h, &mut max_h, vertical_pb(padding, border));
    }
    height = clamp_min_max(height, min_h, max_h);
    height
}
