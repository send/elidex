//! CSS value parsing utilities.
//!
//! Provides parsers for lengths, percentages, keywords, and the `auto` value
//! used across property declaration parsing.

use cssparser::{Parser, Token};
use elidex_plugin::{CalcExpr, CssValue, LengthUnit};

/// Try to parse a Dimension token or bare zero into a `CssValue`.
///
/// Shared logic for `parse_length` and `parse_length_or_percentage`.
fn try_dimension_or_zero(token: &cssparser::Token) -> Option<CssValue> {
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } if value.is_finite() => parse_length_unit(unit)
            .ok()
            .map(|u| CssValue::Length(value, u)),
        cssparser::Token::Number { value: 0.0, .. } => Some(CssValue::Length(0.0, LengthUnit::Px)),
        _ => None,
    }
}

/// Parse a CSS length value (e.g. `10px`, `2em`, `0`).
///
/// Unitless `0` is treated as `0px` per CSS specification.
/// Also accepts `calc()` expressions that resolve to a length.
/// Rejects `calc()` containing percentage terms (use [`parse_length_or_percentage`]
/// for properties that accept `<length-percentage>`).
#[allow(clippy::result_unit_err)] // cssparser convention: Parser methods return Result<T, ()>.
pub fn parse_length(input: &mut Parser) -> Result<CssValue, ()> {
    // Try calc() first (may resolve to a length).
    if let Ok(val) = input.try_parse(parse_calc) {
        match &val {
            CssValue::Calc(expr) if !expr.contains_percentage() => return Ok(val),
            CssValue::Length(_, _) => return Ok(val),
            _ => return Err(()),
        }
    }
    let token = input.next().map_err(|_| ())?;
    try_dimension_or_zero(token).ok_or(())
}

/// Parse a length or percentage value, including `calc()` expressions.
#[allow(clippy::result_unit_err)] // cssparser convention: Parser methods return Result<T, ()>.
pub fn parse_length_or_percentage(input: &mut Parser) -> Result<CssValue, ()> {
    // Try calc() first.
    if let Ok(val) = input.try_parse(parse_calc) {
        return Ok(val);
    }
    let token = input.next().map_err(|_| ())?;
    if let Some(val) = try_dimension_or_zero(token) {
        return Ok(val);
    }
    match *token {
        cssparser::Token::Percentage { unit_value, .. } if unit_value.is_finite() => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        _ => Err(()),
    }
}

/// Parse a non-negative length or percentage value.
///
/// Returns `Err(())` if the value is negative. `calc()` expressions are
/// accepted — per CSS Values Level 3 §10.1, out-of-range `calc()` results
/// are clamped to the allowed range at used-value time, not at parse time.
#[allow(clippy::result_unit_err)] // cssparser convention: Parser methods return Result<T, ()>.
pub fn parse_non_negative_length_or_percentage(input: &mut Parser) -> Result<CssValue, ()> {
    let val = parse_length_or_percentage(input)?;
    match &val {
        CssValue::Length(px, _) if *px < 0.0 => Err(()),
        CssValue::Percentage(pct) if *pct < 0.0 => Err(()),
        // calc() negativity is clamped at used-value time (CSS Values §10.1).
        _ => Ok(val),
    }
}

/// Parse a length, percentage, or `auto` keyword.
#[allow(clippy::result_unit_err)] // cssparser convention: Parser methods return Result<T, ()>.
pub fn parse_length_percentage_or_auto(input: &mut Parser) -> Result<CssValue, ()> {
    if let Ok(val) = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("auto") {
            Ok(CssValue::Auto)
        } else {
            Err(())
        }
    }) {
        return Ok(val);
    }
    parse_length_or_percentage(input)
}

/// Check if an identifier is a CSS global keyword (`initial`, `inherit`, `unset`).
pub fn parse_global_keyword(ident: &str) -> Option<CssValue> {
    match ident.to_ascii_lowercase().as_str() {
        "initial" => Some(CssValue::Initial),
        "inherit" => Some(CssValue::Inherit),
        "unset" => Some(CssValue::Unset),
        _ => None,
    }
}

