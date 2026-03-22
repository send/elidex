//! CSS custom property and `var()` resolution.

mod collection;
mod resolution;

use std::collections::{HashMap, HashSet};

use elidex_plugin::CssValue;

use super::helpers::PropertyMap;
use collection::expand_resolved_shorthand;
use resolution::{resolve_var_value, substitute_vars};

pub(super) use collection::{build_custom_properties, merge_winners};

/// Resolve all `var()` references in the winners map.
///
/// Handles two cases:
/// 1. `CssValue::Var` -- whole-value `var()` (e.g. `color: var(--x)`)
/// 2. `CssValue::RawTokens` containing `var()` -- compound values
///    (e.g. `border: var(--bw) solid var(--bc)`)
///
/// Returns a map of property name -> resolved `CssValue` for properties that
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
        match value {
            CssValue::Var(..) => {
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
            CssValue::RawTokens(raw) if raw.contains("var(") => {
                // Compound value with var() -- substitute and re-parse.
                let mut visited = HashSet::new();
                if let Some(substituted) = substitute_vars(raw, custom_props, 0, &mut visited) {
                    let decl_text = format!("{name}: {substituted}");
                    let decls = elidex_css::parse_declaration_block(&decl_text);
                    for decl in &decls {
                        resolved.insert(decl.property.clone(), decl.value.clone());
                    }
                }
                // If substitution fails, property is "invalid at computed-value
                // time" -- no entry in resolved, so merge_winners will drop it.
            }
            _ => {}
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use elidex_plugin::{ComputedStyle, CssColor, CssValue, Display, LengthUnit};

    use super::resolution::{parse_raw_value, parse_var_args, resolve_var_value, substitute_vars};
    use crate::resolve::helpers::PropertyMap;
    use crate::resolve::{build_computed_style, ResolveContext};

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport: elidex_plugin::Size::new(1920.0, 1080.0),
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
            style.border_bottom.color,
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

    // --- Compound var() in multi-token values (CSS Variables Level 1 3) ---

    #[test]
    fn compound_var_border_shorthand() {
        // border: var(--bw) solid var(--bc)
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--bw".into(), "2px".into());
        parent
            .custom_properties
            .insert("--bc".into(), "#ff0000".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("var(--bw) solid var(--bc)".into());
        winners.insert("border", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(
            (style.border_top.width - 2.0).abs() < f32::EPSILON,
            "border-top width: {}",
            style.border_top.width
        );
        assert_eq!(style.border_top.style, elidex_plugin::BorderStyle::Solid);
        assert_eq!(style.border_top.color, CssColor::RED);
        // All four sides should be set by the border shorthand.
        assert!(
            (style.border_left.width - 2.0).abs() < f32::EPSILON,
            "border-left width: {}",
            style.border_left.width
        );
    }

    #[test]
    fn compound_var_margin() {
        // margin: 0 var(--x)
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--x".into(), "10px".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("0 var(--x)".into());
        winners.insert("margin", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.margin_top, elidex_plugin::Dimension::ZERO);
        assert_eq!(style.margin_right, elidex_plugin::Dimension::Length(10.0));
        assert_eq!(style.margin_bottom, elidex_plugin::Dimension::ZERO);
        assert_eq!(style.margin_left, elidex_plugin::Dimension::Length(10.0));
    }

    #[test]
    fn compound_var_border_side() {
        // border-bottom: var(--bw) solid var(--bc)
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--bw".into(), "1px".into());
        parent
            .custom_properties
            .insert("--bc".into(), "#30363d".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("var(--bw) solid var(--bc)".into());
        winners.insert("border-bottom", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(
            (style.border_bottom.width - 1.0).abs() < f32::EPSILON,
            "border-bottom width: {}",
            style.border_bottom.width
        );
        assert_eq!(style.border_bottom.style, elidex_plugin::BorderStyle::Solid);
        assert_eq!(
            style.border_bottom.color,
            CssColor::new(0x30, 0x36, 0x3d, 255)
        );
    }

    #[test]
    fn compound_var_with_fallback() {
        // border: var(--bw, 3px) solid var(--bc, blue)
        let parent = ComputedStyle::default(); // no custom properties
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("var(--bw, 3px) solid var(--bc, blue)".into());
        winners.insert("border", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(
            (style.border_top.width - 3.0).abs() < f32::EPSILON,
            "fallback width: {}",
            style.border_top.width
        );
        assert_eq!(style.border_top.style, elidex_plugin::BorderStyle::Solid);
        assert_eq!(style.border_top.color, CssColor::BLUE);
    }

    #[test]
    fn compound_var_unresolvable_drops_property() {
        // If a var() in a compound value has no value and no fallback,
        // the entire property is invalid at computed-value time.
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("var(--undefined) solid red".into());
        winners.insert("border", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        // Border should remain at defaults (no border).
        assert!(
            style.border_top.width < f32::EPSILON,
            "unresolvable var should leave border at default"
        );
    }

    #[test]
    fn substitute_vars_unit() {
        let mut props = HashMap::new();
        props.insert("--x".into(), "10px".into());
        props.insert("--color".into(), "red".into());

        let mut visited = HashSet::new();
        assert_eq!(
            substitute_vars("var(--x) solid var(--color)", &props, 0, &mut visited),
            Some("10px solid red".to_string())
        );
        assert_eq!(
            substitute_vars("0 var(--x)", &props, 0, &mut visited),
            Some("0 10px".to_string())
        );
        // No var() -- passthrough.
        assert_eq!(
            substitute_vars("1px solid red", &props, 0, &mut visited),
            Some("1px solid red".to_string())
        );
        // Undefined with no fallback -- None.
        assert_eq!(
            substitute_vars("var(--undefined)", &props, 0, &mut visited),
            None
        );
        // Fallback used.
        assert_eq!(
            substitute_vars("var(--undefined, 5px)", &props, 0, &mut visited),
            Some("5px".to_string())
        );
    }

    #[test]
    fn substitute_vars_nested_parens() {
        let mut props = HashMap::new();
        props.insert("--x".into(), "10px".into());
        // Fallback with nested parentheses: var(--undefined, calc(1px + 2px))
        let mut visited = HashSet::new();
        assert_eq!(
            substitute_vars("var(--undefined, calc(1px + 2px))", &props, 0, &mut visited),
            Some("calc(1px + 2px)".to_string())
        );
    }

    #[test]
    fn parse_var_args_unit() {
        // Simple: var(--x) -> ("--x", None, consumed)
        let (name, fb, consumed) = parse_var_args("--x)").unwrap();
        assert_eq!(name, "--x");
        assert!(fb.is_none());
        assert_eq!(consumed, 4);

        // With fallback: var(--x, 10px) -> ("--x", Some(" 10px"), consumed)
        let (name, fb, consumed) = parse_var_args("--x, 10px)").unwrap();
        assert_eq!(name, "--x");
        assert_eq!(fb.unwrap().trim(), "10px");
        assert_eq!(consumed, 10);

        // Nested parens: var(--x, calc(1px + 2px))
        let (name, fb, consumed) = parse_var_args("--x, calc(1px + 2px))").unwrap();
        assert_eq!(name, "--x");
        assert_eq!(fb.unwrap().trim(), "calc(1px + 2px)");
        assert_eq!(consumed, 21);
    }
}
