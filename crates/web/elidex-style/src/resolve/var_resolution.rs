//! CSS custom property and `var()` resolution.

use std::collections::{HashMap, HashSet};

use elidex_plugin::{ComputedStyle, CssValue};

use super::helpers::PropertyMap;

/// Maximum recursion depth for resolving `var()` references (cycle protection).
const MAX_VAR_DEPTH: usize = 32;

/// Build the custom properties map: inherit all from parent, then override
/// with any custom properties declared on this element.
///
/// Per CSS Variables Level 1:
/// - `RawTokens`: set the property to the raw value.
/// - `Initial`: remove the property (custom properties have no initial value;
///   their initial value is the "guaranteed-invalid value").
/// - `Inherit`/`Unset`: keep the inherited value (custom properties are
///   always inherited, so both behave as `inherit` — already handled by the
///   parent clone).
pub(super) fn build_custom_properties(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) -> HashMap<String, String> {
    let mut props = parent_style.custom_properties.clone();
    for (name, value) in winners {
        if name.starts_with("--") {
            match value {
                CssValue::RawTokens(raw) => {
                    props.insert((*name).to_string(), raw.clone());
                }
                CssValue::Initial => {
                    props.remove(*name);
                }
                // Inherit/Unset: keep inherited value (already cloned from parent).
                _ => {}
            }
        }
    }
    props
}

/// Resolve all `CssValue::Var` references in the winners map.
///
/// Returns a map of property name → resolved `CssValue` for properties that
/// had `var()` references.
pub(super) fn resolve_var_references(
    winners: &PropertyMap<'_>,
    custom_props: &HashMap<String, String>,
) -> HashMap<String, CssValue> {
    let mut resolved = HashMap::new();
    for (name, value) in winners {
        if name.starts_with("--") {
            continue; // Custom properties themselves don't get var-resolved here.
        }
        if let CssValue::Var(..) = value {
            let mut visited = HashSet::new();
            if let Some(val) = resolve_var_value(value, custom_props, 0, &mut visited) {
                // Expand shorthand property names to their longhands so that
                // downstream resolvers (which only look up longhand keys) can
                // find the value.  When var() is the entire shorthand value,
                // parse_property_value stores it under the shorthand name
                // (e.g. "background") because var() is detected before the
                // shorthand match.  After resolution we re-key it here.
                expand_resolved_shorthand(&mut resolved, name, val);
            }
        }
    }
    resolved
}

/// Expand a resolved shorthand value into longhand entries.
///
/// For simple shorthands where the resolved value maps to a single longhand
/// (e.g. `background` → `background-color`), insert under the longhand key.
/// For per-side border shorthands (`border-top`, etc.), insert under the 3
/// longhand keys with appropriate defaults.
fn expand_resolved_shorthand(resolved: &mut HashMap<String, CssValue>, name: &str, val: CssValue) {
    match name {
        "background" => {
            resolved.insert("background-color".to_string(), val);
        }
        "border-spacing" => {
            // border-spacing shorthand → 2 longhands (same value for both).
            resolved.insert("border-spacing-h".to_string(), val.clone());
            resolved.insert("border-spacing-v".to_string(), val);
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            // The resolved value is a single color/keyword — treat it as the
            // most common case: `border-side: <width> <style> <color>` where
            // only one component was specified via var().
            // For a single resolved color, store it as the border-side-color.
            let side = &name["border-".len()..];
            match &val {
                CssValue::Color(_) => {
                    resolved.insert(format!("border-{side}-color"), val);
                }
                CssValue::Keyword(k) => {
                    // Could be a style keyword (solid, dashed, none, etc.)
                    let style_keywords = [
                        "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge",
                        "inset", "outset",
                    ];
                    if style_keywords.contains(&k.as_str()) {
                        resolved.insert(format!("border-{side}-style"), val);
                    } else {
                        resolved.insert(format!("border-{side}-color"), val);
                    }
                }
                CssValue::Length(_, _) => {
                    resolved.insert(format!("border-{side}-width"), val);
                }
                _ => {
                    // Fallback: store under the shorthand name as-is.
                    resolved.insert(name.to_string(), val);
                }
            }
        }
        _ => {
            resolved.insert((*name).to_string(), val);
        }
    }
}

