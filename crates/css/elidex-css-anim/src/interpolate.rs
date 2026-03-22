//! CSS value interpolation for animations and transitions.
//!
//! Implements interpolation between CSS computed values for animatable
//! properties (CSS Transitions Level 1 §4).

use elidex_plugin::transform_math::{
    decompose_2d, interpolate_decomposed, project_to_2d, recompose_2d, IDENTITY_4X4,
};
use elidex_plugin::{CssColor, CssValue, TransformFunction};

/// Interpolate between two CSS values at progress `t` (0.0..=1.0).
///
/// Continuous interpolation for numeric types (Number, Length, Percentage,
/// Color, Time). Falls back to discrete interpolation (flip at 50%) for
/// non-numeric or mismatched types.
///
/// The `property` parameter enables property-specific interpolation rules
/// (e.g. `visibility` uses special interpolation per CSS Transitions Level 1).
#[must_use]
pub fn interpolate(from: &CssValue, to: &CssValue, t: f32, property: &str) -> Option<CssValue> {
    // CSS Transitions Level 1: visibility has special interpolation.
    // Any value between `visible` and another value produces `visible`;
    // it only flips to the non-visible value at the endpoint.
    if property == "visibility" {
        if let (CssValue::Keyword(a), CssValue::Keyword(b)) = (from, to) {
            if a == "visible" && b != "visible" {
                return Some(if t < 1.0 { from.clone() } else { to.clone() });
            }
            if b == "visible" && a != "visible" {
                return Some(if t > 0.0 { to.clone() } else { from.clone() });
            }
        }
    }

    match (from, to) {
        // Number ↔ Number
        (CssValue::Number(a), CssValue::Number(b)) => Some(CssValue::Number(lerp(*a, *b, t))),

        // Length ↔ Length (same unit)
        (CssValue::Length(a, ua), CssValue::Length(b, ub)) if ua == ub => {
            Some(CssValue::Length(lerp(*a, *b, t), *ua))
        }

        // Percentage ↔ Percentage
        (CssValue::Percentage(a), CssValue::Percentage(b)) => {
            Some(CssValue::Percentage(lerp(*a, *b, t)))
        }

        // Color ↔ Color (RGBA interpolation, premultiplied alpha)
        (CssValue::Color(a), CssValue::Color(b)) => {
            Some(CssValue::Color(interpolate_color(a, b, t)))
        }

        // Time ↔ Time
        (CssValue::Time(a), CssValue::Time(b)) => Some(CssValue::Time(lerp(*a, *b, t))),

        // TransformList ↔ TransformList (CSS Transforms L1 §9)
        (CssValue::TransformList(a), CssValue::TransformList(b)) => {
            interpolate_transform_lists(a, b, t)
        }

        // transform: none ↔ TransformList — treat none as empty list
        (CssValue::Keyword(k), CssValue::TransformList(b)) if k == "none" => {
            interpolate_transform_lists(&[], b, t)
        }
        (CssValue::TransformList(a), CssValue::Keyword(k)) if k == "none" => {
            interpolate_transform_lists(a, &[], t)
        }

        // Discrete interpolation for non-animatable types (auto, keywords,
        // strings, mismatched types). Per CSS Transitions L1 §4, these
        // flip at the 50% point rather than smoothly interpolating.
        _ => Some(discrete(from, to, t)),
    }
}

/// Discrete interpolation: returns `from` before 50%, `to` at/after 50%.
fn discrete(from: &CssValue, to: &CssValue, t: f32) -> CssValue {
    if t < 0.5 {
        from.clone()
    } else {
        to.clone()
    }
}

/// Linear interpolation: `a + (b - a) * t`.
///
/// Returns `a` if the result is not finite (NaN or infinity), which can
/// happen when inputs are NaN or extreme values near `f32::MAX` overflow.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let result = a + (b - a) * t;
    if result.is_finite() {
        result
    } else {
        a
    }
}

