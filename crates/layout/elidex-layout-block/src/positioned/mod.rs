//! CSS positioned layout (relative, absolute, fixed).
//!
//! Implements CSS 2.1 §9.3 (relative), §10.3.7/§10.6.4 (absolute constraint
//! equations), and §9.9.1 (stacking context rules).

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, Dimension, Direction, Display, Position, Rect};
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
/// Records `(content_x, content_y)` — the container's content-area origin — as
/// the hypothetical static position for each abspos child.  Used by flex, grid,
/// and table layouts that skip abspos children during item collection.
#[must_use]
pub fn collect_abspos_static_positions(
    dom: &EcsDom,
    children: &[Entity],
    content_x: f32,
    content_y: f32,
) -> HashMap<Entity, (f32, f32)> {
    let mut map = HashMap::new();
    for &child in children {
        if try_get_style(dom, child).is_some_and(|s| is_absolutely_positioned(&s)) {
            map.insert(child, (content_x, content_y));
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Relative positioning
// ---------------------------------------------------------------------------

/// Apply relative positioning offsets to a `LayoutBox`.
///
/// CSS 2.1 §9.4.3:
///   Vertical: top wins over bottom (always)
///   Horizontal: direction-dependent — LTR: left wins; RTL: right wins
pub fn apply_relative_offset(
    lb: &mut elidex_plugin::LayoutBox,
    style: &ComputedStyle,
    containing_width: f32,
    containing_height: Option<f32>,
) {
    let ch = containing_height.unwrap_or(0.0);

    // Horizontal offset
    let left = resolve_offset(&style.left, containing_width);
    let right = resolve_offset(&style.right, containing_width);
    let dx = match (left, right) {
        (Some(l), Some(r)) => {
            // Both specified: direction determines winner.
            if style.direction == Direction::Rtl {
                -r // RTL: right wins
            } else {
                l // LTR: left wins
            }
        }
        (Some(l), None) => l,
        (None, Some(r)) => -r,
        (None, None) => 0.0,
    };

    // Vertical offset: top always wins over bottom.
    let top = resolve_offset(&style.top, ch);
    let bottom = resolve_offset(&style.bottom, ch);
    let dy = match (top, bottom) {
        (Some(t), _) => t, // top always wins
        (None, Some(b)) => -b,
        (None, None) => 0.0,
    };

    lb.content.x += dx;
    lb.content.y += dy;
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
#[allow(clippy::too_many_arguments, clippy::implicit_hasher)]
pub fn layout_positioned_children(
    dom: &mut EcsDom,
    entity: Entity,
    cb_padding_box: &Rect,
    viewport: Option<(f32, f32)>,
    static_positions: &HashMap<Entity, (f32, f32)>,
    font_db: &FontDatabase,
    layout_child: crate::ChildLayoutFn,
    depth: u32,
) {
    if depth >= MAX_LAYOUT_DEPTH {
        return;
    }
    let (abs_children, fixed_children) = collect_positioned_descendants(dom, entity);

    // Layout absolute children against this element's padding box.
    for child in abs_children {
        let sp = static_positions
            .get(&child)
            .copied()
            .unwrap_or((cb_padding_box.x, cb_padding_box.y));
        layout_absolutely_positioned(
            dom,
            child,
            cb_padding_box,
            sp,
            font_db,
            layout_child,
            depth,
            viewport,
        );
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
            (*cb_padding_box, (cb_padding_box.x, cb_padding_box.y))
        } else if let Some((vw, vh)) = viewport {
            (Rect::new(0.0, 0.0, vw, vh), (0.0, 0.0))
        } else {
            continue;
        };
        let sp = static_positions.get(&child).copied().unwrap_or(sp_default);
        layout_absolutely_positioned(dom, child, &cb, sp, font_db, layout_child, depth, viewport);
    }
}

/// Layout a single absolutely positioned element against its containing block.
///
/// CSS 2.1 §10.3.7 / §10.6.4 constraint equations.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn layout_absolutely_positioned(
    dom: &mut EcsDom,
    entity: Entity,
    cb: &Rect,
    static_position: (f32, f32),
    font_db: &FontDatabase,
    layout_child: crate::ChildLayoutFn,
    depth: u32,
    viewport: Option<(f32, f32)>,
) {
    let style = get_style(dom, entity);
    let padding = resolve_padding(&style, cb.width);
    let border = sanitize_border(&style);
    let h_pb = horizontal_pb(&padding, &border);
    let v_pb = crate::vertical_pb(&padding, &border);
    let is_border_box = style.box_sizing == elidex_plugin::BoxSizing::BorderBox;

    let margin_top_raw = crate::block::resolve_margin(style.margin_top, cb.width);
    let margin_bottom_raw = crate::block::resolve_margin(style.margin_bottom, cb.width);
    let margin_left_raw = crate::block::resolve_margin(style.margin_left, cb.width);
    let margin_right_raw = crate::block::resolve_margin(style.margin_right, cb.width);

    let left = resolve_offset(&style.left, cb.width);
    let right = resolve_offset(&style.right, cb.width);
    let top = resolve_offset(&style.top, cb.height);
    let bottom = resolve_offset(&style.bottom, cb.height);

    let intrinsic = crate::get_intrinsic_size(dom, entity);

    // -----------------------------------------------------------------------
    // Horizontal axis: CSS 2.1 §10.3.7
    // left + margin-left + border-left + padding-left + width + padding-right
    // + border-right + margin-right + right = cb.width
    // -----------------------------------------------------------------------

    let width_specified = match style.width {
        Dimension::Length(px) => {
            let w = sanitize(px);
            // CSS 2.1 §10.3.7: box-sizing: border-box → subtract padding+border.
            Some(if is_border_box && intrinsic.is_none() {
                (w - h_pb).max(0.0)
            } else {
                w
            })
        }
        Dimension::Percentage(pct) => {
            let w = sanitize(cb.width * pct / 100.0);
            Some(if is_border_box && intrinsic.is_none() {
                (w - h_pb).max(0.0)
            } else {
                w
            })
        }
        Dimension::Auto => intrinsic.map(|(iw, _)| iw),
    };

    let (mut content_width, margin_left, margin_right, used_left) = resolve_horizontal(
        left,
        width_specified,
        right,
        margin_left_raw,
        margin_right_raw,
        h_pb,
        cb.width,
        static_position.0 - cb.x,
        &style,
        || shrink_to_fit_width(dom, entity, font_db, depth, cb.width, h_pb),
    );

    // CSS 2.1 §10.4: apply min-width / max-width after constraint resolution.
    {
        let mut min_w = resolve_min_max(style.min_width, cb.width, 0.0);
        let mut max_w = resolve_min_max(style.max_width, cb.width, f32::INFINITY);
        if is_border_box && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_w, &mut max_w, h_pb);
        }
        content_width = clamp_min_max(content_width, min_w, max_w);
    }

    // -----------------------------------------------------------------------
    // Vertical axis: CSS 2.1 §10.6.4
    // -----------------------------------------------------------------------

    let height_specified = match style.height {
        Dimension::Length(px) => {
            let h = sanitize(px);
            Some(if is_border_box && intrinsic.is_none() {
                (h - v_pb).max(0.0)
            } else {
                h
            })
        }
        Dimension::Percentage(pct) => {
            if cb.height >= 0.0 && cb.height.is_finite() {
                let h = sanitize(cb.height * pct / 100.0);
                Some(if is_border_box && intrinsic.is_none() {
                    (h - v_pb).max(0.0)
                } else {
                    h
                })
            } else {
                None
            }
        }
        Dimension::Auto => intrinsic.map(|(_, ih)| ih),
    };

    // If height is auto (no intrinsic, no specified), we need content height.
    // Do a preliminary layout to determine it.
    //
    // CSS Sizing L3 "definite size": auto height depends on content layout,
    // so it is indefinite. Children with percentage heights resolve to auto
    // (containing_height: None). This is spec-correct — no re-layout needed.
    // Servo uses the same single-pass approach (SizeConstraint::MinMax).
    let content_height_from_layout = if height_specified.is_none() {
        let child_input = LayoutInput {
            containing_width: content_width,
            containing_height: None,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport,
        };
        let lb = layout_child(dom, entity, &child_input);
        Some(lb.content.height)
    } else {
        None
    };

    let (mut content_height, margin_top, margin_bottom, used_top) = resolve_vertical(
        top,
        bottom,
        height_specified,
        content_height_from_layout,
        margin_top_raw,
        margin_bottom_raw,
        v_pb,
        cb.height,
        static_position.1 - cb.y,
        &style,
    );

    // CSS 2.1 §10.7: apply min-height / max-height after constraint resolution.
    {
        let ch = cb.height;
        let mut min_h = resolve_min_max(style.min_height, ch, 0.0);
        let mut max_h = resolve_min_max(style.max_height, ch, f32::INFINITY);
        if is_border_box && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_h, &mut max_h, v_pb);
        }
        content_height = clamp_min_max(content_height, min_h, max_h);
    }

    // Compute final position relative to viewport (cb origin + offset).
    let content_x = cb.x + used_left + margin_left + border.left + padding.left;
    let content_y = cb.y + used_top + margin_top + border.top + padding.top;

    // Final layout with resolved dimensions.
    let child_input = LayoutInput {
        containing_width: content_width,
        containing_height: Some(content_height),
        offset_x: content_x,
        offset_y: content_y,
        font_db,
        depth: depth + 1,
        float_ctx: None,
        viewport,
    };
    let child_lb = layout_child(dom, entity, &child_input);

    // Overwrite the LayoutBox with correct position and margins.
    let lb = elidex_plugin::LayoutBox {
        content: Rect::new(content_x, content_y, content_width, content_height),
        padding,
        border,
        margin: elidex_plugin::EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
    };
    let _ = dom.world_mut().insert_one(entity, lb);

    // Shift descendants to match final position.
    let dx = content_x - child_lb.content.x;
    let dy = content_y - child_lb.content.y;
    if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
        let grandchildren = dom.composed_children(entity);
        crate::block::children::shift_descendants(dom, &grandchildren, (dx, dy));
    }
}