/// Resolve a single `CssValue::Var` to a concrete value.
///
/// Uses both depth limiting and a visited set for cycle detection.
/// If a custom property name is already in the visited set, the reference
/// is circular and resolution fails (returns `None`).
#[must_use]
pub(super) fn resolve_var_value(
    value: &CssValue,
    custom_props: &HashMap<String, String>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Option<CssValue> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }

    let CssValue::Var(ref name, ref fallback) = value else {
        return Some(value.clone());
    };

    // Cycle detection: if we've already visited this property, bail out.
    if !visited.insert(name.clone()) {
        return None;
    }

    // Look up the custom property.
    if let Some(raw) = custom_props.get(name) {
        // The raw value itself might contain var() references.
        let parsed = parse_raw_value(raw);
        if let CssValue::Var(..) = &parsed {
            // Recursively resolve nested var().
            let result = resolve_var_value(&parsed, custom_props, depth + 1, visited);
            visited.remove(name);
            return result;
        }
        visited.remove(name);
        return Some(parsed);
    }

    visited.remove(name);

    // Property not found: use fallback if available.
    match fallback {
        Some(fb) => {
            if let CssValue::Var(..) = fb.as_ref() {
                resolve_var_value(fb, custom_props, depth + 1, visited)
            } else {
                Some(*fb.clone())
            }
        }
        None => None, // No fallback, var() unresolvable.
    }
}

/// Parse a raw token string into a typed [`CssValue`].
///
/// Delegates to `elidex_css::parse_raw_token_value()` which uses cssparser
/// internally.
pub(super) fn parse_raw_value(raw: &str) -> CssValue {
    elidex_css::parse_raw_token_value(raw)
}

