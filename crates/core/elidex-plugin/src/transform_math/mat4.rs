//! 4x4 matrix operations (row-major: `m[row * 4 + col]`).

use super::affine::IDENTITY;
use super::resolve_translate_value;
use super::ZERO_EPSILON;
use crate::TransformFunction;

/// Convert degrees to radians.
fn deg_to_rad(deg: f32) -> f64 {
    f64::from(deg) * std::f64::consts::PI / 180.0
}

/// Maximum absolute value for `tan()` results to prevent Infinity propagation.
/// CSS skew(90deg) produces tan(π/2) = ∞; we clamp to a large finite value
/// so downstream matrix operations remain well-defined.
const TAN_CLAMP: f64 = 1e10;

/// Clamp a value to `[-TAN_CLAMP, TAN_CLAMP]`.
fn clamp_finite(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(-TAN_CLAMP, TAN_CLAMP)
    } else {
        v.signum() * TAN_CLAMP
    }
}

/// 4x4 identity matrix.
pub const IDENTITY_4X4: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

/// 4x4 matrix multiplication (row-major).
#[must_use]
pub fn mul_4x4(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    debug_assert!(
        a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()),
        "mul_4x4: non-finite input"
    );
    let mut out = [0.0; 16];
    for row in 0..4 {
        for col in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[row * 4 + k] * b[k * 4 + col];
            }
            out[row * 4 + col] = sum;
        }
    }
    out
}

/// Project a 4x4 matrix to a 2D affine `[a, b, c, d, e, f]`.
///
/// For pure 2D transforms (m\[3\]==0, m\[7\]==0) this is exact.
/// For 3D/perspective transforms, uses a first-order Taylor (Jacobian)
/// linearization of the rational perspective projection at the origin.
/// This correctly captures foreshortening near the transform-origin and
/// is exact for affine transforms. The approximation degrades for strong
/// perspective (small d) or large displacements from the origin.
#[must_use]
pub fn project_to_2d(m: &[f64; 16]) -> [f64; 6] {
    let w = m[15];
    if !w.is_finite() || w.abs() < ZERO_EPSILON {
        return IDENTITY;
    }
    // Jacobian linearization of (X/W, Y/W) at origin (0,0):
    //   X = m[0]*x + m[4]*y + m[12]
    //   Y = m[1]*x + m[5]*y + m[13]
    //   W = m[3]*x + m[7]*y + m[15]
    //
    // d(X/W)/dx = (m[0]*W - X*m[3]) / W^2 evaluated at (0,0)
    //            = (m[0]*m[15] - m[12]*m[3]) / m[15]^2
    let w2 = w * w;
    let result = [
        (m[0] * w - m[12] * m[3]) / w2, // a = dx'/dx
        (m[1] * w - m[13] * m[3]) / w2, // b = dy'/dx
        (m[4] * w - m[12] * m[7]) / w2, // c = dx'/dy
        (m[5] * w - m[13] * m[7]) / w2, // d = dy'/dy
        m[12] / w,                      // e = x'(0,0)
        m[13] / w,                      // f = y'(0,0)
    ];
    // Guard against non-finite results from extreme perspective (w ≈ ZERO_EPSILON).
    if result.iter().any(|v| !v.is_finite()) {
        return IDENTITY;
    }
    result
}

/// Determinant of a 4x4 matrix.
#[allow(clippy::many_single_char_names)]
#[must_use]
pub fn determinant_4x4(m: &[f64; 16]) -> f64 {
    let (a, b, c, d) = (m[0], m[1], m[2], m[3]);
    let (e, f, g, h) = (m[4], m[5], m[6], m[7]);
    let (i, j, k, l) = (m[8], m[9], m[10], m[11]);
    let (mn, n, o, p) = (m[12], m[13], m[14], m[15]);

    a * (f * (k * p - l * o) - g * (j * p - l * n) + h * (j * o - k * n))
        - b * (e * (k * p - l * o) - g * (i * p - l * mn) + h * (i * o - k * mn))
        + c * (e * (j * p - l * n) - f * (i * p - l * mn) + h * (i * n - j * mn))
        - d * (e * (j * o - k * n) - f * (i * o - k * mn) + g * (i * n - j * mn))
}

