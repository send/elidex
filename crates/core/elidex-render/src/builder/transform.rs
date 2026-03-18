//! Transform computation helpers for display list building.

use elidex_plugin::transform_math::{compute_element_transform, is_affine_identity, Perspective};
use elidex_plugin::{ComputedStyle, LayoutBox};

/// Result of computing an element's CSS transform.
pub(crate) enum TransformResult {
    /// No transform (parent perspective also absent) — no `PushTransform` needed.
    None,
    /// backface-visibility: hidden and the element faces away — skip subtree.
    BackfaceHidden,
    /// Projected 2D affine — emit `PushTransform`.
    Affine([f64; 6]),
}

/// Compute the CSS transform for an element.
pub(crate) fn element_transform(
    style: &ComputedStyle,
    lb: &LayoutBox,
    parent_perspective: &Perspective,
) -> TransformResult {
    if !style.has_transform && parent_perspective.distance.is_none() {
        return TransformResult::None;
    }
    let bb = lb.border_box();
    match compute_element_transform(style, &bb, parent_perspective) {
        Some(affine) => {
            if is_affine_identity(&affine) {
                TransformResult::None
            } else {
                TransformResult::Affine(affine)
            }
        }
        None => TransformResult::BackfaceHidden,
    }
}
