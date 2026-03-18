#[cfg(test)]
mod tests {
    use crate::transform_math::affine::*;
    use crate::transform_math::decompose::*;
    use crate::transform_math::mat4::*;
    use crate::transform_math::*;
    use crate::TransformFunction;

    const EPSILON: f64 = 1e-9;

    fn assert_near(a: f64, b: f64, msg: &str) {
        assert!((a - b).abs() < EPSILON, "{msg}: {a} != {b}");
    }

    #[test]
    fn mat4_translate() {
        let m = translate_4x4(10.0, 20.0, 0.0);
        assert_eq!(m[12], 10.0);
        assert_eq!(m[13], 20.0);
        assert_eq!(m[14], 0.0);
        assert_eq!(m[0], 1.0);
        assert_eq!(m[5], 1.0);
    }

    #[test]
    fn mat4_rotate_z_90() {
        let m = rotate_z_4x4(90.0);
        assert_near(m[0], 0.0, "cos90");
        assert_near(m[1], 1.0, "sin90");
        assert_near(m[4], -1.0, "-sin90");
        assert_near(m[5], 0.0, "cos90");
    }

    #[test]
    fn mat4_rotate_x_90() {
        let m = rotate_x_4x4(90.0);
        assert_eq!(m[0], 1.0);
        assert_near(m[5], 0.0, "cos90");
        assert_near(m[6], 1.0, "sin90");
        assert_near(m[9], -1.0, "-sin90");
        assert_near(m[10], 0.0, "cos90");
    }

    #[test]
    fn mat4_rotate_y_90() {
        let m = rotate_y_4x4(90.0);
        assert_near(m[0], 0.0, "cos90");
        assert_near(m[2], -1.0, "-sin90");
        assert_eq!(m[5], 1.0);
        assert_near(m[8], 1.0, "sin90");
        assert_near(m[10], 0.0, "cos90");
    }

    #[test]
    fn mat4_scale_3d() {
        let m = scale_4x4(2.0, 3.0, 0.5);
        assert_eq!(m[0], 2.0);
        assert_eq!(m[5], 3.0);
        assert_eq!(m[10], 0.5);
        assert_eq!(m[15], 1.0);
    }

    #[test]
    fn mat4_perspective() {
        let m = perspective_4x4(500.0);
        assert_eq!(m[0], 1.0);
        assert_eq!(m[5], 1.0);
        assert_eq!(m[10], 1.0);
        assert_near(m[11], -1.0 / 500.0, "perspective");
        assert_eq!(m[15], 1.0);
    }

    #[test]
    fn project_to_2d_identity() {
        let proj = project_to_2d(&IDENTITY_4X4);
        assert_eq!(proj, IDENTITY);
    }

    #[test]
    fn project_to_2d_with_perspective() {
        // perspective(500px) + rotateY(45deg)
        let persp = perspective_4x4(500.0);
        let rot = rotate_y_4x4(45.0);
        let combined = mul_4x4(&persp, &rot);
        let proj = project_to_2d(&combined);
        // Should produce a non-identity 2D affine
        assert!((proj[0] - 1.0).abs() > 0.01 || (proj[2]).abs() > 0.01);

        // Jacobian linearization: for pure 2D (no perspective terms),
        // the result must match the simple m[i]/m[15] formula.
        let rot2d = rotate_z_4x4(30.0);
        let proj2d = project_to_2d(&rot2d);
        assert_near(proj2d[0], rot2d[0], "2D a");
        assert_near(proj2d[1], rot2d[1], "2D b");
        assert_near(proj2d[4], rot2d[12], "2D e");
    }

    #[test]
    fn backface_hidden_detection() {
        // rotateY(180deg) flips the element -- should be detected as back-facing
        let m = rotate_y_4x4(180.0);
        // CSS backface check: mat[0]*mat[5] - mat[1]*mat[4] < 0
        let face = (m[0] * m[5] - m[1] * m[4]) * m[15];
        assert!(
            face < 0.0,
            "180deg Y rotation should face away: face={face}"
        );

        // Identity should face forward
        let id = IDENTITY_4X4;
        let face_id = (id[0] * id[5] - id[1] * id[4]) * id[15];
        assert!(
            face_id > 0.0,
            "identity should face forward: face={face_id}"
        );

        // compute_transform should return None for backface-hidden + 180deg Y
        let result = compute_transform(
            &[TransformFunction::RotateY(180.0)],
            (50.0, 50.0, 0.0),
            (100.0, 100.0),
            None,
            (0.0, 0.0),
            true,
        );
        assert!(
            result.is_none(),
            "backface-hidden rotateY(180deg) should be None"
        );

        // And Some for backface-visible
        let result = compute_transform(
            &[TransformFunction::RotateY(180.0)],
            (50.0, 50.0, 0.0),
            (100.0, 100.0),
            None,
            (0.0, 0.0),
            false,
        );
        assert!(
            result.is_some(),
            "backface-visible rotateY(180deg) should be Some"
        );
    }

