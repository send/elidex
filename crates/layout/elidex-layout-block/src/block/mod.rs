//! Block formatting context layout algorithm.
//!
//! Computes the CSS box model (content, padding, border, margin) for
//! block-level elements, handling width/height resolution, margin auto
//! centering, and vertical stacking of child blocks.

pub(crate) mod children;
pub mod float;
pub(crate) mod fragmentation;
mod margin;
mod replaced;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    BoxSizing, Dimension, Display, EdgeSizes, Float, LayoutBox, LogicalEdges, Overflow, Position,
    Rect, WritingModeContext,
};
use elidex_text::FontDatabase;

use crate::sanitize;
use crate::{
    adjust_min_max_for_border_box, clamp_min_max, resolve_dimension_value, resolve_min_max,
    sanitize_border, LayoutInput, MAX_LAYOUT_DEPTH,
};

pub use children::{resolve_block_height, shift_descendants, stack_block_children, StackResult};
pub use margin::resolve_margin;

use children::shift_block_children;
use margin::{apply_margin_auto_centering, collapse_margins};
use replaced::{resolve_replaced_height, resolve_replaced_width};

/// Returns `true` if the display value establishes a block-level box.
///
/// Atomic inline-level boxes (`inline-block`, `inline-flex`, `inline-grid`,
/// `inline-table`) are NOT block-level — they participate in inline
/// formatting context (CSS 2.1 §9.2.2) and are laid out as atomic units
/// within inline flow.
pub fn is_block_level(display: Display) -> bool {
    matches!(
        display,
        Display::Block
            | Display::Flex
            | Display::Grid
            | Display::ListItem
            | Display::Table
            | Display::TableCaption
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableRow
            | Display::TableCell
            | Display::TableColumn
            | Display::TableColumnGroup
    )
}

/// Returns `true` if any child is block-level (block formatting context).
///
/// When this returns `true` and inline children are also present,
/// [`stack_block_children`] wraps consecutive inline runs in anonymous
/// block boxes (CSS 2.1 §9.2.1.1).
fn children_are_block(dom: &EcsDom, children: &[Entity]) -> bool {
    children
        .iter()
        .any(|&child| crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display)))
}

/// Layout a block-level element, inserting `LayoutBox` on it and all descendants.
pub fn layout_block(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height: None,
        containing_inline_size: containing_width,
        offset_x,
        offset_y,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    layout_block_inner(dom, entity, &input, crate::layout_block_only).layout_box
}

/// Layout a block-level element with an explicit containing height.
///
/// Used when the parent has a definite height (e.g. `height: 500px`) so that
/// children with `height: 50%` can be resolved.
pub fn layout_block_with_height(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height,
        containing_inline_size: containing_width,
        offset_x,
        offset_y,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    layout_block_inner(dom, entity, &input, crate::layout_block_only).layout_box
}

