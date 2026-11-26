//! Flex property parsers: flex-grow/shrink, order, flex-basis, flex/flex-flow shorthands.

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use elidex_plugin::css_resolve::{parse_flex_basis, parse_flex_factor};

use super::{single_decl, Declaration};

/// Build the three longhand declarations for the `flex` shorthand.
fn flex_triple(grow: CssValue, shrink: CssValue, basis: CssValue) -> Vec<Declaration> {
    vec![
        Declaration::new("flex-grow", grow),
        Declaration::new("flex-shrink", shrink),
        Declaration::new("flex-basis", basis),
    ]
}

/// Parse an integer property (order).
pub(super) fn parse_integer_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let tok = i.next().map_err(|_| ())?;
            match *tok {
                Token::Number {
                    int_value: Some(n), ..
                } =>
                {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(single_decl(name, CssValue::Number(n as f32)))
                }
                _ => Err(()),
            }
        })
        .unwrap_or_default()
}

/// CSS Flexbox 1 §7.1: the grow/shrink pair stays adjacent on either side of the basis.
pub(super) fn parse_flex_shorthand(input: &mut Parser) -> Vec<Declaration> {
    if input.try_parse(|p| p.expect_ident_matching("none")).is_ok() {
        return flex_triple(CssValue::Number(0.0), CssValue::Number(0.0), CssValue::Auto);
    }
    let one = CssValue::Number(1.0);
    if let Ok(grow) = input.try_parse(parse_flex_factor) {
        let shrink = input
            .try_parse(parse_flex_factor)
            .unwrap_or_else(|()| one.clone());
        let basis = input
            .try_parse(parse_flex_basis)
            .unwrap_or(CssValue::Length(0.0, LengthUnit::Px));
        return flex_triple(grow, shrink, basis);
    }
    let Ok(basis) = input.try_parse(parse_flex_basis) else {
        return Vec::new();
    };
    let grow = input
        .try_parse(parse_flex_factor)
        .unwrap_or_else(|()| one.clone());
    let shrink = input.try_parse(parse_flex_factor).unwrap_or(one);
    flex_triple(grow, shrink, basis)
}

/// Parse the `flex-flow` shorthand: `<direction> || <wrap>`.
pub(super) fn parse_flex_flow_shorthand(input: &mut Parser) -> Vec<Declaration> {
    let direction_keywords = ["row", "row-reverse", "column", "column-reverse"];
    let wrap_keywords = ["nowrap", "wrap", "wrap-reverse"];

    let mut direction: Option<CssValue> = None;
    let mut wrap: Option<CssValue> = None;

    for _ in 0..2 {
        if input.is_exhausted() {
            break;
        }
        if let Ok(kw) = input.try_parse(|i| -> Result<(String, bool), ()> {
            let ident = i.expect_ident().map_err(|_| ())?;
            let lower = ident.to_ascii_lowercase();
            if direction.is_none() && direction_keywords.iter().any(|&k| k == lower) {
                Ok((lower, true))
            } else if wrap.is_none() && wrap_keywords.iter().any(|&k| k == lower) {
                Ok((lower, false))
            } else {
                Err(())
            }
        }) {
            if kw.1 {
                direction = Some(CssValue::Keyword(kw.0));
            } else {
                wrap = Some(CssValue::Keyword(kw.0));
            }
        } else {
            break;
        }
    }

    if direction.is_none() && wrap.is_none() {
        return Vec::new();
    }

    vec![
        Declaration::new(
            "flex-direction",
            direction.unwrap_or(CssValue::Keyword("row".into())),
        ),
        Declaration::new(
            "flex-wrap",
            wrap.unwrap_or(CssValue::Keyword("nowrap".into())),
        ),
    ]
}
