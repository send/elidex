//! CSS declaration parsing and shorthand expansion.
//!
//! Parses property-value pairs into [`Declaration`] structs, expanding
//! shorthand properties (`margin`, `padding`, `border`) into their
//! longhand equivalents.

use cssparser::{Parser, ParserInput, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::color::parse_color;
use crate::values::{
    parse_global_keyword, parse_length_or_percentage, parse_length_percentage_or_auto,
};

/// The origin of a stylesheet in the cascade.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Origin {
    /// Browser default styles.
    #[default]
    UserAgent = 0,
    /// Author (page) styles.
    Author = 1,
}

/// A single CSS declaration (property-value pair).
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// Property name (always lowercase longhand).
    pub property: String,
    /// Parsed value.
    pub value: CssValue,
    /// Whether this declaration has `!important`.
    pub important: bool,
}

/// Parse an inline style attribute string into declarations.
///
/// Shorthand properties are expanded into their longhand equivalents.
pub fn parse_declaration_block(css: &str) -> Vec<Declaration> {
    let mut pi = ParserInput::new(css);
    let mut input = Parser::new(&mut pi);
    let mut declarations = Vec::new();

    while !input.is_exhausted() {
        let result: Result<Vec<Declaration>, ()> = input.try_parse(|i| {
            let name = i
                .expect_ident()
                .map_err(|_| ())?
                .as_ref()
                .to_ascii_lowercase();
            i.expect_colon().map_err(|_| ())?;
            let mut decls = parse_property_value(&name, i, false);
            // Check for !important (browsers support this in inline styles).
            if i.try_parse(cssparser::parse_important).is_ok() {
                for d in &mut decls {
                    d.important = true;
                }
            }
            Ok(decls)
        });

        if let Ok(decls) = result {
            declarations.extend(decls);
        } else {
            // Skip tokens until next semicolon or end.
            skip_to_semicolon(&mut input);
            continue;
        }

        // Consume optional semicolon.
        let _ = input.try_parse(|i| i.expect_semicolon().map_err(|_| ()));
    }

    declarations
}

/// Skip tokens until we find a semicolon or exhaust input.
fn skip_to_semicolon(input: &mut Parser) {
    while !input.is_exhausted() {
        match input.next() {
            Ok(&Token::Semicolon) | Err(_) => break,
            _ => {} // consume and continue
        }
    }
}

/// Parse a property value and return longhand declarations.
///
/// Shorthand properties are expanded into their longhand equivalents.
/// The `important` flag is applied to all generated declarations.
///
/// **Contract:** Returns an empty `Vec` for both unknown properties and
/// known properties with unparseable values. The caller (e.g.
/// `DeclarationListParser`) treats an empty result as an error, which
/// triggers cssparser's standard error recovery (skip the declaration).
pub(crate) fn parse_property_value(
    name: &str,
    input: &mut Parser,
    important: bool,
) -> Vec<Declaration> {
    // Check for global keywords first.
    if let Ok(val) = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        parse_global_keyword(ident.as_ref()).ok_or(())
    }) {
        // Shorthand properties must expand global keywords into longhand declarations.
        return expand_global_keyword(name, val, important);
    }

    match name {
        // --- Shorthand properties ---
        "margin" => expand_four_sides(input, "margin", important, parse_margin_value),
        "padding" => expand_four_sides(input, "padding", important, parse_padding_value),
        "border" => parse_border_shorthand(input, important),

        // --- Keyword properties ---
        "display" => parse_keyword_property(
            input,
            name,
            important,
            &["block", "inline", "inline-block", "none", "flex"],
        ),
        "position" => parse_keyword_property(
            input,
            name,
            important,
            &["static", "relative", "absolute", "fixed"],
        ),
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            parse_keyword_property(
                input,
                name,
                important,
                &["none", "solid", "dashed", "dotted"],
            )
        }

        // --- Color properties ---
        "color"
        | "background-color"
        | "border-top-color"
        | "border-right-color"
        | "border-bottom-color"
        | "border-left-color" => parse_color_property(input, name, important),

        // --- Length/percentage/auto properties ---
        "width" | "height" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            parse_lpa_property(input, name, important)
        }

        // --- Length/percentage properties (no auto) ---
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            parse_lp_property(input, name, important)
        }

        // --- Border width ---
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            parse_border_width_property(input, name, important)
        }

        // --- Font properties ---
        "font-size" => parse_font_size(input, important),
        "font-family" => parse_font_family(input, important),

        // --- Unknown property: silently drop ---
        _ => Vec::new(),
    }
}

// --- Property-specific parsers ---

