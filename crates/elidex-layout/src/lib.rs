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
use elidex_plugin::{ComputedStyle, Dimension, EdgeSizes};

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

/// Clamp edge values to non-negative: negative values become `0.0`,
/// zero and positive values are preserved as-is. NaN and infinity also become `0.0`.
pub(crate) fn sanitize_edge_values(top: f32, right: f32, bottom: f32, left: f32) -> EdgeSizes {
    EdgeSizes {
        top: if top.is_finite() && top > 0.0 {
            top
        } else {
            0.0
        },
        right: if right.is_finite() && right > 0.0 {
            right
        } else {
            0.0
        },
        bottom: if bottom.is_finite() && bottom > 0.0 {
            bottom
        } else {
            0.0
        },
        left: if left.is_finite() && left > 0.0 {
            left
        } else {
            0.0
        },
    }
}

/// Resolve a CSS dimension to a pixel value.
/// - Length: use directly
/// - Percentage: relative to `containing`
/// - Auto: returns `auto_value`
pub(crate) fn resolve_dimension_value(dim: Dimension, containing: f32, auto_value: f32) -> f32 {
    match dim {
        Dimension::Length(px) => px,
        Dimension::Percentage(pct) => containing * pct / 100.0,
        Dimension::Auto => auto_value,
    }
}

/// Get the computed style for an entity, or a default if none is attached.
fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
    dom.world()
        .get::<&ComputedStyle>(entity)
        .map(|s| (*s).clone())
        .unwrap_or_default()
}
