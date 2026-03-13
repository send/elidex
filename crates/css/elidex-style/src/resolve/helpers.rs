//! Shared helper functions for CSS value resolution.

use std::collections::HashMap;

use elidex_plugin::{BorderStyle, ComputedStyle, CssValue};

use crate::inherit::{get_initial_value, is_inherited};

// Re-export core resolve functions from elidex-plugin.
pub(crate) use elidex_plugin::css_resolve::{
    resolve_calc_expr, resolve_dimension, resolve_i32, resolve_length, resolve_non_negative_f32,
    resolve_to_px,
};

/// Cascade winner map: property name → winning CSS value.
pub(crate) type PropertyMap<'a> = HashMap<&'a str, &'a CssValue>;

/// Resolve `inherit` / `initial` / `unset` keywords, returning the effective
/// [`CssValue`] to use for further resolution.
pub(super) fn resolve_keyword(
    property: &str,
    value: &CssValue,
    parent_style: &ComputedStyle,
) -> Option<CssValue> {
    match value {
        CssValue::Inherit => Some(crate::get_computed(property, parent_style)),
        CssValue::Initial => Some(get_initial_value(property)),
        CssValue::Unset => {
            if is_inherited(property) {
                Some(crate::get_computed(property, parent_style))
            } else {
                Some(get_initial_value(property))
            }
        }
        _ => None,
    }
}

/// Get the cascade winner for a property, resolving `inherit`/`initial`/`unset`
/// keywords. Returns `None` if the property is not in the winners map (caller
/// should apply inheritance or initial value as appropriate).
pub(super) fn get_resolved_winner(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) -> Option<CssValue> {
    let value = winners.get(property)?;
    Some(resolve_keyword_or_clone(property, value, parent_style))
}

/// If the value is `inherit`/`initial`/`unset`, resolve it; otherwise clone.
pub(super) fn resolve_keyword_or_clone(
    property: &str,
    value: &CssValue,
    parent_style: &ComputedStyle,
) -> CssValue {
    resolve_keyword(property, value, parent_style).unwrap_or_else(|| value.clone())
}

/// Resolve a property and apply the result via a setter closure.
pub(super) fn resolve_prop<T>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    convert: impl Fn(&CssValue) -> T,
    set: impl FnOnce(T),
) {
    if let Some(value) = get_resolved_winner(property, winners, parent_style) {
        set(convert(&value));
    }
}

/// Resolve a border-style keyword to a [`BorderStyle`] enum value.
pub(super) fn resolve_border_style_value(value: &CssValue) -> BorderStyle {
    match value {
        CssValue::Keyword(ref k) => BorderStyle::from_keyword(k).unwrap_or_default(),
        _ => BorderStyle::default(),
    }
}

/// Resolve an inherited keyword-enum property.
///
/// If present in the winners map, converts the keyword via `from_keyword`.
/// If absent, inherits from `parent_value`. This unifies the common pattern
/// used by text-transform, text-align, white-space, and list-style-type.
pub(super) fn resolve_inherited_keyword_enum<T: Copy>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    parent_value: T,
    from_keyword: impl Fn(&str) -> Option<T>,
) -> T {
    match get_resolved_winner(property, winners, parent_style) {
        Some(CssValue::Keyword(ref k)) => from_keyword(k).unwrap_or(parent_value),
        Some(_) | None => parent_value,
    }
}

/// Resolve a non-inherited keyword enum property from the winners map.
///
/// Returns `None` if the property is absent from the winners (caller keeps
/// the default). Returns `Some(T)` when a value is found, mapping the keyword
/// through `from_keyword` or falling back to `T::default()`.
pub(super) fn resolve_keyword_enum<T: Default>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    from_keyword: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    let value = get_resolved_winner(property, winners, parent_style)?;
    Some(match value {
        CssValue::Keyword(ref k) => from_keyword(k).unwrap_or_default(),
        _ => T::default(),
    })
}

/// Resolve a CSS keyword property to its corresponding enum variant.
///
/// Matches the keyword string from a `CssValue::Keyword` against a list of
/// `(keyword_str, EnumVariant)` pairs via `$parser`, falling back to the
/// enum's `Default` if no match. The result is assigned to `$field` only
/// when the property is present in the cascade winners.
macro_rules! resolve_keyword_enum_prop {
    ($prop:expr, $winners:expr, $parent_style:expr, $field:expr, $parser:expr) => {
        if let Some(val) =
            $crate::resolve::helpers::resolve_keyword_enum($prop, $winners, $parent_style, $parser)
        {
            $field = val;
        }
    };
}

pub(crate) use resolve_keyword_enum_prop;

#[cfg(test)]
mod tests {
    use elidex_plugin::{ComputedStyle, CssColor, CssValue, Display};

    use super::*;

    #[test]
    fn inherit_keyword_resolves_to_parent() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword_or_clone("display", &CssValue::Inherit, &parent);
        assert_eq!(resolved, CssValue::Keyword("block".to_string()));
    }

    #[test]
    fn unset_behavior() {
        // Inherited property (color): unset → parent value
        let parent = ComputedStyle {
            color: CssColor::RED,
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let inherited = resolve_keyword_or_clone("color", &CssValue::Unset, &parent);
        assert_eq!(inherited, CssValue::Color(CssColor::RED));
        // Non-inherited property (display): unset → initial value
        let non_inherited = resolve_keyword_or_clone("display", &CssValue::Unset, &parent);
        assert_eq!(non_inherited, CssValue::Keyword("inline".to_string()));
    }
}
