//! Shared CSS value resolution helpers.
//!
//! Pure functions for resolving relative CSS values (em, rem, vw, vh, calc)
//! to absolute pixel values. Used by CSS property handlers during style
//! resolution.

use crate::{CalcExpr, CssValue, Dimension, LengthUnit, ParseError, ResolveContext};

/// Resolve a CSS length value to pixels.
///
/// Non-finite results (NaN/Infinity from overflow) are clamped to `0.0`.
#[must_use]
pub fn resolve_length(value: f32, unit: LengthUnit, ctx: &ResolveContext) -> f32 {
    let result = match unit {
        LengthUnit::Em => value * ctx.em_base,
        LengthUnit::Rem => value * ctx.root_font_size,
        LengthUnit::Vw => value * ctx.viewport_width / 100.0,
        LengthUnit::Vh => value * ctx.viewport_height / 100.0,
        LengthUnit::Vmin => value * ctx.viewport_width.min(ctx.viewport_height) / 100.0,
        LengthUnit::Vmax => value * ctx.viewport_width.max(ctx.viewport_height) / 100.0,
        // Px, Fr, and unknown units pass through unchanged.
        _ => value,
    };
    if result.is_finite() {
        result
    } else {
        0.0
    }
}

/// Resolve a [`CssValue`] to a [`Dimension`].
#[must_use]
pub fn resolve_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match value {
        CssValue::Length(v, unit) => Dimension::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => Dimension::Percentage(*p),
        CssValue::Number(n) if *n == 0.0 => Dimension::Length(0.0),
        CssValue::Calc(expr) => Dimension::Length(resolve_calc_expr(expr, 0.0, ctx)),
        _ => Dimension::Auto,
    }
}

/// Resolve a [`CssValue`] to a pixel value (for padding/border-width).
///
/// Percentage values resolve to `0.0` (Phase 4 TODO: resolve against
/// containing block).
#[must_use]
pub fn resolve_to_px(value: &CssValue, ctx: &ResolveContext) -> f32 {
    match value {
        CssValue::Length(v, unit) => resolve_length(*v, *unit, ctx),
        CssValue::Calc(expr) => resolve_calc_expr(expr, 0.0, ctx),
        CssValue::Number(n) if *n == 0.0 => 0.0,
        _ => 0.0,
    }
}

/// Resolve a `calc()` expression tree to a pixel value.
///
/// Uses a typed resolver that distinguishes `Length` (dimensional) from
/// `Scalar` (unitless number). `percentage_base` is the reference value
/// for percentage terms (e.g. containing block width).
#[must_use]
pub fn resolve_calc_expr(expr: &CalcExpr, percentage_base: f32, ctx: &ResolveContext) -> f32 {
    let result = match resolve_calc_typed(expr, percentage_base, ctx) {
        CalcResolved::Length(l) => l,
        CalcResolved::Scalar(_) => 0.0,
    };
    if result.is_finite() {
        result
    } else {
        0.0
    }
}

/// Resolve a [`CssValue::Number`] to a non-negative `f32`.
#[must_use]
pub fn resolve_non_negative_f32(value: &CssValue, default: f32) -> f32 {
    match value {
        CssValue::Number(n) => n.max(0.0),
        _ => default,
    }
}

/// Resolve a [`CssValue::Number`] to an `i32`.
///
/// Non-finite values (NaN/Infinity) return `default`.
#[must_use]
pub fn resolve_i32(value: &CssValue, default: i32) -> i32 {
    match value {
        #[allow(clippy::cast_possible_truncation)]
        CssValue::Number(n) if n.is_finite() => *n as i32,
        _ => default,
    }
}

/// Resolve a `CssValue::Keyword` to an enum variant via a `from_keyword` function.
///
/// Returns `None` if the value is not a `Keyword` or the keyword is unrecognized.
#[must_use]
pub fn resolve_keyword_to_enum<T: Default>(
    value: &CssValue,
    from_keyword: fn(&str) -> Option<T>,
) -> Option<T> {
    match value {
        CssValue::Keyword(ref k) => Some(from_keyword(k).unwrap_or_default()),
        _ => None,
    }
}

