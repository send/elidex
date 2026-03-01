//! Layout algorithms (block, inline, flexbox) for elidex.
//!
//! Implements CSS box layout including block formatting contexts,
//! inline layout, and the flexbox algorithm.

mod block;
mod flex;
pub mod hit_test;
mod inline;
mod layout;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, EdgeSizes};

pub use hit_test::{hit_test, HitTestResult};
pub use layout::layout_tree;

/// Maximum recursion depth for layout tree walking.
///
/// Prevents stack overflow on deeply nested DOMs. Shared between
/// block, inline, and flex layout modules.
const MAX_LAYOUT_DEPTH: u32 = 1000;

/// Replace non-finite f32 values (NaN, infinity) with 0.0.
fn sanitize(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Clamp a single value to non-negative: negative, NaN, and infinity become `0.0`.
#[must_use]
fn sanitize_non_negative(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

/// Clamp edge values to non-negative: negative values become `0.0`,
/// zero and positive values are preserved as-is. NaN and infinity also become `0.0`.
#[must_use]
pub(crate) fn sanitize_edge_values(top: f32, right: f32, bottom: f32, left: f32) -> EdgeSizes {
    EdgeSizes {
        top: sanitize_non_negative(top),
        right: sanitize_non_negative(right),
        bottom: sanitize_non_negative(bottom),
        left: sanitize_non_negative(left),
    }
}

/// Sanitize padding from a computed style (non-negative, finite).
#[must_use]
pub(crate) fn sanitize_padding(style: &ComputedStyle) -> EdgeSizes {
    sanitize_edge_values(
        style.padding_top,
        style.padding_right,
        style.padding_bottom,
        style.padding_left,
    )
}

/// Sanitize border widths from a computed style (non-negative, finite).
#[must_use]
pub(crate) fn sanitize_border(style: &ComputedStyle) -> EdgeSizes {
    sanitize_edge_values(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    )
}

/// Sum of horizontal (left + right) padding and border.
#[must_use]
pub(crate) fn horizontal_pb(padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    padding.left + padding.right + border.left + border.right
}

/// Sum of vertical (top + bottom) padding and border.
#[must_use]
pub(crate) fn vertical_pb(padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    padding.top + padding.bottom + border.top + border.bottom
}

/// Resolve a CSS dimension to a pixel value.
/// - Length: use directly
/// - Percentage: relative to `containing`
/// - Auto: returns `auto_value`
#[must_use]
pub(crate) fn resolve_dimension_value(dim: Dimension, containing: f32, auto_value: f32) -> f32 {
    match dim {
        Dimension::Length(px) => px,
        Dimension::Percentage(pct) => containing * pct / 100.0,
        Dimension::Auto => auto_value,
    }
}

/// Resolve a `Dimension` to a pixel value for min/max constraints.
///
/// `Auto` returns `default_value` (0.0 for min-*, infinity for max-*).
/// Percentages against indefinite or non-positive containing sizes return
/// `default_value`. Negative results are clamped to 0.
#[must_use]
pub(crate) fn resolve_min_max(dim: Dimension, containing: f32, default_value: f32) -> f32 {
    match dim {
        Dimension::Length(px) if px.is_finite() => px.max(0.0),
        Dimension::Percentage(pct) => {
            // Guard against indefinite containing sizes (flex) and zero/negative.
            if containing > 0.0 && containing < f32::MAX / 2.0 {
                sanitize(containing * pct / 100.0).max(0.0)
            } else {
                default_value
            }
        }
        _ => default_value,
    }
}

/// Clamp `value` between `min` and `max`, with `min` winning on conflict.
///
/// Equivalent to `value.max(min).min(max).max(min)`.
#[must_use]
pub(crate) fn clamp_min_max(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max).max(min)
}

/// Adjust min/max constraint values for `box-sizing: border-box`.
///
/// Subtracts `pb` (padding + border sum on the relevant axis) from both
/// `min` and `max`, clamping to 0. `max` is only adjusted when finite
/// (infinity means no constraint).
pub(crate) fn adjust_min_max_for_border_box(min: &mut f32, max: &mut f32, pb: f32) {
    *min = (*min - pb).max(0.0);
    if *max < f32::INFINITY {
        *max = (*max - pb).max(0.0);
    }
}

/// Resolve an explicit height (Length or Percentage) to content-box pixels.
///
/// Returns `None` for `auto`. For `border-box`, subtracts vertical padding + border.
/// Used by both block and flex layout for height resolution.
#[must_use]
pub(crate) fn resolve_explicit_height(
    style: &ComputedStyle,
    containing_height: Option<f32>,
) -> Option<f32> {
    let bb_pb = || {
        let p = sanitize_padding(style);
        let b = sanitize_border(style);
        vertical_pb(&p, &b)
    };
    match style.height {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - bb_pb()).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                (resolved - bb_pb()).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    }
}

/// Get the computed style for an entity, or a default if none is attached.
#[must_use]
fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
    try_get_style(dom, entity).unwrap_or_default()
}

/// Try to get the computed style for an entity. Returns `None` for text nodes
/// or entities without a style component.
#[must_use]
fn try_get_style(dom: &EcsDom, entity: Entity) -> Option<ComputedStyle> {
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .map(|s| (*s).clone())
}
