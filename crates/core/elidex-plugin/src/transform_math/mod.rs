//! CSS Transforms matrix math (CSS Transforms L1 S3 / L2 S3).
//!
//! Provides 4x4 matrix operations for CSS transform computation,
//! 2D affine helpers, and the `compute_transform` entry point used
//! by the display list builder and hit testing.

pub mod affine;
pub mod decompose;
pub mod mat4;

mod tests;

use crate::{BackfaceVisibility, ComputedStyle, CssValue, Dimension, Rect, TransformFunction};
use mat4::{function_to_4x4, perspective_4x4, translate_4x4};

/// Threshold below which a floating-point value is treated as zero.
///
/// Used across transform math (matrix inversion, perspective, decomposition,
/// identity detection) to avoid division by near-zero values.
pub const ZERO_EPSILON: f64 = 1e-10;

/// CSS `perspective` property value and its resolved origin, propagated
/// from a parent element to its children.
///
/// Bundles the two values that always travel together through the display
/// list walk and hit testing: the perspective distance and the viewport-
/// coordinate origin point.
#[derive(Clone, Copy, Debug)]
pub struct Perspective {
    /// `perspective` property value (distance in px), or `None` if unset.
    pub distance: Option<f32>,
    /// Resolved `perspective-origin` in viewport coordinates.
    pub origin: (f64, f64),
}

impl Default for Perspective {
    fn default() -> Self {
        Self {
            distance: None,
            origin: (0.0, 0.0),
        }
    }
}

// Re-exports for public API compatibility.
pub use affine::{invert_affine, mul_affine, IDENTITY};

/// Check if a 2D affine matrix is approximately the identity.
#[must_use]
pub fn is_affine_identity(m: &[f64; 6]) -> bool {
    m.iter()
        .zip(IDENTITY.iter())
        .all(|(a, b)| (a - b).abs() < ZERO_EPSILON)
}
pub use decompose::{decompose_2d, interpolate_decomposed, recompose_2d, Decomposed2d};
pub use mat4::{determinant_4x4, mul_4x4, project_to_2d, IDENTITY_4X4};