/// Resolve horizontal constraint equation (CSS 2.1 §10.3.7).
#[allow(clippy::too_many_arguments, clippy::similar_names)]
fn resolve_horizontal(
    left: Option<f32>,
    width: Option<f32>,
    right: Option<f32>,
    margin_left_raw: f32,
    margin_right_raw: f32,
    h_pb: f32,
    cb_width: f32,
    static_x: f32,
    style: &ComputedStyle,
    shrink_to_fit: impl FnOnce() -> f32,
) -> (f32, f32, f32, f32) {
    let ml_auto = matches!(style.margin_left, Dimension::Auto);
    let mr_auto = matches!(style.margin_right, Dimension::Auto);

    // Set auto margins to 0 initially.
    let mut ml = if ml_auto { 0.0 } else { margin_left_raw };
    let mut mr = if mr_auto { 0.0 } else { margin_right_raw };

    match (left, width, right) {
        // Case 1: all three auto — CSS 2.1 §10.3.7
        // LTR: left = static position; RTL: right = static position.
        (None, None, None) => {
            let w = shrink_to_fit();
            let l = if style.direction == Direction::Rtl {
                // RTL: right = static_right, solve left.
                // static_right = cb_width - static_x (measured from right edge).
                let static_right = cb_width - static_x;
                cb_width - static_right - w - h_pb - ml - mr
            } else {
                static_x
            };
            (w, ml, mr, l)
        }
        // Case: left+width auto, right specified
        (None, None, Some(r)) => {
            let w = shrink_to_fit();
            let l = cb_width - r - w - h_pb - ml - mr;
            (w, ml, mr, l)
        }
        // Sub 4: left auto only (width + right specified) → solve left
        (None, Some(w), Some(r)) => {
            let l = cb_width - r - w - h_pb - ml - mr;
            (w, ml, mr, l)
        }
        // Sub 2: left+right auto, width specified — CSS 2.1 §10.3.7
        // LTR: left = static position; RTL: right = static position.
        (None, Some(w), None) => {
            let l = if style.direction == Direction::Rtl {
                let static_right = cb_width - static_x;
                cb_width - static_right - w - h_pb - ml - mr
            } else {
                static_x
            };
            (w, ml, mr, l)
        }
        // Case: width+right auto, left specified
        (Some(l), None, None) => {
            let w = shrink_to_fit();
            (w, ml, mr, l)
        }
        // Case: width auto, left+right specified → stretch
        (Some(l), None, Some(r)) => {
            let w = (cb_width - l - r - h_pb - ml - mr).max(0.0);
            (w, ml, mr, l)
        }
        // Case: right auto, left+width specified
        (Some(l), Some(w), None) => (w, ml, mr, l),
        // Over-constrained: all three specified
        (Some(l), Some(w), Some(r)) => {
            let available = cb_width - l - w - h_pb - r;
            if ml_auto && mr_auto {
                if available < 0.0 {
                    // CSS 2.1 §10.3.7: negative centering — absorb overflow
                    // into the end-side margin (LTR → right, RTL → left).
                    if style.direction == Direction::Rtl {
                        mr = 0.0;
                        ml = available;
                    } else {
                        ml = 0.0;
                        mr = available;
                    }
                } else {
                    let half = available / 2.0;
                    ml = half;
                    mr = available - half;
                }
            } else if ml_auto {
                ml = available - mr;
            } else if mr_auto {
                mr = available - ml;
            } else {
                // All non-auto, over-constrained.
                // CSS 2.1 §10.3.7: LTR → ignore right; RTL → ignore left.
                if style.direction == Direction::Rtl {
                    let l_adj = cb_width - r - w - h_pb - ml - mr;
                    return (w, ml, mr, l_adj);
                }
            }
            (w, ml, mr, l)
        }
    }
}