/// Interpolate two RGBA colors component-wise with premultiplied alpha.
///
/// Per CSS Color Level 4 §12, color interpolation uses premultiplied alpha
/// to avoid darkening artifacts at semi-transparent midpoints.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn interpolate_color(from: &CssColor, to: &CssColor, t: f32) -> CssColor {
    let from_alpha = f32::from(from.a) / 255.0;
    let to_alpha = f32::from(to.a) / 255.0;
    // Premultiply
    let from_r = f32::from(from.r) * from_alpha;
    let from_g = f32::from(from.g) * from_alpha;
    let from_b = f32::from(from.b) * from_alpha;
    let to_r = f32::from(to.r) * to_alpha;
    let to_g = f32::from(to.g) * to_alpha;
    let to_b = f32::from(to.b) * to_alpha;
    // Interpolate in premultiplied space
    let alpha = lerp(from_alpha, to_alpha, t);
    let red = lerp(from_r, to_r, t);
    let green = lerp(from_g, to_g, t);
    let blue = lerp(from_b, to_b, t);
    // Un-premultiply
    if alpha < 1e-3 {
        CssColor::new(0, 0, 0, 0)
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        CssColor::new(
            (red / alpha).round().clamp(0.0, 255.0) as u8,
            (green / alpha).round().clamp(0.0, 255.0) as u8,
            (blue / alpha).round().clamp(0.0, 255.0) as u8,
            (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }
}

/// All animatable CSS property names (CSS Transitions Level 1 §3).
///
/// Used by the transition detection system to compare old vs new computed values.
/// Includes all properties that can be interpolated with the current `CssValue` types
/// (lengths, colors, numbers, percentages, keywords).
///
/// Properties not yet interpolatable:
/// - `filter`, `clip-path`: require dedicated `ComputedStyle` fields
/// - `background-position`, `background-size`: stored in `BackgroundLayer`, not as `CssValue`
pub const ANIMATABLE_PROPERTIES: &[&str] = &[
    // Transforms
    "transform",
    "perspective",
    // Opacity & color
    "opacity",
    "color",
    "background-color",
    "background-position",
    "background-size",
    // Border colors
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    // Sizing
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    // Margins
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    // Padding
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    // Border widths
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    // Border radius
    "border-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    // Outline (color and width can be interpolated with current types)
    "outline-color",
    "outline-width",
    "outline-offset",
    // Typography
    "font-size",
    "font-weight",
    "font-style",
    "letter-spacing",
    "word-spacing",
    "line-height",
    "text-indent",
    "text-decoration-color",
    // Positioning
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    // Flex
    "flex-basis",
    "flex-grow",
    "flex-shrink",
    "order",
    // Grid / gap
    "row-gap",
    "column-gap",
    // Miscellaneous
    "list-style-type",
    "visibility",
    "vertical-align",
    "column-count",
    "column-width",
    "column-rule-color",
    "column-rule-width",
    "tab-size",
];

/// Returns `true` if the given CSS property name is animatable.
///
/// Based on CSS Transitions Level 1 §3 animatable properties list.
#[must_use]
pub fn is_animatable(property: &str) -> bool {
    ANIMATABLE_PROPERTIES.contains(&property)
}

// ---------------------------------------------------------------------------
// Transform list interpolation (CSS Transforms L1 §9)
// ---------------------------------------------------------------------------

/// Interpolate two transform function lists.
///
/// CSS Transforms L1 §9:
/// 1. If both lists have the same length and matching function types,
///    interpolate each function's numeric parameters individually.
/// 2. Otherwise, convert both to 2D matrices, decompose, interpolate
///    decomposed values, and recompose.
fn interpolate_transform_lists(
    from: &[TransformFunction],
    to: &[TransformFunction],
    t: f32,
) -> Option<CssValue> {
    // Try per-function interpolation if lists match
    if from.len() == to.len() && functions_match(from, to) {
        let mut result = Vec::with_capacity(from.len());
        for (a, b) in from.iter().zip(to.iter()) {
            result.push(interpolate_function(a, b, t));
        }
        return Some(CssValue::TransformList(result));
    }

    // Fallback: matrix decomposition interpolation
    let from_affine = functions_to_affine(from);
    let to_affine = functions_to_affine(to);
    let from_d = decompose_2d(from_affine)?;
    let to_d = decompose_2d(to_affine)?;
    let mid = interpolate_decomposed(&from_d, &to_d, f64::from(t));
    let m = recompose_2d(&mid);
    Some(CssValue::TransformList(vec![TransformFunction::Matrix(m)]))
}

/// Check if two transform lists have matching function types (same variant in same order).
fn functions_match(a: &[TransformFunction], b: &[TransformFunction]) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(fa, fb)| std::mem::discriminant(fa) == std::mem::discriminant(fb))
}

