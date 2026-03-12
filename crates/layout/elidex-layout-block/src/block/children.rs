//! Block child stacking, shifting, and height resolution.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{BoxSizing, Clear, ComputedStyle, Dimension, EdgeSizes, Float, LayoutBox};

use crate::{
    adjust_min_max_for_border_box, clamp_min_max, resolve_min_max, vertical_pb, LayoutInput,
};

use super::float::FloatContext;
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
///
/// Floated children (CSS 2.1 §9.5) are removed from normal flow and
/// placed via the float context. Cleared children advance past floats.
pub fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    input: &LayoutInput<'_>,
    layout_child: crate::ChildLayoutFn,
    is_bfc: bool,
) -> StackResult {
    let mut cursor_y = input.offset_y;
    let mut prev_margin_bottom: Option<f32> = None;
    let mut first_child_margin_top: Option<f32> = None;
    let mut last_child_margin_bottom: Option<f32> = None;
    // TODO(Phase 4): CSS 2.1 §9.5 — floats in non-BFC blocks should
    // propagate to the nearest ancestor BFC. Currently each block creates
    // a fresh FloatContext, so floats are scoped to their immediate parent.
    // Proper propagation requires threading the float context through the
    // layout tree.
    let mut float_ctx = FloatContext::new(input.containing_width);

    for &child in children {
        let Some(child_style) = crate::try_get_style(dom, child) else {
            continue; // text node in block context: skip
        };
        let child_display = child_style.display;
        let child_float = child_style.float;
        let child_clear = child_style.clear;
        let child_margin_top_dim = child_style.margin_top;

        if !is_block_level(child_display) {
            continue;
        }

        // --- Clear: advance past floats (CSS 2.1 §9.5.2) ---
        // Applied to both floated and non-floated children.
        let has_clearance = if child_clear == Clear::None {
            false
        } else {
            let new_y = float_ctx.clear_y(child_clear, cursor_y);
            let cleared = new_y > cursor_y;
            cursor_y = new_y;
            cleared
        };

        // --- Floated children: out of normal flow (CSS 2.1 §9.5) ---
        if child_float != Float::None {
            layout_float(
                dom,
                child,
                child_float,
                &mut float_ctx,
                input,
                cursor_y,
                layout_child,
            );
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        // Both positive -> max, both negative -> min, mixed -> sum.
        // Clearance breaks margin adjacency — margins do not collapse when
        // the element has clearance (CSS 2.1 §8.3.1).
        let child_margin_top = resolve_margin(child_margin_top_dim, input.containing_width);
        // Only record a first-child top margin for parent/child collapse
        // when the child does not have clearance; clearance breaks
        // margin adjacency with the parent as well (CSS 2.1 §8.3.1).
        if first_child_margin_top.is_none() && !has_clearance {
            first_child_margin_top = Some(child_margin_top);
        }
        if let Some(prev_mb) = prev_margin_bottom {
            if !has_clearance {
                let collapsed = collapse_margins(prev_mb, child_margin_top);
                cursor_y -= prev_mb + child_margin_top - collapsed;
            }
        }

        // Dispatch child layout via callback (routes to block/flex/grid
        // based on the child's display type).
        let child_input = LayoutInput {
            offset_y: cursor_y,
            ..*input
        };
        let child_box = layout_child(dom, child, &child_input);
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
        last_child_margin_bottom = Some(child_box.margin.bottom);
    }

    // CSS 2.1 §10.6.7: Only elements that establish a BFC have their
    // height increased to contain floats. Non-BFC blocks let floats overflow.
    let normal_height = cursor_y - input.offset_y;
    let height = if is_bfc {
        let float_bottom = float_ctx.float_bottom();
        let float_extend = if float_bottom > 0.0 {
            (float_bottom - input.offset_y).max(0.0)
        } else {
            0.0
        };
        normal_height.max(float_extend)
    } else {
        normal_height
    };

    StackResult {
        height,
        first_child_margin_top,
        last_child_margin_bottom,
    }
}

/// Layout a floated child and place it via the float context.
///
/// Floated elements use shrink-to-fit width: they do not expand to fill
/// the containing block (CSS 2.1 §10.3.5).
fn layout_float(
    dom: &mut EcsDom,
    child: Entity,
    float_side: Float,
    float_ctx: &mut FloatContext,
    input: &LayoutInput<'_>,
    cursor_y: f32,
    layout_child: crate::ChildLayoutFn,
) {
    let child_style = crate::get_style(dom, child);
    let containing_width = input.containing_width;

    // Resolve margins for the float's margin box.
    let margin_top = resolve_margin(child_style.margin_top, containing_width);
    let margin_right = resolve_margin(child_style.margin_right, containing_width);
    let margin_bottom = resolve_margin(child_style.margin_bottom, containing_width);
    let margin_left = resolve_margin(child_style.margin_left, containing_width);

    let padding = crate::sanitize_padding(&child_style);
    let border = crate::sanitize_border(&child_style);
    let h_pb = crate::horizontal_pb(&padding, &border);

    // Shrink-to-fit width: use specified width if given, otherwise
    // use 0 as auto (content will determine actual width).
    // For simplicity, we layout the float at position (0, 0) first to
    // get its dimensions, then reposition.
    let shrink_width = match child_style.width {
        Dimension::Length(px) if px.is_finite() => {
            if child_style.box_sizing == BoxSizing::BorderBox {
                (px - h_pb).max(0.0)
            } else {
                px
            }
        }
        Dimension::Percentage(pct) => {
            let resolved = containing_width * pct / 100.0;
            if child_style.box_sizing == BoxSizing::BorderBox {
                (resolved - h_pb).max(0.0)
            } else {
                resolved
            }
        }
        // TODO(Phase 4): CSS 2.1 §10.3.5 — float with auto width should use
        // shrink-to-fit: min(max(preferred_min_width, available), preferred_width).
        // Currently uses available width as a fallback until intrinsic sizing is
        // implemented.
        _ => containing_width - margin_left - margin_right - h_pb,
    };

    // Layout the float's contents at a temporary position.
    let temp_input = LayoutInput {
        containing_width: shrink_width.max(0.0),
        containing_height: input.containing_height,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: input.font_db,
        depth: input.depth + 1,
    };
    let child_box = layout_child(dom, child, &temp_input);
    let content_width = child_box.content.width;
    let content_height = child_box.content.height;

    // Margin box dimensions for float placement.
    let margin_box_width = content_width + h_pb + margin_left + margin_right;
    let margin_box_height =
        content_height + crate::vertical_pb(&padding, &border) + margin_top + margin_bottom;

    // Place the float via FloatContext.
    let (float_x, float_y) =
        float_ctx.place_float(float_side, margin_box_width, margin_box_height, cursor_y);

    // Reposition the float's LayoutBox to the placed position.
    // float_x is relative to the containing block; add parent offset for absolute position.
    let final_x = input.offset_x + float_x + margin_left + border.left + padding.left;
    let final_y = float_y + margin_top + border.top + padding.top;

    // Overwrite the LayoutBox that layout_child inserted at a temporary
    // position — hecs `insert_one` on an existing component is an upsert.
    let lb = LayoutBox {
        content: elidex_plugin::Rect {
            x: final_x,
            y: final_y,
            width: content_width,
            height: content_height,
        },
        padding,
        border,
        margin: EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
    };
    let _ = dom.world_mut().insert_one(child, lb);

    // Reposition descendants relative to the new origin.
    let delta_x = final_x - child_box.content.x;
    let delta_y = final_y - child_box.content.y;
    if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(child);
        shift_descendants(dom, &grandchildren, delta_x, delta_y);
    }
}

