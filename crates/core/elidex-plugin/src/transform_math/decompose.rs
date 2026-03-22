//! 2D matrix decomposition (CSS Transforms L1 S12 "unmatrix") and interpolation.

/// Decomposed 2D transform: translate, rotate, scale, skew.
///
/// CSS Transforms L1 S12 defines this decomposition for interpolation
/// of mismatched transform lists.
#[derive(Clone, Copy, Debug)]
pub struct Decomposed2d {
    pub translate_x: f64,
    pub translate_y: f64,
    /// Rotation angle in radians.
    pub rotate: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    /// Skew angle in radians.
    pub skew: f64,
}

/// Decompose a 2D affine matrix `[a, b, c, d, e, f]` into translate, rotate, scale, skew.
///
/// Based on the "unmatrix" algorithm from CSS Transforms L1 S12.
/// Returns `None` if the matrix is singular (determinant ~ 0).
#[allow(clippy::many_single_char_names)]
#[must_use]
pub fn decompose_2d(m: [f64; 6]) -> Option<Decomposed2d> {
    let [a, b, c, d, e, f] = m;

    // 1. Translation
    let translate_x = e;
    let translate_y = f;

    // 2. Compute scale_x from first row (a, b)
    let mut scale_x = (a * a + b * b).sqrt();
    if scale_x < super::ZERO_EPSILON {
        return None;
    }

    // Normalize row 0
    let mut r0x = a / scale_x;
    let mut r0y = b / scale_x;

    // 3. Skew = dot(row0, row1)
    let mut skew = r0x * c + r0y * d;

    // 4. Make row1 orthogonal: row1 -= skew * row0
    let r1x = c - skew * r0x;
    let r1y = d - skew * r0y;

    // 5. scale_y from row1
    let scale_y = (r1x * r1x + r1y * r1y).sqrt();
    if scale_y < super::ZERO_EPSILON {
        return None;
    }

    // Normalize skew
    skew /= scale_y;

    // 6. Check orientation: if cross product < 0, negate scale_x and row0
    let cross = r0x * (d - skew * r0y) - r0y * (c - skew * r0x);
    if cross.is_finite() && cross < 0.0 {
        scale_x = -scale_x;
        r0x = -r0x;
        r0y = -r0y;
    }

    // 7. Rotation
    let rotate = r0y.atan2(r0x);

    Some(Decomposed2d {
        translate_x,
        translate_y,
        rotate,
        scale_x,
        scale_y,
        skew,
    })
}

/// Recompose a `Decomposed2d` back into a 2D affine matrix `[a, b, c, d, e, f]`.
#[must_use]
pub fn recompose_2d(d: &Decomposed2d) -> [f64; 6] {
    let (sin, cos) = d.rotate.sin_cos();
    // Matrix = translate * rotate * skew * scale
    // skew is applied as shear on the Y axis of the pre-rotated frame
    let a = d.scale_x * cos;
    let b = d.scale_x * sin;
    let c = d.scale_y * (-sin + cos * d.skew);
    let d_val = d.scale_y * (cos + sin * d.skew);
    [a, b, c, d_val, d.translate_x, d.translate_y]
}

/// Linearly interpolate two `Decomposed2d` values.
#[must_use]
pub fn interpolate_decomposed(a: &Decomposed2d, b: &Decomposed2d, t: f64) -> Decomposed2d {
    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + (b - a) * t
    }
    // CSS Transforms L1 S12: rotation uses shortest-path interpolation.
    let mut angle_diff = b.rotate - a.rotate;
    if angle_diff > std::f64::consts::PI {
        angle_diff -= 2.0 * std::f64::consts::PI;
    } else if angle_diff < -std::f64::consts::PI {
        angle_diff += 2.0 * std::f64::consts::PI;
    }

    let result = Decomposed2d {
        translate_x: lerp(a.translate_x, b.translate_x, t),
        translate_y: lerp(a.translate_y, b.translate_y, t),
        rotate: a.rotate + angle_diff * t,
        scale_x: lerp(a.scale_x, b.scale_x, t),
        scale_y: lerp(a.scale_y, b.scale_y, t),
        skew: lerp(a.skew, b.skew, t),
    };
    debug_assert!(
        result.translate_x.is_finite()
            && result.translate_y.is_finite()
            && result.rotate.is_finite()
            && result.scale_x.is_finite()
            && result.scale_y.is_finite()
            && result.skew.is_finite(),
        "interpolate_decomposed produced non-finite values"
    );
    result
}