/// Convert a transform function list to a 2D affine matrix.
///
/// **Known limitation:** Percentage values in `translate()` / `translate3d()` are
/// evaluated against a reference size of 0 (treated as 0px) because the element's
/// layout box dimensions are not available at interpolation time. This only affects
/// the matrix decomposition fallback path (mismatched function lists). Per-function
/// interpolation of matching lists preserves percentage values correctly.
fn functions_to_affine(funcs: &[TransformFunction]) -> [f64; 6] {
    if funcs.is_empty() {
        return elidex_plugin::transform_math::IDENTITY;
    }
    let mut mat = IDENTITY_4X4;
    for f in funcs {
        let fm = elidex_plugin::transform_math::function_to_4x4_no_resolve(f);
        mat = elidex_plugin::transform_math::mul_4x4(&mat, &fm);
    }
    project_to_2d(&mat)
}

/// Interpolate a single pair of matching transform functions.
fn interpolate_function(a: &TransformFunction, b: &TransformFunction, t: f32) -> TransformFunction {
    match (a, b) {
        (TransformFunction::Translate(ax, ay), TransformFunction::Translate(bx, by)) => {
            TransformFunction::Translate(lerp_css_value(ax, bx, t), lerp_css_value(ay, by, t))
        }
        (TransformFunction::TranslateX(ax), TransformFunction::TranslateX(bx)) => {
            TransformFunction::TranslateX(lerp_css_value(ax, bx, t))
        }
        (TransformFunction::TranslateY(ay), TransformFunction::TranslateY(by)) => {
            TransformFunction::TranslateY(lerp_css_value(ay, by, t))
        }
        (TransformFunction::TranslateZ(az), TransformFunction::TranslateZ(bz)) => {
            TransformFunction::TranslateZ(lerp_css_value(az, bz, t))
        }
        (
            TransformFunction::Translate3d(ax, ay, az),
            TransformFunction::Translate3d(bx, by, bz),
        ) => TransformFunction::Translate3d(
            lerp_css_value(ax, bx, t),
            lerp_css_value(ay, by, t),
            lerp_css_value(az, bz, t),
        ),
        (TransformFunction::Rotate(ad), TransformFunction::Rotate(bd)) => {
            TransformFunction::Rotate(lerp(*ad, *bd, t))
        }
        (TransformFunction::RotateX(ad), TransformFunction::RotateX(bd)) => {
            TransformFunction::RotateX(lerp(*ad, *bd, t))
        }
        (TransformFunction::RotateY(ad), TransformFunction::RotateY(bd)) => {
            TransformFunction::RotateY(lerp(*ad, *bd, t))
        }
        (TransformFunction::RotateZ(ad), TransformFunction::RotateZ(bd)) => {
            TransformFunction::RotateZ(lerp(*ad, *bd, t))
        }
        (
            TransformFunction::Rotate3d(ax, ay, az, ad),
            TransformFunction::Rotate3d(bx, by, bz, bd),
        ) => {
            // CSS Transforms L2: rotate3d interpolation lerps axis and angle
            let lf = |a: f64, b: f64| a + (b - a) * f64::from(t);
            TransformFunction::Rotate3d(lf(*ax, *bx), lf(*ay, *by), lf(*az, *bz), lerp(*ad, *bd, t))
        }
        (TransformFunction::Scale(asx, asy), TransformFunction::Scale(bsx, bsy)) => {
            TransformFunction::Scale(lerp(*asx, *bsx, t), lerp(*asy, *bsy, t))
        }
        (TransformFunction::ScaleX(as_), TransformFunction::ScaleX(bs)) => {
            TransformFunction::ScaleX(lerp(*as_, *bs, t))
        }
        (TransformFunction::ScaleY(as_), TransformFunction::ScaleY(bs)) => {
            TransformFunction::ScaleY(lerp(*as_, *bs, t))
        }
        (TransformFunction::ScaleZ(as_), TransformFunction::ScaleZ(bs)) => {
            TransformFunction::ScaleZ(lerp(*as_, *bs, t))
        }
        (TransformFunction::Scale3d(asx, asy, asz), TransformFunction::Scale3d(bsx, bsy, bsz)) => {
            TransformFunction::Scale3d(
                lerp(*asx, *bsx, t),
                lerp(*asy, *bsy, t),
                lerp(*asz, *bsz, t),
            )
        }
        (TransformFunction::Skew(aax, aay), TransformFunction::Skew(bax, bay)) => {
            TransformFunction::Skew(lerp(*aax, *bax, t), lerp(*aay, *bay, t))
        }
        (TransformFunction::SkewX(aa), TransformFunction::SkewX(ba)) => {
            TransformFunction::SkewX(lerp(*aa, *ba, t))
        }
        (TransformFunction::SkewY(aa), TransformFunction::SkewY(ba)) => {
            TransformFunction::SkewY(lerp(*aa, *ba, t))
        }
        (TransformFunction::Matrix(am), TransformFunction::Matrix(bm)) => {
            let mut m = [0.0; 6];
            for i in 0..6 {
                m[i] = am[i] + (bm[i] - am[i]) * f64::from(t);
            }
            TransformFunction::Matrix(m)
        }
        (TransformFunction::Matrix3d(am), TransformFunction::Matrix3d(bm)) => {
            let mut m = [0.0; 16];
            for i in 0..16 {
                m[i] = am[i] + (bm[i] - am[i]) * f64::from(t);
            }
            TransformFunction::Matrix3d(m)
        }
        (TransformFunction::PerspectiveFunc(ad), TransformFunction::PerspectiveFunc(bd)) => {
            TransformFunction::PerspectiveFunc(lerp(*ad, *bd, t))
        }
        // Should not happen (functions_match ensures same discriminant), but fallback
        _ => a.clone(),
    }
}

