//! Helper functions for block child stacking.

use std::cell::RefCell;
use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    BoxSizing, Dimension, Display, EdgeSizes, Float, LayoutBox, Point, WritingModeContext,
};

use crate::LayoutInput;

use super::super::float::FloatContext;
use super::super::is_block_level;
use super::super::margin::resolve_margin;
use super::shift::shift_descendants;

/// Result of flushing an inline run.
pub(super) struct InlineRunResult {
    /// Block-axis extent consumed by the inline run.
    pub block_extent: f32,
    pub first_baseline: Option<f32>,
    /// If fragmentation was applied, the line index after which to break.
    pub break_after_line: Option<usize>,
}

/// Flush an inline run as an anonymous block box (CSS 2.1 §9.2.1.1).
///
/// Lays out consecutive inline/text children via the inline formatting
/// context and returns the block extent consumed and the first baseline.
/// Writing-mode-aware: uses inline-axis size for the line width and
/// positions the content origin based on the block cursor.
///
/// `skip_lines` is set when resuming an inline run after a fragmentation
/// break — it tells the inline layout engine how many line boxes to skip.
#[allow(clippy::too_many_arguments)]
pub(super) fn flush_inline_run(
    dom: &mut EcsDom,
    inline_children: &[Entity],
    parent_entity: Entity,
    input: &LayoutInput<'_>,
    wm: WritingModeContext,
    cursor_block: f32,
    cursor_block_origin: f32,
    env: &crate::LayoutEnv<'_>,
    static_positions: &mut HashMap<Entity, Point>,
    skip_lines: usize,
) -> InlineRunResult {
    let parent_style = crate::get_style(dom, parent_entity);
    let is_horizontal = wm.is_horizontal();

    // Inline-axis available size for line breaking.
    // CSS Writing Modes Level 3 §3.1: In vertical modes, inline axis = vertical.
    let inline_size = if is_horizontal {
        input.containing.width
    } else {
        input.containing.height_or_width()
    };

    // Content origin: inline-start position is fixed, block position is the cursor.
    let content_origin = if is_horizontal {
        Point::new(input.offset.x, cursor_block)
    } else {
        Point::new(cursor_block, input.offset.y)
    };

    // Build fragmentation constraint for inline layout if fragmentainer is active.
    let frag_constraint = input.fragmentainer.map(|frag| {
        let consumed = (cursor_block - cursor_block_origin).max(0.0);
        crate::inline::InlineFragConstraint {
            available_block: (frag.available_block_size - consumed).max(0.0),
            orphans: parent_style.orphans,
            widows: parent_style.widows,
            skip_lines,
        }
    });

    let result = crate::inline::layout_inline_context_fragmented(
        dom,
        inline_children,
        inline_size,
        parent_entity,
        content_origin,
        env,
        frag_constraint.as_ref(),
    );
    static_positions.extend(result.static_positions);

    // In vertical modes, layout_inline_context returns the total column width
    // (block-axis extent) as `height`, which is the block extent we need.
    InlineRunResult {
        block_extent: result.height,
        first_baseline: result.first_baseline,
        break_after_line: result.break_after_line,
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
pub(super) fn layout_float(
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
        containing: elidex_plugin::CssSize {
            width: if is_horizontal {
                shrink_inline.max(0.0)
            } else {
                input.containing.width
            },
            height: if is_horizontal {
                input.containing.height
            } else {
                Some(shrink_inline.max(0.0))
            },
        },
        containing_inline_size: shrink_inline.max(0.0),
        offset: elidex_plugin::Point::ZERO,
        font_db: input.font_db,
        depth: input.depth + 1,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let child_box = layout_child(dom, child, &temp_input).layout_box;
    let content_width = child_box.content.size.width;
    let content_height = child_box.content.size.height;

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
    let float_pos = float_ctx.borrow_mut().place_float(
        float_side,
        margin_box_inline,
        margin_box_block,
        cursor_block,
    );
    let (float_inline, float_block) = (float_pos.x, float_pos.y);

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
        (input.offset.x + final_inline, final_block)
    } else {
        // In vertical modes, the inline position maps to Y, block to X.
        // offset_y is the inline-axis origin.
        (final_block, input.offset.y + final_inline)
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
    let delta_x = final_x - child_box.content.origin.x;
    let delta_y = final_y - child_box.content.origin.y;
    if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(child);
        shift_descendants(
            dom,
            &grandchildren,
            elidex_plugin::Vector::new(delta_x, delta_y),
        );
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
    style: &elidex_plugin::ComputedStyle,
    content_block_size: f32,
    containing_block_size: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    is_replaced: bool,
    wm: &WritingModeContext,
) -> f32 {
    use crate::{adjust_min_max_for_border_box, clamp_min_max, resolve_min_max};

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
