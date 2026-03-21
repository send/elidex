//! Block child stacking, shifting, and height resolution.

use std::cell::RefCell;
use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    BoxSizing, Clear, ComputedStyle, Dimension, Display, EdgeSizes, Float, LayoutBox,
    WritingModeContext,
};

use crate::{adjust_min_max_for_border_box, clamp_min_max, resolve_min_max, LayoutInput};

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

/// Stack block-level children along the block axis with margin collapse.
///
/// Writing-mode-aware: the block axis is vertical in `horizontal-tb` and
/// horizontal in vertical writing modes. Children are stacked along the
/// block axis, with margin collapse applied on block-axis margins.
///
/// Shared by block children layout and document-root layout. Returns
/// the total block extent consumed and first/last child margin info for
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
    let parent_style = crate::get_style(dom, parent_entity);
    let wm = WritingModeContext::new(parent_style.writing_mode, parent_style.direction);
    let is_horizontal = wm.is_horizontal();

    // Block-axis cursor: tracks Y in horizontal-tb, X in vertical modes.
    let mut cursor_block = if is_horizontal {
        input.offset_y
    } else {
        input.offset_x
    };
    let cursor_block_origin = cursor_block;

    let mut prev_margin_block_end: Option<f32> = None;
    let mut first_child_margin_block_start: Option<f32> = None;
    let mut last_child_margin_block_end: Option<f32> = None;

    // CSS 2.1 §9.5: BFC-establishing elements create their own FloatContext.
    // FloatContext works in abstract 2D space: "containing_width" = inline-axis size,
    // "y" positions = block-axis positions.
    let inline_containing = input.containing_inline_size;
    let local_ctx = RefCell::new(FloatContext::new(inline_containing));
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
            let static_pos = if is_horizontal {
                (input.offset_x, cursor_block)
            } else {
                (cursor_block, input.offset_y)
            };
            static_positions.insert(child, static_pos);
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
                wm,
                cursor_block,
                layout_child,
                &mut static_positions,
            );
            if first_baseline.is_none() {
                first_baseline = run_result
                    .first_baseline
                    .map(|bl| (cursor_block - cursor_block_origin) + bl);
            }
            cursor_block += run_result.block_extent;
            // Anonymous block box has zero margins.
            if first_child_margin_block_start.is_none() {
                first_child_margin_block_start = Some(0.0);
            }
            prev_margin_block_end = Some(0.0);
            last_child_margin_block_end = Some(0.0);
            inline_run.clear();
        }

        let child_style = child_style.unwrap();
        let child_float = child_style.float;
        let child_clear = child_style.clear;

        // Block-start margin of the child (for collapse).
        // CSS WM: block-start = top (horizontal), right (vertical-rl), left (vertical-lr).
        let child_margin_block_start_dim = if is_horizontal {
            child_style.margin_top
        } else if wm.is_block_reversed() {
            child_style.margin_right
        } else {
            child_style.margin_left
        };

        // --- Clear: advance past floats (CSS 2.1 §9.5.2) ---
        // FloatContext clear_y operates on block-axis positions.
        let has_clearance = if child_clear == Clear::None {
            false
        } else {
            let new_block = float_ctx.borrow().clear_y(child_clear, cursor_block);
            let cleared = new_block > cursor_block;
            cursor_block = new_block;
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
                wm,
                cursor_block,
                layout_child,
            );
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        let child_margin_block_start =
            resolve_margin(child_margin_block_start_dim, input.containing_inline_size);
        if first_child_margin_block_start.is_none() && !has_clearance {
            first_child_margin_block_start = Some(child_margin_block_start);
        }
        if let Some(prev_mbe) = prev_margin_block_end {
            if !has_clearance {
                let collapsed = collapse_margins(prev_mbe, child_margin_block_start);
                cursor_block -= prev_mbe + child_margin_block_start - collapsed;
            }
        }

        // Dispatch child layout via callback.
        // Set the block-axis offset in the child input.
        let child_input = if is_horizontal {
            LayoutInput {
                offset_y: cursor_block,
                float_ctx: Some(float_ctx),
                ..*input
            }
        } else {
            LayoutInput {
                offset_x: cursor_block,
                float_ctx: Some(float_ctx),
                ..*input
            }
        };
        let child_box = layout_child(dom, child, &child_input).layout_box;

        // Capture baseline from first in-flow block child (CSS 2.1 §10.8.1).
        if first_baseline.is_none() {
            if let Some(child_bl) = child_box.first_baseline {
                let child_block_pos = if is_horizontal {
                    child_box.content.y
                } else {
                    child_box.content.x
                };
                first_baseline = Some(child_block_pos - cursor_block_origin + child_bl);
            }
        }

        // Advance cursor by the child's block extent (margin box).
        let child_block_extent = if is_horizontal {
            child_box.margin_box().height
        } else {
            child_box.margin_box().width
        };
        cursor_block += child_block_extent;

        // Track block-end margin for collapse with next sibling.
        let child_margin_block_end = if is_horizontal {
            child_box.margin.bottom
        } else if wm.is_block_reversed() {
            child_box.margin.left
        } else {
            child_box.margin.right
        };
        prev_margin_block_end = Some(child_margin_block_end);
        last_child_margin_block_end = Some(child_margin_block_end);
    }

    // Flush trailing inline run (CSS 2.1 §9.2.1.1).
    if !inline_run.is_empty() {
        let run_result = flush_inline_run(
            dom,
            &inline_run,
            parent_entity,
            input,
            wm,
            cursor_block,
            layout_child,
            &mut static_positions,
        );
        if first_baseline.is_none() {
            first_baseline = run_result
                .first_baseline
                .map(|bl| (cursor_block - cursor_block_origin) + bl);
        }
        cursor_block += run_result.block_extent;
        if first_child_margin_block_start.is_none() {
            first_child_margin_block_start = Some(0.0);
        }
        last_child_margin_block_end = Some(0.0);
    }

    // CSS 2.1 §10.6.7: Only elements that establish a BFC have their
    // block extent increased to contain floats.
    let normal_extent = cursor_block - cursor_block_origin;
    let height = if is_bfc {
        let float_bottom = float_ctx.borrow().float_bottom();
        let float_extend = if float_bottom > 0.0 {
            (float_bottom - cursor_block_origin).max(0.0)
        } else {
            0.0
        };
        normal_extent.max(float_extend)
    } else {
        normal_extent
    };

    StackResult {
        height,
        first_child_margin_top: first_child_margin_block_start,
        last_child_margin_bottom: last_child_margin_block_end,
        static_positions,
        first_baseline,
    }
}