/// Inner recursive implementation with depth tracking.
///
/// Writing-mode-aware: resolves inline-size and block-size based on the
/// element's `writing-mode`, stacks children along the block axis, and
/// produces a physical `LayoutBox` at the end.
///
/// Uses the provided `layout_child` callback to dispatch child layout,
/// which allows the orchestrator to route flex/grid containers to
/// their respective layout algorithms.
#[allow(clippy::too_many_lines)]
// Sequential algorithm phases sharing extensive local state; splitting would add indirection without improving clarity.
pub fn layout_block_inner(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: crate::ChildLayoutFn,
) -> crate::LayoutOutcome {
    let containing_width = input.containing_width;
    let containing_height = input.containing_height;
    let offset_x = input.offset_x;
    let offset_y = input.offset_y;
    let font_db = input.font_db;
    let depth = input.depth;

    let style = crate::get_style(dom, entity);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let is_horizontal = wm.is_horizontal();

    // --- Containing dimensions for inline and block axes ---
    // `containing_inline` is for margin/padding % (CSS Box Model L3 §5.3),
    // determined by the **containing block's** writing mode.
    let containing_inline = input.containing_inline_size;
    let containing_block: Option<f32> = if is_horizontal {
        input.containing_height
    } else {
        Some(input.containing_width)
    };
    // Available inline space for auto inline-size resolution.
    // This is the containing block's dimension along **this element's** inline axis.
    // Differs from containing_inline in orthogonal flow (CSS WM L3 §7.3.1).
    let available_inline = if is_horizontal {
        containing_width
    } else {
        // Vertical element: inline axis = Y. Available = containing height.
        // If indefinite, fall back to containing width (approximate).
        containing_height.unwrap_or(containing_width)
    };

    // --- Resolve padding and border (protect against NaN/infinity/negative) ---
    // CSS Box Model Level 3 §5.3: padding/margin percentages refer to the
    // containing block's **inline size** (containing_inline_size).
    let padding = crate::resolve_padding(&style, containing_inline);
    let border = sanitize_border(&style);
    let i_pb = crate::inline_pb(&wm, &padding, &border);
    let l_padding = LogicalEdges::from_physical(padding, wm);
    let l_border = LogicalEdges::from_physical(border, wm);

    // --- Resolve margins (all four sides, then map to inline/block) ---
    let margin_top = resolve_margin(style.margin_top, containing_inline);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_inline);
    let margin_left_raw = resolve_margin(style.margin_left, containing_inline);
    let margin_right_raw = resolve_margin(style.margin_right, containing_inline);

    // Block-axis margins (for collapse). Physical: top/bottom in horizontal,
    // left/right (depending on block direction) in vertical.
    let (margin_block_start, margin_block_end) = if is_horizontal {
        (margin_top, margin_bottom)
    } else if wm.is_block_reversed() {
        (margin_right_raw, margin_left_raw)
    } else {
        (margin_left_raw, margin_right_raw)
    };

    // Inline-axis margin dimensions (for auto centering).
    let (margin_inline_start_dim, margin_inline_end_dim) = if is_horizontal {
        (style.margin_left, style.margin_right)
    } else {
        (style.margin_top, style.margin_bottom)
    };
    let margin_inline_start_raw = if is_horizontal {
        margin_left_raw
    } else {
        margin_top
    };
    let margin_inline_end_raw = if is_horizontal {
        margin_right_raw
    } else {
        margin_bottom
    };

    // --- Check for replaced element (e.g. <img> with decoded ImageData) ---
    let intrinsic = crate::get_intrinsic_size(dom, entity);

    // --- Resolve inline-size ---
    // CSS: `width` and `height` are physical properties. In vertical modes,
    // physical height = inline-size, physical width = block-size.
    let inline_size_dim = if is_horizontal {
        style.width
    } else {
        style.height
    };
    let inline_extra = margin_inline_start_raw + margin_inline_end_raw + i_pb;
    let mut content_inline = if let Some((iw, ih)) = intrinsic {
        // Replaced element: resolve physical dimensions, then extract inline-axis.
        let phys_w = resolve_replaced_width(&style, containing_width, iw, ih, &padding, &border);
        if is_horizontal {
            phys_w
        } else {
            resolve_replaced_height(&style, phys_w, iw, ih, &padding, &border)
        }
    } else {
        sanitize(resolve_dimension_value(
            inline_size_dim,
            available_inline,
            (available_inline - inline_extra).max(0.0),
        ))
    };
    // box-sizing: border-box — subtract inline p+b from specified inline-size.
    if style.box_sizing == BoxSizing::BorderBox && intrinsic.is_none() {
        if let Dimension::Length(_) | Dimension::Percentage(_) = inline_size_dim {
            content_inline = (content_inline - i_pb).max(0.0);
        }
    }

    // --- Apply min/max inline-size constraints ---
    let (min_inline_dim, max_inline_dim) = if is_horizontal {
        (style.min_width, style.max_width)
    } else {
        (style.min_height, style.max_height)
    };
    {
        let mut min_i = resolve_min_max(min_inline_dim, containing_inline, 0.0);
        let mut max_i = resolve_min_max(max_inline_dim, containing_inline, f32::INFINITY);
        if style.box_sizing == BoxSizing::BorderBox && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_i, &mut max_i, i_pb);
        }
        content_inline = clamp_min_max(content_inline, min_i, max_i);
    }

    // --- Inline-axis margin auto centering ---
    let used_inline = content_inline + i_pb;
    let (margin_inline_start, margin_inline_end) =
        if matches!(inline_size_dim, Dimension::Auto) && intrinsic.is_none() {
            (margin_inline_start_raw, margin_inline_end_raw)
        } else {
            apply_margin_auto_centering(
                margin_inline_start_dim,
                margin_inline_end_dim,
                available_inline,
                used_inline,
                style.direction,
            )
        };

    // --- Compute physical content position ---
    // Inline position: offset along inline axis + inline-start margin/border/padding.
    // Block position: offset along block axis + block-start margin/border/padding.
    let (content_x, content_y);
    if is_horizontal {
        content_x = offset_x + margin_inline_start + border.left + padding.left;
        content_y = offset_y + margin_block_start + border.top + padding.top;
    } else {
        // Vertical: inline axis = Y, block axis = X.
        content_x = offset_x + margin_block_start + l_border.block_start + l_padding.block_start;
        content_y = offset_y + margin_inline_start + l_border.inline_start + l_padding.inline_start;
    }

    // --- Layout children (stop recursion at depth limit) ---
    let children = crate::composed_children_flat(dom, entity);
    let mut collapsed_margin_block_start = margin_block_start;
    let mut collapsed_margin_block_end = margin_block_end;

    // Compute definite block-size for children's percentage resolution.
    // In horizontal: explicit height. In vertical: explicit width.
    let child_containing_block = if is_horizontal {
        crate::resolve_explicit_height(&style, containing_height)
    } else {
        // In vertical mode, block-size = physical width. If style.width is explicit,
        // use it; otherwise None.
        match style.width {
            Dimension::Length(px) if px.is_finite() => {
                let b_pb_val = crate::block_pb(&wm, &padding, &border);
                if style.box_sizing == BoxSizing::BorderBox {
                    Some((px - b_pb_val).max(0.0))
                } else {
                    Some(px)
                }
            }
            Dimension::Percentage(pct) => containing_block.map(|cb| {
                let resolved = cb * pct / 100.0;
                let b_pb_val = crate::block_pb(&wm, &padding, &border);
                if style.box_sizing == BoxSizing::BorderBox {
                    (resolved - b_pb_val).max(0.0)
                } else {
                    resolved
                }
            }),
            _ => None,
        }
    };

    // Physical dimensions for child containing block.
    let (child_phys_width, child_phys_height) = if is_horizontal {
        (content_inline, child_containing_block)
    } else {
        // Vertical: inline-size = physical height, block-size = physical width.
        // content_inline is the inline-size (physical height).
        // child_containing_block is the block-size (physical width) if known.
        (
            child_containing_block.unwrap_or(containing_width),
            Some(content_inline),
        )
    };

    let mut static_positions_stash = std::collections::HashMap::new();
    let mut block_first_baseline: Option<f32> = None;
    let mut child_break_token: Option<crate::BreakToken> = None;
    let mut propagated_break_before: Option<elidex_plugin::BreakValue> = None;
    let mut propagated_break_after: Option<elidex_plugin::BreakValue> = None;
    // Inline-only break line (separate from child_break_token to avoid double-wrapping).
    let mut inline_break_after_line: Option<usize> = None;
    let mut inline_content_block: f32 = 0.0;

    // --- Fragmentation: compute child fragmentainer context ---
    let is_first_fragment = input.break_token.is_none();
    let block_start_pb = l_border.block_start + l_padding.block_start;
    let block_end_pb = l_border.block_end + l_padding.block_end;

    let child_fragmentainer = input.fragmentainer.map(|frag| {
        let pb_consumed = match style.box_decoration_break {
            elidex_plugin::BoxDecorationBreak::Slice if is_first_fragment => block_start_pb,
            elidex_plugin::BoxDecorationBreak::Slice => 0.0,
            elidex_plugin::BoxDecorationBreak::Cloned => block_start_pb + block_end_pb,
        };
        crate::FragmentainerContext {
            available_block_size: (frag.available_block_size - pb_consumed).max(0.0),
            fragmentation_type: frag.fragmentation_type,
        }
    });

    let content_block = if let Some((iw, ih)) = intrinsic {
        // Replaced element: resolve block-size from physical dimensions.
        if is_horizontal {
            resolve_replaced_height(&style, content_inline, iw, ih, &padding, &border)
        } else {
            resolve_replaced_width(&style, containing_width, iw, ih, &padding, &border)
        }
    } else if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        0.0
    } else if children_are_block(dom, &children) {
        let child_containing_inline_size = crate::compute_inline_containing(
            style.writing_mode,
            child_phys_width,
            child_phys_height,
        );
        // Extract child break token for stack_block_children resumption.
        // layout_block_inner wraps stack's BreakToken as child_break_token in its own.
        let child_bt_for_stack = input
            .break_token
            .and_then(|bt| bt.child_break_token.as_deref());
        let child_input = LayoutInput {
            containing_width: child_phys_width,
            containing_height: child_phys_height,
            containing_inline_size: child_containing_inline_size,
            offset_x: content_x,
            offset_y: content_y,
            font_db,
            depth: depth + 1,
            float_ctx: input.float_ctx,
            viewport: input.viewport,
            fragmentainer: child_fragmentainer.as_ref(),
            break_token: child_bt_for_stack,
            subgrid: None,
        };
        let is_bfc = establishes_bfc(&style);
        let mut result =
            stack_block_children(dom, &children, &child_input, layout_child, is_bfc, entity);

        // Capture fragmentation results from stack_block_children.
        child_break_token = result.break_token.take();
        propagated_break_before = result.propagated_break_before;
        propagated_break_after = result.propagated_break_after;

        // Parent-child margin collapse (CSS 2.1 §8.3.1):
        // First child's block-start margin collapses with parent's block-start margin
        // when parent has no block-start border/padding and doesn't establish BFC.
        // CSS Fragmentation L3 §3.1: suppress in continuation fragments.
        let suppress_parent_child_collapse = input.break_token.is_some();
        let block_start_border = if is_horizontal {
            border.top
        } else {
            l_border.block_start
        };
        let block_start_padding = if is_horizontal {
            padding.top
        } else {
            l_padding.block_start
        };
        if block_start_border == 0.0
            && block_start_padding == 0.0
            && !is_bfc
            && !suppress_parent_child_collapse
        {
            if let Some(first_mbs) = result.first_child_margin_top {
                let new_mbs = collapse_margins(margin_block_start, first_mbs);
                let delta = (new_mbs - collapsed_margin_block_start) - first_mbs;
                shift_block_children(dom, &children, delta, wm);
                collapsed_margin_block_start = new_mbs;
            }
        }
        // Last child's block-end margin collapses with parent's block-end margin.
        let block_end_border = if is_horizontal {
            border.bottom
        } else {
            l_border.block_end
        };
        let block_end_padding = if is_horizontal {
            padding.bottom
        } else {
            l_padding.block_end
        };
        let block_size_dim = if is_horizontal {
            style.height
        } else {
            style.width
        };
        if block_end_border == 0.0
            && block_end_padding == 0.0
            && matches!(block_size_dim, Dimension::Auto)
            && !is_bfc
        {
            if let Some(last_mbe) = result.last_child_margin_bottom {
                collapsed_margin_block_end = collapse_margins(margin_block_end, last_mbe);
            }
        }

        static_positions_stash = result.static_positions;
        block_first_baseline = if block_start_border == 0.0 && block_start_padding == 0.0 && !is_bfc
        {
            if let Some(first_mbs) = result.first_child_margin_top {
                result.first_baseline.map(|bl| bl - first_mbs)
            } else {
                result.first_baseline
            }
        } else {
            result.first_baseline
        };

        result.height
    } else {
        // Inline context: the first argument is the available inline-axis space.
        let inline_size = if is_horizontal {
            content_inline
        } else {
            // CSS Writing Modes Level 3 §3.1: In vertical modes, the inline axis
            // is vertical (height). Use containing height when known.
            child_phys_height.unwrap_or(content_inline)
        };
        // Extract inline resume skip_lines from break token (H2: inline resume).
        let inline_skip_lines = input
            .break_token
            .and_then(|bt| bt.mode_data.as_ref())
            .and_then(|md| match md {
                crate::BreakTokenData::Block {
                    inline_break_line, ..
                } => *inline_break_line,
                _ => None,
            })
            .unwrap_or(0);
        // Build fragmentation constraint for inline layout if fragmentainer is active.
        let inline_frag_constraint =
            child_fragmentainer
                .as_ref()
                .map(|frag| crate::inline::InlineFragConstraint {
                    available_block: frag.available_block_size,
                    orphans: style.orphans,
                    widows: style.widows,
                    skip_lines: inline_skip_lines,
                });
        let inline_result = crate::inline::layout_inline_context_fragmented(
            dom,
            &children,
            inline_size,
            &style,
            font_db,
            entity,
            (content_x, content_y),
            layout_child,
            inline_frag_constraint.as_ref(),
        );
        static_positions_stash = inline_result.static_positions;
        block_first_baseline = inline_result.first_baseline;
        // Record inline break for later (handled after LayoutBox is built,
        // without going through the child_break_token wrapping path).
        inline_break_after_line = inline_result.break_after_line;
        inline_content_block = inline_result.height;
        // inline_result.height is always the block-axis extent:
        // horizontal-tb: physical height (line boxes stacked vertically)
        // vertical-rl/lr: physical width (column widths stacked horizontally)
        inline_result.height
    };

    // --- Resolve block-size ---
    let block_size = resolve_block_height(
        &style,
        content_block,
        containing_block,
        &padding,
        &border,
        intrinsic.is_some(),
        &wm,
    );

    // --- Build physical LayoutBox ---
    // Convert inline/block sizes and positions to physical (x, y, width, height).
    let (phys_width, phys_height) = if is_horizontal {
        (content_inline, block_size)
    } else {
        (block_size, content_inline)
    };

    // Recompute content position using collapsed block-start margin.
    let (final_x, final_y) = if is_horizontal {
        let fx = offset_x + margin_inline_start + border.left + padding.left;
        let fy = offset_y + collapsed_margin_block_start + border.top + padding.top;
        (fx, fy)
    } else {
        let fx =
            offset_x + collapsed_margin_block_start + l_border.block_start + l_padding.block_start;
        let fy = offset_y + margin_inline_start + l_border.inline_start + l_padding.inline_start;
        (fx, fy)
    };

    // Build physical margin EdgeSizes. Block-axis margins may have been collapsed.
    let margin = if is_horizontal {
        EdgeSizes::new(
            collapsed_margin_block_start,
            margin_inline_end,
            collapsed_margin_block_end,
            margin_inline_start,
        )
    } else {
        // Vertical: block-axis = left/right, inline-axis = top/bottom.
        let (phys_left, phys_right) = if wm.is_block_reversed() {
            // vertical-rl: block-start = right, block-end = left
            (collapsed_margin_block_end, collapsed_margin_block_start)
        } else {
            // vertical-lr: block-start = left, block-end = right
            (collapsed_margin_block_start, collapsed_margin_block_end)
        };
        EdgeSizes::new(
            margin_inline_start,
            phys_right,
            margin_inline_end,
            phys_left,
        )
    };

    let lb = LayoutBox {
        content: Rect::new(final_x, final_y, phys_width, phys_height),
        padding,
        border,
        margin,
        first_baseline: block_first_baseline,
    };

    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // Layout positioned descendants owned by this containing block.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != Position::Static || is_root || style.has_transform;
    if is_cb {
        let pb = lb.padding_box();
        crate::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            input.viewport,
            &static_positions_stash,
            font_db,
            layout_child,
            depth,
        );
    }

    // Inline-only fragmentation break: return directly without wrapping
    // (no child_break_token nesting — mode_data goes on the top-level token).
    if let Some(break_line) = inline_break_after_line {
        return crate::LayoutOutcome {
            layout_box: lb,
            break_token: Some(crate::BreakToken {
                entity,
                consumed_block_size: inline_content_block + block_start_pb,
                child_break_token: None,
                mode_data: Some(crate::BreakTokenData::Block {
                    child_index: 0,
                    inline_break_line: Some(break_line),
                }),
            }),
            propagated_break_before,
            propagated_break_after,
        };
    }

    // If stack_block_children produced a break token, wrap it in a parent break token.
    if let Some(child_bt) = child_break_token {
        return crate::LayoutOutcome {
            layout_box: lb,
            break_token: Some(crate::BreakToken {
                entity,
                consumed_block_size: child_bt.consumed_block_size + block_start_pb,
                child_break_token: Some(Box::new(child_bt)),
                mode_data: None,
            }),
            propagated_break_before,
            propagated_break_after,
        };
    }

    crate::LayoutOutcome {
        layout_box: lb,
        break_token: None,
        propagated_break_before,
        propagated_break_after,
    }
}

/// CSS 2.1 §9.4.1: Does this element establish a new block formatting context?
///
/// Elements that establish a BFC prevent margin collapse with children.
fn establishes_bfc(style: &elidex_plugin::ComputedStyle) -> bool {
    style.float != Float::None
        || matches!(style.position, Position::Absolute | Position::Fixed)
        || style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible
        || matches!(
            style.display,
            Display::InlineBlock
                | Display::Flex
                | Display::InlineFlex
                | Display::Grid
                | Display::InlineGrid
                | Display::Table
                | Display::InlineTable
                | Display::TableCell
        )
}
