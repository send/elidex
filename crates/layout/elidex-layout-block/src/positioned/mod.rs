//! CSS positioned layout (relative, absolute, fixed).
//!
//! Implements CSS 2.1 §9.3 (relative), §10.3.7/§10.6.4 (absolute constraint
//! equations), §9.9.1 (stacking context rules), and CSS Writing Modes L3
//! writing-mode-aware constraint equation axis mapping.

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    ComputedStyle, Dimension, Display, Point, Position, Rect, Size, Vector, WritingModeContext,
};
use elidex_text::FontDatabase;

use crate::{
    adjust_min_max_for_border_box, clamp_min_max, get_style, horizontal_pb, resolve_min_max,
    resolve_padding, sanitize, sanitize_border, try_get_style, LayoutInput, MAX_LAYOUT_DEPTH,
};

// ---------------------------------------------------------------------------
// Offset resolution
// ---------------------------------------------------------------------------

/// Resolve a CSS offset (top/right/bottom/left) against containing block dimension.
///
/// Returns `None` for `Dimension::Auto`.
/// CSS 2.1 §8.3: percentage against indefinite CB dimension → treat as auto.
/// A definite 0px CB dimension correctly resolves to 0.
#[must_use]
pub fn resolve_offset(dim: &Dimension, containing: f32) -> Option<f32> {
    match dim {
        Dimension::Length(px) => Some(sanitize(*px)),
        Dimension::Percentage(pct) => {
            if containing >= 0.0 && containing.is_finite() {
                Some(sanitize(containing * pct / 100.0))
            } else {
                None
            }
        }
        Dimension::Auto => None,
    }
}

/// Returns `true` if the element is absolutely positioned (absolute or fixed).
#[must_use]
pub fn is_absolutely_positioned(style: &ComputedStyle) -> bool {
    matches!(style.position, Position::Absolute | Position::Fixed)
}

