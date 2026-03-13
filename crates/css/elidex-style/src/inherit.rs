//! CSS property inheritance metadata and initial values.
//!
//! Provides lookup functions for determining whether a property is inherited
//! and for retrieving the CSS initial value of any known property.

use elidex_plugin::CssValue;

/// Returns `true` if the named CSS property is inherited.
///
/// Delegates to the default CSS property registry so that inheritance metadata
/// has a single source of truth (`CssPropertyHandler::is_inherited()`).
///
/// Custom properties (`--*`) are always inherited per CSS Variables Level 1.
/// Unknown properties are treated as non-inherited.
pub(crate) fn is_inherited(property: &str) -> bool {
    if property.starts_with("--") {
        return true;
    }
    let registry = crate::default_css_property_registry();
    if let Some(handler) = registry.resolve(property) {
        return handler.is_inherited(property);
    }
    false
}

/// Returns the CSS initial value for a known property.
///
/// Delegates to the default CSS property registry so that initial values
/// have a single source of truth (`CssPropertyHandler::initial_value()`).
/// Unknown properties return `CssValue::Initial` as a fallback.
pub(crate) fn get_initial_value(property: &str) -> CssValue {
    let registry = crate::default_css_property_registry();
    registry
        .resolve(property)
        .map_or(CssValue::Initial, |h| h.initial_value(property))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, LengthUnit};

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
        // Unknown properties (not in any handler) return CssValue::Initial.
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
        // min-width/min-height: Length(0) matching ComputedStyle default (Dimension::ZERO).
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
        // border-spacing is a shorthand (not in any handler); longhands are
        // border-spacing-h and border-spacing-v.
        assert_eq!(get_initial_value("border-spacing"), CssValue::Initial);
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
