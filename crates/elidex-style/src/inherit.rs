//! CSS property inheritance metadata and initial values.
//!
//! Provides lookup functions for determining whether a property is inherited
//! and for retrieving the CSS initial value of any known property.

use elidex_plugin::{CssColor, CssValue, LengthUnit};

/// Inherited properties.
const INHERITED_PROPERTIES: &[&str] = &[
    "color",
    "font-size",
    "font-weight",
    "font-family",
    "line-height",
    "text-transform",
];

/// Returns `true` if the named CSS property is inherited.
///
/// Custom properties (`--*`) are always inherited per CSS Variables Level 1.
/// Unknown properties are treated as non-inherited.
pub(crate) fn is_inherited(property: &str) -> bool {
    property.starts_with("--") || INHERITED_PROPERTIES.contains(&property)
}

/// Returns the CSS initial value for a known property.
///
/// Unknown properties return `CssValue::Initial` as a fallback.
// Sync: When adding a property, also update build_computed_style and get_computed_as_css_value.
pub(crate) fn get_initial_value(property: &str) -> CssValue {
    match property {
        // Inherited
        "color" => CssValue::Color(CssColor::BLACK),
        "font-size" => CssValue::Length(16.0, LengthUnit::Px),
        "font-family" => CssValue::List(vec![CssValue::Keyword("serif".to_string())]),

        // Inherited text
        "font-weight" => CssValue::Number(400.0),
        "line-height" => CssValue::Keyword("normal".to_string()),
        "text-transform" | "text-decoration-line" => CssValue::Keyword("none".to_string()),

        // Display / position
        "display" => CssValue::Keyword("inline".to_string()),
        "position" => CssValue::Keyword("static".to_string()),

        // Background
        "background-color" => CssValue::Color(CssColor::TRANSPARENT),

        // Sizing (auto)
        "width" | "height" | "flex-basis" => CssValue::Auto,

        // Margins and padding
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "padding-top"
        | "padding-right" | "padding-bottom" | "padding-left" => {
            CssValue::Length(0.0, LengthUnit::Px)
        }

        // Border width (CSS initial = medium = 3px)
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            CssValue::Length(3.0, LengthUnit::Px)
        }

        // Border style
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            CssValue::Keyword("none".to_string())
        }

        // Border color (currentcolor)
        "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
            CssValue::Keyword("currentcolor".to_string())
        }

        // Box model
        "box-sizing" => CssValue::Keyword("content-box".to_string()),
        "border-radius" => CssValue::Length(0.0, LengthUnit::Px),

        // Flex container
        "flex-direction" => CssValue::Keyword("row".to_string()),
        "flex-wrap" => CssValue::Keyword("nowrap".to_string()),
        "justify-content" => CssValue::Keyword("flex-start".to_string()),
        "align-items" | "align-content" => CssValue::Keyword("stretch".to_string()),

        // Flex item
        "align-self" => CssValue::Keyword("auto".to_string()),
        "flex-grow" | "order" => CssValue::Number(0.0),
        "flex-shrink" | "opacity" => CssValue::Number(1.0),

        _ => CssValue::Initial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_properties() {
        assert!(is_inherited("color"));
        assert!(is_inherited("font-size"));
        assert!(is_inherited("font-family"));
    }

    #[test]
    fn non_inherited_properties() {
        assert!(!is_inherited("display"));
        assert!(!is_inherited("margin-top"));
        assert!(!is_inherited("background-color"));
        assert!(!is_inherited("border-top-width"));
    }

    #[test]
    fn unknown_property_not_inherited() {
        assert!(!is_inherited("unknown-property"));
    }

    #[test]
    fn initial_values() {
        assert_eq!(
            get_initial_value("display"),
            CssValue::Keyword("inline".to_string())
        );
        assert_eq!(get_initial_value("color"), CssValue::Color(CssColor::BLACK));
        assert_eq!(get_initial_value("width"), CssValue::Auto);
        assert_eq!(
            get_initial_value("margin-top"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("border-top-width"),
            CssValue::Length(3.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("border-top-color"),
            CssValue::Keyword("currentcolor".to_string())
        );
        assert_eq!(get_initial_value("unknown"), CssValue::Initial);
    }

    // --- Custom property inheritance (M3-0) ---

    #[test]
    fn custom_property_is_inherited() {
        assert!(is_inherited("--bg"));
        assert!(is_inherited("--text-color"));
        assert!(is_inherited("--anything"));
    }

    #[test]
    fn non_custom_property_inheritance_unchanged() {
        // Ensure regular non-inherited properties remain non-inherited.
        assert!(!is_inherited("display"));
        assert!(!is_inherited("background-color"));
    }

    // --- M3-1 text property inheritance ---

    #[test]
    fn font_weight_inherited() {
        assert!(is_inherited("font-weight"));
    }

    #[test]
    fn line_height_inherited() {
        assert!(is_inherited("line-height"));
    }

    #[test]
    fn text_transform_inherited() {
        assert!(is_inherited("text-transform"));
    }

    #[test]
    fn text_decoration_line_not_inherited() {
        assert!(!is_inherited("text-decoration-line"));
    }

    #[test]
    fn initial_values_m3_1() {
        assert_eq!(get_initial_value("font-weight"), CssValue::Number(400.0));
        assert_eq!(
            get_initial_value("line-height"),
            CssValue::Keyword("normal".to_string())
        );
        assert_eq!(
            get_initial_value("text-transform"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            get_initial_value("text-decoration-line"),
            CssValue::Keyword("none".to_string())
        );
    }

    // --- M3-2: Box model initial values ---

    #[test]
    fn box_model_properties_not_inherited() {
        assert!(!is_inherited("box-sizing"));
        assert!(!is_inherited("border-radius"));
        assert!(!is_inherited("opacity"));
    }

    #[test]
    fn initial_values_m3_2() {
        assert_eq!(
            get_initial_value("box-sizing"),
            CssValue::Keyword("content-box".to_string())
        );
        assert_eq!(
            get_initial_value("border-radius"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(get_initial_value("opacity"), CssValue::Number(1.0));
    }
}
