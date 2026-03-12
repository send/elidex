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
    "font-style",
    "font-family",
    "line-height",
    "text-transform",
    "text-align",
    "white-space",
    "list-style-type",
    "border-collapse",
    "border-spacing-h",
    "border-spacing-v",
    "caption-side",
    "direction",
    "writing-mode",
    "text-orientation",
    "visibility",
    "letter-spacing",
    "word-spacing",
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

        // Inherited text / keyword "normal" initial values
        "font-weight" => CssValue::Number(400.0),
        "font-style" | "line-height" | "white-space" | "content" | "unicode-bidi"
        | "letter-spacing" | "word-spacing" => CssValue::Keyword("normal".to_string()),
        // Non-inherited text decoration style/color
        "text-decoration-style" => CssValue::Keyword("solid".to_string()),
        "text-decoration-color" => CssValue::Keyword("currentcolor".to_string()),

        // Keyword "none" initial values
        "text-transform"
        | "text-decoration-line"
        | "border-top-style"
        | "border-right-style"
        | "border-bottom-style"
        | "border-left-style"
        | "grid-template-columns"
        | "grid-template-rows"
        | "float"
        | "clear" => CssValue::Keyword("none".to_string()),
        "text-align" => CssValue::Keyword("start".to_string()),
        "list-style-type" => CssValue::Keyword("disc".to_string()),

        // Writing mode / BiDi
        "direction" => CssValue::Keyword("ltr".to_string()),
        "writing-mode" => CssValue::Keyword("horizontal-tb".to_string()),
        "text-orientation" => CssValue::Keyword("mixed".to_string()),

        // Vertical-align (non-inherited)
        "vertical-align" => CssValue::Keyword("baseline".to_string()),

        // Display / position
        "display" => CssValue::Keyword("inline".to_string()),
        "position" => CssValue::Keyword("static".to_string()),

        // Background
        "background-color" => CssValue::Color(CssColor::TRANSPARENT),

        // Visibility (inherited) / overflow
        "visibility" | "overflow" => CssValue::Keyword("visible".to_string()),

        // Sizing (auto)
        "width" | "height" | "flex-basis" | "max-width" | "max-height" | "grid-auto-columns"
        | "grid-auto-rows" | "grid-column-start" | "grid-column-end" | "grid-row-start"
        | "grid-row-end" => CssValue::Auto,

        // Margins, padding, min-width/min-height, border-radius, gap (all initial = 0px)
        "min-width" | "min-height" | "margin-top" | "margin-right" | "margin-bottom"
        | "margin-left" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left"
        | "border-radius" | "row-gap" | "column-gap" | "border-spacing" | "border-spacing-h"
        | "border-spacing-v" => CssValue::Length(0.0, LengthUnit::Px),

        // Border width (CSS initial = medium = 3px)
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            CssValue::Length(3.0, LengthUnit::Px)
        }

        // Border color (currentcolor)
        "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
            CssValue::Keyword("currentcolor".to_string())
        }

        // Table
        "border-collapse" => CssValue::Keyword("separate".to_string()),
        "table-layout" | "align-self" => CssValue::Keyword("auto".to_string()),
        "caption-side" => CssValue::Keyword("top".to_string()),

        // Box model
        "box-sizing" => CssValue::Keyword("content-box".to_string()),

        // Flex/Grid container
        "flex-direction" | "grid-auto-flow" => CssValue::Keyword("row".to_string()),
        "flex-wrap" => CssValue::Keyword("nowrap".to_string()),
        "justify-content" => CssValue::Keyword("flex-start".to_string()),
        "align-items" | "align-content" => CssValue::Keyword("stretch".to_string()),
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

    // L4: M3-5 gap and text-align properties
    #[test]
    fn gap_properties_not_inherited() {
        assert!(!is_inherited("row-gap"));
        assert!(!is_inherited("column-gap"));
    }

    #[test]
    fn text_align_inherited() {
        assert!(is_inherited("text-align"));
    }

    #[test]
    fn initial_values_m3_5() {
        assert_eq!(
            get_initial_value("row-gap"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("column-gap"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("text-align"),
            CssValue::Keyword("start".to_string())
        );
    }

    // --- M3-6: white-space, overflow, list-style-type, min/max ---

    #[test]
    fn white_space_inherited() {
        assert!(is_inherited("white-space"));
    }

    #[test]
    fn list_style_type_inherited() {
        assert!(is_inherited("list-style-type"));
    }

    #[test]
    fn m3_6_non_inherited() {
        assert!(!is_inherited("overflow"));
        assert!(!is_inherited("min-width"));
        assert!(!is_inherited("max-width"));
        assert!(!is_inherited("min-height"));
        assert!(!is_inherited("max-height"));
    }

    #[test]
    fn initial_values_m3_6() {
        assert_eq!(
            get_initial_value("white-space"),
            CssValue::Keyword("normal".to_string())
        );
        assert_eq!(
            get_initial_value("overflow"),
            CssValue::Keyword("visible".to_string())
        );
        assert_eq!(
            get_initial_value("list-style-type"),
            CssValue::Keyword("disc".to_string())
        );
        assert_eq!(
            get_initial_value("min-width"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(get_initial_value("max-width"), CssValue::Auto);
        assert_eq!(
            get_initial_value("min-height"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(get_initial_value("max-height"), CssValue::Auto);
    }

    // --- M3.5-1: Grid property inheritance ---

    #[test]
    fn grid_properties_not_inherited() {
        assert!(!is_inherited("grid-template-columns"));
        assert!(!is_inherited("grid-template-rows"));
        assert!(!is_inherited("grid-auto-flow"));
        assert!(!is_inherited("grid-auto-columns"));
        assert!(!is_inherited("grid-auto-rows"));
        assert!(!is_inherited("grid-column-start"));
        assert!(!is_inherited("grid-column-end"));
        assert!(!is_inherited("grid-row-start"));
        assert!(!is_inherited("grid-row-end"));
    }

    // --- M3.5-2: Table property inheritance ---

    #[test]
    fn border_collapse_inherited() {
        assert!(is_inherited("border-collapse"));
    }

    #[test]
    fn border_spacing_inherited() {
        assert!(is_inherited("border-spacing-h"));
        assert!(is_inherited("border-spacing-v"));
    }

    #[test]
    fn caption_side_inherited() {
        assert!(is_inherited("caption-side"));
    }

    #[test]
    fn table_layout_not_inherited() {
        assert!(!is_inherited("table-layout"));
    }

    #[test]
    fn initial_values_table() {
        assert_eq!(
            get_initial_value("border-collapse"),
            CssValue::Keyword("separate".to_string())
        );
        assert_eq!(
            get_initial_value("border-spacing"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("border-spacing-h"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("border-spacing-v"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            get_initial_value("table-layout"),
            CssValue::Keyword("auto".to_string())
        );
        assert_eq!(
            get_initial_value("caption-side"),
            CssValue::Keyword("top".to_string())
        );
    }

    // --- M4-0: float/clear/visibility/vertical-align ---

    #[test]
    fn visibility_inherited() {
        assert!(is_inherited("visibility"));
    }

    #[test]
    fn float_clear_not_inherited() {
        assert!(!is_inherited("float"));
        assert!(!is_inherited("clear"));
        assert!(!is_inherited("vertical-align"));
    }

    #[test]
    fn initial_values_m4_0() {
        assert_eq!(
            get_initial_value("visibility"),
            CssValue::Keyword("visible".to_string())
        );
        assert_eq!(
            get_initial_value("float"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            get_initial_value("clear"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            get_initial_value("vertical-align"),
            CssValue::Keyword("baseline".to_string())
        );
    }

    // --- M3.5-4: Writing mode / BiDi property inheritance ---

    #[test]
    fn direction_inherited() {
        assert!(is_inherited("direction"));
    }

    #[test]
    fn writing_mode_inherited() {
        assert!(is_inherited("writing-mode"));
    }

    #[test]
    fn text_orientation_inherited() {
        assert!(is_inherited("text-orientation"));
    }

    #[test]
    fn unicode_bidi_not_inherited() {
        assert!(!is_inherited("unicode-bidi"));
    }

    #[test]
    fn initial_values_i18n() {
        assert_eq!(
            get_initial_value("direction"),
            CssValue::Keyword("ltr".to_string())
        );
        assert_eq!(
            get_initial_value("unicode-bidi"),
            CssValue::Keyword("normal".to_string())
        );
        assert_eq!(
            get_initial_value("writing-mode"),
            CssValue::Keyword("horizontal-tb".to_string())
        );
        assert_eq!(
            get_initial_value("text-orientation"),
            CssValue::Keyword("mixed".to_string())
        );
    }

    #[test]
    fn initial_values_grid() {
        assert_eq!(
            get_initial_value("grid-template-columns"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            get_initial_value("grid-template-rows"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            get_initial_value("grid-auto-flow"),
            CssValue::Keyword("row".to_string())
        );
        assert_eq!(get_initial_value("grid-auto-columns"), CssValue::Auto);
        assert_eq!(get_initial_value("grid-auto-rows"), CssValue::Auto);
        assert_eq!(get_initial_value("grid-column-start"), CssValue::Auto);
        assert_eq!(get_initial_value("grid-column-end"), CssValue::Auto);
        assert_eq!(get_initial_value("grid-row-start"), CssValue::Auto);
        assert_eq!(get_initial_value("grid-row-end"), CssValue::Auto);
    }
}
