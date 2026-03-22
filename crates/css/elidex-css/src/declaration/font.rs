//! Typography property parsers: font-size, font-weight, font-family, line-height,
//! and the `font` shorthand (CSS Fonts Level 3 section 4).

use cssparser::{Parser, Token};
use elidex_plugin::CssValue;

use crate::values::parse_length_or_percentage;

use super::{parse_value_property, single_decl, try_parse_keyword, Declaration};

const FONT_SIZE_KEYWORDS: &[&str] = &[
    "xx-small",
    "xx-large",
    "xxx-large",
    "x-small",
    "x-large",
    "small",
    "medium",
    "large",
    "smaller",
    "larger",
];

pub(super) fn parse_font_size(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword sizes first.
    if let Ok(kw) = input.try_parse(|i| try_parse_keyword(i, FONT_SIZE_KEYWORDS).map_err(|_| ())) {
        return single_decl("font-size", CssValue::Keyword(kw));
    }
    // Fall back to length/percentage.
    parse_value_property(input, "font-size", parse_length_or_percentage)
}

pub(super) fn parse_font_weight(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword first: normal (400), bold (700), bolder/lighter (relative).
    if let Ok(kw) = input.try_parse(|i| {
        try_parse_keyword(i, &["normal", "bold", "bolder", "lighter"]).map_err(|_| ())
    }) {
        return single_decl("font-weight", CssValue::Keyword(kw));
    }
    // Try numeric weight (100-900).
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let tok = i.next().map_err(|_| ())?;
            let Token::Number { value: n, .. } = *tok else {
                return Err(());
            };
            // CSS font-weight accepts 1-1000; we support the common 100-900 range.
            if !(1.0..=1000.0).contains(&n) {
                return Err(());
            }
            Ok(single_decl("font-weight", CssValue::Number(n)))
        })
        .unwrap_or_default()
}

pub(super) fn parse_line_height(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword: normal.
    if let Ok(kw) = input.try_parse(|i| try_parse_keyword(i, &["normal"]).map_err(|_| ())) {
        return single_decl("line-height", CssValue::Keyword(kw));
    }
    // Try unitless number (e.g. 1.5).
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let tok = i.next().map_err(|_| ())?;
        if let Token::Number { value, .. } = *tok {
            if value >= 0.0 {
                return Ok(single_decl("line-height", CssValue::Number(value)));
            }
        }
        Err(())
    }) {
        return decls;
    }
    // Fall back to length/percentage.
    parse_value_property(input, "line-height", parse_length_or_percentage)
}

/// Parse the `font` shorthand property (CSS Fonts Level 3 section 4).
///
/// Syntax: `[<font-style> || <font-weight>]? <font-size> [/ <line-height>]? <font-family>#`
///
/// Omitted optional components are reset to their initial values:
/// - font-style: normal
/// - font-weight: normal
/// - line-height: normal
pub(super) fn parse_font_shorthand(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let mut style: Option<CssValue> = None;
            let mut weight: Option<CssValue> = None;

            // Try to parse optional font-style and/or font-weight (in any order, at most once each).
            for _ in 0..2 {
                if style.is_none() {
                    if let Ok(kw) = i.try_parse(|i2| {
                        try_parse_keyword(i2, &["italic", "oblique"]).map_err(|_| ())
                    }) {
                        style = Some(CssValue::Keyword(kw));
                        continue;
                    }
                }
                if weight.is_none() {
                    if let Ok(kw) = i.try_parse(|i2| {
                        try_parse_keyword(i2, &["bold", "bolder", "lighter"]).map_err(|_| ())
                    }) {
                        weight = Some(CssValue::Keyword(kw));
                        continue;
                    }
                    // Try numeric weight (100-900).
                    if let Ok(n) = i.try_parse(|i2| -> Result<f32, ()> {
                        let tok = i2.next().map_err(|_| ())?;
                        let Token::Number { value: n, .. } = *tok else {
                            return Err(());
                        };
                        if (1.0..=1000.0).contains(&n) {
                            Ok(n)
                        } else {
                            Err(())
                        }
                    }) {
                        weight = Some(CssValue::Number(n));
                        continue;
                    }
                }
                // "normal" can be either font-style or font-weight; consume it for whichever
                // slot is still empty (prefer style first, matching spec grammar order).
                if (style.is_none() || weight.is_none())
                    && i.try_parse(|i2| try_parse_keyword(i2, &["normal"]).map_err(|_| ()))
                        .is_ok()
                {
                    // "normal" is the initial value — just mark the slot as explicitly set.
                    if style.is_none() {
                        style = Some(CssValue::Keyword("normal".into()));
                    } else {
                        weight = Some(CssValue::Keyword("normal".into()));
                    }
                    continue;
                }
                break;
            }

            // Required: font-size.
            let font_size = parse_font_size_value(i)?;

            // Optional: / line-height.
            let line_height = if i
                .try_parse(|i2| -> Result<(), ()> {
                    let tok = i2.next().map_err(|_| ())?;
                    if matches!(tok, Token::Delim('/')) {
                        Ok(())
                    } else {
                        Err(())
                    }
                })
                .is_ok()
            {
                Some(parse_line_height_value(i)?)
            } else {
                None
            };

            // Required: font-family (comma-separated list, must not be empty).
            let families = parse_font_family_list(i)?;

            Ok(vec![
                Declaration::new(
                    "font-style",
                    style.unwrap_or(CssValue::Keyword("normal".into())),
                ),
                Declaration::new(
                    "font-weight",
                    weight.unwrap_or(CssValue::Keyword("normal".into())),
                ),
                Declaration::new("font-size", font_size),
                Declaration::new(
                    "line-height",
                    line_height.unwrap_or(CssValue::Keyword("normal".into())),
                ),
                Declaration::new("font-family", families),
            ])
        })
        .unwrap_or_default()
}

