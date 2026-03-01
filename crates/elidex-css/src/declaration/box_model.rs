//! Box model property parsers: margin/padding shorthand, border shorthand/width.
//!
//! Shorthand expansion patterns:
//! - `expand_four_sides()`: Generic 1-4 value → 4 longhands (margin, padding). Takes a
//!   parse function, reuses it for each side.
//! - `parse_border_shorthand()`: Any-order multi-type (width/style/color in any order)
//!   → 12 longhands (4 sides x 3 properties).
//! - `parse_border_width_property()`: Single longhand with keyword fallback (thin/medium/thick).

use cssparser::Parser;
use elidex_plugin::{CssValue, LengthUnit};

use crate::color::parse_color;
use crate::values::parse_length_or_percentage;

use super::{try_parse_keyword, Declaration};

pub(super) const SIDES: &[&str] = &["top", "right", "bottom", "left"];

/// Expand a 1-4 value shorthand (margin, padding) into four longhand declarations.
pub(super) fn expand_four_sides(
    input: &mut Parser,
    prefix: &str,
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
            important: false,
        },
        Declaration {
            property: format!("{prefix}-right"),
            value: right,
            important: false,
        },
        Declaration {
            property: format!("{prefix}-bottom"),
            value: bottom,
            important: false,
        },
        Declaration {
            property: format!("{prefix}-left"),
            value: left,
            important: false,
        },
    ]
}

/// Parse a border-width keyword (`thin`, `medium`, `thick`) into pixel values.
fn parse_border_width_keyword(input: &mut Parser) -> Result<CssValue, ()> {
    let kw = try_parse_keyword(input, &["thin", "medium", "thick"]).map_err(|_| ())?;
    match kw.as_str() {
        "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
        "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
        "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
        _ => unreachable!(),
    }
}

pub(super) fn parse_border_width_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            // Try keyword first (thin/medium/thick), then fall back to length.
            let val = i
                .try_parse(parse_border_width_keyword)
                .or_else(|()| parse_length_or_percentage(i))?;
            Ok(super::single_decl(name, val))
        })
        .unwrap_or_default()
}

/// Parse border components: `[width] [style] [color]` in any order.
///
/// Returns `(width, style, color)` with CSS defaults for missing values,
/// or `None` if no component was recognised at all.
fn parse_border_components(input: &mut Parser) -> Option<(CssValue, CssValue, CssValue)> {
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
                let lower = ident.to_ascii_lowercase();
                match lower.as_str() {
                    "none" | "solid" | "dashed" | "dotted" => Ok(CssValue::Keyword(lower)),
                    _ => Err(()),
                }
            }) {
                style = Some(s);
                continue;
            }
        }

        // Try width (keyword or length).
        if width.is_none() {
            if let Ok(w) = input.try_parse(|i| {
                i.try_parse(parse_border_width_keyword)
                    .or_else(|()| parse_length_or_percentage(i))
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
        return None;
    }

    let w = width.unwrap_or(CssValue::Length(3.0, LengthUnit::Px)); // CSS default: medium
    let s = style.unwrap_or(CssValue::Keyword("none".into())); // CSS default: none
    let c = color.unwrap_or(CssValue::Keyword("currentcolor".into()));

    Some((w, s, c))
}

/// Parse the `border` shorthand: `[width] [style] [color]` in any order.
///
/// Produces 12 longhand declarations (4 sides x 3 properties).
pub(super) fn parse_border_shorthand(input: &mut Parser) -> Vec<Declaration> {
    let Some((w, s, c)) = parse_border_components(input) else {
        return Vec::new();
    };

    let mut decls = Vec::with_capacity(12);
    for side in SIDES {
        decls.push(Declaration {
            property: format!("border-{side}-width"),
            value: w.clone(),
            important: false,
        });
        decls.push(Declaration {
            property: format!("border-{side}-style"),
            value: s.clone(),
            important: false,
        });
        decls.push(Declaration {
            property: format!("border-{side}-color"),
            value: c.clone(),
            important: false,
        });
    }
    decls
}

/// Parse a `border-{side}` shorthand: `[width] [style] [color]` in any order.
///
/// Produces 3 longhand declarations for the given side.
pub(super) fn parse_border_side_shorthand(input: &mut Parser, side: &str) -> Vec<Declaration> {
    let Some((w, s, c)) = parse_border_components(input) else {
        return Vec::new();
    };

    vec![
        Declaration {
            property: format!("border-{side}-width"),
            value: w,
            important: false,
        },
        Declaration {
            property: format!("border-{side}-style"),
            value: s,
            important: false,
        },
        Declaration {
            property: format!("border-{side}-color"),
            value: c,
            important: false,
        },
    ]
}
