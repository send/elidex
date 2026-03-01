//! Typography property parsers: font-size, font-family.

use cssparser::{Parser, Token};
use elidex_plugin::CssValue;

use crate::values::parse_length_or_percentage;

use super::{parse_value_property, single_decl, try_parse_keyword, Declaration};

const FONT_SIZE_KEYWORDS: &[&str] = &[
    "xx-small", "xx-large", "x-small", "x-large", "small", "medium", "large", "smaller", "larger",
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
    // Try keyword first: normal (400), bold (700).
    if let Ok(kw) = input.try_parse(|i| try_parse_keyword(i, &["normal", "bold"]).map_err(|_| ())) {
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

    vec![Declaration {
        property: "font-family".to_string(),
        value: CssValue::List(families),
        important: false,
    }]
}