/// Convert a single `TransformFunction` to a 4x4 matrix.
#[allow(clippy::many_single_char_names)]
pub(super) fn function_to_4x4(
    func: &TransformFunction,
    ref_width: f32,
    ref_height: f32,
) -> [f64; 16] {
    match func {
        TransformFunction::Translate(x, y) => {
            let tx = resolve_translate_value(x, ref_width);
            let ty = resolve_translate_value(y, ref_height);
            translate_4x4(tx, ty, 0.0)
        }
        TransformFunction::TranslateX(x) => {
            let tx = resolve_translate_value(x, ref_width);
            translate_4x4(tx, 0.0, 0.0)
        }
        TransformFunction::TranslateY(y) => {
            let ty = resolve_translate_value(y, ref_height);
            translate_4x4(0.0, ty, 0.0)
        }
        TransformFunction::Translate3d(x, y, z) => {
            let tx = resolve_translate_value(x, ref_width);
            let ty = resolve_translate_value(y, ref_height);
            let tz = resolve_translate_value(z, 0.0);
            translate_4x4(tx, ty, tz)
        }
        TransformFunction::TranslateZ(z) => {
            let tz = resolve_translate_value(z, 0.0);
            translate_4x4(0.0, 0.0, tz)
        }
        TransformFunction::Rotate(deg) | TransformFunction::RotateZ(deg) => rotate_z_4x4(*deg),
        TransformFunction::RotateX(deg) => rotate_x_4x4(*deg),
        TransformFunction::RotateY(deg) => rotate_y_4x4(*deg),
        TransformFunction::Rotate3d(x, y, z, deg) => rotate_3d_4x4(*x, *y, *z, *deg),
        TransformFunction::Scale(sx, sy) => scale_4x4(f64::from(*sx), f64::from(*sy), 1.0),
        TransformFunction::ScaleX(sx) => scale_4x4(f64::from(*sx), 1.0, 1.0),
        TransformFunction::ScaleY(sy) => scale_4x4(1.0, f64::from(*sy), 1.0),
        TransformFunction::Scale3d(sx, sy, sz) => {
            scale_4x4(f64::from(*sx), f64::from(*sy), f64::from(*sz))
        }
        TransformFunction::ScaleZ(sz) => scale_4x4(1.0, 1.0, f64::from(*sz)),
        TransformFunction::Skew(ax, ay) => skew_4x4(*ax, *ay),
        TransformFunction::SkewX(ax) => skew_4x4(*ax, 0.0),
        TransformFunction::SkewY(ay) => skew_4x4(0.0, *ay),
        TransformFunction::Matrix(m) => {
            // Embed 2D affine [a,b,c,d,e,f] into 4x4
            let [a, b, c, d, e, f] = *m;
            [
                a, b, 0.0, 0.0, //
                c, d, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                e, f, 0.0, 1.0,
            ]
        }
        TransformFunction::Matrix3d(m) => *m,
        TransformFunction::PerspectiveFunc(d) => perspective_4x4(f64::from(*d)),
    }
}

#[must_use]
pub(super) fn translate_4x4(tx: f64, ty: f64, tz: f64) -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        tx, ty, tz, 1.0,
    ]
}

#[must_use]
pub(super) fn scale_4x4(sx: f64, sy: f64, sz: f64) -> [f64; 16] {
    [
        sx, 0.0, 0.0, 0.0, //
        0.0, sy, 0.0, 0.0, //
        0.0, 0.0, sz, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

#[must_use]
pub(super) fn rotate_z_4x4(deg: f32) -> [f64; 16] {
    let (sin, cos) = deg_to_rad(deg).sin_cos();
    [
        cos, sin, 0.0, 0.0, //
        -sin, cos, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

#[must_use]
pub(super) fn rotate_x_4x4(deg: f32) -> [f64; 16] {
    let (sin, cos) = deg_to_rad(deg).sin_cos();
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, cos, sin, 0.0, //
        0.0, -sin, cos, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

#[must_use]
pub(super) fn rotate_y_4x4(deg: f32) -> [f64; 16] {
    let (sin, cos) = deg_to_rad(deg).sin_cos();
    [
        cos, 0.0, -sin, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        sin, 0.0, cos, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Rodrigues' rotation formula for arbitrary axis.
#[must_use]
pub(super) fn rotate_3d_4x4(x: f64, y: f64, z: f64, deg: f32) -> [f64; 16] {
    // Guard against NaN in axis components — NaN propagates through sqrt and
    // normalized values, corrupting the entire matrix.
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return IDENTITY_4X4;
    }
    let len = (x * x + y * y + z * z).sqrt();
    if len < ZERO_EPSILON {
        return IDENTITY_4X4;
    }
    let (x, y, z) = (x / len, y / len, z / len);
    let (sin, cos) = deg_to_rad(deg).sin_cos();
    let t = 1.0 - cos;
    [
        t * x * x + cos,
        t * x * y + sin * z,
        t * x * z - sin * y,
        0.0,
        t * x * y - sin * z,
        t * y * y + cos,
        t * y * z + sin * x,
        0.0,
        t * x * z + sin * y,
        t * y * z - sin * x,
        t * z * z + cos,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

#[allow(clippy::similar_names)]
fn skew_4x4(ax_deg: f32, ay_deg: f32) -> [f64; 16] {
    let tan_x = clamp_finite(deg_to_rad(ax_deg).tan());
    let tan_y = clamp_finite(deg_to_rad(ay_deg).tan());
    [
        1.0, tan_y, 0.0, 0.0, //
        tan_x, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

#[must_use]
pub(super) fn perspective_4x4(d: f64) -> [f64; 16] {
    if !d.is_finite() || d.abs() < ZERO_EPSILON {
        return IDENTITY_4X4;
    }
    [
        1.0,
        0.0,
        0.0,
        0.0, //
        0.0,
        1.0,
        0.0,
        0.0, //
        0.0,
        0.0,
        1.0,
        -1.0 / d, //
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}
