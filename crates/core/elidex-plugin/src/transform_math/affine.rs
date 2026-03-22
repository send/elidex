//! 2D affine matrix operations.
//!
//! Layout: `[a, b, c, d, e, f]`
//! ```text
//! | a c e |
//! | b d f |
//! | 0 0 1 |
//! ```

use crate::Point;

/// 2D affine identity.
pub const IDENTITY: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// 2D affine matrix multiplication.
#[must_use]
pub fn mul_affine(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    debug_assert!(
        a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()),
        "mul_affine: non-finite input"
    );
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

/// Apply a 2D affine transform to a point.
///
/// Returns the transformed point.
#[must_use]
pub fn apply_affine(m: &[f64; 6], p: Point<f64>) -> Point<f64> {
    Point::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

/// Invert a 2D affine matrix. Returns `None` if determinant ~ 0.
#[must_use]
pub fn invert_affine(m: [f64; 6]) -> Option<[f64; 6]> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < super::ZERO_EPSILON {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        m[3] * inv_det,
        -m[1] * inv_det,
        -m[2] * inv_det,
        m[0] * inv_det,
        (m[2] * m[5] - m[3] * m[4]) * inv_det,
        (m[1] * m[4] - m[0] * m[5]) * inv_det,
    ])
}
