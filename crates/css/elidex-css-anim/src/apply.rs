//! Apply animated CSS values to `ComputedStyle` fields.
//!
//! Maps property names to the corresponding `ComputedStyle` fields,
//! applying interpolated values from active transitions and animations.

use elidex_plugin::{
    ComputedStyle, CssColor, CssValue, Dimension, LengthUnit, LineHeight, VerticalAlign, Visibility,
};

/// Apply an animated value to the corresponding `ComputedStyle` field.
///
/// Only handles animatable properties (those listed in [`crate::interpolate::is_animatable`]).
/// Unknown or non-animatable properties are silently ignored.
#[allow(clippy::too_many_lines)]
pub fn apply_animated_value(style: &mut ComputedStyle, property: &str, value: &CssValue) {
    match property {
        "opacity" => {
            if let CssValue::Number(n) = value {
                style.opacity = n.clamp(0.0, 1.0);
            }
        }
        "color" => {
            if let CssValue::Color(c) = value {
                style.color = *c;
            }
        }
        "background-color" => {
            if let CssValue::Color(c) = value {
                style.background_color = *c;
            }
        }
        "border-top-color" => apply_color(&mut style.border_top.color, value),
        "border-right-color" => apply_color(&mut style.border_right.color, value),
        "border-bottom-color" => apply_color(&mut style.border_bottom.color, value),
        "border-left-color" => apply_color(&mut style.border_left.color, value),
        "width" => apply_dimension(&mut style.width, value),
        "height" => apply_dimension(&mut style.height, value),
        "min-width" => apply_dimension(&mut style.min_width, value),
        "min-height" => apply_dimension(&mut style.min_height, value),
        "max-width" => apply_dimension(&mut style.max_width, value),
        "max-height" => apply_dimension(&mut style.max_height, value),
        "margin-top" => apply_dimension(&mut style.margin_top, value),
        "margin-right" => apply_dimension(&mut style.margin_right, value),
        "margin-bottom" => apply_dimension(&mut style.margin_bottom, value),
        "margin-left" => apply_dimension(&mut style.margin_left, value),
        "padding-top" => apply_px(&mut style.padding.top, value),
        "padding-right" => apply_px(&mut style.padding.right, value),
        "padding-bottom" => apply_px(&mut style.padding.bottom, value),
        "padding-left" => apply_px(&mut style.padding.left, value),
        "border-top-width" => apply_px(&mut style.border_top.width, value),
        "border-right-width" => apply_px(&mut style.border_right.width, value),
        "border-bottom-width" => apply_px(&mut style.border_bottom.width, value),
        "border-left-width" => apply_px(&mut style.border_left.width, value),
        "font-size" => {
            if let CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) = value {
                style.font_size = v.max(0.0);
            }
        }
        "font-weight" => {
            if let CssValue::Number(v) = value {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    style.font_weight = (*v as u16).clamp(1, 1000);
                }
            }
        }
        "letter-spacing" => {
            if let CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) = value {
                style.letter_spacing = Some(*v);
            }
        }
        "word-spacing" => {
            if let CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) = value {
                style.word_spacing = Some(*v);
            }
        }
        "line-height" => {
            if let CssValue::Length(v, LengthUnit::Px) = value {
                style.line_height = LineHeight::Px(*v);
            } else if let CssValue::Number(v) = value {
                style.line_height = LineHeight::Number(*v);
            }
        }
        "border-radius" => {
            if let CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) = value {
                style.border_radius = v.max(0.0);
            }
        }
        "row-gap" => apply_px(&mut style.row_gap, value),
        "column-gap" => apply_px(&mut style.column_gap, value),
        "flex-grow" => {
            if let CssValue::Number(v) = value {
                style.flex_grow = v.max(0.0);
            }
        }
        "flex-shrink" => {
            if let CssValue::Number(v) = value {
                style.flex_shrink = v.max(0.0);
            }
        }
        "order" => {
            if let CssValue::Number(v) = value {
                #[allow(clippy::cast_possible_truncation)]
                {
                    style.order = *v as i32;
                }
            }
        }
        // top/right/bottom/left position offsets and z-index are not currently
        // stored in ComputedStyle (they are resolved during layout). Animation
        // of these properties is deferred to a future phase.
        "visibility" => {
            if let CssValue::Keyword(kw) = value {
                if let Some(v) = Visibility::from_keyword(kw) {
                    style.visibility = v;
                }
            }
        }
        "text-decoration-color" => {
            if let CssValue::Color(c) = value {
                style.text_decoration_color = Some(*c);
            }
        }
        "vertical-align" => {
            if let CssValue::Length(v, LengthUnit::Px) = value {
                style.vertical_align = VerticalAlign::Length(*v);
            } else if let CssValue::Percentage(p) = value {
                style.vertical_align = VerticalAlign::Percentage(*p);
            }
        }
        _ => {}
    }
}