/// Merge resolved `var()` values back into the winners map.
pub(super) fn merge_winners<'a>(
    original: &PropertyMap<'a>,
    resolved: &'a HashMap<String, CssValue>,
) -> PropertyMap<'a> {
    let mut merged: PropertyMap<'a> = HashMap::new();
    // Copy originals, replacing var()-resolved entries and dropping unresolved
    // Var values. Unresolved var() should make the property "invalid at
    // computed-value time" (CSS Variables Level 1 §5), which means downstream
    // resolvers see no entry and apply inherited/initial as appropriate.
    for (name, value) in original {
        if name.starts_with("--") {
            continue;
        }
        if let Some(resolved_val) = resolved.get(*name) {
            merged.insert(name, resolved_val);
        } else if !matches!(value, CssValue::Var(..)) {
            merged.insert(name, value);
        }
        // Unresolved CssValue::Var is intentionally dropped.
    }
    // Include resolved entries with new keys (from shorthand expansion,
    // e.g. "background" → "background-color").
    for (name, value) in resolved {
        if !merged.contains_key(name.as_str()) {
            merged.insert(name.as_str(), value);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use elidex_plugin::{ComputedStyle, CssColor, CssValue, Display, LengthUnit};

    use super::*;
    use crate::resolve::helpers::PropertyMap;
    use crate::resolve::{build_computed_style, ResolveContext};

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    // --- Custom property resolution ---

    #[test]
    fn custom_property_inherited() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".to_string(), "red".to_string());
        let winners: PropertyMap = HashMap::new();
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--bg"),
            Some(&"red".to_string())
        );
    }

    #[test]
    fn custom_property_overridden() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".to_string(), "red".to_string());
        let blue = CssValue::RawTokens("blue".to_string());
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("--bg", &blue);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--bg"),
            Some(&"blue".to_string())
        );
    }

    #[test]
    fn custom_property_initial_removes() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".to_string(), "red".to_string());
        let initial = CssValue::Initial;
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("--bg", &initial);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(!style.custom_properties.contains_key("--bg"));
    }

    #[test]
    fn custom_properties_from_winners() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("#0d1117".into());
        winners.insert("--bg", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--bg"),
            Some(&"#0d1117".to_string())
        );
    }

    // --- var() resolution ---

    #[test]
    fn var_resolution_simple() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--main-color".to_string(), "red".to_string());
        let var_ref = CssValue::Var("--main-color".to_string(), None);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("color", &var_ref);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, CssColor::RED);
    }

    #[test]
    fn var_resolution_fallback() {
        let parent = ComputedStyle::default();
        let var_ref = CssValue::Var(
            "--undefined".to_string(),
            Some(Box::new(CssValue::Color(CssColor::BLUE))),
        );
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("color", &var_ref);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, CssColor::BLUE);
    }

    #[test]
    fn var_resolution_cycle_detection() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--a".to_string(), "var(--b)".to_string());
        parent
            .custom_properties
            .insert("--b".to_string(), "var(--a)".to_string());
        let var_ref = CssValue::Var("--a".to_string(), None);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("color", &var_ref);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, parent.color);
    }

    #[test]
    fn unresolved_var_treated_as_invalid() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--undefined".into(), None);
        winners.insert("display", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, Display::Inline);
    }

    #[test]
    fn var_resolution_undefined_no_fallback() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--undefined".into(), None);
        winners.insert("color", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, CssColor::BLACK);
    }

    #[test]
    fn var_resolution_background_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--bg".into(), None);
        winners.insert("background-color", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
    }

    #[test]
    fn var_resolution_display() {
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--d".into(), "flex".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--d".into(), None);
        winners.insert("display", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, Display::Flex);
    }

    // --- parse_raw_value ---

    #[test]
    fn parse_raw_value_types() {
        for (input, expected) in [
            ("#ff0000", CssValue::Color(CssColor::RED)),
            ("block", CssValue::Keyword("block".into())),
            ("16px", CssValue::Length(16.0, LengthUnit::Px)),
        ] {
            assert_eq!(
                parse_raw_value(input),
                expected,
                "parse_raw_value({input:?})"
            );
        }
        // Multi-token raw value should remain as RawTokens
        let multi = parse_raw_value("\"Courier New\", monospace");
        assert!(matches!(multi, CssValue::RawTokens(_)), "multi-token");
    }

    // --- Cycle detection (unit tests) ---

    #[test]
    fn var_circular_reference_returns_none() {
        let mut custom_props = HashMap::new();
        custom_props.insert("--a".into(), "var(--b)".into());
        custom_props.insert("--b".into(), "var(--a)".into());

        let var_a = CssValue::Var("--a".into(), None);
        let mut visited = HashSet::new();
        let result = resolve_var_value(&var_a, &custom_props, 0, &mut visited);
        assert!(result.is_none(), "circular var() should resolve to None");
    }

    #[test]
    fn var_self_reference_returns_none() {
        let mut custom_props = HashMap::new();
        custom_props.insert("--x".into(), "var(--x)".into());

        let var_x = CssValue::Var("--x".into(), None);
        let mut visited = HashSet::new();
        let result = resolve_var_value(&var_x, &custom_props, 0, &mut visited);
        assert!(
            result.is_none(),
            "self-referencing var() should resolve to None"
        );
    }

    // --- var() with font-family ---

    #[test]
    fn var_resolution_font_family_comma_list() {
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert(
            "--fonts".into(),
            "'SFMono-Regular', Consolas, monospace".into(),
        );
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--fonts".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.font_family,
            vec![
                "SFMono-Regular".to_string(),
                "Consolas".to_string(),
                "monospace".to_string(),
            ]
        );
    }

    #[test]
    fn var_resolution_font_family_double_quoted() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--fonts".into(), "\"Courier New\", monospace".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--fonts".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.font_family,
            vec!["Courier New".to_string(), "monospace".to_string()]
        );
    }

    #[test]
    fn var_resolution_font_family_single_generic() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--mono".into(), "monospace".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--mono".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.font_family, vec!["monospace".to_string()]);
    }

    // --- var() shorthand expansion ---

    #[test]
    fn var_background_shorthand_expands_to_background_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--bg".into(), None);
        winners.insert("background", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
    }

    #[test]
    fn var_border_side_shorthand_expands_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--border".into(), "#30363d".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--border".into(), None);
        winners.insert("border-bottom", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.border_bottom_color,
            CssColor::new(0x30, 0x36, 0x3d, 255)
        );
    }

    #[test]
    fn var_border_spacing_shorthand_expands() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--spacing".into(), "10px".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let var_val = CssValue::Var("--spacing".into(), None);
        winners.insert("border-spacing", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(
            (style.border_spacing_h - 10.0).abs() < f32::EPSILON,
            "border-spacing-h should be 10, got {}",
            style.border_spacing_h
        );
        assert!(
            (style.border_spacing_v - 10.0).abs() < f32::EPSILON,
            "border-spacing-v should be 10, got {}",
            style.border_spacing_v
        );
    }
}
