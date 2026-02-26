//! CSS property inheritance metadata and initial values.
//!
//! Provides lookup functions for determining whether a property is inherited
//! and for retrieving the CSS initial value of any known property.

use elidex_plugin::{CssColor, CssValue, LengthUnit};

/// Inherited properties (Phase 1).
const INHERITED_PROPERTIES: &[&str] = &["color", "font-size", "font-family"];

/// Returns `true` if the named CSS property is inherited.
///
/// Unknown properties are treated as non-inherited.
pub fn is_inherited(property: &str) -> bool {
    INHERITED_PROPERTIES.contains(&property)
}

/// Returns the CSS initial value for a known property.
///
/// Unknown properties return `CssValue::Initial` as a fallback.
pub fn get_initial_value(property: &str) -> CssValue {
    match property {
        // Inherited
        "color" => CssValue::Color(CssColor::BLACK),
        "font-size" => CssValue::Length(16.0, LengthUnit::Px),
        "font-family" => CssValue::List(vec![CssValue::Keyword("serif".to_string())]),

        // Display / position
        "display" => CssValue::Keyword("inline".to_string()),
        "position" => CssValue::Keyword("static".to_string()),

        // Background
        "background-color" => CssValue::Color(CssColor::TRANSPARENT),

        // Sizing
        "width" | "height" => CssValue::Auto,

        // Margins and padding
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "padding-top"
        | "padding-right" | "padding-bottom" | "padding-left" => {
            CssValue::Length(0.0, LengthUnit::Px)
        }

        // Border width (CSS initial = medium = 3px)
        "border-top-width" | "border-right-width" | "border-bottom-width"
        | "border-left-width" => CssValue::Length(3.0, LengthUnit::Px),

        // Border style
        "border-top-style" | "border-right-style" | "border-bottom-style"
        | "border-left-style" => CssValue::Keyword("none".to_string()),

        // Border color (currentcolor)
        "border-top-color" | "border-right-color" | "border-bottom-color"
        | "border-left-color" => CssValue::Keyword("currentcolor".to_string()),

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
        assert_eq!(
            get_initial_value("color"),
            CssValue::Color(CssColor::BLACK)
        );
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
}
