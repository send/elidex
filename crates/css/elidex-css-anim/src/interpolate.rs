//! CSS value interpolation for animations and transitions.
//!
//! Implements interpolation between CSS computed values for animatable
//! properties (CSS Transitions Level 1 §4).

use elidex_plugin::{CssColor, CssValue};

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
/// Properties requiring new `ComputedStyle` fields not yet implemented:
/// transform, filter, clip-path, background-position, background-size,
/// perspective, rotate, scale, translate.
pub const ANIMATABLE_PROPERTIES: &[&str] = &[
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
}
