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

/// Resolve all `var()` references in the winners map.
///
/// Handles two cases:
/// 1. `CssValue::Var` — whole-value `var()` (e.g. `color: var(--x)`)
/// 2. `CssValue::RawTokens` containing `var()` — compound values
///    (e.g. `border: var(--bw) solid var(--bc)`)
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
                // Compound value with var() — substitute and re-parse.
                if let Some(substituted) = substitute_vars(raw, custom_props, 0) {
                    let decl_text = format!("{name}: {substituted}");
                    let decls = elidex_css::parse_declaration_block(&decl_text);
                    for decl in &decls {
                        resolved.insert(decl.property.clone(), decl.value.clone());
                    }
                }
                // If substitution fails, property is "invalid at computed-value
                // time" — no entry in resolved, so merge_winners will drop it.
            }
            _ => {}
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

/// Check whether a `CssValue` contains an unresolved `var()` reference.
fn has_var_reference(value: &CssValue) -> bool {
    match value {
        CssValue::Var(..) => true,
        CssValue::RawTokens(raw) => raw.contains("var("),
        _ => false,
    }
}

/// Substitute all `var()` references in a raw CSS value string.
///
/// Returns `None` if any `var()` reference is unresolvable (no custom property
/// value and no fallback), which makes the property "invalid at computed-value
/// time" per CSS Variables Level 1 §5.
fn substitute_vars(
    raw: &str,
    custom_props: &HashMap<String, String>,
    depth: usize,
) -> Option<String> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }
    if !raw.contains("var(") {
        return Some(raw.to_string());
    }

    let mut result = String::with_capacity(raw.len());
    let mut remaining = raw;

    loop {
        let Some(var_start) = remaining.find("var(") else {
            result.push_str(remaining);
            break;
        };

        // Copy text before var(
        result.push_str(&remaining[..var_start]);

        // Parse var() arguments
        let after_open = &remaining[var_start + 4..];
        let (name, fallback, consumed) = parse_var_args(after_open)?;

        // Look up the custom property
        let name = name.trim();
        if let Some(value) = custom_props.get(name) {
            // The value might itself contain var() — recurse.
            let resolved = substitute_vars(value.trim(), custom_props, depth + 1)?;
            result.push_str(&resolved);
        } else if let Some(fb) = fallback {
            // Use fallback, which might contain var().
            let resolved = substitute_vars(fb.trim(), custom_props, depth + 1)?;
            result.push_str(&resolved);
        } else {
            return None; // Unresolvable — invalid at computed-value time.
        }

        remaining = &after_open[consumed..];
    }

    Some(result)
}

/// Parse `var()` arguments from a string positioned after the opening `var(`.
///
/// Returns `(name, optional_fallback, bytes_consumed)` including the closing `)`.
/// Handles nested parentheses in the fallback value (e.g. `var(--x, calc(1px + 2px))`).
fn parse_var_args(s: &str) -> Option<(&str, Option<&str>, usize)> {
    let mut depth: u32 = 1;
    let mut comma_pos = None;

    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let name_end = comma_pos.unwrap_or(i);
                    let name = &s[..name_end];
                    let fallback = comma_pos.map(|cp| &s[cp + 1..i]);
                    return Some((name, fallback, i + 1));
                }
            }
            ',' if depth == 1 && comma_pos.is_none() => {
                comma_pos = Some(i);
            }
            _ => {}
        }
    }
    None // Unclosed var()
}

/// Merge resolved `var()` values back into the winners map.
pub(super) fn merge_winners<'a>(
    original: &PropertyMap<'a>,
    resolved: &'a HashMap<String, CssValue>,
) -> PropertyMap<'a> {
    let mut merged: PropertyMap<'a> = HashMap::new();
    // Copy originals, replacing var()-resolved entries and dropping unresolved
    // var() references. Unresolved var() should make the property "invalid at
    // computed-value time" (CSS Variables Level 1 §5), which means downstream
    // resolvers see no entry and apply inherited/initial as appropriate.
    for (name, value) in original {
        if name.starts_with("--") {
            continue;
        }
        if let Some(resolved_val) = resolved.get(*name) {
            merged.insert(name, resolved_val);
        } else if !has_var_reference(value) {
            merged.insert(name, value);
        }
        // Unresolved var() references (Var or RawTokens with var()) are
        // intentionally dropped.
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

    // --- Compound var() in multi-token values (CSS Variables Level 1 §3) ---

    #[test]
    fn compound_var_border_shorthand() {
        // border: var(--bw) solid var(--bc)
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bw".into(), "2px".into());
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
        parent
            .custom_properties
            .insert("--x".into(), "10px".into());
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let raw = CssValue::RawTokens("0 var(--x)".into());
        winners.insert("margin", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.margin_top, elidex_plugin::Dimension::ZERO);
        assert_eq!(
            style.margin_right,
            elidex_plugin::Dimension::Length(10.0)
        );
        assert_eq!(style.margin_bottom, elidex_plugin::Dimension::ZERO);
        assert_eq!(
            style.margin_left,
            elidex_plugin::Dimension::Length(10.0)
        );
    }

    #[test]
    fn compound_var_border_side() {
        // border-bottom: var(--bw) solid var(--bc)
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bw".into(), "1px".into());
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

        assert_eq!(
            substitute_vars("var(--x) solid var(--color)", &props, 0),
            Some("10px solid red".to_string())
        );
        assert_eq!(
            substitute_vars("0 var(--x)", &props, 0),
            Some("0 10px".to_string())
        );
        // No var() — passthrough.
        assert_eq!(
            substitute_vars("1px solid red", &props, 0),
            Some("1px solid red".to_string())
        );
        // Undefined with no fallback — None.
        assert_eq!(substitute_vars("var(--undefined)", &props, 0), None);
        // Fallback used.
        assert_eq!(
            substitute_vars("var(--undefined, 5px)", &props, 0),
            Some("5px".to_string())
        );
    }

    #[test]
    fn substitute_vars_nested_parens() {
        let mut props = HashMap::new();
        props.insert("--x".into(), "10px".into());
        // Fallback with nested parentheses: var(--undefined, calc(1px + 2px))
        assert_eq!(
            substitute_vars("var(--undefined, calc(1px + 2px))", &props, 0),
            Some("calc(1px + 2px)".to_string())
        );
    }

    #[test]
    fn parse_var_args_unit() {
        // Simple: var(--x) → ("--x", None, consumed)
        let (name, fb, consumed) = parse_var_args("--x)").unwrap();
        assert_eq!(name, "--x");
        assert!(fb.is_none());
        assert_eq!(consumed, 4);

        // With fallback: var(--x, 10px) → ("--x", Some(" 10px"), consumed)
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