/// Resolve vertical constraint equation (CSS 2.1 §10.6.4).
#[allow(clippy::too_many_arguments, clippy::similar_names)]
fn resolve_vertical(
    top: Option<f32>,
    bottom: Option<f32>,
    height: Option<f32>,
    content_height: Option<f32>,
    margin_top_raw: f32,
    margin_bottom_raw: f32,
    v_pb: f32,
    cb_height: f32,
    static_y: f32,
    style: &ComputedStyle,
) -> (f32, f32, f32, f32) {
    let mt_auto = matches!(style.margin_top, Dimension::Auto);
    let mb_auto = matches!(style.margin_bottom, Dimension::Auto);

    let mut mt = if mt_auto { 0.0 } else { margin_top_raw };
    let mut mb = if mb_auto { 0.0 } else { margin_bottom_raw };

    // Effective height: specified or content-based.
    let h = height.or(content_height).unwrap_or(0.0);

    // CSS 2.1 §10.6.4: the constraint has 3 unknowns (top, height, bottom).
    // Height is pre-resolved above (specified or content-based), so we match
    // only on top/bottom. When height was auto and no content is available,
    // `h` is 0 (the stretch case is handled in (Some, Some) when both offsets
    // are specified and height is truly unknown).
    match (top, bottom) {
        (None, None) => {
            // top = static position
            (h, mt, mb, static_y)
        }
        (Some(t), None) => (h, mt, mb, t),
        (None, Some(b)) => {
            let t = cb_height - b - h - v_pb - mt - mb;
            (h, mt, mb, t)
        }
        (Some(t), Some(b)) => {
            if height.is_none() {
                // Height auto with top+bottom → stretch (CSS 2.1 §10.6.4 rule 5)
                let stretch_h = (cb_height - t - b - v_pb - mt - mb).max(0.0);
                (stretch_h, mt, mb, t)
            } else {
                // Over-constrained
                let available = cb_height - t - h - v_pb - b;
                if mt_auto && mb_auto {
                    // CSS 2.1 §10.6.4: margin-top = margin-bottom (always equal,
                    // no directional asymmetry unlike horizontal §10.3.7).
                    let half = available / 2.0;
                    mt = half;
                    mb = available - half;
                } else if mt_auto {
                    mt = available - mb;
                } else if mb_auto {
                    mb = available - mt;
                }
                (h, mt, mb, t)
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
