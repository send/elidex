//! Shared helper functions for CSS value resolution.

use std::collections::HashMap;

use elidex_plugin::{BorderStyle, ComputedStyle, CssValue};

use crate::inherit::{get_initial_value, is_inherited};

// Re-export core resolve functions from elidex-plugin.
pub(crate) use elidex_plugin::css_resolve::{
    resolve_calc_expr, resolve_dimension, resolve_i32, resolve_length, resolve_non_negative_f32,
    resolve_to_px,
};
pub(super) use elidex_plugin::css_resolve::keyword_from;

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
        CssValue::Inherit => Some(super::get_computed_as_css_value(property, parent_style)),
        CssValue::Initial => Some(get_initial_value(property)),
        CssValue::Unset => {
            if is_inherited(property) {
                Some(super::get_computed_as_css_value(property, parent_style))
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
    use elidex_plugin::{CalcExpr, ComputedStyle, CssColor, CssValue, Dimension, Display, LengthUnit, ResolveContext};

    use super::*;

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    // 5a: All length unit conversions in one table-driven test.
    #[test]
    fn resolve_length_units() {
        let cases: &[(f32, LengthUnit, ResolveContext, f32)] = &[
            (10.0, LengthUnit::Px, default_ctx(), 10.0),
            (
                2.0,
                LengthUnit::Em,
                ResolveContext {
                    em_base: 20.0,
                    ..default_ctx()
                },
                40.0,
            ),
            (
                2.0,
                LengthUnit::Rem,
                ResolveContext {
                    root_font_size: 18.0,
                    ..default_ctx()
                },
                36.0,
            ),
            (50.0, LengthUnit::Vw, default_ctx(), 960.0),
            (50.0, LengthUnit::Vh, default_ctx(), 540.0),
            (10.0, LengthUnit::Vmin, default_ctx(), 108.0),
            (10.0, LengthUnit::Vmax, default_ctx(), 192.0),
        ];
        for (val, unit, ctx, expected) in cases {
            let result = resolve_length(*val, *unit, ctx);
            assert_eq!(result, *expected, "{val} {unit:?}");
        }
    }

    // 5b: NaN and Infinity safety in one table-driven test.
    #[test]
    fn resolve_length_non_finite_returns_zero() {
        let ctx = default_ctx();
        for (val, unit) in [
            (f32::NAN, LengthUnit::Px),
            (f32::NAN, LengthUnit::Em),
            (f32::INFINITY, LengthUnit::Px),
            (f32::NEG_INFINITY, LengthUnit::Em),
        ] {
            assert_eq!(resolve_length(val, unit, &ctx), 0.0, "{val:?} {unit:?}");
        }
    }

    // 5c: resolve_i32 normal and edge cases in one table-driven test.
    #[test]
    fn resolve_i32_values() {
        for (input, default, expected) in [
            (CssValue::Number(42.0), 0, 42),
            (CssValue::Number(-3.0), 0, -3),
            (CssValue::Number(f32::NAN), 5, 5),
            (CssValue::Number(f32::INFINITY), 0, 0),
        ] {
            assert_eq!(resolve_i32(&input, default), expected, "{input:?}");
        }
    }

    // --- calc() resolution tests ---

    #[test]
    fn resolve_calc_addition() {
        let ctx = default_ctx();
        let expr = CalcExpr::Add(
            Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
            Box::new(CalcExpr::Length(20.0, LengthUnit::Px)),
        );
        assert_eq!(resolve_calc_expr(&expr, 0.0, &ctx), 30.0);
    }

    #[test]
    fn resolve_calc_subtraction() {
        let ctx = default_ctx();
        let expr = CalcExpr::Sub(
            Box::new(CalcExpr::Length(100.0, LengthUnit::Px)),
            Box::new(CalcExpr::Length(30.0, LengthUnit::Px)),
        );
        assert_eq!(resolve_calc_expr(&expr, 0.0, &ctx), 70.0);
    }

    #[test]
    fn resolve_calc_mul_div() {
        let ctx = default_ctx();
        let mul = CalcExpr::Mul(
            Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
            Box::new(CalcExpr::Number(3.0)),
        );
        assert_eq!(resolve_calc_expr(&mul, 0.0, &ctx), 30.0);

        let div = CalcExpr::Div(
            Box::new(CalcExpr::Length(100.0, LengthUnit::Px)),
            Box::new(CalcExpr::Number(4.0)),
        );
        assert_eq!(resolve_calc_expr(&div, 0.0, &ctx), 25.0);
    }

    #[test]
    fn resolve_calc_with_em() {
        let ctx = default_ctx(); // em_base = 16.0
        let expr = CalcExpr::Add(
            Box::new(CalcExpr::Length(2.0, LengthUnit::Em)),
            Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
        );
        // 2em = 32px + 10px = 42px
        assert_eq!(resolve_calc_expr(&expr, 0.0, &ctx), 42.0);
    }

    #[test]
    fn resolve_calc_with_percentage() {
        let ctx = default_ctx();
        let expr = CalcExpr::Sub(
            Box::new(CalcExpr::Percentage(100.0)),
            Box::new(CalcExpr::Length(20.0, LengthUnit::Px)),
        );
        // 100% of 800 - 20px = 780px
        assert_eq!(resolve_calc_expr(&expr, 800.0, &ctx), 780.0);
    }

    #[test]
    fn resolve_calc_div_by_zero() {
        let ctx = default_ctx();
        let expr = CalcExpr::Div(
            Box::new(CalcExpr::Length(100.0, LengthUnit::Px)),
            Box::new(CalcExpr::Number(0.0)),
        );
        assert_eq!(resolve_calc_expr(&expr, 0.0, &ctx), 0.0);
    }

    #[test]
    fn resolve_dimension_calc() {
        let ctx = default_ctx();
        let val = CssValue::Calc(Box::new(CalcExpr::Add(
            Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
            Box::new(CalcExpr::Length(5.0, LengthUnit::Px)),
        )));
        assert_eq!(resolve_dimension(&val, &ctx), Dimension::Length(15.0));
    }

    #[test]
    fn resolve_to_px_calc() {
        let ctx = default_ctx();
        let val = CssValue::Calc(Box::new(CalcExpr::Mul(
            Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
            Box::new(CalcExpr::Number(2.0)),
        )));
        assert_eq!(resolve_to_px(&val, &ctx), 20.0);
    }

    #[test]
    fn inherit_keyword_resolves_to_parent() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword_or_clone("display", &CssValue::Inherit, &parent);
        assert_eq!(resolved, CssValue::Keyword("block".to_string()));
    }

    // 5d: Unset behavior for inherited and non-inherited properties.
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