/// Resolve a `Dimension` to pixels for transform-origin / perspective-origin.
///
/// Non-finite results (from NaN/Infinity in inputs) are clamped to 0.
// TODO: handle Dimension::Calc when calc() support is added to Dimension (Phase 4).
#[must_use]
pub fn resolve_origin_dim(dim: &Dimension, ref_size: f32) -> f32 {
    let v = match dim {
        Dimension::Length(px) => *px,
        Dimension::Percentage(pct) => ref_size * pct / 100.0,
        Dimension::Auto => ref_size * 0.5,
    };
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Resolve a `CssValue` (Length/Percentage) to a pixel value for translate functions.
///
/// Non-finite results (from NaN/Infinity in inputs) are clamped to 0.
pub(crate) fn resolve_translate_value(val: &CssValue, ref_size: f32) -> f64 {
    let v = match val {
        CssValue::Length(v, _) => f64::from(*v), // relative units resolved to px at style time
        CssValue::Percentage(pct) => f64::from(ref_size) * f64::from(*pct) / 100.0,
        CssValue::Number(n) if *n == 0.0 => 0.0,
        _ => 0.0,
    };
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Convert a `TransformFunction` to a 4x4 matrix without resolving percentages.
///
/// Used by animation interpolation where translate values are already resolved.
/// Percentage values evaluate against 0 (effectively ignored).
#[must_use]
pub fn function_to_4x4_no_resolve(func: &TransformFunction) -> [f64; 16] {
    function_to_4x4(func, 0.0, 0.0)
}

/// Compute the final 2D affine transform for a CSS element.
///
/// Implements CSS Transforms L1 S3 / L2 S3:
/// 1. Apply parent `perspective` property (if any)
/// 2. Translate to `transform-origin`
/// 3. Multiply each transform function left-to-right as 4x4 matrices
/// 4. Translate back from `transform-origin`
/// 5. Project 4x4 -> 2D affine
///
/// Returns `None` if `backface_hidden` is true and the element faces away.
#[must_use]
pub fn compute_transform(
    functions: &[TransformFunction],
    origin: (f64, f64, f64),
    ref_size: (f32, f32),
    parent_perspective: &Perspective,
    backface_hidden: bool,
) -> Option<[f64; 6]> {
    let mut mat = IDENTITY_4X4;

    // 1. Parent perspective
    if let Some(d) = parent_perspective.distance {
        if d > 0.0 {
            // Translate to perspective origin, apply perspective, translate back
            let (px, py) = parent_perspective.origin;
            mat = mul_4x4(&mat, &translate_4x4(px, py, 0.0));
            mat = mul_4x4(&mat, &perspective_4x4(f64::from(d)));
            mat = mul_4x4(&mat, &translate_4x4(-px, -py, 0.0));
        }
    }

    let (origin_x, origin_y, origin_z) = origin;
    if !functions.is_empty() {
        // 2. Translate to transform-origin (CSS Transforms L1 S3: includes Z)
        mat = mul_4x4(&mat, &translate_4x4(origin_x, origin_y, origin_z));

        // 3. Apply each transform function
        for f in functions {
            let fm = function_to_4x4(f, ref_size.0, ref_size.1);
            mat = mul_4x4(&mat, &fm);
        }

        // 4. Translate back from transform-origin
        mat = mul_4x4(&mat, &translate_4x4(-origin_x, -origin_y, -origin_z));
    }

    // CSS Transforms L2 S5: An element is back-facing when the z-component
    // of its transformed normal vector (0,0,1) is negative. After the 4x4
    // transform, the z-component of the normal is computed from the upper-left
    // 3x3 submatrix: if the 2D cross product of the first two columns of the
    // 3x3 has a negative z-component, the face is pointing away.
    // In practice: check mat[0]*mat[5] - mat[1]*mat[4] (the "2D determinant"
    // of the projection), adjusted for w-division.
    if backface_hidden {
        let w = mat[15];
        let face = (mat[0] * mat[5] - mat[1] * mat[4]) * w;
        if face < 0.0 {
            return None;
        }
    }

    // 5. Project to 2D affine
    Some(mat4::project_to_2d(&mat))
}

/// Compute the 2D affine transform for an element from its style and border box.
///
/// Shared helper used by both display list builder and hit testing to avoid
/// duplicating transform-origin resolution + `compute_transform` call logic.
///
/// Returns `None` if `backface-visibility: hidden` and the element faces away.
#[must_use]
pub fn compute_element_transform(
    style: &ComputedStyle,
    bb: &Rect,
    parent_perspective: &Perspective,
) -> Option<[f64; 6]> {
    let ox = resolve_origin_dim(&style.transform_origin.0, bb.width);
    let oy = resolve_origin_dim(&style.transform_origin.1, bb.height);
    let oz_raw = style.transform_origin.2;
    let oz = f64::from(if oz_raw.is_finite() { oz_raw } else { 0.0 });
    // Guard against f32 addition overflow (e.g. bb.x near f32::MAX).
    let sum_x = bb.x + ox;
    let sum_y = bb.y + oy;
    compute_transform(
        &style.transform,
        (
            f64::from(if sum_x.is_finite() { sum_x } else { 0.0 }),
            f64::from(if sum_y.is_finite() { sum_y } else { 0.0 }),
            oz,
        ),
        (bb.width, bb.height),
        parent_perspective,
        style.backface_visibility == BackfaceVisibility::Hidden,
    )
}

/// Compute absolute perspective-origin coordinates from style + border box.
///
/// Returns `(origin_x, origin_y)` in viewport coordinates. Used by display list
/// builder (walk.rs) and hit testing to avoid duplicating the same calculation.
#[must_use]
pub fn compute_perspective_origin(
    perspective_origin: &(Dimension, Dimension),
    bb_x: f32,
    bb_y: f32,
    bb_width: f32,
    bb_height: f32,
) -> (f64, f64) {
    (
        f64::from(bb_x + resolve_origin_dim(&perspective_origin.0, bb_width)),
        f64::from(bb_y + resolve_origin_dim(&perspective_origin.1, bb_height)),
    )
}

/// Resolve the perspective and perspective-origin to propagate to children.
///
/// Returns `(perspective, perspective_origin)`. Shared by display list builder
/// and hit testing to avoid duplicating the same logic.
#[must_use]
pub fn resolve_child_perspective(style: &ComputedStyle, bb: &Rect) -> Perspective {
    let distance = style.perspective;
    let origin = if distance.is_some() {
        compute_perspective_origin(&style.perspective_origin, bb.x, bb.y, bb.width, bb.height)
    } else {
        (0.0, 0.0)
    };
    Perspective { distance, origin }
}

/// Properties that create a stacking context when specified in `will-change`.
const WILL_CHANGE_STACKING_PROPS: &[&str] = &[
    "transform",
    "opacity",
    "filter",
    "backdrop-filter",
    "perspective",
    "clip-path",
    "mask",
    "isolation",
    "mix-blend-mode",
];

/// Check if a `will-change` property list creates a stacking context.
#[must_use]
pub fn will_change_creates_stacking(props: &[String]) -> bool {
    props
        .iter()
        .any(|p| WILL_CHANGE_STACKING_PROPS.contains(&p.as_str()))
}