/// Parse a font-size value (keyword or length/percentage) without wrapping in a Declaration.
fn parse_font_size_value(input: &mut Parser) -> Result<CssValue, ()> {
    if let Ok(kw) = input.try_parse(|i| try_parse_keyword(i, FONT_SIZE_KEYWORDS).map_err(|_| ())) {
        return Ok(CssValue::Keyword(kw));
    }
    parse_length_or_percentage(input)
}

/// Parse a line-height value (keyword `normal`, unitless number, or length/percentage).
fn parse_line_height_value(input: &mut Parser) -> Result<CssValue, ()> {
    if let Ok(kw) = input.try_parse(|i| try_parse_keyword(i, &["normal"]).map_err(|_| ())) {
        return Ok(CssValue::Keyword(kw));
    }
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let tok = i.next().map_err(|_| ())?;
        if let Token::Number { value, .. } = *tok {
            if value >= 0.0 {
                return Ok(CssValue::Number(value));
            }
        }
        Err(())
    }) {
        return Ok(val);
    }
    parse_length_or_percentage(input)
}

/// Parse a comma-separated font-family list, returning `CssValue::List`.
/// Returns `Err(())` if the list is empty.
fn parse_font_family_list(input: &mut Parser) -> Result<CssValue, ()> {
    let mut families = Vec::new();

    loop {
        if input.is_exhausted() {
            break;
        }

        let family = input.try_parse(|i| -> Result<CssValue, ()> {
            let tok = i.next().map_err(|_| ())?;
            match tok {
                Token::Ident(ref name) => {
                    let mut full_name = name.as_ref().to_string();
                    while let Ok(part) = i.try_parse(|i2| -> Result<String, ()> {
                        let ident = i2.expect_ident().map_err(|_| ())?;
                        Ok(ident.as_ref().to_string())
                    }) {
                        full_name.push(' ');
                        full_name.push_str(&part);
                    }
                    Ok(CssValue::Keyword(full_name))
                }
                Token::QuotedString(ref s) => Ok(CssValue::String(s.as_ref().to_string())),
                _ => Err(()),
            }
        });

        match family {
            Ok(f) => families.push(f),
            Err(()) => break,
        }

        if input
            .try_parse(|i| i.expect_comma().map_err(|_| ()))
            .is_err()
        {
            break;
        }
    }

    if families.is_empty() {
        return Err(());
    }

    Ok(CssValue::List(families))
}

pub(super) fn parse_font_family(input: &mut Parser) -> Vec<Declaration> {
    let mut families = Vec::new();

    loop {
        if input.is_exhausted() {
            break;
        }

        let family = input.try_parse(|i| -> Result<CssValue, ()> {
            let tok = i.next().map_err(|_| ())?;
            match tok {
                Token::Ident(ref name) => {
                    // Unquoted font family names can be multi-word (e.g. "Times New Roman").
                    // Greedily consume consecutive identifiers, joining with spaces.
                    let mut full_name = name.as_ref().to_string();
                    while let Ok(part) = i.try_parse(|i2| -> Result<String, ()> {
                        let ident = i2.expect_ident().map_err(|_| ())?;
                        Ok(ident.as_ref().to_string())
                    }) {
                        full_name.push(' ');
                        full_name.push_str(&part);
                    }
                    Ok(CssValue::Keyword(full_name))
                }
                Token::QuotedString(ref s) => Ok(CssValue::String(s.as_ref().to_string())),
                _ => Err(()),
            }
        });

        match family {
            Ok(f) => families.push(f),
            Err(()) => break,
        }

        // Skip comma.
        if input
            .try_parse(|i| i.expect_comma().map_err(|_| ()))
            .is_err()
        {
            break;
        }
    }

    if families.is_empty() {
        return Vec::new();
    }

    vec![Declaration::new("font-family", CssValue::List(families))]
}