/// Wrap an `AsRef<str>` value in `CssValue::Keyword`.
#[must_use]
pub fn keyword_from<T: AsRef<str>>(val: &T) -> CssValue {
    CssValue::Keyword(val.as_ref().to_string())
}

/// Map a CSS unit string to a [`LengthUnit`].
#[must_use]
pub fn parse_length_unit(unit: &str) -> LengthUnit {
    match unit.to_ascii_lowercase().as_str() {
        "em" => LengthUnit::Em,
        "rem" => LengthUnit::Rem,
        "vw" => LengthUnit::Vw,
        "vh" => LengthUnit::Vh,
        "vmin" => LengthUnit::Vmin,
        "vmax" => LengthUnit::Vmax,
        _ => LengthUnit::Px, // px and unknown units
    }
}

// ---------------------------------------------------------------------------
// CSS parse helpers shared by handler crates
// ---------------------------------------------------------------------------

/// Parse a length or percentage value from a CSS token stream.
///
/// Accepts:
/// - A bare `0` number → `Length(0.0, Px)`
/// - A `<percentage>` → `Percentage`
/// - A `<dimension>` → `Length` with the appropriate unit
///
/// # Errors
///
/// Returns [`ParseError`] if none of the above forms is found.
pub fn parse_length_or_percentage(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected length or percentage"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError::simple("expected length or percentage")),
    }
}

/// Parse a non-negative length or percentage value.
///
/// Delegates to [`parse_length_or_percentage`] and then rejects negative
/// lengths and negative percentages.
///
/// # Errors
///
/// Returns [`ParseError`] if the value is negative or not a length/percentage.
pub fn parse_non_negative_length_or_percentage(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let value = parse_length_or_percentage(input)?;
    match &value {
        CssValue::Length(v, _) if *v < 0.0 => {
            Err(ParseError::simple("negative values not allowed"))
        }
        CssValue::Percentage(p) if *p < 0.0 => {
            Err(ParseError::simple("negative values not allowed"))
        }
        _ => Ok(value),
    }
}

/// Parse a non-negative length value (no percentages).
///
/// Accepts a `<dimension>` or a bare `0`. Rejects negative values and
/// percentages.
///
/// # Errors
///
/// Returns [`ParseError`] if the input is not a valid non-negative length.
pub fn parse_non_negative_length(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected length value"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            if value < 0.0 {
                return Err(ParseError {
                    property: String::new(),
                    input: format!("{value}{unit}"),
                    message: "negative length not allowed".into(),
                });
            }
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError::simple("expected length value")),
    }
}

// --- Internal helpers ---

enum CalcResolved {
    Length(f32),
    Scalar(f32),
}