    #[test]
    fn affine_compose_multiple() {
        // translate(10, 20) then rotate(90deg)
        let t = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let r = {
            let rad = std::f64::consts::FRAC_PI_2;
            let (sin, cos) = rad.sin_cos();
            [cos, sin, -sin, cos, 0.0, 0.0]
        };
        let result = mul_affine(t, r);
        // After translate then rotate, e and f should change
        assert_near(result[4], 10.0, "e");
        assert_near(result[5], 20.0, "f");
    }

    #[test]
    fn invert_affine_roundtrip() {
        let m = [2.0, 0.5, -0.3, 1.5, 10.0, 20.0];
        let inv = invert_affine(m).expect("should be invertible");
        let identity = mul_affine(m, inv);
        assert_near(identity[0], 1.0, "a");
        assert_near(identity[1], 0.0, "b");
        assert_near(identity[2], 0.0, "c");
        assert_near(identity[3], 1.0, "d");
        assert_near(identity[4], 0.0, "e");
        assert_near(identity[5], 0.0, "f");
    }

    #[test]
    fn rotate3d_axis_angle() {
        // rotate3d(0, 0, 1, 90deg) should equal rotateZ(90deg)
        let m3d = rotate_3d_4x4(0.0, 0.0, 1.0, 90.0);
        let mz = rotate_z_4x4(90.0);
        for i in 0..16 {
            assert_near(m3d[i], mz[i], &format!("element {i}"));
        }
    }

    #[test]
    fn origin_z_affects_perspective_projection() {
        // With perspective + rotateY, origin_z shifts the element along Z before rotation,
        // producing a different 2D projection than origin_z = 0.
        let funcs = vec![TransformFunction::RotateY(45.0)];
        let a = compute_transform(
            &funcs,
            (50.0, 50.0, 0.0),
            (100.0, 100.0),
            Some(500.0),
            (50.0, 50.0),
            false,
        )
        .unwrap();
        let b = compute_transform(
            &funcs,
            (50.0, 50.0, 100.0),
            (100.0, 100.0),
            Some(500.0),
            (50.0, 50.0),
            false,
        )
        .unwrap();
        // The two should differ because origin_z changes the 4x4 matrix
        let differs = a.iter().zip(b.iter()).any(|(x, y)| (x - y).abs() > 1e-6);
        assert!(
            differs,
            "origin_z=100 should produce different result than origin_z=0"
        );
    }

    #[test]
    fn decompose_identity() {
        let d = decompose_2d(IDENTITY).unwrap();
        assert_near(d.translate_x, 0.0, "tx");
        assert_near(d.translate_y, 0.0, "ty");
        assert_near(d.rotate, 0.0, "rotate");
        assert_near(d.scale_x, 1.0, "sx");
        assert_near(d.scale_y, 1.0, "sy");
        assert_near(d.skew, 0.0, "skew");
    }

    #[test]
    fn decompose_translate() {
        let m = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let d = decompose_2d(m).unwrap();
        assert_near(d.translate_x, 10.0, "tx");
        assert_near(d.translate_y, 20.0, "ty");
        assert_near(d.scale_x, 1.0, "sx");
        assert_near(d.scale_y, 1.0, "sy");
    }

    #[test]
    fn decompose_scale() {
        let m = [2.0, 0.0, 0.0, 3.0, 0.0, 0.0];
        let d = decompose_2d(m).unwrap();
        assert_near(d.scale_x, 2.0, "sx");
        assert_near(d.scale_y, 3.0, "sy");
        assert_near(d.rotate, 0.0, "rotate");
    }

