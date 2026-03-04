//! Path construction helpers for Canvas 2D.
//!
//! Converts Canvas 2D `arc()` calls to cubic Bezier curves since
//! tiny-skia's `PathBuilder` does not have a native arc method.

use std::f32::consts::{FRAC_PI_2, TAU};

use tiny_skia::PathBuilder;

/// Approximate a circular arc with cubic Bezier curves.
///
/// Implements the Canvas 2D `arc(x, y, radius, startAngle, endAngle, anticlockwise)`
/// specification. The arc is split into quarter-circle (or smaller) segments,
/// each approximated by a single cubic Bezier curve.
///
/// The magic constant `0.5522847498` is derived from `(4/3) * tan(π/8)`,
/// which gives the optimal control point distance for a quarter-circle
/// Bezier approximation.
pub(crate) fn arc_to_beziers(
    pb: &mut PathBuilder,
    cx: f32,
    cy: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    anticlockwise: bool,
) {
    // Per Canvas 2D spec, non-finite arguments are silently ignored.
    if !cx.is_finite() || !cy.is_finite() || !start_angle.is_finite() || !end_angle.is_finite() {
        return;
    }

    // TODO(Phase 4): per Canvas 2D spec, negative radius should throw an
    // IndexSizeError DOMException. Currently treated as a no-op (same as radius == 0).
    if !radius.is_finite() || radius <= 0.0 {
        return;
    }

    // Normalize the sweep angle.
    let mut sweep = end_angle - start_angle;
    if anticlockwise {
        // Ensure sweep is negative for anticlockwise.
        if sweep > 0.0 {
            sweep -= TAU * ((sweep / TAU).ceil());
        }
        if sweep == 0.0 && end_angle != start_angle {
            sweep = -TAU;
        }
    } else {
        // Ensure sweep is positive for clockwise.
        if sweep < 0.0 {
            sweep += TAU * ((-sweep / TAU).ceil());
        }
        if sweep == 0.0 && end_angle != start_angle {
            sweep = TAU;
        }
    }

    if sweep.abs() < f32::EPSILON {
        // Degenerate: just move/line to the start point.
        let sx = cx + radius * start_angle.cos();
        let sy = cy + radius * start_angle.sin();
        if pb.last_point().is_some() {
            pb.line_to(sx, sy);
        } else {
            pb.move_to(sx, sy);
        }
        return;
    }

    // Clamp sweep to full circle.
    let sweep = sweep.clamp(-TAU, TAU);

    // Split into segments of at most 90 degrees (π/2).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n_segments = (sweep.abs() / FRAC_PI_2).ceil() as usize;
    #[allow(clippy::cast_precision_loss)]
    let segment_angle = sweep / n_segments as f32;

    let mut angle = start_angle;
    for i in 0..n_segments {
        let a1 = angle;
        let a2 = angle + segment_angle;
        arc_segment(pb, cx, cy, radius, a1, a2, i == 0);
        angle = a2;
    }
}

/// Draw a single arc segment (≤ 90 degrees) as a cubic Bezier.
#[allow(clippy::similar_names)]
fn arc_segment(pb: &mut PathBuilder, cx: f32, cy: f32, radius: f32, a1: f32, a2: f32, first: bool) {
    let half = (a2 - a1) / 2.0;
    // Control point distance: (4/3) * tan(half_angle).
    let k = (4.0 / 3.0) * (half / 2.0).tan();

    let cos1 = a1.cos();
    let sin1 = a1.sin();
    let cos2 = a2.cos();
    let sin2 = a2.sin();

    let x1 = cx + radius * cos1;
    let y1 = cy + radius * sin1;
    let x2 = cx + radius * cos2;
    let y2 = cy + radius * sin2;

    // Control points.
    let cp1x = x1 - k * radius * sin1;
    let cp1y = y1 + k * radius * cos1;
    let cp2x = x2 + k * radius * sin2;
    let cp2y = y2 - k * radius * cos2;

    if first {
        if pb.last_point().is_some() {
            pb.line_to(x1, y1);
        } else {
            pb.move_to(x1, y1);
        }
    }

    pb.cubic_to(cp1x, cp1y, cp2x, cp2y, x2, y2);
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;

    #[test]
    fn arc_to_beziers_variants() {
        // (cx, cy, radius, start, end, anticlockwise, should_produce_path, description)
        #[allow(clippy::type_complexity)]
        let cases: &[(f32, f32, f32, f32, f32, bool, bool, &str)] = &[
            (50.0, 50.0, 25.0, 0.0, TAU, false, true, "full circle"),
            (
                0.0,
                0.0,
                10.0,
                0.0,
                FRAC_PI_2,
                false,
                true,
                "quarter circle",
            ),
            (0.0, 0.0, 10.0, 0.0, PI, false, true, "half circle"),
            (
                0.0,
                0.0,
                10.0,
                0.0,
                -FRAC_PI_2,
                true,
                true,
                "anticlockwise arc",
            ),
            (0.0, 0.0, 0.0, 0.0, PI, false, false, "zero radius"),
        ];

        for &(cx, cy, radius, start, end, acw, should_produce, desc) in cases {
            let mut pb = PathBuilder::new();
            arc_to_beziers(&mut pb, cx, cy, radius, start, end, acw);
            let path = pb.finish();
            if should_produce {
                let p = path.unwrap_or_else(|| panic!("case: {desc} should produce a path"));
                assert!(!p.is_empty(), "case: {desc} should be non-empty");
            } else {
                assert!(
                    path.is_none() || path.unwrap().is_empty(),
                    "case: {desc} should be empty/none"
                );
            }
        }
    }
}