fn parse_keyword_property(
    input: &mut Parser,
    name: &str,
    important: bool,
    allowed: &[&str],
) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let ident = i.expect_ident().map_err(|_| ())?;
            let lower = ident.to_ascii_lowercase();
            if allowed.iter().any(|a| *a == lower) {
                Ok(vec![Declaration {
                    property: name.to_string(),
                    value: CssValue::Keyword(lower),
                    important,
                }])
            } else {
                Err(())
            }
        })
        .unwrap_or_default()
}

fn parse_color_property(input: &mut Parser, name: &str, important: bool) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let color = parse_color(i)?;
            Ok(vec![Declaration {
                property: name.to_string(),
                value: CssValue::Color(color),
                important,
            }])
        })
        .unwrap_or_default()
}

fn parse_lpa_property(input: &mut Parser, name: &str, important: bool) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = parse_length_percentage_or_auto(i)?;
            Ok(vec![Declaration {
                property: name.to_string(),
                value: val,
                important,
            }])
        })
        .unwrap_or_default()
}

fn parse_lp_property(input: &mut Parser, name: &str, important: bool) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = parse_length_or_percentage(i)?;
            Ok(vec![Declaration {
                property: name.to_string(),
                value: val,
                important,
            }])
        })
        .unwrap_or_default()
}

fn parse_border_width_property(
    input: &mut Parser,
    name: &str,
    important: bool,
) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            // Try keyword first (thin/medium/thick).
            if let Ok(val) = i.try_parse(|i2| -> Result<CssValue, ()> {
                let ident = i2.expect_ident().map_err(|_| ())?;
                match ident.to_ascii_lowercase().as_str() {
                    "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
                    "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
                    "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
                    _ => Err(()),
                }
            }) {
                return Ok(vec![Declaration {
                    property: name.to_string(),
                    value: val,
                    important,
                }]);
            }
            // Fall back to length.
            let val = parse_length_or_percentage(i)?;
            Ok(vec![Declaration {
                property: name.to_string(),
                value: val,
                important,
            }])
        })
        .unwrap_or_default()
}

fn parse_font_size(input: &mut Parser, important: bool) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            // Try keyword sizes first.
            if let Ok(val) = i.try_parse(|i2| -> Result<CssValue, ()> {
                let ident = i2.expect_ident().map_err(|_| ())?;
                match ident.to_ascii_lowercase().as_str() {
                    "xx-small" | "xx-large" | "x-small" | "x-large" | "small" | "medium"
                    | "large" | "smaller" | "larger" => {
                        Ok(CssValue::Keyword(ident.to_ascii_lowercase()))
                    }
                    _ => Err(()),
                }
            }) {
                return Ok(vec![Declaration {
                    property: "font-size".to_string(),
                    value: val,
                    important,
                }]);
            }
            let val = parse_length_or_percentage(i)?;
            Ok(vec![Declaration {
                property: "font-size".to_string(),
                value: val,
                important,
            }])
        })
        .unwrap_or_default()
}

fn parse_font_family(input: &mut Parser, important: bool) -> Vec<Declaration> {
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
        important,
    }]
}

// --- Shorthand expansion helpers ---