    #[test]
    fn decompose_rotate_90() {
        let rad = std::f64::consts::FRAC_PI_2;
        let (sin, cos) = rad.sin_cos();
        let m = [cos, sin, -sin, cos, 0.0, 0.0];
        let d = decompose_2d(m).unwrap();
        assert_near(d.rotate, rad, "rotate");
        assert_near(d.scale_x, 1.0, "sx");
        assert_near(d.scale_y, 1.0, "sy");
    }

    #[test]
    fn decompose_recompose_roundtrip() {
        // A complex transform: translate + rotate + scale + skew
        let m = [1.5, 0.3, -0.2, 2.0, 15.0, 25.0];
        let d = decompose_2d(m).unwrap();
        let r = recompose_2d(&d);
        for i in 0..6 {
            assert_near(r[i], m[i], &format!("element {i}"));
        }
    }

    #[test]
    fn interpolate_decomposed_midpoint() {
        let a = Decomposed2d {
            translate_x: 0.0,
            translate_y: 0.0,
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew: 0.0,
        };
        let b = Decomposed2d {
            translate_x: 100.0,
            translate_y: 200.0,
            rotate: std::f64::consts::PI,
            scale_x: 2.0,
            scale_y: 3.0,
            skew: 0.5,
        };
        let mid = interpolate_decomposed(&a, &b, 0.5);
        assert_near(mid.translate_x, 50.0, "tx");
        assert_near(mid.translate_y, 100.0, "ty");
        assert_near(mid.rotate, std::f64::consts::FRAC_PI_2, "rotate");
        assert_near(mid.scale_x, 1.5, "sx");
        assert_near(mid.scale_y, 2.0, "sy");
        assert_near(mid.skew, 0.25, "skew");
    }

    #[test]
    fn interpolate_rotation_shortest_path() {
        // -170deg and +170deg should interpolate through +/-180deg (2deg gap), not through 0deg (340deg gap).
        let a = Decomposed2d {
            rotate: -170.0_f64.to_radians(),
            ..Decomposed2d {
                translate_x: 0.0,
                translate_y: 0.0,
                rotate: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
                skew: 0.0,
            }
        };
        let b = Decomposed2d {
            rotate: 170.0_f64.to_radians(),
            ..a
        };
        let mid = interpolate_decomposed(&a, &b, 0.5);
        // Shortest path midpoint: -180deg (or equivalently +180deg)
        assert!(
            (mid.rotate.abs() - std::f64::consts::PI).abs() < 0.01,
            "midpoint should be near +/-pi, got {} rad ({} deg)",
            mid.rotate,
            mid.rotate.to_degrees()
        );
    }

    // --- Safety tests ---

    #[test]
    fn skew_90_degrees_finite() {
        // skew(90deg) produces tan(π/2) ≈ ∞ — must be clamped to finite.
        use crate::transform_math::mat4::*;
        let funcs = vec![TransformFunction::Skew(90.0, 0.0)];
        let m = function_to_4x4(&funcs[0], 100.0, 100.0);
        for (i, &val) in m.iter().enumerate() {
            assert!(
                val.is_finite(),
                "skew(90deg) matrix[{i}] must be finite, got {val}"
            );
        }
    }

    #[test]
    fn rotate3d_nan_axis_returns_identity() {
        let m = rotate_3d_4x4(f64::NAN, 0.0, 1.0, 45.0);
        assert_eq!(m, IDENTITY_4X4, "NaN axis should produce identity");
    }

    #[test]
    fn rotate3d_infinity_axis_returns_identity() {
        let m = rotate_3d_4x4(f64::INFINITY, 0.0, 0.0, 45.0);
        assert_eq!(m, IDENTITY_4X4, "Infinity axis should produce identity");
    }

    #[test]
    fn skew_negative_90_finite() {
        let funcs = vec![TransformFunction::Skew(-90.0, -90.0)];
        let m = function_to_4x4(&funcs[0], 100.0, 100.0);
        for (i, &val) in m.iter().enumerate() {
            assert!(
                val.is_finite(),
                "skew(-90deg) matrix[{i}] must be finite, got {val}"
            );
        }
    }

    #[test]
    fn compute_perspective_origin_basic() {
        use crate::Dimension;
        let origin = (Dimension::Percentage(50.0), Dimension::Percentage(50.0));
        let (ox, oy) = compute_perspective_origin(&origin, 10.0, 20.0, 200.0, 100.0);
        assert_near(ox, f64::from(10.0 + 100.0), "origin x");
        assert_near(oy, f64::from(20.0 + 50.0), "origin y");
    }
}
