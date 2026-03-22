//! Parse helpers for CSS background properties.

use elidex_plugin::{CssValue, ParseError};

use super::gradient;

pub(crate) fn parse_bg_image(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try "none"
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }
    // Try url()
    if let Ok(url) = input.try_parse(cssparser::Parser::expect_url) {
        return Ok(CssValue::Url(url.as_ref().to_string()));
    }
    if let Ok(gradient) = gradient::parse_gradient(input) {
        return Ok(gradient);
    }
    Err(ParseError {
        property: "background-image".into(),
        input: String::new(),
        message: "expected none, url(), or gradient".into(),
    })
}

pub(crate) fn parse_bg_repeat(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let first = input.expect_ident().map_err(|_| ParseError {
        property: "background-repeat".into(),
        input: String::new(),
        message: "expected repeat keyword".into(),
    })?;
    let first_lower = first.to_ascii_lowercase();

    // Shorthand keywords
    match first_lower.as_str() {
        "repeat-x" => {
            return Ok(CssValue::List(vec![
                CssValue::Keyword("repeat".into()),
                CssValue::Keyword("no-repeat".into()),
            ]));
        }
        "repeat-y" => {
            return Ok(CssValue::List(vec![
                CssValue::Keyword("no-repeat".into()),
                CssValue::Keyword("repeat".into()),
            ]));
        }
        _ => {}
    }

    let valid = ["repeat", "no-repeat", "space", "round"];
    if !valid.contains(&first_lower.as_str()) {
        return Err(ParseError {
            property: "background-repeat".into(),
            input: first_lower,
            message: "invalid repeat keyword".into(),
        });
    }

    // Try second keyword
    let second = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        let lower = ident.to_ascii_lowercase();
        if valid.contains(&lower.as_str()) {
            Ok(lower)
        } else {
            Err(())
        }
    });

    match second {
        Ok(s) => Ok(CssValue::List(vec![
            CssValue::Keyword(first_lower),
            CssValue::Keyword(s),
        ])),
        Err(()) => {
            // 1-value: same for both axes
            Ok(CssValue::Keyword(first_lower))
        }
    }
}

pub(crate) fn parse_box_keyword(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    elidex_plugin::parse_css_keyword(input, &["border-box", "padding-box", "content-box"])
}

pub(crate) fn parse_attachment(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    elidex_plugin::parse_css_keyword(input, &["scroll", "fixed", "local"])
}

pub(crate) fn parse_bg_size(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // cover | contain
    if input
        .try_parse(|i| i.expect_ident_matching("cover"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("cover".into()));
    }
    if input
        .try_parse(|i| i.expect_ident_matching("contain"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("contain".into()));
    }
    // auto | <length-percentage>
    let first = parse_size_value(input)?;
    let second = input.try_parse(parse_size_value).ok();
    match second {
        Some(s) => Ok(CssValue::List(vec![first, s])),
        None => Ok(first),
    }
}

pub(crate) fn parse_size_value(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    elidex_plugin::css_resolve::parse_non_negative_length_or_percentage(input)
}