/// Result of flushing an inline run.
struct InlineRunResult {
    /// Block-axis extent consumed by the inline run.
    block_extent: f32,
    first_baseline: Option<f32>,
}

/// Flush an inline run as an anonymous block box (CSS 2.1 §9.2.1.1).
///
/// Lays out consecutive inline/text children via the inline formatting
/// context and returns the block extent consumed and the first baseline.
/// Writing-mode-aware: uses inline-axis size for the line width and
/// positions the content origin based on the block cursor.
#[allow(clippy::too_many_arguments)]
fn flush_inline_run(
    dom: &mut EcsDom,
    inline_children: &[Entity],
    parent_entity: Entity,
    input: &LayoutInput<'_>,
    wm: WritingModeContext,
    cursor_block: f32,
    layout_child: crate::ChildLayoutFn,
    static_positions: &mut HashMap<Entity, (f32, f32)>,
) -> InlineRunResult {
    let parent_style = crate::get_style(dom, parent_entity);
    let is_horizontal = wm.is_horizontal();

    // Inline-axis available size for line breaking.
    // CSS Writing Modes Level 3 §3.1: In vertical modes, inline axis = vertical.
    let inline_size = if is_horizontal {
        input.containing_width
    } else {
        input.containing_height.unwrap_or(input.containing_width)
    };

    // Content origin: inline-start position is fixed, block position is the cursor.
    let content_origin = if is_horizontal {
        (input.offset_x, cursor_block)
    } else {
        (cursor_block, input.offset_y)
    };

    let result = crate::inline::layout_inline_context(
        dom,
        inline_children,
        inline_size,
        &parent_style,
        input.font_db,
        parent_entity,
        content_origin,
        layout_child,
    );
    static_positions.extend(result.static_positions);

    // In vertical modes, layout_inline_context returns the total column width
    // (block-axis extent) as `height`, which is the block extent we need.
    InlineRunResult {
        block_extent: result.height,
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
/// Writing-mode-aware: `float: left` = inline-start, `float: right` = inline-end
/// (CSS Writing Modes Level 3 §3.3). `FloatContext` operates in abstract
/// inline/block space; this function converts to physical coordinates
/// for the final `LayoutBox`.
///
/// Floated elements use shrink-to-fit inline-size: they do not expand to
/// fill the containing block (CSS 2.1 §10.3.5).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_float(
    dom: &mut EcsDom,
    child: Entity,
    float_side: Float,
    float_ctx: &RefCell<FloatContext>,
    input: &LayoutInput<'_>,
    wm: WritingModeContext,
    cursor_block: f32,
    layout_child: crate::ChildLayoutFn,
) {
    let child_style = crate::get_style(dom, child);
    let is_horizontal = wm.is_horizontal();

    // Resolve margins for the float's margin box.
    // CSS Box Model Level 3 §5.3: margin % refers to containing block's inline size.
    let containing_inline = input.containing_inline_size;
    let margin_top = resolve_margin(child_style.margin_top, containing_inline);
    let margin_right = resolve_margin(child_style.margin_right, containing_inline);
    let margin_bottom = resolve_margin(child_style.margin_bottom, containing_inline);
    let margin_left = resolve_margin(child_style.margin_left, containing_inline);

    let padding = crate::resolve_padding(&child_style, containing_inline);
    let border = crate::sanitize_border(&child_style);

    // Inline-axis padding+border for shrink-to-fit sizing.
    let i_pb = crate::inline_pb(&wm, &padding, &border);
    let b_pb = crate::block_pb(&wm, &padding, &border);

    // Inline-axis margins.
    let (margin_inline_start, margin_inline_end) = if is_horizontal {
        (margin_left, margin_right)
    } else {
        (margin_top, margin_bottom)
    };
    // Block-axis margins.
    let (margin_block_start, margin_block_end) = if is_horizontal {
        (margin_top, margin_bottom)
    } else if wm.is_block_reversed() {
        (margin_right, margin_left)
    } else {
        (margin_left, margin_right)
    };

    // Inline-size dimension from style (CSS width is physical, but float sizing
    // operates on inline-axis: width in horizontal, height in vertical).
    let inline_size_dim = if is_horizontal {
        child_style.width
    } else {
        child_style.height
    };

    // Shrink-to-fit inline-size.
    let shrink_inline = match inline_size_dim {
        Dimension::Length(px) if px.is_finite() => {
            if child_style.box_sizing == BoxSizing::BorderBox {
                (px - i_pb).max(0.0)
            } else {
                px
            }
        }
        Dimension::Percentage(pct) => {
            let resolved = containing_inline * pct / 100.0;
            if child_style.box_sizing == BoxSizing::BorderBox {
                (resolved - i_pb).max(0.0)
            } else {
                resolved
            }
        }
        // CSS 2.1 §10.3.5: shrink-to-fit for auto inline-size floats.
        _ => {
            let available =
                (containing_inline - margin_inline_start - margin_inline_end - i_pb).max(0.0);
            let preferred = max_content_width(dom, child, input.font_db, input.depth);
            preferred.min(available).max(0.0)
        }
    };

    // Layout the float's contents at a temporary position.
    let temp_input = LayoutInput {
        containing_width: if is_horizontal {
            shrink_inline.max(0.0)
        } else {
            input.containing_width
        },
        containing_height: if is_horizontal {
            input.containing_height
        } else {
            Some(shrink_inline.max(0.0))
        },
        containing_inline_size: shrink_inline.max(0.0),
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: input.font_db,
        depth: input.depth + 1,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let child_box = layout_child(dom, child, &temp_input).layout_box;
    let content_width = child_box.content.width;
    let content_height = child_box.content.height;

    // Physical → inline/block extents for float placement.
    let (content_inline, content_block) = if is_horizontal {
        (content_width, content_height)
    } else {
        (content_height, content_width)
    };

    // Margin box dimensions in inline/block space for float placement.
    let margin_box_inline = content_inline + i_pb + margin_inline_start + margin_inline_end;
    let margin_box_block = content_block + b_pb + margin_block_start + margin_block_end;

    // Place the float via FloatContext (operates in inline/block space).
    let (float_inline, float_block) = float_ctx.borrow_mut().place_float(
        float_side,
        margin_box_inline,
        margin_box_block,
        cursor_block,
    );

    // Convert float position from inline/block to physical coordinates.
    // float_inline is relative to the containing block's inline-start edge.
    let l_padding = elidex_plugin::LogicalEdges::from_physical(padding, wm);
    let l_border = elidex_plugin::LogicalEdges::from_physical(border, wm);

    let final_inline =
        float_inline + margin_inline_start + l_border.inline_start + l_padding.inline_start;
    let final_block =
        float_block + margin_block_start + l_border.block_start + l_padding.block_start;

    // Convert to physical (x, y).
    let (final_x, final_y) = if is_horizontal {
        (input.offset_x + final_inline, final_block)
    } else {
        // In vertical modes, the inline position maps to Y, block to X.
        // offset_y is the inline-axis origin.
        (final_block, input.offset_y + final_inline)
    };

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

/// Shift all block-level children along the block axis by `delta`,
/// iteratively including descendants.
///
/// Writing-mode-aware: shifts Y in `horizontal-tb`, X in vertical modes.
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
pub(super) fn shift_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    delta: f32,
    wm: WritingModeContext,
) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    if wm.is_horizontal() {
        shift_descendants_inner(dom, children, 0.0, delta, true);
    } else {
        shift_descendants_inner(dom, children, delta, 0.0, true);
    }
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

/// Resolve the final block-axis size for a block element.
///
/// Writing-mode-aware: resolves the block-axis dimension (`height` in
/// `horizontal-tb`, `width` in vertical modes) with border-box adjustment
/// and min/max block-size constraints.
///
/// `content_block_size` is used when the block-size is auto.
pub fn resolve_block_height(
    style: &ComputedStyle,
    content_block_size: f32,
    containing_block_size: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    is_replaced: bool,
    wm: &WritingModeContext,
) -> f32 {
    let is_horizontal = wm.is_horizontal();

    // Block-axis dimension: height in horizontal, width in vertical.
    let block_dim = if is_horizontal {
        style.height
    } else {
        style.width
    };
    // Block-axis min/max constraints.
    let (min_block_dim, max_block_dim) = if is_horizontal {
        (style.min_height, style.max_height)
    } else {
        (style.min_width, style.max_width)
    };
    // Block-axis padding+border.
    let b_pb = crate::block_pb(wm, padding, border);

    let mut block_size = if is_replaced {
        content_block_size
    } else {
        match block_dim {
            Dimension::Length(px) if px.is_finite() => {
                if style.box_sizing == BoxSizing::BorderBox {
                    (px - b_pb).max(0.0)
                } else {
                    px
                }
            }
            Dimension::Percentage(pct) => containing_block_size.map_or(content_block_size, |cb| {
                let resolved = cb * pct / 100.0;
                if style.box_sizing == BoxSizing::BorderBox {
                    (resolved - b_pb).max(0.0)
                } else {
                    resolved
                }
            }),
            _ => content_block_size,
        }
    };

    // Apply min/max block-size constraints.
    let cb = containing_block_size.unwrap_or(0.0);
    let mut min_b = resolve_min_max(min_block_dim, cb, 0.0);
    let mut max_b = resolve_min_max(max_block_dim, cb, f32::INFINITY);
    if style.box_sizing == BoxSizing::BorderBox && !is_replaced {
        adjust_min_max_for_border_box(&mut min_b, &mut max_b, b_pb);
    }
    block_size = clamp_min_max(block_size, min_b, max_b);
    block_size
}
