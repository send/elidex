//! CSS value parsing utilities.
//!
//! Provides parsers for lengths, percentages, keywords, and the `auto` value
//! used across property declaration parsing.

use cssparser::Parser;
use elidex_plugin::{CssValue, LengthUnit};

/// Parse a CSS length value (e.g. `10px`, `2em`, `0`).
///
/// Unitless `0` is treated as `0px` per CSS specification.
#[allow(clippy::result_unit_err)]
pub fn parse_length(input: &mut Parser) -> Result<CssValue, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let u = parse_length_unit(unit)?;
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Parse a length or percentage value.
#[allow(clippy::result_unit_err)]
pub fn parse_length_or_percentage(input: &mut Parser) -> Result<CssValue, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let u = parse_length_unit(unit)?;
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Parse a length, percentage, or `auto` keyword.
#[allow(clippy::result_unit_err)]
pub fn parse_length_percentage_or_auto(input: &mut Parser) -> Result<CssValue, ()> {
    if input
        .try_parse(|i| {
            let ident = i.expect_ident().map_err(|_| ())?;
            if ident.eq_ignore_ascii_case("auto") {
                Ok(CssValue::Auto)
            } else {
                Err(())
            }
        })
        .is_ok()
    {
        return Ok(CssValue::Auto);
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
fn parse_length_unit(unit: &str) -> Result<LengthUnit, ()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn length(css: &str) -> Result<CssValue, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_length(&mut parser)
    }

    fn lp(css: &str) -> Result<CssValue, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_length_or_percentage(&mut parser)
    }

    fn lpa(css: &str) -> Result<CssValue, ()> {
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
        assert_eq!(lp("50%"), Ok(CssValue::Percentage(50.0)));
    }

    #[test]
    fn parse_auto_keyword() {
        assert_eq!(lpa("auto"), Ok(CssValue::Auto));
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
}
