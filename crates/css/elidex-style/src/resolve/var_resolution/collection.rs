//! Custom property collection and shorthand expansion.

use std::collections::HashMap;

use elidex_plugin::{ComputedStyle, CssValue};

use super::super::helpers::PropertyMap;
use super::resolution::has_var_reference;

/// Build the custom properties map: inherit all from parent, then override
/// with any custom properties declared on this element.
///
/// Per CSS Variables Level 1:
/// - `RawTokens`: set the property to the raw value.
/// - `Initial`: remove the property (custom properties have no initial value;
///   their initial value is the "guaranteed-invalid value").
/// - `Inherit`/`Unset`: keep the inherited value (custom properties are
///   always inherited, so both behave as `inherit` -- already handled by the
///   parent clone).
pub(in crate::resolve) fn build_custom_properties(
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

/// Expand a resolved shorthand value into longhand entries.
///
/// For simple shorthands where the resolved value maps to a single longhand
/// (e.g. `background` -> `background-color`), insert under the longhand key.
/// For per-side border shorthands (`border-top`, etc.), insert under the 3
/// longhand keys with appropriate defaults.
pub(super) fn expand_resolved_shorthand(
    resolved: &mut HashMap<String, CssValue>,
    name: &str,
    val: CssValue,
) {
    match name {
        "background" => {
            resolved.insert("background-color".to_string(), val);
        }
        "border-spacing" => {
            // border-spacing shorthand -> 2 longhands (same value for both).
            resolved.insert("border-spacing-h".to_string(), val.clone());
            resolved.insert("border-spacing-v".to_string(), val);
        }
        "border-radius" => {
            // border-radius shorthand -> 4 longhands (same value for all corners).
            resolved.insert("border-top-left-radius".to_string(), val.clone());
            resolved.insert("border-top-right-radius".to_string(), val.clone());
            resolved.insert("border-bottom-right-radius".to_string(), val.clone());
            resolved.insert("border-bottom-left-radius".to_string(), val);
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            // The resolved value is a single color/keyword -- treat it as the
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
        "overflow" => {
            // overflow shorthand -> same value for both axes.
            resolved.insert("overflow-x".to_string(), val.clone());
            resolved.insert("overflow-y".to_string(), val);
        }
        // NOTE: grid-template and grid shorthands have complex syntax
        // (rows / columns, areas interleave, auto-flow patterns). When var()
        // resolves to a shorthand key, we cannot reliably split the value into
        // longhands. As a fallback, the resolved value is assigned to rows and
        // columns identically. This matches the simplistic approach used for
        // other shorthands (border, overflow) and is correct for single-value
        // cases like `none`. Full shorthand re-parsing after var() substitution
        // is deferred to the handler dispatch migration (M4-LFS-post).
        "grid-template" => {
            resolved.insert("grid-template-rows".to_string(), val.clone());
            resolved.insert("grid-template-columns".to_string(), val.clone());
            resolved.insert(
                "grid-template-areas".to_string(),
                CssValue::Keyword("none".to_string()),
            );
        }
        "grid" => {
            resolved.insert("grid-template-rows".to_string(), val.clone());
            resolved.insert("grid-template-columns".to_string(), val.clone());
            resolved.insert(
                "grid-template-areas".to_string(),
                CssValue::Keyword("none".to_string()),
            );
            resolved.insert(
                "grid-auto-flow".to_string(),
                CssValue::Keyword("row".to_string()),
            );
            resolved.insert("grid-auto-rows".to_string(), CssValue::Auto);
            resolved.insert("grid-auto-columns".to_string(), CssValue::Auto);
        }
        _ => {
            resolved.insert((*name).to_string(), val);
        }
    }
}

/// Merge resolved `var()` values back into the winners map.
pub(in crate::resolve) fn merge_winners<'a>(
    original: &PropertyMap<'a>,
    resolved: &'a HashMap<String, CssValue>,
) -> PropertyMap<'a> {
    let mut merged: PropertyMap<'a> = HashMap::new();
    // Copy originals, replacing var()-resolved entries and dropping unresolved
    // var() references. Unresolved var() should make the property "invalid at
    // computed-value time" (CSS Variables Level 1 5), which means downstream
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
    // e.g. "background" -> "background-color").
    for (name, value) in resolved {
        if !merged.contains_key(name.as_str()) {
            merged.insert(name.as_str(), value);
        }
    }
    merged
}