/// Lerp a `CssValue` that is a Length or Percentage (for translate args).
///
/// **Known limitation (M3):** Mismatched Length↔Percentage interpolation uses a
/// discrete flip at t=0.5 (each side lerps toward 0, then switches). The CSS spec
/// says this should produce `calc(lerp(length) + lerp(percentage))` intermediate
/// values, but `CssValue` does not support `calc()` expressions. This causes a
/// visual discontinuity at the midpoint.
fn lerp_css_value(a: &CssValue, b: &CssValue, t: f32) -> CssValue {
    match (a, b) {
        (CssValue::Length(av, au), CssValue::Length(bv, bu)) if au == bu => {
            CssValue::Length(lerp(*av, *bv, t), *au)
        }
        (CssValue::Percentage(av), CssValue::Percentage(bv)) => {
            CssValue::Percentage(lerp(*av, *bv, t))
        }
        // Mismatched units: discrete flip at 50% (see doc comment above).
        (CssValue::Length(av, au), CssValue::Percentage(bv)) => {
            if t < 0.5 {
                CssValue::Length(lerp(*av, 0.0, t), *au)
            } else {
                CssValue::Percentage(lerp(0.0, *bv, t))
            }
        }
        (CssValue::Percentage(av), CssValue::Length(bv, bu)) => {
            if t < 0.5 {
                CssValue::Percentage(lerp(*av, 0.0, t))
            } else {
                CssValue::Length(lerp(0.0, *bv, t), *bu)
            }
        }
        _ => a.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::LengthUnit;

    #[test]
    fn interpolate_numbers() {
        let from = CssValue::Number(0.0);
        let to = CssValue::Number(1.0);
        assert_eq!(
            interpolate(&from, &to, 0.5, ""),
            Some(CssValue::Number(0.5))
        );
    }

    #[test]
    fn interpolate_lengths_px() {
        let from = CssValue::Length(10.0, LengthUnit::Px);
        let to = CssValue::Length(20.0, LengthUnit::Px);
        assert_eq!(
            interpolate(&from, &to, 0.5, ""),
            Some(CssValue::Length(15.0, LengthUnit::Px))
        );
    }

    #[test]
    fn interpolate_lengths_em() {
        let from = CssValue::Length(1.0, LengthUnit::Em);
        let to = CssValue::Length(3.0, LengthUnit::Em);
        assert_eq!(
            interpolate(&from, &to, 0.25, ""),
            Some(CssValue::Length(1.5, LengthUnit::Em))
        );
    }

    #[test]
    fn interpolate_percentages() {
        let from = CssValue::Percentage(0.0);
        let to = CssValue::Percentage(100.0);
        assert_eq!(
            interpolate(&from, &to, 0.5, ""),
            Some(CssValue::Percentage(50.0))
        );
    }

    #[test]
    fn interpolate_colors() {
        let from = CssValue::Color(CssColor::BLACK);
        let to = CssValue::Color(CssColor::WHITE);
        let result = interpolate(&from, &to, 0.5, "");
        let Some(CssValue::Color(c)) = result else {
            panic!("expected Color, got {result:?}");
        };
        assert_eq!(c.r, 128);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 128);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn interpolate_colors_with_alpha() {
        // With premultiplied alpha: from is fully transparent red, to is opaque red.
        // At t=0.5: alpha=0.5, premul_r=lerp(0, 255, 0.5)=127.5, unpremul_r=127.5/0.5=255
        let from = CssColor::new(255, 0, 0, 0);
        let to = CssColor::new(255, 0, 0, 255);
        let result = interpolate_color(&from, &to, 0.5);
        assert_eq!(result.a, 128);
        assert_eq!(result.r, 255); // red stays 255 with premultiplied
    }

    #[test]
    fn interpolate_time() {
        let from = CssValue::Time(0.0);
        let to = CssValue::Time(1.0);
        assert_eq!(interpolate(&from, &to, 0.5, ""), Some(CssValue::Time(0.5)));
    }

    #[test]
    fn interpolate_keywords_discrete() {
        let from = CssValue::Keyword("block".into());
        let to = CssValue::Keyword("none".into());
        // Before 50%: from
        assert_eq!(
            interpolate(&from, &to, 0.3, ""),
            Some(CssValue::Keyword("block".into()))
        );
        // At/after 50%: to
        assert_eq!(
            interpolate(&from, &to, 0.5, ""),
            Some(CssValue::Keyword("none".into()))
        );
    }

    #[test]
    fn interpolate_auto_discrete() {
        let from = CssValue::Auto;
        let to = CssValue::Length(100.0, LengthUnit::Px);
        assert_eq!(interpolate(&from, &to, 0.3, ""), Some(CssValue::Auto));
        assert_eq!(
            interpolate(&from, &to, 0.7, ""),
            Some(CssValue::Length(100.0, LengthUnit::Px))
        );
    }

    #[test]
    fn is_animatable_properties() {
        assert!(is_animatable("opacity"));
        assert!(is_animatable("color"));
        assert!(is_animatable("width"));
        assert!(is_animatable("margin-top"));
        assert!(is_animatable("flex-basis"));
        assert!(is_animatable("font-style"));
        assert!(is_animatable("list-style-type"));
        assert!(is_animatable("top"));
        assert!(is_animatable("z-index"));
        assert!(is_animatable("border-top-left-radius"));
        // Newly added properties
        assert!(is_animatable("outline-color"));
        assert!(is_animatable("outline-width"));
        assert!(is_animatable("outline-offset"));
        assert!(is_animatable("text-indent"));
        assert!(is_animatable("column-count"));
        assert!(is_animatable("column-width"));
        assert!(is_animatable("column-rule-color"));
        assert!(is_animatable("column-rule-width"));
        assert!(is_animatable("tab-size"));
        // Non-animatable
        assert!(!is_animatable("display"));
        assert!(!is_animatable("position"));
        assert!(!is_animatable("text-align"));
    }

    #[test]
    fn is_animatable_matches_constant() {
        // Verify is_animatable() returns true for every entry in ANIMATABLE_PROPERTIES
        // and false for known non-animatable properties.
        for &prop in ANIMATABLE_PROPERTIES {
            assert!(is_animatable(prop), "{prop} should be animatable");
        }
        assert!(!is_animatable("display"));
        assert!(!is_animatable("position"));
        assert!(!is_animatable("text-align"));
        assert!(!is_animatable("border-spacing"));
        // Verify specific properties are animatable.
        assert!(is_animatable("top"));
        assert!(is_animatable("z-index"));
        assert!(is_animatable("border-top-left-radius"));
        assert!(is_animatable("outline-color"));
        assert!(is_animatable("column-rule-color"));
    }

    #[test]
    fn interpolate_transparent_colors() {
        // Both fully transparent → should produce transparent
        let from = CssColor::new(255, 0, 0, 0);
        let to = CssColor::new(0, 255, 0, 0);
        let result = interpolate_color(&from, &to, 0.5);
        assert_eq!(result.a, 0);
    }

    #[test]
    fn transform_is_animatable() {
        assert!(is_animatable("transform"));
        assert!(is_animatable("perspective"));
    }

    #[test]
    fn interpolate_matching_translate() {
        let from = CssValue::TransformList(vec![TransformFunction::Translate(
            CssValue::Length(0.0, LengthUnit::Px),
            CssValue::Length(0.0, LengthUnit::Px),
        )]);
        let to = CssValue::TransformList(vec![TransformFunction::Translate(
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(200.0, LengthUnit::Px),
        )]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &result {
            assert_eq!(funcs.len(), 1);
            if let TransformFunction::Translate(CssValue::Length(x, _), CssValue::Length(y, _)) =
                &funcs[0]
            {
                assert!((x - 50.0).abs() < 0.01, "x={x}");
                assert!((y - 100.0).abs() < 0.01, "y={y}");
            } else {
                panic!("expected Translate with lengths");
            }
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_matching_rotate() {
        let from = CssValue::TransformList(vec![TransformFunction::Rotate(0.0)]);
        let to = CssValue::TransformList(vec![TransformFunction::Rotate(90.0)]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &result {
            if let TransformFunction::Rotate(d) = &funcs[0] {
                assert!((*d - 45.0).abs() < 0.01, "d={d}");
            } else {
                panic!("expected Rotate");
            }
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_matching_scale() {
        let from = CssValue::TransformList(vec![TransformFunction::Scale(1.0, 1.0)]);
        let to = CssValue::TransformList(vec![TransformFunction::Scale(3.0, 5.0)]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &result {
            if let TransformFunction::Scale(sx, sy) = &funcs[0] {
                assert!((*sx - 2.0).abs() < 0.01);
                assert!((*sy - 3.0).abs() < 0.01);
            } else {
                panic!("expected Scale");
            }
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_matching_multi_function() {
        // translate(0) rotate(0) → translate(100px) rotate(90deg)
        let from = CssValue::TransformList(vec![
            TransformFunction::Translate(
                CssValue::Length(0.0, LengthUnit::Px),
                CssValue::Length(0.0, LengthUnit::Px),
            ),
            TransformFunction::Rotate(0.0),
        ]);
        let to = CssValue::TransformList(vec![
            TransformFunction::Translate(
                CssValue::Length(100.0, LengthUnit::Px),
                CssValue::Length(0.0, LengthUnit::Px),
            ),
            TransformFunction::Rotate(90.0),
        ]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &result {
            assert_eq!(funcs.len(), 2);
            // Both functions should be interpolated individually
            assert!(matches!(&funcs[0], TransformFunction::Translate(..)));
            assert!(matches!(&funcs[1], TransformFunction::Rotate(_)));
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_mismatched_uses_decomposition() {
        // translate(100px) vs rotate(90deg) → must use matrix decomposition
        let from = CssValue::TransformList(vec![TransformFunction::Translate(
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(0.0, LengthUnit::Px),
        )]);
        let to = CssValue::TransformList(vec![TransformFunction::Rotate(90.0)]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &result {
            assert_eq!(funcs.len(), 1);
            assert!(matches!(&funcs[0], TransformFunction::Matrix(_)));
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_none_to_transform() {
        // none → rotate(90deg)
        let from = CssValue::Keyword("none".into());
        let to = CssValue::TransformList(vec![TransformFunction::Rotate(90.0)]);
        let result = interpolate(&from, &to, 0.5, "transform").unwrap();
        // Uses decomposition since lists don't match (empty vs rotate)
        if let CssValue::TransformList(funcs) = &result {
            assert_eq!(funcs.len(), 1);
            assert!(matches!(&funcs[0], TransformFunction::Matrix(_)));
        } else {
            panic!("expected TransformList");
        }
    }

    #[test]
    fn interpolate_transform_at_endpoints() {
        let from = CssValue::TransformList(vec![TransformFunction::Scale(1.0, 1.0)]);
        let to = CssValue::TransformList(vec![TransformFunction::Scale(2.0, 2.0)]);
        // t=0 → from
        let r0 = interpolate(&from, &to, 0.0, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &r0 {
            if let TransformFunction::Scale(sx, _) = &funcs[0] {
                assert!((*sx - 1.0).abs() < 0.01);
            }
        }
        // t=1 → to
        let r1 = interpolate(&from, &to, 1.0, "transform").unwrap();
        if let CssValue::TransformList(funcs) = &r1 {
            if let TransformFunction::Scale(sx, _) = &funcs[0] {
                assert!((*sx - 2.0).abs() < 0.01);
            }
        }
    }
}
