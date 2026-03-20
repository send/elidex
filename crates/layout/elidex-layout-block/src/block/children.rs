//! Block child stacking, shifting, and height resolution.

use std::cell::RefCell;
use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    BoxSizing, Clear, ComputedStyle, Dimension, Display, EdgeSizes, Float, LayoutBox,
};

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
    /// Static positions for absolutely positioned descendants (CSS 2.1 §10.6.5).
    pub static_positions: HashMap<Entity, (f32, f32)>,
    /// First baseline from children (CSS 2.1 §10.8.1).
    ///
    /// For block formatting contexts: the first in-flow child's baseline,
    /// offset-adjusted to the parent's content area.
    /// For anonymous inline runs: the inline layout's first baseline.
    pub first_baseline: Option<f32>,
}

/// Stack block-level children with vertical margin collapse.
///
/// Shared by block children layout and document-root layout. Returns
/// the total height consumed and first/last child margin info for
/// parent-child collapse (CSS 2.1 §8.3.1).
///
/// Floated children (CSS 2.1 §9.5) are removed from normal flow and
/// placed via the float context. Cleared children advance past floats.
///
/// Consecutive non-block children (text nodes and inline elements) are
/// wrapped in anonymous block boxes per CSS 2.1 §9.2.1.1 — their inline
/// content is laid out via [`layout_inline_context`](crate::inline::layout_inline_context).
#[allow(clippy::too_many_lines)]
pub fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    input: &LayoutInput<'_>,
    layout_child: crate::ChildLayoutFn,
    is_bfc: bool,
    parent_entity: Entity,
) -> StackResult {
    let mut cursor_y = input.offset_y;
    let mut prev_margin_bottom: Option<f32> = None;
    let mut first_child_margin_top: Option<f32> = None;
    let mut last_child_margin_bottom: Option<f32> = None;
    // CSS 2.1 §9.5: BFC-establishing elements create their own FloatContext.
    // Non-BFC blocks forward the ancestor's FloatContext (via RefCell) so
    // that floats inside non-BFC children are visible to the enclosing BFC.
    let local_ctx = RefCell::new(FloatContext::new(input.containing_width));
    let float_ctx: &RefCell<FloatContext> = if let Some(ancestor_ctx) = input.float_ctx {
        if is_bfc {
            &local_ctx
        } else {
            ancestor_ctx
        }
    } else {
        &local_ctx
    };
    let mut inline_run: Vec<Entity> = Vec::new();
    let mut static_positions: HashMap<Entity, (f32, f32)> = HashMap::new();
    let mut first_baseline: Option<f32> = None;

    for &child in children {
        let child_style = crate::try_get_style(dom, child);

        // Skip display: none entirely.
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }

        // CSS 2.1 §9.3.1/§9.6: absolutely positioned elements are removed from flow.
        // Record static position before skipping (CSS 2.1 §10.6.5).
        if child_style
            .as_ref()
            .is_some_and(crate::positioned::is_absolutely_positioned)
        {
            static_positions.insert(child, (input.offset_x, cursor_y));
            continue;
        }

        let is_block = child_style
            .as_ref()
            .is_some_and(|s| is_block_level(s.display));

        if !is_block {
            // Text node or inline element — collect for anonymous block box.
            inline_run.push(child);
            continue;
        }

        // Flush any pending inline run before this block child (CSS 2.1 §9.2.1.1).
        if !inline_run.is_empty() {
            let run_result = flush_inline_run(
                dom,
                &inline_run,
                parent_entity,
                input,
                cursor_y,
                layout_child,
                &mut static_positions,
            );
            // Capture baseline from inline run (offset by cursor_y - input.offset_y).
            if first_baseline.is_none() {
                first_baseline = run_result
                    .first_baseline
                    .map(|bl| (cursor_y - input.offset_y) + bl);
            }
            cursor_y += run_result.height;
            // Anonymous block box has zero margins.
            if first_child_margin_top.is_none() {
                first_child_margin_top = Some(0.0);
            }
            prev_margin_bottom = Some(0.0);
            last_child_margin_bottom = Some(0.0);
            inline_run.clear();
        }

        let child_style = child_style.unwrap();
        let child_float = child_style.float;
        let child_clear = child_style.clear;
        let child_margin_top_dim = child_style.margin_top;

        // --- Clear: advance past floats (CSS 2.1 §9.5.2) ---
        // Applied to both floated and non-floated children.
        let has_clearance = if child_clear == Clear::None {
            false
        } else {
            let new_y = float_ctx.borrow().clear_y(child_clear, cursor_y);
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
                float_ctx,
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
            float_ctx: Some(float_ctx),
            ..*input
        };
        let child_box = layout_child(dom, child, &child_input).layout_box;
        // Capture baseline from first in-flow block child (CSS 2.1 §10.8.1).
        if first_baseline.is_none() {
            if let Some(child_bl) = child_box.first_baseline {
                // Offset: child content.y relative to parent content area top.
                first_baseline = Some(child_box.content.y - input.offset_y + child_bl);
            }
        }
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
        last_child_margin_bottom = Some(child_box.margin.bottom);
    }

    // Flush trailing inline run (CSS 2.1 §9.2.1.1).
    if !inline_run.is_empty() {
        let run_result = flush_inline_run(
            dom,
            &inline_run,
            parent_entity,
            input,
            cursor_y,
            layout_child,
            &mut static_positions,
        );
        if first_baseline.is_none() {
            first_baseline = run_result
                .first_baseline
                .map(|bl| (cursor_y - input.offset_y) + bl);
        }
        cursor_y += run_result.height;
        if first_child_margin_top.is_none() {
            first_child_margin_top = Some(0.0);
        }
        last_child_margin_bottom = Some(0.0);
    }

    // CSS 2.1 §10.6.7: Only elements that establish a BFC have their
    // height increased to contain floats. Non-BFC blocks let floats overflow.
    let normal_height = cursor_y - input.offset_y;
    let height = if is_bfc {
        let float_bottom = float_ctx.borrow().float_bottom();
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
        static_positions,
        first_baseline,
    }
}