fn resolve_calc_typed(expr: &CalcExpr, percentage_base: f32, ctx: &ResolveContext) -> CalcResolved {
    match expr {
        CalcExpr::Length(v, unit) => CalcResolved::Length(resolve_length(*v, *unit, ctx)),
        CalcExpr::Percentage(p) => CalcResolved::Length(percentage_base * p / 100.0),
        CalcExpr::Number(n) => CalcResolved::Scalar(*n),
        CalcExpr::Add(a, b) | CalcExpr::Sub(a, b) => {
            let is_sub = matches!(expr, CalcExpr::Sub(..));
            let left = resolve_calc_typed(a, percentage_base, ctx);
            let right = resolve_calc_typed(b, percentage_base, ctx);
            let op: fn(f32, f32) -> f32 = if is_sub { |a, b| a - b } else { |a, b| a + b };
            match (left, right) {
                (CalcResolved::Length(l1), CalcResolved::Length(l2)) => {
                    CalcResolved::Length(op(l1, l2))
                }
                (CalcResolved::Scalar(s1), CalcResolved::Scalar(s2)) => {
                    CalcResolved::Scalar(op(s1, s2))
                }
                _ => CalcResolved::Length(0.0),
            }
        }
        CalcExpr::Mul(a, b) => {
            let left = resolve_calc_typed(a, percentage_base, ctx);
            let right = resolve_calc_typed(b, percentage_base, ctx);
            match (left, right) {
                (CalcResolved::Length(l), CalcResolved::Scalar(s))
                | (CalcResolved::Scalar(s), CalcResolved::Length(l)) => CalcResolved::Length(l * s),
                (CalcResolved::Scalar(s1), CalcResolved::Scalar(s2)) => {
                    CalcResolved::Scalar(s1 * s2)
                }
                _ => CalcResolved::Length(0.0),
            }
        }
        CalcExpr::Div(a, b) => {
            let left = resolve_calc_typed(a, percentage_base, ctx);
            let right = resolve_calc_typed(b, percentage_base, ctx);
            match (left, right) {
                (CalcResolved::Length(l), CalcResolved::Scalar(s)) => {
                    if s == 0.0 {
                        CalcResolved::Length(0.0)
                    } else {
                        CalcResolved::Length(l / s)
                    }
                }
                (CalcResolved::Scalar(s1), CalcResolved::Scalar(s2)) => {
                    if s2 == 0.0 {
                        CalcResolved::Scalar(0.0)
                    } else {
                        CalcResolved::Scalar(s1 / s2)
                    }
                }
                _ => CalcResolved::Length(0.0),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

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

    #[test]
    fn resolve_length_non_finite() {
        let ctx = default_ctx();
        assert_eq!(resolve_length(f32::NAN, LengthUnit::Px, &ctx), 0.0);
        assert_eq!(resolve_length(f32::INFINITY, LengthUnit::Em, &ctx), 0.0);
    }

    #[test]
    fn resolve_dimension_variants() {
        let ctx = default_ctx();
        assert_eq!(
            resolve_dimension(&CssValue::Length(10.0, LengthUnit::Px), &ctx),
            Dimension::Length(10.0)
        );
        assert_eq!(
            resolve_dimension(&CssValue::Percentage(50.0), &ctx),
            Dimension::Percentage(50.0)
        );
        assert_eq!(resolve_dimension(&CssValue::Auto, &ctx), Dimension::Auto);
    }

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
    fn resolve_calc_div_by_zero() {
        let ctx = default_ctx();
        let expr = CalcExpr::Div(
            Box::new(CalcExpr::Length(100.0, LengthUnit::Px)),
            Box::new(CalcExpr::Number(0.0)),
        );
        assert_eq!(resolve_calc_expr(&expr, 0.0, &ctx), 0.0);
    }

    #[test]
    fn resolve_i32_values() {
        assert_eq!(resolve_i32(&CssValue::Number(42.0), 0), 42);
        assert_eq!(resolve_i32(&CssValue::Number(f32::NAN), 5), 5);
    }

    #[test]
    fn resolve_keyword_to_enum_known() {
        // Simulate a simple enum with from_keyword
        fn from_kw(s: &str) -> Option<u8> {
            match s {
                "a" => Some(1),
                "b" => Some(2),
                _ => None,
            }
        }
        assert_eq!(
            resolve_keyword_to_enum(&CssValue::Keyword("a".into()), from_kw),
            Some(1)
        );
        assert_eq!(
            resolve_keyword_to_enum(&CssValue::Keyword("b".into()), from_kw),
            Some(2)
        );
    }

    #[test]
    fn resolve_keyword_to_enum_unknown_returns_default() {
        fn from_kw(s: &str) -> Option<u8> {
            match s {
                "a" => Some(1),
                _ => None,
            }
        }
        // Unknown keyword falls back to T::default()
        assert_eq!(
            resolve_keyword_to_enum(&CssValue::Keyword("zzz".into()), from_kw),
            Some(0)
        );
    }

    #[test]
    fn resolve_keyword_to_enum_non_keyword_returns_none() {
        fn from_kw(_s: &str) -> Option<u8> {
            Some(1)
        }
        assert_eq!(
            resolve_keyword_to_enum(&CssValue::Number(42.0), from_kw),
            None
        );
        assert_eq!(resolve_keyword_to_enum(&CssValue::Auto, from_kw), None);
    }
}