/// Build a static-position map for absolutely positioned children.
///
/// Records `content_origin` — the container's content-area origin — as the
/// hypothetical static position for each abspos child.  Used by flex, grid,
/// and table layouts that skip abspos children during item collection.
#[must_use]
pub fn collect_abspos_static_positions(
    dom: &EcsDom,
    children: &[Entity],
    content_origin: Point,
) -> HashMap<Entity, Point> {
    let mut map = HashMap::new();
    for &child in children {
        if try_get_style(dom, child).is_some_and(|s| is_absolutely_positioned(&s)) {
            map.insert(child, content_origin);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Relative positioning
// ---------------------------------------------------------------------------

/// Apply relative positioning offsets to a `LayoutBox`.
///
/// CSS Writing Modes L3 §4.3 maps the constraint rules to logical axes:
/// - Inline axis: inline-start wins over inline-end (direction-dependent in physical)
/// - Block axis: block-start wins over block-end
///
/// In horizontal-tb LTR: left wins over right, top wins over bottom (CSS 2.1 behavior).
/// In horizontal-tb RTL: right wins over left, top wins over bottom.
/// In vertical-rl LTR: top wins over bottom (inline), right wins over left (block).
/// In vertical-lr LTR: top wins over bottom (inline), left wins over right (block).
pub fn apply_relative_offset(
    lb: &mut elidex_plugin::LayoutBox,
    style: &ComputedStyle,
    containing_width: f32,
    containing_height: Option<f32>,
) {
    let ch = containing_height.unwrap_or(0.0);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);

    // Resolve all four physical offsets.
    let left = resolve_offset(&style.left, containing_width);
    let right = resolve_offset(&style.right, containing_width);
    let top = resolve_offset(&style.top, ch);
    let bottom = resolve_offset(&style.bottom, ch);

    if wm.is_horizontal() {
        // Inline axis = horizontal: direction determines winner.
        let dx = match (left, right) {
            (Some(l), Some(r)) => {
                if wm.is_inline_reversed() {
                    -r // RTL: right (inline-start) wins
                } else {
                    l // LTR: left (inline-start) wins
                }
            }
            (Some(l), None) => l,
            (None, Some(r)) => -r,
            (None, None) => 0.0,
        };
        // Block axis = vertical: block-start (top) always wins.
        let dy = match (top, bottom) {
            (Some(t), _) => t,
            (None, Some(b)) => -b,
            (None, None) => 0.0,
        };
        lb.content.origin += Vector::new(dx, dy);
    } else {
        // Vertical writing mode: inline axis = vertical, block axis = horizontal.
        // Inline axis (top/bottom): direction determines winner.
        let dy = match (top, bottom) {
            (Some(t), Some(b)) => {
                if wm.is_inline_reversed() {
                    -b // RTL: bottom (inline-start) wins
                } else {
                    t // LTR: top (inline-start) wins
                }
            }
            (Some(t), None) => t,
            (None, Some(b)) => -b,
            (None, None) => 0.0,
        };
        // Block axis (left/right): block-start wins.
        // vertical-rl: block-start = right; vertical-lr: block-start = left.
        let dx = match (left, right) {
            (Some(l), Some(r)) => {
                if wm.is_block_reversed() {
                    -r // vertical-rl: right (block-start) wins
                } else {
                    l // vertical-lr: left (block-start) wins
                }
            }
            (Some(l), None) => l,
            (None, Some(r)) => -r,
            (None, None) => 0.0,
        };
        lb.content.origin += Vector::new(dx, dy);
    }
}

// ---------------------------------------------------------------------------
// Collecting positioned descendants
// ---------------------------------------------------------------------------

/// Walk descendants of `entity` to collect absolutely positioned elements
/// whose containing block is `entity` (no closer positioned ancestor between).
/// Fixed elements are collected separately (their CB is viewport).
/// Stops descending into positioned descendants (they own their own abs children).
pub fn collect_positioned_descendants(dom: &EcsDom, entity: Entity) -> (Vec<Entity>, Vec<Entity>) {
    let mut abs = Vec::new();
    let mut fixed = Vec::new();
    collect_inner(dom, entity, &mut abs, &mut fixed, 0);
    (abs, fixed)
}

fn collect_inner(
    dom: &EcsDom,
    parent: Entity,
    abs: &mut Vec<Entity>,
    fixed: &mut Vec<Entity>,
    depth: u32,
) {
    if depth >= MAX_LAYOUT_DEPTH {
        return;
    }
    for child in dom.children_iter(parent) {
        let Some(style) = try_get_style(dom, child) else {
            // Text node — continue to next sibling.
            continue;
        };
        if style.display == Display::None {
            continue;
        }
        match style.position {
            Position::Absolute => abs.push(child),
            Position::Fixed => fixed.push(child),
            Position::Relative | Position::Sticky => {
                if style.display == Display::Contents {
                    // display:contents doesn't generate a box → not a CB.
                    collect_inner(dom, child, abs, fixed, depth + 1);
                }
                // else: positioned + generates box → this child owns its own abs children.
                // If it also has transform it owns fixed children too, but that's
                // handled by its own layout_positioned_children call.
            }
            Position::Static => {
                if style.has_transform && style.display != Display::Contents {
                    // CSS Transforms L1 §2: a transform establishes a containing
                    // block for all descendants, including fixed-positioned ones.
                    // Stop fixed collection here — this element will handle its own
                    // fixed descendants via layout_positioned_children.
                    // Absolute children are also owned by this transform element,
                    // but we still collect them for the nearest positioned ancestor
                    // since absolute CB resolution is position-based in current arch.
                    collect_inner(dom, child, abs, &mut Vec::new(), depth + 1);
                } else {
                    // Static child → transparent, continue scan.
                    collect_inner(dom, child, abs, fixed, depth + 1);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Absolute positioning layout
// ---------------------------------------------------------------------------

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
    let (abs_children, fixed_children) = collect_positioned_descendants(dom, entity);

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

// ---------------------------------------------------------------------------
// Physical ↔ logical property mapping for positioned layout
// ---------------------------------------------------------------------------

/// Physical properties mapped to the inline axis.
struct InlineAxisProps {
    /// Resolved inline-start offset (physical left or top depending on WM).
    start: Option<f32>,
    /// Resolved inline-end offset (physical right or bottom depending on WM).
    end: Option<f32>,
    /// Specified inline-size (physical width or height depending on WM).
    size: Option<f32>,
    /// Raw margin at inline-start side.
    margin_start_raw: f32,
    /// Raw margin at inline-end side.
    margin_end_raw: f32,
    /// Whether inline-start margin is auto.
    margin_start_auto: bool,
    /// Whether inline-end margin is auto.
    margin_end_auto: bool,
    /// Inline-axis padding + border.
    pb: f32,
    /// Containing block inline size.
    containing: f32,
    /// Static position on inline axis (relative to CB origin).
    static_offset: f32,
}

/// Physical properties mapped to the block axis.
struct BlockAxisProps {
    /// Resolved block-start offset.
    start: Option<f32>,
    /// Resolved block-end offset.
    end: Option<f32>,
    /// Specified block-size (`None` if auto).
    size: Option<f32>,
    /// Content size from layout (used when size is auto).
    content_size: Option<f32>,
    /// Raw margin at block-start side.
    margin_start_raw: f32,
    /// Raw margin at block-end side.
    margin_end_raw: f32,
    /// Whether block-start margin is auto.
    margin_start_auto: bool,
    /// Whether block-end margin is auto.
    margin_end_auto: bool,
    /// Block-axis padding + border.
    pb: f32,
    /// Containing block block size.
    containing: f32,
    /// Static position on block axis (relative to CB origin).
    static_offset: f32,
}

/// Result of axis constraint resolution, in logical terms.
struct AxisResult {
    /// Resolved content size along this axis.
    size: f32,
    /// Margin at the start side.
    margin_start: f32,
    /// Margin at the end side.
    margin_end: f32,
    /// Offset from CB edge to margin edge (start side).
    offset: f32,
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
    };
    let child_lb = (env.layout_child)(dom, entity, &child_input).layout_box;

    // Overwrite the LayoutBox with correct position and margins.
    let lb = elidex_plugin::LayoutBox {
        content: Rect::from_origin_size(content_origin, Size::new(content_width, content_height)),
        padding,
        border,
        margin: elidex_plugin::EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
        first_baseline: child_lb.first_baseline,
    };
    let _ = dom.world_mut().insert_one(entity, lb);

    // Shift descendants to match final position.
    let delta = content_origin - child_lb.content.origin;
    if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(entity);
        crate::block::children::shift_descendants(dom, &grandchildren, delta);
    }
}

// ---------------------------------------------------------------------------
// Axis constraint solvers
// ---------------------------------------------------------------------------

/// Resolve a CSS size dimension (width or height) to a content-box pixel value.
///
/// Handles Length, Percentage (against `containing`), and Auto (falls back to
/// `intrinsic`). Applies border-box adjustment when `adjust_border_box` is true.
#[must_use]
fn resolve_size_value(
    dim: Dimension,
    containing: f32,
    pb: f32,
    adjust_border_box: bool,
    intrinsic: Option<f32>,
) -> Option<f32> {
    match dim {
        Dimension::Length(px) => {
            let v = sanitize(px);
            Some(if adjust_border_box {
                (v - pb).max(0.0)
            } else {
                v
            })
        }
        Dimension::Percentage(pct) => {
            if containing >= 0.0 && containing.is_finite() {
                let v = sanitize(containing * pct / 100.0);
                Some(if adjust_border_box {
                    (v - pb).max(0.0)
                } else {
                    v
                })
            } else {
                None
            }
        }
        Dimension::Auto => intrinsic,
    }
}

/// Resolve the inline-axis constraint equation.
///
/// This is the generalization of CSS 2.1 §10.3.7 for any writing mode.
/// The inline axis supports shrink-to-fit, direction-dependent over-constrained
/// handling, and direction-dependent static position.
///
/// Returns `(size, margin_start, margin_end, offset)` where offset is from
/// the inline-start edge of the containing block.
#[allow(clippy::similar_names)]
fn resolve_inline_axis(props: &InlineAxisProps, shrink_to_fit: impl FnOnce() -> f32) -> AxisResult {
    let mut ms = if props.margin_start_auto {
        0.0
    } else {
        props.margin_start_raw
    };
    let mut me = if props.margin_end_auto {
        0.0
    } else {
        props.margin_end_raw
    };

    match (props.start, props.size, props.end) {
        // All three auto: use static position for start, shrink-to-fit for size.
        (None, None, None) => {
            let w = shrink_to_fit();
            let offset = props.static_offset;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start+size auto, end specified: shrink-to-fit, solve start.
        (None, None, Some(e)) => {
            let w = shrink_to_fit();
            let offset = props.containing - e - w - props.pb - ms - me;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start auto only (size + end specified): solve start.
        (None, Some(w), Some(e)) => {
            let offset = props.containing - e - w - props.pb - ms - me;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start+end auto, size specified: start = static position.
        (None, Some(w), None) => {
            let offset = props.static_offset;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // size+end auto, start specified: shrink-to-fit.
        (Some(s), None, None) => {
            let w = shrink_to_fit();
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
        // size auto, start+end specified: stretch.
        (Some(s), None, Some(e)) => {
            let w = (props.containing - s - e - props.pb - ms - me).max(0.0);
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
        // end auto, start+size specified.
        (Some(s), Some(w), None) => AxisResult {
            size: w,
            margin_start: ms,
            margin_end: me,
            offset: s,
        },
        // Over-constrained: all three specified.
        (Some(s), Some(w), Some(e)) => {
            let available = props.containing - s - w - props.pb - e;
            if props.margin_start_auto && props.margin_end_auto {
                if available < 0.0 {
                    // Negative centering: absorb overflow into end-side margin.
                    ms = 0.0;
                    me = available;
                } else {
                    let half = available / 2.0;
                    ms = half;
                    me = available - half;
                }
            } else if props.margin_start_auto {
                ms = available - me;
            } else if props.margin_end_auto {
                me = available - ms;
            } else {
                // All non-auto, over-constrained: ignore inline-end.
                // (The start offset stands as given.)
            }
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
    }
}

/// Resolve the block-axis constraint equation.
///
/// This is the generalization of CSS 2.1 §10.6.4 for any writing mode.
/// The block axis supports stretch (auto size with both offsets specified),
/// always-equal auto margin splitting, and always ignores block-end when
/// over-constrained.
///
/// Returns `(size, margin_start, margin_end, offset)` where offset is from
/// the block-start edge of the containing block.
#[allow(clippy::similar_names)]
fn resolve_block_axis(props: &BlockAxisProps) -> AxisResult {
    let mut ms = if props.margin_start_auto {
        0.0
    } else {
        props.margin_start_raw
    };
    let mut me = if props.margin_end_auto {
        0.0
    } else {
        props.margin_end_raw
    };

    // Effective size: specified or content-based.
    let h = props.size.or(props.content_size).unwrap_or(0.0);

    match (props.start, props.end) {
        (None, None) => {
            // block-start = static position.
            AxisResult {
                size: h,
                margin_start: ms,
                margin_end: me,
                offset: props.static_offset,
            }
        }
        (Some(s), None) => AxisResult {
            size: h,
            margin_start: ms,
            margin_end: me,
            offset: s,
        },
        (None, Some(e)) => {
            let offset = props.containing - e - h - props.pb - ms - me;
            AxisResult {
                size: h,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        (Some(s), Some(e)) => {
            if props.size.is_none() {
                // Auto size with both offsets → stretch.
                let stretch = (props.containing - s - e - props.pb - ms - me).max(0.0);
                AxisResult {
                    size: stretch,
                    margin_start: ms,
                    margin_end: me,
                    offset: s,
                }
            } else {
                // Over-constrained.
                let available = props.containing - s - h - props.pb - e;
                if props.margin_start_auto && props.margin_end_auto {
                    // Block axis: always equal split (no directional asymmetry).
                    let half = available / 2.0;
                    ms = half;
                    me = available - half;
                } else if props.margin_start_auto {
                    ms = available - me;
                } else if props.margin_end_auto {
                    me = available - ms;
                }
                // Over-constrained with no auto margins: block-end ignored,
                // offset stays at `s`.
                AxisResult {
                    size: h,
                    margin_start: ms,
                    margin_end: me,
                    offset: s,
                }
            }
        }
    }
}

/// Compute shrink-to-fit width for an absolutely positioned element.
fn shrink_to_fit_width(
    dom: &EcsDom,
    entity: Entity,
    font_db: &FontDatabase,
    depth: u32,
    cb_width: f32,
    h_pb: f32,
) -> f32 {
    let preferred = crate::block::children::max_content_width(dom, entity, font_db, depth);
    let available = (cb_width - h_pb).max(0.0);
    preferred.min(available).max(0.0)
}
