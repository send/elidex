//! CSS Transforms matrix math (CSS Transforms L1 S3 / L2 S3).
//!
//! Provides 4x4 matrix operations for CSS transform computation,
//! 2D affine helpers, and the `compute_transform` entry point used
//! by the display list builder and hit testing.

pub mod affine;
pub mod decompose;
pub mod mat4;

mod tests;

use crate::{
    BackfaceVisibility, ComputedStyle, CssValue, Dimension, Point, Rect, Size, TransformFunction,
};
use mat4::{apply_around_origin, function_to_4x4, perspective_4x4};

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
    pub origin: Point<f64>,
}

impl Default for Perspective {
    fn default() -> Self {
        Self {
            distance: None,
            origin: Point::new(0.0, 0.0),
        }
    }
}

// Re-exports for public API compatibility.
pub use affine::{apply_affine, invert_affine, mul_affine, IDENTITY};

/// Check if a 2D affine matrix is approximately the identity.
#[must_use]
pub fn is_affine_identity(m: &[f64; 6]) -> bool {
    m.iter()
        .zip(IDENTITY.iter())
        .all(|(a, b)| (a - b).abs() < ZERO_EPSILON)
}
pub use decompose::{decompose_2d, interpolate_decomposed, recompose_2d, Decomposed2d};
pub use mat4::{determinant_4x4, mul_4x4, project_to_2d, IDENTITY_4X4};

/// Convert an `f32` to `f64`, returning `0.0` for non-finite values.
#[must_use]
fn finite_or_zero(v: f32) -> f64 {
    if v.is_finite() {
        f64::from(v)
    } else {
        0.0
    }
}

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

/// Resolve a `(Dimension, Dimension)` pair against a `Rect` to absolute f64
/// coordinates.
///
/// Equivalent to `rect.origin + resolve_origin_dim(dim, rect.size)` per axis,
/// with non-finite guard.
#[must_use]
pub fn resolve_origin_pair(dim: &(Dimension, Dimension), bb: &Rect) -> Point<f64> {
    Point::new(
        finite_or_zero(bb.origin.x + resolve_origin_dim(&dim.0, bb.size.width)),
        finite_or_zero(bb.origin.y + resolve_origin_dim(&dim.1, bb.size.height)),
    )
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
pub(super) fn compute_transform(
    functions: &[TransformFunction],
    origin: mat4::Vec3,
    ref_size: Size,
    parent_perspective: &Perspective,
    backface_hidden: bool,
) -> Option<[f64; 6]> {
    let mut mat = IDENTITY_4X4;

    // 1. Parent perspective (translate to origin, apply, translate back)
    if let Some(d) = parent_perspective.distance {
        if d > 0.0 {
            apply_around_origin(
                &mut mat,
                mat4::Vec3::new(
                    parent_perspective.origin.x,
                    parent_perspective.origin.y,
                    0.0,
                ),
                |m| {
                    *m = mul_4x4(m, &perspective_4x4(f64::from(d)));
                },
            );
        }
    }

    // 2–4. Transform functions around transform-origin
    if !functions.is_empty() {
        apply_around_origin(&mut mat, origin, |m| {
            for f in functions {
                let fm = function_to_4x4(f, ref_size.width, ref_size.height);
                *m = mul_4x4(m, &fm);
            }
        });
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
    let o = resolve_origin_pair(&(style.transform_origin.0, style.transform_origin.1), bb);
    let oz = finite_or_zero(style.transform_origin.2);
    compute_transform(
        &style.transform,
        mat4::Vec3::new(o.x, o.y, oz),
        bb.size,
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
    bb: &Rect,
) -> Point<f64> {
    resolve_origin_pair(perspective_origin, bb)
}

/// Resolve the perspective and perspective-origin to propagate to children.
///
/// Returns `(perspective, perspective_origin)`. Shared by display list builder
/// and hit testing to avoid duplicating the same logic.
#[must_use]
pub fn resolve_child_perspective(style: &ComputedStyle, bb: &Rect) -> Perspective {
    let distance = style.perspective;
    let origin = if distance.is_some() {
        compute_perspective_origin(&style.perspective_origin, bb)
    } else {
        Point::new(0.0, 0.0)
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
