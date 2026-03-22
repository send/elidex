//! Layout for absolutely positioned elements.
//!
//! CSS 2.1 §10.3.7 / §10.6.4 constraint equations, extended for writing modes
//! per CSS Writing Modes L3 §4.3 and CSS Positioned Layout L3.

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{Dimension, Point, Rect, Size, Vector, WritingModeContext};

use crate::{
    adjust_min_max_for_border_box, clamp_min_max, get_style, horizontal_pb, resolve_min_max,
    resolve_padding, sanitize_border, try_get_style, LayoutInput, MAX_LAYOUT_DEPTH,
};

use super::constraints::{
    resolve_block_axis, resolve_inline_axis, resolve_size_value, shrink_to_fit_width,
    BlockAxisProps, InlineAxisProps,
};
use super::resolve_offset;

/// Layout all absolutely positioned descendants owned by this containing block.
///
/// Called after the containing block's normal-flow layout is complete.
/// `static_positions` maps entities to their hypothetical static position.
#[allow(clippy::implicit_hasher)]
pub fn layout_positioned_children(
    dom: &mut EcsDom,
    entity: Entity,
    cb_padding_box: &Rect,
    static_positions: &HashMap<Entity, Point>,
    env: &crate::LayoutEnv<'_>,
) {
    if env.depth >= MAX_LAYOUT_DEPTH {
        return;
    }
    let (abs_children, fixed_children) = super::collect_positioned_descendants(dom, entity);

    // Layout absolute children against this element's padding box.
    for child in abs_children {
        let sp = static_positions
            .get(&child)
            .copied()
            .unwrap_or(cb_padding_box.origin);
        layout_absolutely_positioned(dom, child, cb_padding_box, sp, env);
    }

    // Layout fixed children.
    //
    // `collect_positioned_descendants` stops fixed collection at transform
    // boundaries (CSS Transforms L1 §2), so every fixed child collected here
    // belongs to THIS element as its containing block.
    //
    // - If this element has `has_transform`, it is the CB per CSS Transforms L1 §2.
    // - Otherwise, this is a positioned/root element with no intervening transform,
    //   so the viewport is the CB per CSS Positioned Layout L3 §3.
    let has_transform = try_get_style(dom, entity).is_some_and(|s| s.has_transform);
    for child in fixed_children {
        let (cb, sp_default) = if has_transform {
            (*cb_padding_box, cb_padding_box.origin)
        } else if let Some(vp) = env.viewport {
            (Rect::new(0.0, 0.0, vp.width, vp.height), Point::ZERO)
        } else {
            continue;
        };
        let sp = static_positions.get(&child).copied().unwrap_or(sp_default);
        layout_absolutely_positioned(dom, child, &cb, sp, env);
    }
}

