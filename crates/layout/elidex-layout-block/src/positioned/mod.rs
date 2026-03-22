//! CSS positioned layout (relative, absolute, fixed).
//!
//! Implements CSS 2.1 §9.3 (relative), §10.3.7/§10.6.4 (absolute constraint
//! equations), §9.9.1 (stacking context rules), and CSS Writing Modes L3
//! writing-mode-aware constraint equation axis mapping.

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

mod constraints;
mod layout;

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    ComputedStyle, Dimension, Display, Point, Position, Vector, WritingModeContext,
};

#[cfg(test)]
pub(crate) use crate::LayoutInput;
#[cfg(test)]
pub(crate) use elidex_plugin::Rect;

use crate::{sanitize, try_get_style, MAX_LAYOUT_DEPTH};

pub use layout::{layout_absolutely_positioned, layout_positioned_children};

// Re-export constraint types for tests.
#[cfg(test)]
pub(crate) use constraints::{
    resolve_block_axis, resolve_inline_axis, BlockAxisProps, InlineAxisProps,
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
