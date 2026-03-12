//! CSS value interpolation for animations and transitions.
//!
//! Implements interpolation between CSS computed values for animatable
//! properties (CSS Transitions Level 1 §4).

use elidex_plugin::{CssColor, CssValue, LengthUnit};

/// Interpolate between two CSS values at progress `t` (0.0..=1.0).
///
/// Returns `None` if the values cannot be interpolated (discrete properties).
#[must_use]
pub fn interpolate(from: &CssValue, to: &CssValue, t: f32) -> Option<CssValue> {
    match (from, to) {
        // Number ↔ Number
        (CssValue::Number(a), CssValue::Number(b)) => Some(CssValue::Number(lerp(*a, *b, t))),

        // Length ↔ Length (same unit or both px)
        (CssValue::Length(a, ua), CssValue::Length(b, ub)) if ua == ub => {
            Some(CssValue::Length(lerp(*a, *b, t), *ua))
        }
        // Both lengths resolved to px
        (CssValue::Length(a, LengthUnit::Px), CssValue::Length(b, LengthUnit::Px)) => {
            Some(CssValue::Length(lerp(*a, *b, t), LengthUnit::Px))
        }

        // Percentage ↔ Percentage
        (CssValue::Percentage(a), CssValue::Percentage(b)) => {
            Some(CssValue::Percentage(lerp(*a, *b, t)))
        }

        // Color ↔ Color (RGBA interpolation)
        (CssValue::Color(a), CssValue::Color(b)) => Some(CssValue::Color(interpolate_color(a, b, t))),

        // Time ↔ Time
        (CssValue::Time(a), CssValue::Time(b)) => Some(CssValue::Time(lerp(*a, *b, t))),

        // Discrete: keyword, string, auto, etc. — flip at 50%
        _ => {
            if t < 0.5 {
                Some(from.clone())
            } else {
                Some(to.clone())
            }
        }
    }
}

/// Linear interpolation.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Interpolate two RGBA colors component-wise.
///
/// Uses linear RGB interpolation (not premultiplied alpha for simplicity).
#[must_use]
pub fn interpolate_color(from: &CssColor, to: &CssColor, t: f32) -> CssColor {
    CssColor::new(
        lerp_u8(from.r, to.r, t),
        lerp_u8(from.g, to.g, t),
        lerp_u8(from.b, to.b, t),
        lerp_u8(from.a, to.a, t),
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let result = f32::from(a) + (f32::from(b) - f32::from(a)) * t;
    result.round().clamp(0.0, 255.0) as u8
}

/// Returns `true` if the given CSS property name is animatable.
///
/// Based on CSS Transitions Level 1 §3 animatable properties list.
#[must_use]
pub fn is_animatable(property: &str) -> bool {
    matches!(
        property,
        "opacity"
            | "color"
            | "background-color"
            | "border-top-color"
            | "border-right-color"
            | "border-bottom-color"
            | "border-left-color"
            | "width"
            | "height"
            | "min-width"
            | "min-height"
            | "max-width"
            | "max-height"
            | "margin-top"
            | "margin-right"
            | "margin-bottom"
            | "margin-left"
            | "padding-top"
            | "padding-right"
            | "padding-bottom"
            | "padding-left"
            | "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width"
            | "font-size"
            | "font-weight"
            | "letter-spacing"
            | "word-spacing"
            | "line-height"
            | "border-radius"
            | "row-gap"
            | "column-gap"
            | "border-spacing"
            | "flex-grow"
            | "flex-shrink"
            | "order"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_numbers() {
        let from = CssValue::Number(0.0);
        let to = CssValue::Number(1.0);
        assert_eq!(interpolate(&from, &to, 0.5), Some(CssValue::Number(0.5)));
    }

    #[test]
    fn interpolate_lengths_px() {
        let from = CssValue::Length(10.0, LengthUnit::Px);
        let to = CssValue::Length(20.0, LengthUnit::Px);
        assert_eq!(
            interpolate(&from, &to, 0.5),
            Some(CssValue::Length(15.0, LengthUnit::Px))
        );
    }

    #[test]
    fn interpolate_lengths_em() {
        let from = CssValue::Length(1.0, LengthUnit::Em);
        let to = CssValue::Length(3.0, LengthUnit::Em);
        assert_eq!(
            interpolate(&from, &to, 0.25),
            Some(CssValue::Length(1.5, LengthUnit::Em))
        );
    }

    #[test]
    fn interpolate_percentages() {
        let from = CssValue::Percentage(0.0);
        let to = CssValue::Percentage(100.0);
        assert_eq!(
            interpolate(&from, &to, 0.5),
            Some(CssValue::Percentage(50.0))
        );
    }

    #[test]
    fn interpolate_colors() {
        let from = CssValue::Color(CssColor::BLACK);
        let to = CssValue::Color(CssColor::WHITE);
        let result = interpolate(&from, &to, 0.5);
        if let Some(CssValue::Color(c)) = result {
            assert_eq!(c.r, 128);
            assert_eq!(c.g, 128);
            assert_eq!(c.b, 128);
            assert_eq!(c.a, 255);
        } else {
            panic!("expected Color");
        }
    }

    #[test]
    fn interpolate_colors_with_alpha() {
        let from = CssColor::new(255, 0, 0, 0);
        let to = CssColor::new(255, 0, 0, 255);
        let result = interpolate_color(&from, &to, 0.5);
        assert_eq!(result.a, 128);
    }

    #[test]
    fn interpolate_time() {
        let from = CssValue::Time(0.0);
        let to = CssValue::Time(1.0);
        assert_eq!(interpolate(&from, &to, 0.5), Some(CssValue::Time(0.5)));
    }

    #[test]
    fn interpolate_keywords_discrete() {
        let from = CssValue::Keyword("block".into());
        let to = CssValue::Keyword("none".into());
        // Before 50%: from
        assert_eq!(
            interpolate(&from, &to, 0.3),
            Some(CssValue::Keyword("block".into()))
        );
        // At/after 50%: to
        assert_eq!(
            interpolate(&from, &to, 0.5),
            Some(CssValue::Keyword("none".into()))
        );
    }

    #[test]
    fn interpolate_auto_discrete() {
        let from = CssValue::Auto;
        let to = CssValue::Length(100.0, LengthUnit::Px);
        assert_eq!(interpolate(&from, &to, 0.3), Some(CssValue::Auto));
        assert_eq!(
            interpolate(&from, &to, 0.7),
            Some(CssValue::Length(100.0, LengthUnit::Px))
        );
    }

    #[test]
    fn is_animatable_properties() {
        assert!(is_animatable("opacity"));
        assert!(is_animatable("color"));
        assert!(is_animatable("width"));
        assert!(is_animatable("margin-top"));
        assert!(!is_animatable("display"));
        assert!(!is_animatable("position"));
        assert!(!is_animatable("text-align"));
    }

    #[test]
    fn lerp_u8_boundary() {
        assert_eq!(lerp_u8(0, 255, 0.0), 0);
        assert_eq!(lerp_u8(0, 255, 1.0), 255);
        assert_eq!(lerp_u8(0, 255, 0.5), 128);
    }
}