/// Map a CSS unit string to a `LengthUnit`.
pub(crate) fn parse_length_unit(unit: &str) -> Result<LengthUnit, ()> {
    match unit.to_ascii_lowercase().as_str() {
        "px" => Ok(LengthUnit::Px),
        "em" => Ok(LengthUnit::Em),
        "rem" => Ok(LengthUnit::Rem),
        "vw" => Ok(LengthUnit::Vw),
        "vh" => Ok(LengthUnit::Vh),
        "vmin" => Ok(LengthUnit::Vmin),
        "vmax" => Ok(LengthUnit::Vmax),
        _ => Err(()),
    }
}

// --- calc() expression parser (CSS Values Level 3 §8) ---

/// Maximum nesting depth for `calc()` parenthesized sub-expressions.
///
/// Prevents stack overflow from deeply nested input like `calc(((((...)))))`.
const MAX_CALC_DEPTH: u32 = 32;

/// Maximum number of AST nodes in a `calc()` expression.
///
/// Prevents stack overflow in recursive resolvers (`resolve_calc_typed`,
/// `infer_calc_type`) from deeply left-recursive trees built by long flat
/// expressions like `calc(1px + 1px + 1px + ...)`.
const MAX_CALC_NODES: u32 = 256;

/// Parse a `calc()` function into a `CssValue::Calc`.
///
/// After parsing the expression tree, validates type correctness per
/// CSS Values Level 3 §8.1.1.
fn parse_calc(input: &mut Parser) -> Result<CssValue, ()> {
    input.expect_function_matching("calc").map_err(|_| ())?;
    let mut node_count: u32 = 0;
    let expr = input
        .parse_nested_block(|block| {
            parse_calc_sum(block, 0, &mut node_count)
                .map_err(|()| block.current_source_location().new_custom_error(()))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())?;
    // Validate type correctness before accepting.
    validate_calc_types(&expr)?;
    Ok(CssValue::Calc(Box::new(expr)))
}

/// Parse a calc sum: `<product> [ ['+' | '-'] <product> ]*`.
///
/// Note: CSS Values Level 3 §8.1 requires `+` and `-` to be surrounded
/// by whitespace. The cssparser tokenizer enforces this: `10px+20px` is
/// tokenized as `Dimension(10px)` `Dimension(20px)` (no `Delim('+')`),
/// so `try_parse` fails to find an operator and the expression ends
/// after the first term, causing the overall parse to fail on remaining
/// input. This means the whitespace requirement is enforced at the
/// tokenizer level, not the parser level.
fn parse_calc_sum(input: &mut Parser, depth: u32, nodes: &mut u32) -> Result<CalcExpr, ()> {
    let mut left = parse_calc_product(input, depth, nodes)?;
    loop {
        let op = input.try_parse(|i| {
            let tok = i.next().map_err(|_| ())?;
            match tok {
                Token::Delim('+') => Ok('+'),
                Token::Delim('-') => Ok('-'),
                _ => Err(()),
            }
        });
        match op {
            Ok('+') => {
                let right = parse_calc_product(input, depth, nodes)?;
                *nodes += 1;
                if *nodes > MAX_CALC_NODES {
                    return Err(());
                }
                left = CalcExpr::Add(Box::new(left), Box::new(right));
            }
            Ok('-') => {
                let right = parse_calc_product(input, depth, nodes)?;
                *nodes += 1;
                if *nodes > MAX_CALC_NODES {
                    return Err(());
                }
                left = CalcExpr::Sub(Box::new(left), Box::new(right));
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse a calc product: `<value> [ ['*' | '/'] <value> ]*`.
fn parse_calc_product(input: &mut Parser, depth: u32, nodes: &mut u32) -> Result<CalcExpr, ()> {
    let mut left = parse_calc_value(input, depth, nodes)?;
    loop {
        let op = input.try_parse(|i| {
            let tok = i.next().map_err(|_| ())?;
            match tok {
                Token::Delim('*') => Ok('*'),
                Token::Delim('/') => Ok('/'),
                _ => Err(()),
            }
        });
        match op {
            Ok('*') => {
                let right = parse_calc_value(input, depth, nodes)?;
                *nodes += 1;
                if *nodes > MAX_CALC_NODES {
                    return Err(());
                }
                left = CalcExpr::Mul(Box::new(left), Box::new(right));
            }
            Ok('/') => {
                let right = parse_calc_value(input, depth, nodes)?;
                *nodes += 1;
                if *nodes > MAX_CALC_NODES {
                    return Err(());
                }
                left = CalcExpr::Div(Box::new(left), Box::new(right));
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse a calc leaf value: `<number>` | `<dimension>` | `<percentage>` | `( <sum> )`.
fn parse_calc_value(input: &mut Parser, depth: u32, nodes: &mut u32) -> Result<CalcExpr, ()> {
    *nodes += 1;
    if *nodes > MAX_CALC_NODES {
        return Err(());
    }

    // Try parenthesized sub-expression (with depth limit).
    if let Ok(expr) = input.try_parse(|i| {
        i.expect_parenthesis_block().map_err(|_| ())?;
        if depth >= MAX_CALC_DEPTH {
            return Err(());
        }
        i.parse_nested_block(|block| {
            parse_calc_sum(block, depth + 1, nodes)
                .map_err(|()| block.current_source_location().new_custom_error(()))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
    }) {
        return Ok(expr);
    }

    let token = input.next().map_err(|_| ())?;
    match *token {
        Token::Number { value, .. } if value.is_finite() => Ok(CalcExpr::Number(value)),
        Token::Dimension {
            value, ref unit, ..
        } if value.is_finite() => {
            let u = parse_length_unit(unit)?;
            Ok(CalcExpr::Length(value, u))
        }
        Token::Percentage { unit_value, .. } if unit_value.is_finite() => {
            Ok(CalcExpr::Percentage(unit_value * 100.0))
        }
        _ => Err(()),
    }
}

// --- calc() type validation (CSS Values Level 3 §8.1.1) ---

/// The resolved type of a `calc()` sub-expression.
#[derive(Clone, Copy, PartialEq)]
enum CalcType {
    /// A `<number>` (unitless).
    Number,
    /// A `<length>` or `<percentage>` (dimensional).
    LengthPercentage,
}

/// Infer the result type of a `calc()` expression, returning `None` if
/// the expression violates CSS Values Level 3 §8.1.1 type rules:
/// - `+`/`-`: both operands must be the same type
/// - `*`: at least one operand must be `<number>`
/// - `/`: the right operand must be `<number>`
fn infer_calc_type(expr: &CalcExpr) -> Option<CalcType> {
    match expr {
        CalcExpr::Number(_) => Some(CalcType::Number),
        CalcExpr::Length(..) | CalcExpr::Percentage(_) => Some(CalcType::LengthPercentage),
        CalcExpr::Add(a, b) | CalcExpr::Sub(a, b) => {
            let ta = infer_calc_type(a)?;
            let tb = infer_calc_type(b)?;
            if ta == tb {
                Some(ta)
            } else {
                None
            }
        }
        CalcExpr::Mul(a, b) => {
            let ta = infer_calc_type(a)?;
            let tb = infer_calc_type(b)?;
            match (ta, tb) {
                (CalcType::Number, CalcType::Number) => Some(CalcType::Number),
                (CalcType::Number, CalcType::LengthPercentage)
                | (CalcType::LengthPercentage, CalcType::Number) => {
                    Some(CalcType::LengthPercentage)
                }
                _ => None,
            }
        }
        CalcExpr::Div(a, b) => {
            let ta = infer_calc_type(a)?;
            let tb = infer_calc_type(b)?;
            if tb != CalcType::Number {
                return None;
            }
            Some(ta)
        }
    }
}

/// Validate `calc()` expression type correctness for a length/percentage context.
///
/// Returns `Err(())` if any sub-expression violates the type rules, or if
/// the overall expression resolves to a pure `<number>` (invalid in a
/// length/percentage context per CSS Values Level 3 §8.1.2).
fn validate_calc_types(expr: &CalcExpr) -> Result<(), ()> {
    match infer_calc_type(expr) {
        Some(CalcType::LengthPercentage) => Ok(()),
        // Pure <number> (e.g. `calc(1 + 2)`) is invalid in length/percentage context.
        Some(CalcType::Number) | None => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn length(css: &str) -> Result<CssValue, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_length(&mut parser)
    }

    fn length_or_pct(css: &str) -> Result<CssValue, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_length_or_percentage(&mut parser)
    }

    fn length_pct_auto(css: &str) -> Result<CssValue, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_length_percentage_or_auto(&mut parser)
    }

    #[test]
    fn parse_px_length() {
        assert_eq!(length("10px"), Ok(CssValue::Length(10.0, LengthUnit::Px)));
    }

    #[test]
    fn parse_em_length() {
        assert_eq!(length("2em"), Ok(CssValue::Length(2.0, LengthUnit::Em)));
    }

    #[test]
    fn parse_rem_length() {
        assert_eq!(length("1.5rem"), Ok(CssValue::Length(1.5, LengthUnit::Rem)));
    }

    #[test]
    fn parse_viewport_units() {
        assert_eq!(length("50vw"), Ok(CssValue::Length(50.0, LengthUnit::Vw)));
        assert_eq!(length("100vh"), Ok(CssValue::Length(100.0, LengthUnit::Vh)));
    }

    #[test]
    fn parse_percentage() {
        assert_eq!(length_or_pct("50%"), Ok(CssValue::Percentage(50.0)));
    }

    #[test]
    fn parse_auto_keyword() {
        assert_eq!(length_pct_auto("auto"), Ok(CssValue::Auto));
    }

    #[test]
    fn parse_zero_no_unit() {
        assert_eq!(length("0"), Ok(CssValue::Length(0.0, LengthUnit::Px)));
    }

    #[test]
    fn parse_negative_length() {
        assert_eq!(length("-5px"), Ok(CssValue::Length(-5.0, LengthUnit::Px)));
    }

    #[test]
    fn global_keywords() {
        assert_eq!(parse_global_keyword("initial"), Some(CssValue::Initial));
        assert_eq!(parse_global_keyword("inherit"), Some(CssValue::Inherit));
        assert_eq!(parse_global_keyword("unset"), Some(CssValue::Unset));
        assert_eq!(parse_global_keyword("INITIAL"), Some(CssValue::Initial));
        assert_eq!(parse_global_keyword("something"), None);
    }

    // --- calc() parsing tests ---

    #[test]
    fn calc_simple_addition() {
        let val = length_or_pct("calc(10px + 20px)").unwrap();
        match val {
            CssValue::Calc(expr) => match *expr {
                CalcExpr::Add(a, b) => {
                    assert_eq!(*a, CalcExpr::Length(10.0, LengthUnit::Px));
                    assert_eq!(*b, CalcExpr::Length(20.0, LengthUnit::Px));
                }
                _ => panic!("expected Add, got {expr:?}"),
            },
            _ => panic!("expected Calc, got {val:?}"),
        }
    }

    #[test]
    fn calc_subtraction() {
        let val = length_or_pct("calc(100% - 20px)").unwrap();
        match val {
            CssValue::Calc(expr) => match *expr {
                CalcExpr::Sub(a, b) => {
                    assert_eq!(*a, CalcExpr::Percentage(100.0));
                    assert_eq!(*b, CalcExpr::Length(20.0, LengthUnit::Px));
                }
                _ => panic!("expected Sub, got {expr:?}"),
            },
            _ => panic!("expected Calc, got {val:?}"),
        }
    }

    #[test]
    fn calc_multiplication() {
        let val = length_or_pct("calc(10px * 3)").unwrap();
        match val {
            CssValue::Calc(expr) => match *expr {
                CalcExpr::Mul(a, b) => {
                    assert_eq!(*a, CalcExpr::Length(10.0, LengthUnit::Px));
                    assert_eq!(*b, CalcExpr::Number(3.0));
                }
                _ => panic!("expected Mul, got {expr:?}"),
            },
            _ => panic!("expected Calc, got {val:?}"),
        }
    }

    #[test]
    fn calc_division() {
        let val = length_or_pct("calc(100px / 2)").unwrap();
        match val {
            CssValue::Calc(expr) => match *expr {
                CalcExpr::Div(a, b) => {
                    assert_eq!(*a, CalcExpr::Length(100.0, LengthUnit::Px));
                    assert_eq!(*b, CalcExpr::Number(2.0));
                }
                _ => panic!("expected Div, got {expr:?}"),
            },
            _ => panic!("expected Calc, got {val:?}"),
        }
    }

    #[test]
    fn calc_parenthesized() {
        // calc((10px + 5px) * 2)
        let val = length_or_pct("calc((10px + 5px) * 2)").unwrap();
        assert!(matches!(val, CssValue::Calc(_)));
    }

    #[test]
    fn calc_in_auto_context() {
        // calc() should work in length-percentage-or-auto context too
        let val = length_pct_auto("calc(50% + 10px)").unwrap();
        assert!(matches!(val, CssValue::Calc(_)));
    }

    #[test]
    fn calc_with_em_units() {
        let val = length_or_pct("calc(2em + 10px)").unwrap();
        match val {
            CssValue::Calc(expr) => match *expr {
                CalcExpr::Add(a, b) => {
                    assert_eq!(*a, CalcExpr::Length(2.0, LengthUnit::Em));
                    assert_eq!(*b, CalcExpr::Length(10.0, LengthUnit::Px));
                }
                _ => panic!("expected Add"),
            },
            _ => panic!("expected Calc"),
        }
    }

    // --- calc() type validation tests (CSS Values Level 3 §8.1.1) ---

    #[test]
    fn calc_rejects_length_times_length() {
        // <length> * <length> is invalid.
        assert!(length_or_pct("calc(10px * 5px)").is_err());
    }

    #[test]
    fn calc_rejects_divide_by_length() {
        // Divisor must be <number>.
        assert!(length_or_pct("calc(100px / 5px)").is_err());
    }

    #[test]
    fn calc_rejects_add_mixed_types() {
        // <length> + <number> is invalid.
        assert!(length_or_pct("calc(10px + 5)").is_err());
    }

    #[test]
    fn calc_allows_number_times_length() {
        // <number> * <length> is valid.
        assert!(length_or_pct("calc(3 * 10px)").is_ok());
    }

    #[test]
    fn calc_allows_length_plus_percentage() {
        // Both are dimensional — allowed in length-percentage contexts.
        assert!(length_or_pct("calc(10px + 50%)").is_ok());
    }

    #[test]
    fn calc_rejects_deeply_nested() {
        // Deeply nested parentheses must be rejected to prevent stack overflow.
        let deep = format!("calc({}1px{})", "(".repeat(40), ")".repeat(40));
        assert!(length_or_pct(&deep).is_err());
    }

    #[test]
    fn calc_rejects_pure_number_in_length_context() {
        // calc(1 + 2) evaluates to a pure <number>, invalid in length/percentage context.
        assert!(length_or_pct("calc(1 + 2)").is_err());
        assert!(length_or_pct("calc(3 * 4)").is_err());
    }

    #[test]
    fn calc_rejects_too_many_nodes() {
        // Long flat expressions exceeding MAX_CALC_NODES must be rejected.
        let terms: Vec<&str> = (0..300).map(|_| "1px").collect();
        let expr = format!("calc({})", terms.join(" + "));
        assert!(length_or_pct(&expr).is_err());
    }
}