/// Layout a single absolutely positioned element against its containing block.
///
/// CSS 2.1 §10.3.7 / §10.6.4 constraint equations, extended for writing modes
/// per CSS Writing Modes L3 §4.3 and CSS Positioned Layout L3.
///
/// The inline axis gets shrink-to-fit behavior, direction-dependent over-constrained
/// handling, and direction-dependent static position. The block axis gets stretch
/// behavior and always-equal-split auto margins.
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn layout_absolutely_positioned(
    dom: &mut EcsDom,
    entity: Entity,
    cb: &Rect,
    static_position: Point,
    env: &crate::LayoutEnv<'_>,
) {
    let style = get_style(dom, entity);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);

    // CSS Box Model L3 §5.3: percentage padding/margin resolve against the
    // containing block's *inline size*.
    let inline_containing = if wm.is_horizontal() {
        cb.size.width
    } else {
        cb.size.height
    };

    let padding = resolve_padding(&style, inline_containing);
    let border = sanitize_border(&style);
    let h_pb = horizontal_pb(&padding, &border);
    let v_pb = crate::vertical_pb(&padding, &border);
    let is_border_box = style.box_sizing == elidex_plugin::BoxSizing::BorderBox;

    // Resolve all four physical margins against inline containing size.
    let margin_top_raw = crate::block::resolve_margin(style.margin_top, inline_containing);
    let margin_bottom_raw = crate::block::resolve_margin(style.margin_bottom, inline_containing);
    let margin_left_raw = crate::block::resolve_margin(style.margin_left, inline_containing);
    let margin_right_raw = crate::block::resolve_margin(style.margin_right, inline_containing);

    // Resolve all four physical offsets.
    let left = resolve_offset(&style.left, cb.size.width);
    let right = resolve_offset(&style.right, cb.size.width);
    let top = resolve_offset(&style.top, cb.size.height);
    let bottom = resolve_offset(&style.bottom, cb.size.height);

    let intrinsic = crate::get_intrinsic_size(dom, entity);

    // Map physical properties to logical axes based on writing mode.
    // In horizontal-tb: inline axis = horizontal, block axis = vertical.
    // In vertical-rl/lr: inline axis = vertical, block axis = horizontal.
    let (
        inline_start,
        inline_end,
        block_start,
        block_end,
        inline_size_dim,
        block_size_dim,
        inline_min,
        inline_max,
        block_min,
        block_max,
        inline_pb,
        block_pb,
        inline_margin_start_raw,
        inline_margin_end_raw,
        block_margin_start_raw,
        block_margin_end_raw,
        inline_margin_start_auto,
        inline_margin_end_auto,
        block_margin_start_auto,
        block_margin_end_auto,
        cb_inline_size,
        cb_block_size,
        static_inline,
        static_block,
        intrinsic_inline,
        intrinsic_block,
    ) = if wm.is_horizontal() {
        // horizontal-tb
        let (is, ie) = if wm.is_inline_reversed() {
            (right, left) // RTL: inline-start = right
        } else {
            (left, right)
        };
        let (ms_raw, me_raw) = if wm.is_inline_reversed() {
            (margin_right_raw, margin_left_raw)
        } else {
            (margin_left_raw, margin_right_raw)
        };
        let (ms_auto, me_auto) = if wm.is_inline_reversed() {
            (
                matches!(style.margin_right, Dimension::Auto),
                matches!(style.margin_left, Dimension::Auto),
            )
        } else {
            (
                matches!(style.margin_left, Dimension::Auto),
                matches!(style.margin_right, Dimension::Auto),
            )
        };
        let static_i = if wm.is_inline_reversed() {
            cb.size.width - (static_position.x - cb.origin.x)
        } else {
            static_position.x - cb.origin.x
        };
        (
            is,
            ie,
            top,
            bottom,
            style.width,
            style.height,
            style.min_width,
            style.max_width,
            style.min_height,
            style.max_height,
            h_pb,
            v_pb,
            ms_raw,
            me_raw,
            margin_top_raw,
            margin_bottom_raw,
            ms_auto,
            me_auto,
            matches!(style.margin_top, Dimension::Auto),
            matches!(style.margin_bottom, Dimension::Auto),
            cb.size.width,
            cb.size.height,
            static_i,
            static_position.y - cb.origin.y,
            intrinsic.map(|s| s.width),
            intrinsic.map(|s| s.height),
        )
    } else {
        // vertical-rl / vertical-lr
        // Inline axis = vertical (top/bottom/height)
        // Block axis = horizontal (left/right/width)
        let (is, ie) = if wm.is_inline_reversed() {
            (bottom, top) // RTL: inline-start = bottom
        } else {
            (top, bottom)
        };
        let (bs, be) = if wm.is_block_reversed() {
            (right, left) // vertical-rl: block-start = right
        } else {
            (left, right)
        };
        let (ims_raw, ime_raw) = if wm.is_inline_reversed() {
            (margin_bottom_raw, margin_top_raw)
        } else {
            (margin_top_raw, margin_bottom_raw)
        };
        let (bms_raw, bme_raw) = if wm.is_block_reversed() {
            (margin_right_raw, margin_left_raw)
        } else {
            (margin_left_raw, margin_right_raw)
        };
        let (ims_auto, ime_auto) = if wm.is_inline_reversed() {
            (
                matches!(style.margin_bottom, Dimension::Auto),
                matches!(style.margin_top, Dimension::Auto),
            )
        } else {
            (
                matches!(style.margin_top, Dimension::Auto),
                matches!(style.margin_bottom, Dimension::Auto),
            )
        };
        let (bms_auto, bme_auto) = if wm.is_block_reversed() {
            (
                matches!(style.margin_right, Dimension::Auto),
                matches!(style.margin_left, Dimension::Auto),
            )
        } else {
            (
                matches!(style.margin_left, Dimension::Auto),
                matches!(style.margin_right, Dimension::Auto),
            )
        };
        let static_i = if wm.is_inline_reversed() {
            cb.size.height - (static_position.y - cb.origin.y)
        } else {
            static_position.y - cb.origin.y
        };
        let static_b = if wm.is_block_reversed() {
            cb.size.width - (static_position.x - cb.origin.x)
        } else {
            static_position.x - cb.origin.x
        };
        (
            is,
            ie,
            bs,
            be,
            style.height, // inline-size = height
            style.width,  // block-size = width
            style.min_height,
            style.max_height,
            style.min_width,
            style.max_width,
            v_pb, // inline pb = vertical pb
            h_pb, // block pb = horizontal pb
            ims_raw,
            ime_raw,
            bms_raw,
            bme_raw,
            ims_auto,
            ime_auto,
            bms_auto,
            bme_auto,
            cb.size.height, // CB inline size
            cb.size.width,  // CB block size
            static_i,
            static_b,
            intrinsic.map(|s| s.height), // intrinsic inline = height
            intrinsic.map(|s| s.width),  // intrinsic block = width
        )
    };

    // -----------------------------------------------------------------------
    // Resolve inline-size (specified or intrinsic)
    // -----------------------------------------------------------------------
    let inline_size_specified = resolve_size_value(
        inline_size_dim,
        cb_inline_size,
        inline_pb,
        is_border_box && intrinsic.is_none(),
        intrinsic_inline,
    );

    // -----------------------------------------------------------------------
    // Inline axis constraint equation
    // -----------------------------------------------------------------------
    let inline_result = resolve_inline_axis(
        &InlineAxisProps {
            start: inline_start,
            end: inline_end,
            size: inline_size_specified,
            margin_start_raw: inline_margin_start_raw,
            margin_end_raw: inline_margin_end_raw,
            margin_start_auto: inline_margin_start_auto,
            margin_end_auto: inline_margin_end_auto,
            pb: inline_pb,
            containing: cb_inline_size,
            static_offset: static_inline,
        },
        || {
            // Shrink-to-fit: always computes in physical width for now.
            shrink_to_fit_width(dom, entity, env.font_db, env.depth, cb.size.width, h_pb)
        },
    );

    // Map inline result back to physical width/height.
    let mut content_inline = inline_result.size;

    // Apply min/max inline-size.
    {
        let mut min_i = resolve_min_max(inline_min, cb_inline_size, 0.0);
        let mut max_i = resolve_min_max(inline_max, cb_inline_size, f32::INFINITY);
        if is_border_box && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_i, &mut max_i, inline_pb);
        }
        content_inline = clamp_min_max(content_inline, min_i, max_i);
    }

    // -----------------------------------------------------------------------
    // Resolve block-size
    // -----------------------------------------------------------------------
    let block_size_specified = resolve_size_value(
        block_size_dim,
        cb_block_size,
        block_pb,
        is_border_box && intrinsic.is_none(),
        intrinsic_block,
    );

    // If block-size is auto, do preliminary layout to get content size.
    let content_block_from_layout = if block_size_specified.is_none() {
        let child_input = LayoutInput {
            viewport: env.viewport,
            ..LayoutInput::probe(env, content_inline)
        };
        let lb = (env.layout_child)(dom, entity, &child_input).layout_box;
        // Content block from layout: in horizontal = height, in vertical = width.
        Some(if wm.is_horizontal() {
            lb.content.size.height
        } else {
            lb.content.size.width
        })
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // Block axis constraint equation
    // -----------------------------------------------------------------------
    let block_result = resolve_block_axis(&BlockAxisProps {
        start: block_start,
        end: block_end,
        size: block_size_specified,
        content_size: content_block_from_layout,
        margin_start_raw: block_margin_start_raw,
        margin_end_raw: block_margin_end_raw,
        margin_start_auto: block_margin_start_auto,
        margin_end_auto: block_margin_end_auto,
        pb: block_pb,
        containing: cb_block_size,
        static_offset: static_block,
    });

    let mut content_block = block_result.size;

    // Apply min/max block-size.
    {
        let mut min_b = resolve_min_max(block_min, cb_block_size, 0.0);
        let mut max_b = resolve_min_max(block_max, cb_block_size, f32::INFINITY);
        if is_border_box && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_b, &mut max_b, block_pb);
        }
        content_block = clamp_min_max(content_block, min_b, max_b);
    }

    // -----------------------------------------------------------------------
    // Convert logical results back to physical coordinates
    // -----------------------------------------------------------------------
    let (
        content_width,
        content_height,
        used_left,
        used_top,
        margin_left,
        margin_right,
        margin_top,
        margin_bottom,
    ) = if wm.is_horizontal() {
        let (ml, mr) = if wm.is_inline_reversed() {
            (inline_result.margin_end, inline_result.margin_start)
        } else {
            (inline_result.margin_start, inline_result.margin_end)
        };
        // Inline offset → left offset. For RTL, inline-start offset is from
        // the right edge, so convert: left = cb_width - offset - size - pb - margins.
        let used_left = if wm.is_inline_reversed() {
            cb_inline_size
                - inline_result.offset
                - content_inline
                - inline_pb
                - inline_result.margin_start
                - inline_result.margin_end
        } else {
            inline_result.offset
        };
        (
            content_inline,
            content_block,
            used_left,
            block_result.offset,
            ml,
            mr,
            block_result.margin_start,
            block_result.margin_end,
        )
    } else {
        // Vertical modes: inline axis = Y, block axis = X.
        // Inline result → used_top, block result → used_left.
        let (mt, mb) = if wm.is_inline_reversed() {
            (inline_result.margin_end, inline_result.margin_start)
        } else {
            (inline_result.margin_start, inline_result.margin_end)
        };
        let (ml, mr) = if wm.is_block_reversed() {
            (block_result.margin_end, block_result.margin_start)
        } else {
            (block_result.margin_start, block_result.margin_end)
        };
        // Convert inline-start offset to physical top.
        let used_top = if wm.is_inline_reversed() {
            cb_inline_size
                - inline_result.offset
                - content_inline
                - inline_pb
                - inline_result.margin_start
                - inline_result.margin_end
        } else {
            inline_result.offset
        };
        // Convert block-start offset to physical left.
        let used_left = if wm.is_block_reversed() {
            cb_block_size
                - block_result.offset
                - content_block
                - block_pb
                - block_result.margin_start
                - block_result.margin_end
        } else {
            block_result.offset
        };
        (
            content_block,  // physical width = block size
            content_inline, // physical height = inline size
            used_left,
            used_top,
            ml,
            mr,
            mt,
            mb,
        )
    };

    // Compute final position relative to viewport (cb origin + offset).
    let content_origin = cb.origin
        + Vector::new(
            used_left + margin_left + border.left + padding.left,
            used_top + margin_top + border.top + padding.top,
        );

    // Final layout with resolved dimensions.
    let child_inline_size = if wm.is_horizontal() {
        content_width
    } else {
        content_height
    };
    let child_input = LayoutInput {
        containing: elidex_plugin::CssSize::definite(content_width, content_height),
        containing_inline_size: child_inline_size,
        offset: content_origin,
        font_db: env.font_db,
        depth: env.depth + 1,
        float_ctx: None,
        viewport: env.viewport,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: env.layout_generation,
    };
    let child_lb = (env.layout_child)(dom, entity, &child_input).layout_box;

    // Overwrite the LayoutBox with correct position and margins.
    let lb = elidex_plugin::LayoutBox {
        content: Rect::from_origin_size(content_origin, Size::new(content_width, content_height)),
        padding,
        border,
        margin: elidex_plugin::EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
        first_baseline: child_lb.first_baseline,
        layout_generation: env.layout_generation,
    };
    let _ = dom.world_mut().insert_one(entity, lb);

    // Shift descendants to match final position.
    let delta = content_origin - child_lb.content.origin;
    if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(entity);
        crate::block::children::shift_descendants(dom, &grandchildren, delta);
    }
}