fn apply_color(field: &mut CssColor, value: &CssValue) {
    if let CssValue::Color(c) = value {
        *field = *c;
    }
}

fn apply_dimension(field: &mut Dimension, value: &CssValue) {
    match value {
        CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) => {
            *field = Dimension::Length(*v);
        }
        CssValue::Percentage(p) => {
            *field = Dimension::Percentage(*p);
        }
        CssValue::Auto => {
            *field = Dimension::Auto;
        }
        _ => {}
    }
}

fn apply_px(field: &mut f32, value: &CssValue) {
    if let CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) = value {
        *field = v.max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_opacity() {
        let mut style = ComputedStyle::default();
        apply_animated_value(&mut style, "opacity", &CssValue::Number(0.5));
        assert!((style.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_opacity_clamp() {
        let mut style = ComputedStyle::default();
        apply_animated_value(&mut style, "opacity", &CssValue::Number(1.5));
        assert!((style.opacity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_color_value() {
        let mut style = ComputedStyle::default();
        apply_animated_value(&mut style, "color", &CssValue::Color(CssColor::RED));
        assert_eq!(style.color, CssColor::RED);
    }

    #[test]
    fn apply_width() {
        let mut style = ComputedStyle::default();
        apply_animated_value(
            &mut style,
            "width",
            &CssValue::Length(200.0, LengthUnit::Px),
        );
        assert_eq!(style.width, Dimension::Length(200.0));
    }

    #[test]
    fn apply_margin() {
        let mut style = ComputedStyle::default();
        apply_animated_value(
            &mut style,
            "margin-top",
            &CssValue::Length(10.0, LengthUnit::Px),
        );
        assert_eq!(style.margin_top, Dimension::Length(10.0));
    }

    #[test]
    fn apply_font_size() {
        let mut style = ComputedStyle::default();
        apply_animated_value(
            &mut style,
            "font-size",
            &CssValue::Length(24.0, LengthUnit::Px),
        );
        assert!((style.font_size - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_visibility() {
        let mut style = ComputedStyle::default();
        apply_animated_value(
            &mut style,
            "visibility",
            &CssValue::Keyword("hidden".into()),
        );
        assert_eq!(style.visibility, Visibility::Hidden);
    }

    #[test]
    fn apply_unknown_property_ignored() {
        let mut style = ComputedStyle::default();
        let before = style.clone();
        apply_animated_value(&mut style, "display", &CssValue::Keyword("flex".into()));
        assert_eq!(style, before);
    }

    #[test]
    fn apply_flex_grow() {
        let mut style = ComputedStyle::default();
        apply_animated_value(&mut style, "flex-grow", &CssValue::Number(2.0));
        assert!((style.flex_grow - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_border_radius() {
        let mut style = ComputedStyle::default();
        apply_animated_value(
            &mut style,
            "border-radius",
            &CssValue::Length(8.0, LengthUnit::Px),
        );
        assert!((style.border_radius - 8.0).abs() < f32::EPSILON);
    }
}