/// Result of flushing an inline run.
struct InlineRunResult {
    height: f32,
    first_baseline: Option<f32>,
}

/// Flush an inline run as an anonymous block box (CSS 2.1 §9.2.1.1).
///
/// Lays out consecutive inline/text children via the inline formatting
/// context and returns the total height consumed and the first baseline.
fn flush_inline_run(
    dom: &mut EcsDom,
    inline_children: &[Entity],
    parent_entity: Entity,
    input: &LayoutInput<'_>,
    cursor_y: f32,
    layout_child: crate::ChildLayoutFn,
    static_positions: &mut HashMap<Entity, (f32, f32)>,
) -> InlineRunResult {
    let parent_style = crate::get_style(dom, parent_entity);
    let content_origin = (input.offset_x, cursor_y);
    let result = crate::inline::layout_inline_context(
        dom,
        inline_children,
        input.containing_width,
        &parent_style,
        input.font_db,
        parent_entity,
        content_origin,
        layout_child,
    );
    static_positions.extend(result.static_positions);
    InlineRunResult {
        height: result.height,
        first_baseline: result.first_baseline,
    }
}

/// Compute the max-content width of an element for shrink-to-fit sizing.
///
/// Recursively walks block and inline children. Block children contribute
/// their own max-content width (or explicit width if set). Inline children
/// contribute the sum of text widths without line breaking.
/// Capped at [`crate::MAX_LAYOUT_DEPTH`] recursion depth.
pub(crate) fn max_content_width(
    dom: &EcsDom,
    entity: Entity,
    font_db: &elidex_text::FontDatabase,
    depth: u32,
) -> f32 {
    if depth >= crate::MAX_LAYOUT_DEPTH {
        return 0.0;
    }
    let style = crate::get_style(dom, entity);
    let children = crate::composed_children_flat(dom, entity);
    if children.is_empty() {
        return 0.0;
    }

    let has_block = children
        .iter()
        .any(|&c| crate::try_get_style(dom, c).is_some_and(|s| is_block_level(s.display)));

    if has_block {
        // Mixed or all-block children: take max of each child's contribution.
        let mut max_w = 0.0_f32;
        let mut inline_run: Vec<Entity> = Vec::new();

        for &child in &children {
            let child_style = crate::try_get_style(dom, child);
            if child_style
                .as_ref()
                .is_some_and(|s| s.display == Display::None)
            {
                continue;
            }
            let child_is_block = child_style
                .as_ref()
                .is_some_and(|s| is_block_level(s.display));
            if !child_is_block {
                inline_run.push(child);
                continue;
            }
            // Flush inline run before block child.
            if !inline_run.is_empty() {
                let inline_w = crate::inline::max_content_inline_size(
                    dom,
                    &inline_run,
                    &style,
                    entity,
                    font_db,
                );
                max_w = max_w.max(inline_w);
                inline_run.clear();
            }
            let cs = child_style.unwrap();
            let cp = crate::sanitize_padding(&cs);
            let cb = crate::sanitize_border(&cs);
            let child_h_pb = crate::horizontal_pb(&cp, &cb);
            let child_w = match cs.width {
                Dimension::Length(px) if px.is_finite() => {
                    if cs.box_sizing == BoxSizing::BorderBox {
                        px.max(0.0)
                    } else {
                        px + child_h_pb
                    }
                }
                Dimension::Percentage(_) => {
                    // Percentage width is indeterminate for max-content; use content.
                    max_content_width(dom, child, font_db, depth + 1) + child_h_pb
                }
                _ => max_content_width(dom, child, font_db, depth + 1) + child_h_pb,
            };
            max_w = max_w.max(child_w);
        }
        // Flush trailing inline run.
        if !inline_run.is_empty() {
            let inline_w =
                crate::inline::max_content_inline_size(dom, &inline_run, &style, entity, font_db);
            max_w = max_w.max(inline_w);
        }
        max_w
    } else {
        // All inline children — sum of text widths without line breaking.
        crate::inline::max_content_inline_size(dom, &children, &style, entity, font_db)
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
    float_ctx: &RefCell<FloatContext>,
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

    let padding = crate::resolve_padding(&child_style, containing_width);
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
        // CSS 2.1 §10.3.5: shrink-to-fit width for auto-width floats.
        // preferred = max-content width, available = containing - margins - pb.
        _ => {
            let available = (containing_width - margin_left - margin_right - h_pb).max(0.0);
            let preferred = max_content_width(dom, child, input.font_db, input.depth);
            preferred.min(available).max(0.0)
        }
    };

    // Layout the float's contents at a temporary position.
    let temp_input = LayoutInput {
        containing_width: shrink_width.max(0.0),
        containing_height: input.containing_height,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: input.font_db,
        depth: input.depth + 1,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
    };
    let child_box = layout_child(dom, child, &temp_input).layout_box;
    let content_width = child_box.content.width;
    let content_height = child_box.content.height;

    // Margin box dimensions for float placement.
    let margin_box_width = content_width + h_pb + margin_left + margin_right;
    let margin_box_height =
        content_height + crate::vertical_pb(&padding, &border) + margin_top + margin_bottom;

    // Place the float via FloatContext.
    let (float_x, float_y) = float_ctx.borrow_mut().place_float(
        float_side,
        margin_box_width,
        margin_box_height,
        cursor_y,
    );

    // Reposition the float's LayoutBox to the placed position.
    // float_x is relative to the containing block; add parent offset for absolute position.
    let final_x = input.offset_x + float_x + margin_left + border.left + padding.left;
    let final_y = float_y + margin_top + border.top + padding.top;

    // Overwrite the LayoutBox that layout_child inserted at a temporary
    // position — hecs `insert_one` on an existing component is an upsert.
    let lb = LayoutBox {
        content: elidex_plugin::Rect::new(final_x, final_y, content_width, content_height),
        padding,
        border,
        margin: EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
        first_baseline: child_box.first_baseline,
    };
    let _ = dom.world_mut().insert_one(child, lb);

    // Reposition descendants relative to the new origin.
    let delta_x = final_x - child_box.content.x;
    let delta_y = final_y - child_box.content.y;
    if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(child);
        shift_descendants(dom, &grandchildren, (delta_x, delta_y));
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

/// Shift descendants by (dx, dy), used to reposition float/positioned contents after placement.
pub fn shift_descendants(dom: &mut EcsDom, children: &[Entity], delta: (f32, f32)) {
    shift_descendants_inner(dom, children, delta.0, delta.1, false);
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