/// Expand a global keyword (inherit/initial/unset) for shorthand properties into
/// their longhand equivalents. Longhand properties produce a single declaration.
fn expand_global_keyword(name: &str, val: CssValue, important: bool) -> Vec<Declaration> {
    let longhands: &[&str] = match name {
        "margin" => &["margin-top", "margin-right", "margin-bottom", "margin-left"],
        "padding" => &[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
        "border" => &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
        // Longhand properties: single declaration.
        _ => {
            return vec![Declaration {
                property: name.to_string(),
                value: val,
                important,
            }];
        }
    };
    longhands
        .iter()
        .map(|p| Declaration {
            property: (*p).to_string(),
            value: val.clone(),
            important,
        })
        .collect()
}

fn parse_margin_value(input: &mut Parser) -> Result<CssValue, ()> {
    parse_length_percentage_or_auto(input)
}

fn parse_padding_value(input: &mut Parser) -> Result<CssValue, ()> {
    parse_length_or_percentage(input)
}

/// Expand a 1–4 value shorthand (margin, padding) into four longhand declarations.
fn expand_four_sides(
    input: &mut Parser,
    prefix: &str,
    important: bool,
    parse_fn: fn(&mut Parser) -> Result<CssValue, ()>,
) -> Vec<Declaration> {
    let mut values = Vec::new();
    for _ in 0..4 {
        if let Ok(v) = input.try_parse(parse_fn) {
            values.push(v);
        } else {
            break;
        }
    }

    if values.is_empty() {
        return Vec::new();
    }

    let (top, right, bottom, left) = match values.len() {
        1 => (
            values[0].clone(),
            values[0].clone(),
            values[0].clone(),
            values[0].clone(),
        ),
        2 => (
            values[0].clone(),
            values[1].clone(),
            values[0].clone(),
            values[1].clone(),
        ),
        3 => (
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[1].clone(),
        ),
        _ => (
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[3].clone(),
        ),
    };

    vec![
        Declaration {
            property: format!("{prefix}-top"),
            value: top,
            important,
        },
        Declaration {
            property: format!("{prefix}-right"),
            value: right,
            important,
        },
        Declaration {
            property: format!("{prefix}-bottom"),
            value: bottom,
            important,
        },
        Declaration {
            property: format!("{prefix}-left"),
            value: left,
            important,
        },
    ]
}

/// Parse the `border` shorthand: `[width] [style] [color]` in any order.
///
/// Produces 12 longhand declarations (4 sides x 3 properties).
fn parse_border_shorthand(input: &mut Parser, important: bool) -> Vec<Declaration> {
    let mut width: Option<CssValue> = None;
    let mut style: Option<CssValue> = None;
    let mut color: Option<CssValue> = None;

    // Parse up to 3 components in any order.
    for _ in 0..3 {
        if input.is_exhausted() {
            break;
        }

        // Try style keyword first (most distinctive).
        if style.is_none() {
            if let Ok(s) = input.try_parse(|i| {
                let ident = i.expect_ident().map_err(|_| ())?;
                match ident.to_ascii_lowercase().as_str() {
                    "none" | "solid" | "dashed" | "dotted" => {
                        Ok(CssValue::Keyword(ident.to_ascii_lowercase()))
                    }
                    _ => Err(()),
                }
            }) {
                style = Some(s);
                continue;
            }
        }

        // Try width (length or keyword).
        if width.is_none() {
            if let Ok(w) = input.try_parse(|i| {
                // Keyword widths.
                if let Ok(v) = i.try_parse(|i2| {
                    let ident = i2.expect_ident().map_err(|_| ())?;
                    match ident.to_ascii_lowercase().as_str() {
                        "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
                        "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
                        "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
                        _ => Err(()),
                    }
                }) {
                    return Ok(v);
                }
                parse_length_or_percentage(i)
            }) {
                width = Some(w);
                continue;
            }
        }

        // Try color.
        if color.is_none() {
            if let Ok(c) = input.try_parse(parse_color) {
                color = Some(CssValue::Color(c));
                continue;
            }
        }

        // Nothing matched — stop.
        break;
    }

    if width.is_none() && style.is_none() && color.is_none() {
        return Vec::new();
    }

    let w = width.unwrap_or(CssValue::Length(3.0, LengthUnit::Px)); // CSS default: medium
    let s = style.unwrap_or(CssValue::Keyword("none".into())); // CSS default: none
    let c = color.unwrap_or(CssValue::Keyword("currentcolor".into()));

    let sides = ["top", "right", "bottom", "left"];
    let mut decls = Vec::with_capacity(12);
    for side in &sides {
        decls.push(Declaration {
            property: format!("border-{side}-width"),
            value: w.clone(),
            important,
        });
        decls.push(Declaration {
            property: format!("border-{side}-style"),
            value: s.clone(),
            important,
        });
        decls.push(Declaration {
            property: format!("border-{side}-color"),
            value: c.clone(),
            important,
        });
    }
    decls
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, LengthUnit};

    fn parse_decls(css: &str) -> Vec<Declaration> {
        parse_declaration_block(css)
    }

    fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
        parse_decls(&format!("{property}: {value}"))
    }

    #[test]
    fn parse_display_block() {
        let decls = parse_single("display", "block");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].property, "display");
        assert_eq!(decls[0].value, CssValue::Keyword("block".into()));
    }

    #[test]
    fn parse_color_named() {
        let decls = parse_single("color", "red");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn parse_color_hex() {
        let decls = parse_single("color", "#ff0000");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn parse_background_color() {
        let decls = parse_single("background-color", "blue");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Color(CssColor::BLUE));
    }

    #[test]
    fn parse_font_size_px() {
        let decls = parse_single("font-size", "16px");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Length(16.0, LengthUnit::Px));
    }

    #[test]
    fn parse_font_family_list() {
        let decls = parse_single("font-family", "Arial, sans-serif");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].property, "font-family");
        match &decls[0].value {
            CssValue::List(items) => {
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn parse_font_family_multiword_unquoted() {
        let decls = parse_single("font-family", "Times New Roman, sans-serif");
        assert_eq!(decls.len(), 1);
        match &decls[0].value {
            CssValue::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], CssValue::Keyword("Times New Roman".into()));
                assert_eq!(items[1], CssValue::Keyword("sans-serif".into()));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn parse_width_auto() {
        let decls = parse_single("width", "auto");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_important_flag() {
        let decls = parse_decls("color: red");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
        assert!(!decls[0].important);
    }

    #[test]
    fn parse_inline_important() {
        // Browsers support !important in inline styles.
        let decls = parse_decls("color: red !important");
        assert_eq!(decls.len(), 1);
        assert!(decls[0].important);
        assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn expand_margin_one() {
        let decls = parse_single("margin", "10px");
        assert_eq!(decls.len(), 4);
        for d in &decls {
            assert_eq!(d.value, CssValue::Length(10.0, LengthUnit::Px));
        }
        assert_eq!(decls[0].property, "margin-top");
        assert_eq!(decls[1].property, "margin-right");
        assert_eq!(decls[2].property, "margin-bottom");
        assert_eq!(decls[3].property, "margin-left");
    }

    #[test]
    fn expand_margin_two() {
        let decls = parse_single("margin", "10px 20px");
        assert_eq!(decls.len(), 4);
        assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px)); // top
        assert_eq!(decls[1].value, CssValue::Length(20.0, LengthUnit::Px)); // right
        assert_eq!(decls[2].value, CssValue::Length(10.0, LengthUnit::Px)); // bottom
        assert_eq!(decls[3].value, CssValue::Length(20.0, LengthUnit::Px)); // left
    }

    #[test]
    fn expand_margin_four() {
        let decls = parse_single("margin", "1px 2px 3px 4px");
        assert_eq!(decls.len(), 4);
        assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
        assert_eq!(decls[1].value, CssValue::Length(2.0, LengthUnit::Px));
        assert_eq!(decls[2].value, CssValue::Length(3.0, LengthUnit::Px));
        assert_eq!(decls[3].value, CssValue::Length(4.0, LengthUnit::Px));
    }

    #[test]
    fn expand_padding() {
        let decls = parse_single("padding", "5px 10px");
        assert_eq!(decls.len(), 4);
        assert_eq!(decls[0].property, "padding-top");
        assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
        assert_eq!(decls[1].property, "padding-right");
        assert_eq!(decls[1].value, CssValue::Length(10.0, LengthUnit::Px));
    }

    #[test]
    fn global_keyword_expands_margin_shorthand() {
        let decls = parse_single("margin", "inherit");
        assert_eq!(decls.len(), 4);
        assert_eq!(decls[0].property, "margin-top");
        assert_eq!(decls[0].value, CssValue::Inherit);
        assert_eq!(decls[1].property, "margin-right");
        assert_eq!(decls[2].property, "margin-bottom");
        assert_eq!(decls[3].property, "margin-left");
    }

    #[test]
    fn global_keyword_expands_border_shorthand() {
        let decls = parse_single("border", "initial");
        assert_eq!(decls.len(), 12);
        assert_eq!(decls[0].property, "border-top-width");
        assert_eq!(decls[0].value, CssValue::Initial);
    }

    #[test]
    fn global_keyword_longhand_unchanged() {
        let decls = parse_single("color", "inherit");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].property, "color");
        assert_eq!(decls[0].value, CssValue::Inherit);
    }

    #[test]
    fn expand_border() {
        let decls = parse_single("border", "1px solid black");
        assert_eq!(decls.len(), 12);
        // Check first side (top).
        assert_eq!(decls[0].property, "border-top-width");
        assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
        assert_eq!(decls[1].property, "border-top-style");
        assert_eq!(decls[1].value, CssValue::Keyword("solid".into()));
        assert_eq!(decls[2].property, "border-top-color");
        assert_eq!(decls[2].value, CssValue::Color(CssColor::BLACK));
    }

    #[test]
    fn unknown_property_skipped() {
        let decls = parse_single("-webkit-xxx", "value");
        assert!(decls.is_empty());
    }

    #[test]
    fn multiple_declarations() {
        let decls = parse_decls("color: red; display: block; width: 100px");
        assert_eq!(decls.len(), 3);
        assert_eq!(decls[0].property, "color");
        assert_eq!(decls[1].property, "display");
        assert_eq!(decls[2].property, "width");
    }

    #[test]
    fn global_keyword_inherit() {
        let decls = parse_single("color", "inherit");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].value, CssValue::Inherit);
    }
}
