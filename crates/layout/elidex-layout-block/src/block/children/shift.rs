//! Descendant shifting utilities for block layout.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{LayoutBox, Vector, WritingModeContext};

use super::super::is_block_level;

/// Shift all block-level children along the block axis by `delta`,
/// iteratively including descendants.
///
/// Writing-mode-aware: shifts Y in `horizontal-tb`, X in vertical modes.
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
pub(in crate::block) fn shift_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    delta: f32,
    wm: WritingModeContext,
) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    let v = if wm.is_horizontal() {
        Vector::y_only(delta)
    } else {
        Vector::x_only(delta)
    };
    shift_descendants_inner(dom, children, v, true);
}

/// Shift descendants by (dx, dy), used to reposition float/positioned contents after placement.
pub fn shift_descendants(dom: &mut EcsDom, children: &[Entity], delta: Vector) {
    shift_descendants_inner(dom, children, delta, false);
}

/// Iterative tree walk that shifts `LayoutBox` positions by `delta`.
///
/// When `block_only` is true, only block-level entities (with a `ComputedStyle`)
/// are shifted; non-block children are skipped (but their descendants are still
/// walked).
fn shift_descendants_inner(dom: &mut EcsDom, children: &[Entity], delta: Vector, block_only: bool) {
    let mut stack: Vec<Entity> = children.to_vec();
    while let Some(child) = stack.pop() {
        let skip_shift = block_only
            && !crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display));
        if !skip_shift {
            if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(child) {
                lb.content.origin += delta;
            }
        }
        // Always walk descendants regardless of block_only filter.
        stack.extend(dom.composed_children(child));
    }
}