/// Shift all block-level children's `LayoutBox.content.y` by `delta`,
/// iteratively including descendants.
///
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
pub(super) fn shift_block_children(dom: &mut EcsDom, children: &[Entity], delta: f32) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    shift_descendants_inner(dom, children, 0.0, delta, true);
}

/// Shift descendants by (dx, dy), used to reposition float contents after placement.
fn shift_descendants(dom: &mut EcsDom, children: &[Entity], dx: f32, dy: f32) {
    shift_descendants_inner(dom, children, dx, dy, false);
}

/// Iterative tree walk that shifts `LayoutBox` positions by `(dx, dy)`.
///
/// When `block_only` is true, only block-level entities (with a `ComputedStyle`)
/// are shifted; non-block children are skipped (but their descendants are still
/// walked).
fn shift_descendants_inner(
    dom: &mut EcsDom,
    children: &[Entity],
    dx: f32,
    dy: f32,
    block_only: bool,
) {
    let mut stack: Vec<Entity> = children.to_vec();
    while let Some(child) = stack.pop() {
        let skip_shift = block_only
            && !crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display));
        if !skip_shift {
            if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(child) {
                lb.content.x += dx;
                lb.content.y += dy;
            }
        }
        // Always walk descendants regardless of block_only filter.
        stack.extend(dom.composed_children(child));
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
