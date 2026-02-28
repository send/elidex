//! Layout algorithms (block, inline, flexbox) for elidex.
//!
//! Implements CSS box layout including block formatting contexts,
//! inline layout, and the flexbox algorithm.

mod block;
mod flex;
mod inline;
mod layout;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::ComputedStyle;

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

/// Get the computed style for an entity, or a default if none is attached.
fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
    dom.world()
        .get::<&ComputedStyle>(entity)
        .map(|s| (*s).clone())
        .unwrap_or_default()
}
