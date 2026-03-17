//! Background position parsing.

use elidex_plugin::{CssValue, LengthUnit, ParseError, PropertyDeclaration};

/// Parse `background-position` value and return as a declaration.
pub(crate) fn parse_bg_position_declaration(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let first = parse_position_component(input)?;

    // Try second component
    let second = input.try_parse(parse_position_component).ok();

    let value = match second {
        Some(s) => CssValue::List(vec![first, s]),
        None => {
            // 1-value: first is x, y defaults to center (50%)
            CssValue::List(vec![first, CssValue::Percentage(50.0)])
        }
    };

    Ok(vec![PropertyDeclaration::new("background-position", value)])
}

fn parse_position_component(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try keyword
    if let Ok(ident) = input.try_parse(cssparser::Parser::expect_ident_cloned) {
        let lower = ident.to_ascii_lowercase();
        return match lower.as_str() {
            "left" | "top" => Ok(CssValue::Percentage(0.0)),
            "center" => Ok(CssValue::Percentage(50.0)),
            "right" | "bottom" => Ok(CssValue::Percentage(100.0)),
            _ => Err(ParseError {
                property: "background-position".into(),
                input: lower,
                message: "invalid position keyword".into(),
            }),
        };
    }

    // Try percentage
    if let Ok(pct) = input.try_parse(cssparser::Parser::expect_percentage) {
        return Ok(CssValue::Percentage(pct * 100.0));
    }

    // Try length
    let tok = input.next().map_err(|_| ParseError {
        property: "background-position".into(),
        input: String::new(),
        message: "expected position value".into(),
    })?;
    match tok {
        cssparser::Token::Dimension { value, unit, .. } => {
            let unit_lower = unit.to_ascii_lowercase();
            let lu = match unit_lower.as_str() {
                "em" => LengthUnit::Em,
                "rem" => LengthUnit::Rem,
                "vw" => LengthUnit::Vw,
                "vh" => LengthUnit::Vh,
                _ => LengthUnit::Px,
            };
            Ok(CssValue::Length(*value, lu))
        }
        cssparser::Token::Number { value, .. } if *value == 0.0 => {
            Ok(CssValue::Length(0.0, LengthUnit::Px))
        }
        _ => Err(ParseError {
            property: "background-position".into(),
            input: String::new(),
            message: "expected length or percentage".into(),
        }),
    }
}
