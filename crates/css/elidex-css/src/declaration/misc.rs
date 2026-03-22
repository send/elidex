//! Miscellaneous property parsers (border-radius, opacity, text-decoration,
//! text-align, gap, overflow, max-dimension, list-style, content,
//! border-spacing, background).

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::values::{parse_length_or_percentage, parse_non_negative_length_or_percentage};

use super::{parse_color_property, parse_value_property, try_keyword_value, try_parse_keyword};
use super::{single_decl, Declaration};

// --- Border radius parsing ---

/// Parse `border-radius` shorthand into 4 longhand declarations.
///
/// CSS Backgrounds and Borders Level 3 §5.3:
/// - 1 value: all corners
/// - 2 values: top-left+bottom-right / top-right+bottom-left
/// - 3 values: top-left / top-right+bottom-left / bottom-right
/// - 4 values: top-left / top-right / bottom-right / bottom-left
///
/// Percentages are rejected (require box dimensions for resolution).
/// Negative values are rejected per spec.
pub(super) fn parse_border_radius(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let mut values = Vec::with_capacity(4);
            for _ in 0..4 {
                match i.try_parse(parse_non_negative_length_or_percentage) {
                    Ok(val) => {
                        if matches!(val, CssValue::Percentage(_)) {
                            return Err(());
                        }
                        values.push(val);
                    }
                    Err(()) => break,
                }
            }
            if values.is_empty() {
                return Err(());
            }
            // Expand 1-4 values per CSS shorthand rules.
            let (tl, tr, br, bl) = match values.len() {
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
            Ok(vec![
                Declaration::new("border-top-left-radius", tl),
                Declaration::new("border-top-right-radius", tr),
                Declaration::new("border-bottom-right-radius", br),
                Declaration::new("border-bottom-left-radius", bl),
            ])
        })
        .unwrap_or_default()
}

// --- Opacity parsing ---

/// Parse `opacity` as a number (0.0–1.0) or percentage, clamping out-of-range values.
///
/// Per CSS Color Level 4 §11.2: `<alpha-value> = <number> | <percentage>`.
pub(super) fn parse_opacity(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let token = i.next().map_err(|_| ())?;
            let val = match *token {
                cssparser::Token::Number { value, .. } => value.clamp(0.0, 1.0),
                cssparser::Token::Percentage { unit_value, .. } => unit_value.clamp(0.0, 1.0),
                _ => return Err(()),
            };
            Ok(single_decl("opacity", CssValue::Number(val)))
        })
        .unwrap_or_default()
}

// --- Text decoration parsing ---

/// Parse `text-decoration-line` longhand.
///
/// Accepts `none`, or one or more of `underline`, `overline`, `line-through` (space-separated).
/// The `text-decoration` shorthand is parsed separately by `parse_text_decoration_shorthand`
/// which decomposes into `text-decoration-line`, `text-decoration-style`, and `text-decoration-color`.
pub(super) fn parse_text_decoration_line(input: &mut Parser) -> Vec<Declaration> {
    // Try "none" first.
    if let Ok(val) = try_keyword_value(input, "none", &CssValue::Keyword("none".to_string())) {
        return single_decl("text-decoration-line", val);
    }

    // Collect one or more of: underline, overline, line-through.
    let mut values = Vec::new();
    loop {
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                let ident = i.expect_ident().map_err(|_| ())?;
                let lower = ident.to_ascii_lowercase();
                match lower.as_str() {
                    "underline" | "overline" | "line-through" => {
                        // Avoid duplicates.
                        let kw = CssValue::Keyword(lower);
                        if !values.contains(&kw) {
                            values.push(kw);
                        }
                        Ok(())
                    }
                    _ => Err(()),
                }
            })
            .is_ok();
        if !ok {
            break;
        }
    }

    if values.is_empty() {
        return Vec::new();
    }

    if values.len() == 1 {
        return single_decl("text-decoration-line", values.swap_remove(0));
    }

    single_decl("text-decoration-line", CssValue::List(values))
}

// --- Text decoration shorthand ---

/// Valid `text-decoration-style` keywords (CSS Text Decoration Level 3 §2.2).
const DECORATION_STYLE_KEYWORDS: [&str; 5] = ["solid", "double", "dotted", "dashed", "wavy"];
/// Valid `text-decoration-line` keywords (CSS Text Decoration Level 3 §2.1).
const DECORATION_LINE_KEYWORDS: [&str; 3] = ["underline", "overline", "line-through"];

/// Parse `text-decoration` shorthand: `<line> || <style> || <color>`.
///
/// Produces up to 3 longhand declarations.
pub(super) fn parse_text_decoration_shorthand(input: &mut Parser) -> Vec<Declaration> {
    // "none" alone resets all 3 longhands.  Only take this branch when the
    // input is exhausted after "none", so that "none dashed red" still parses
    // the style/color components.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let val = try_keyword_value(i, "none", &CssValue::Keyword("none".to_string()))?;
        if i.is_exhausted() {
            Ok(val)
        } else {
            Err(())
        }
    }) {
        return vec![
            Declaration::new("text-decoration-line", val),
            Declaration::new(
                "text-decoration-style",
                CssValue::Keyword("solid".to_string()),
            ),
            Declaration::new(
                "text-decoration-color",
                CssValue::Keyword("currentcolor".to_string()),
            ),
        ];
    }

    let mut line_values: Vec<CssValue> = Vec::new();
    let mut has_none = false;
    let mut style_val: Option<CssValue> = None;
    let mut color_val: Option<CssValue> = None;

    // Parse tokens in any order.
    loop {
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                let ident = i.expect_ident().map_err(|_| ())?;
                let lower = ident.to_ascii_lowercase();
                if lower == "none" {
                    // "none" is exclusive: reject if line keywords already seen.
                    if !line_values.is_empty() {
                        return Err(());
                    }
                    has_none = true;
                    Ok(())
                } else if DECORATION_LINE_KEYWORDS.contains(&lower.as_str()) {
                    // Line keywords are exclusive with "none".
                    if has_none {
                        return Err(());
                    }
                    let kw = CssValue::Keyword(lower);
                    if !line_values.contains(&kw) {
                        line_values.push(kw);
                    }
                    Ok(())
                } else if style_val.is_none() && DECORATION_STYLE_KEYWORDS.contains(&lower.as_str())
                {
                    style_val = Some(CssValue::Keyword(lower));
                    Ok(())
                } else if color_val.is_none() && lower == "currentcolor" {
                    color_val = Some(CssValue::Keyword("currentcolor".to_string()));
                    Ok(())
                } else {
                    Err(())
                }
            })
            .is_ok();

        if !ok {
            // Try color value.
            if color_val.is_none() {
                if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
                    let c = crate::color::parse_color(i)?;
                    color_val = Some(CssValue::Color(c));
                    Ok(())
                }) {
                    continue;
                }
            }
            break;
        }
    }

    if line_values.is_empty() && !has_none && style_val.is_none() && color_val.is_none() {
        return Vec::new();
    }

    // Reject if there are remaining unparsed tokens (invalid shorthand).
    if !input.is_exhausted() {
        return Vec::new();
    }

    let mut decls = Vec::new();
    // line
    let line = if line_values.is_empty() {
        CssValue::Keyword("none".to_string())
    } else if line_values.len() == 1 {
        line_values.swap_remove(0)
    } else {
        CssValue::List(line_values)
    };
    decls.push(Declaration::new("text-decoration-line", line));
    decls.push(Declaration::new(
        "text-decoration-style",
        style_val.unwrap_or_else(|| CssValue::Keyword("solid".to_string())),
    ));
    decls.push(Declaration::new(
        "text-decoration-color",
        color_val.unwrap_or_else(|| CssValue::Keyword("currentcolor".to_string())),
    ));
    decls
}

// --- Letter/word spacing ---

/// Parse `letter-spacing` or `word-spacing`: `normal` | `<length>`.
///
/// Percentages are rejected per CSS Text Level 3 §4.2/§4.3.
/// `normal` is preserved as `Keyword("normal")` so computed-value
/// serialization can distinguish it from an explicit `0px`.
pub(super) fn parse_spacing(input: &mut Parser, name: &str) -> Vec<Declaration> {
    if let Ok(val) = try_keyword_value(input, "normal", &CssValue::Keyword("normal".to_string())) {
        return single_decl(name, val);
    }
    parse_value_property(input, name, crate::values::parse_length)
}

// --- Mapped keyword parsing ---

/// Parse a property whose input keywords map to (potentially different) output keywords.
///
/// Each entry in `mappings` is `(&[input_keywords], output_keyword)`. The first matching
/// entry wins, and `output_keyword` is stored as the declaration value.
fn parse_mapped_keyword(
    input: &mut Parser,
    name: &str,
    mappings: &[(&[&str], &str)],
) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let ident = i.expect_ident().map_err(|_| ())?.clone();
            let lower = ident.to_ascii_lowercase();
            for &(inputs, output) in mappings {
                if inputs.contains(&lower.as_str()) {
                    return Ok(single_decl(name, CssValue::Keyword(output.to_string())));
                }
            }
            Err(())
        })
        .unwrap_or_default()
}

// --- Text-align ---

/// Parse `text-align` (CSS Text Level 3 §7.1).
pub(super) fn parse_text_align(input: &mut Parser) -> Vec<Declaration> {
    parse_mapped_keyword(
        input,
        "text-align",
        &[
            (&["start"], "start"),
            (&["end"], "end"),
            (&["left"], "left"),
            (&["center"], "center"),
            (&["right"], "right"),
            (&["justify"], "justify"),
        ],
    )
}

// --- Gap properties ---

/// Parse a gap value: `normal` (→ 0px for flex) or a non-negative length/percentage.
pub(super) fn parse_gap_value(input: &mut Parser) -> Result<CssValue, ()> {
    // `normal` keyword → 0px for flex containers (CSS Box Alignment §8).
    if let Ok(val) = try_keyword_value(input, "normal", &CssValue::Length(0.0, LengthUnit::Px)) {
        return Ok(val);
    }
    // Reject negative gap values (CSS Box Alignment §8).
    parse_non_negative_length_or_percentage(input)
}

/// Parse the `gap` shorthand: 1 value → both row-gap and column-gap,
/// 2 values → row-gap then column-gap.
pub(super) fn parse_gap_shorthand(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let row = parse_gap_value(i)?;
            let col = i.try_parse(parse_gap_value).unwrap_or(row.clone());
            Ok(vec![
                Declaration::new("row-gap", row),
                Declaration::new("column-gap", col),
            ])
        })
        .unwrap_or_default()
}

// --- Overflow parsing ---

/// Parse `overflow` shorthand into `overflow-x`/`overflow-y` longhands.
///
/// Accepts 1 or 2 values: `overflow: <x> [<y>]`.
/// If only one value is given, it applies to both axes.
pub(super) fn parse_overflow(input: &mut Parser) -> Vec<Declaration> {
    const KEYWORDS: &[&str] = &["visible", "hidden", "scroll", "auto", "clip"];
    let first = input.try_parse(|i| -> Result<String, cssparser::ParseError<'_, ()>> {
        let tok = i.expect_ident()?;
        let lower = tok.as_ref().to_ascii_lowercase();
        if KEYWORDS.contains(&lower.as_str()) {
            Ok(lower)
        } else {
            Err(i.new_custom_error(()))
        }
    });
    let Ok(first) = first else {
        return vec![];
    };
    let second = input
        .try_parse(|i| -> Result<String, cssparser::ParseError<'_, ()>> {
            let tok = i.expect_ident()?;
            let lower = tok.as_ref().to_ascii_lowercase();
            if KEYWORDS.contains(&lower.as_str()) {
                Ok(lower)
            } else {
                Err(i.new_custom_error(()))
            }
        })
        .ok();
    let y_decl = Declaration::new(
        "overflow-y",
        CssValue::Keyword(second.unwrap_or_else(|| first.clone())),
    );
    vec![
        Declaration::new("overflow-x", CssValue::Keyword(first)),
        y_decl,
    ]
}

// --- Max dimension parsing ---

/// Parse `max-width`/`max-height`: `none` | `<length>` | `<percentage>`.
pub(super) fn parse_max_dimension(input: &mut Parser, name: &str) -> Vec<Declaration> {
    // Try `none` keyword first (→ Auto = unconstrained).
    if let Ok(val) = try_keyword_value(input, "none", &CssValue::Auto) {
        return single_decl(name, val);
    }
    parse_value_property(input, name, parse_non_negative_length_or_percentage)
}

// --- List-style shorthand ---

/// Parse `list-style` shorthand, extracting only `list-style-type`.
///
/// Rejects declarations with extra unknown tokens after the keyword, while
/// allowing `!important` (starts with `!`) to remain for the caller.
pub(super) fn parse_list_style_shorthand(input: &mut Parser) -> Vec<Declaration> {
    let allowed = &["disc", "circle", "square", "decimal", "none"];
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let kw = try_parse_keyword(i, allowed).map_err(|_| ())?;
            // Reject trailing tokens that are not `!important`.
            // Peek via try_parse (always rolls back on Err).
            let has_extra = i
                .try_parse(|peek| {
                    let tok = peek.next().map_err(|_| ())?;
                    if matches!(tok, Token::Delim('!')) {
                        Err(()) // Likely !important — roll back, allow
                    } else {
                        Ok(()) // Unknown extra token — signal rejection
                    }
                })
                .is_ok();
            if has_extra {
                return Err(());
            }
            Ok(single_decl("list-style-type", CssValue::Keyword(kw)))
        })
        .unwrap_or_default()
}

// --- Content property ---

/// Parse the CSS `content` property.
///
/// Accepts: `none`, `normal`, one or more quoted strings, and `attr(name)`.
pub(super) fn parse_content(input: &mut Parser) -> Vec<Declaration> {
    // Try `none` / `normal` keywords first.
    if let Ok(val) = try_keyword_value(input, "none", &CssValue::Keyword("none".to_string())) {
        return single_decl("content", val);
    }
    if let Ok(val) = try_keyword_value(input, "normal", &CssValue::Keyword("normal".to_string())) {
        return single_decl("content", val);
    }

    // Collect one or more content items: quoted strings, attr(), counter(), counters().
    let mut items: Vec<CssValue> = Vec::new();
    loop {
        // Try quoted string.
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                let tok = i.next().map_err(|_| ())?;
                match tok {
                    Token::QuotedString(ref s) => {
                        items.push(CssValue::String(s.as_ref().to_string()));
                        Ok(())
                    }
                    Token::Function(ref name) if name.eq_ignore_ascii_case("attr") => {
                        i.parse_nested_block(
                            |block| -> Result<(), cssparser::ParseError<'_, ()>> {
                                let attr_name = match block.expect_ident() {
                                    Ok(n) => n.as_ref().to_string(),
                                    Err(e) => return Err(e.into()),
                                };
                                items.push(CssValue::Keyword(format!("attr:{attr_name}")));
                                Ok(())
                            },
                        )
                        .map_err(|_: cssparser::ParseError<'_, ()>| ())?;
                        Ok(())
                    }
                    Token::Function(ref name) if name.eq_ignore_ascii_case("counter") => {
                        i.parse_nested_block(
                            |block| -> Result<(), cssparser::ParseError<'_, ()>> {
                                let counter_name = block.expect_ident()?.as_ref().to_string();
                                // Optional list-style-type after comma.
                                let style = if block.try_parse(Parser::expect_comma).is_ok() {
                                    block.expect_ident()?.as_ref().to_ascii_lowercase()
                                } else {
                                    "decimal".to_string()
                                };
                                items.push(CssValue::Keyword(format!(
                                    "counter:{counter_name}:{style}"
                                )));
                                Ok(())
                            },
                        )
                        .map_err(|_: cssparser::ParseError<'_, ()>| ())?;
                        Ok(())
                    }
                    Token::Function(ref name) if name.eq_ignore_ascii_case("counters") => {
                        i.parse_nested_block(
                            |block| -> Result<(), cssparser::ParseError<'_, ()>> {
                                let counter_name = block.expect_ident()?.as_ref().to_string();
                                block.expect_comma()?;
                                let separator = block.expect_string()?.as_ref().to_string();
                                // Optional list-style-type after comma.
                                let style = if block.try_parse(Parser::expect_comma).is_ok() {
                                    block.expect_ident()?.as_ref().to_ascii_lowercase()
                                } else {
                                    "decimal".to_string()
                                };
                                items.push(CssValue::Keyword(format!(
                                    "counters:{counter_name}:{separator}:{style}"
                                )));
                                Ok(())
                            },
                        )
                        .map_err(|_: cssparser::ParseError<'_, ()>| ())?;
                        Ok(())
                    }
                    _ => Err(()),
                }
            })
            .is_ok();
        if !ok {
            break;
        }
    }

    if items.is_empty() {
        return Vec::new();
    }

    if items.len() == 1 {
        return single_decl("content", items.swap_remove(0));
    }

    single_decl("content", CssValue::List(items))
}

// --- Vertical-align parsing ---

/// Parse `vertical-align`: keyword | `<length>` | `<percentage>`.
pub(super) fn parse_vertical_align(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword first.
    let keywords = &[
        "baseline",
        "sub",
        "super",
        "text-top",
        "text-bottom",
        "middle",
        "top",
        "bottom",
    ];
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let kw = try_parse_keyword(i, keywords).map_err(|_| ())?;
        Ok(single_decl("vertical-align", CssValue::Keyword(kw)))
    }) {
        return decls;
    }

    // Try length or percentage.
    parse_value_property(input, "vertical-align", parse_length_or_percentage)
}

// --- Border-spacing parsing ---

/// Parse `border-spacing`: 1 length → both h and v, 2 lengths → h then v.
///
/// CSS 2.1 §17.6.1: both values must be non-negative lengths (no percentages).
pub(super) fn parse_border_spacing(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let h = parse_non_negative_length_or_percentage(i)?;
            // Reject percentages (CSS 2.1 border-spacing only accepts lengths).
            if matches!(h, CssValue::Percentage(_)) {
                return Err(());
            }
            let v = i
                .try_parse(|i2| {
                    let v = parse_non_negative_length_or_percentage(i2)?;
                    if matches!(v, CssValue::Percentage(_)) {
                        return Err(());
                    }
                    Ok(v)
                })
                .unwrap_or(h.clone());
            Ok(vec![
                Declaration::new("border-spacing-h", h),
                Declaration::new("border-spacing-v", v),
            ])
        })
        .unwrap_or_default()
}

// --- Background shorthand ---

/// Parse the `background` shorthand, extracting `background-color`.
///
/// Currently extracts only the color layer from the shorthand (CSS Backgrounds
/// Level 3 §3.10). Full multi-layer support (background-image, -position, -size,
/// -repeat, -origin, -clip, -attachment) is not yet implemented — those layers
/// are silently discarded when the value is parsed as a single color.
pub(super) fn parse_background_shorthand(input: &mut Parser) -> Vec<Declaration> {
    parse_color_property(input, "background-color")
}

// --- Multi-column shorthands ---

/// Border-style keywords shared by column-rule parsing.
const BORDER_STYLE_KEYWORDS: &[&str] = &[
    "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset",
];

/// Parse `column-rule` shorthand: `<width> || <style> || <color>` (CSS Multi-col Level 1 §5).
///
/// Each component may appear at most once, in any order.
pub(super) fn parse_column_rule_shorthand(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let mut width = None;
            let mut style = None;
            let mut color = None;

            for _ in 0..3 {
                if width.is_none() {
                    if let Ok(v) = i.try_parse(parse_border_width_value) {
                        width = Some(v);
                        continue;
                    }
                }
                if style.is_none() {
                    if let Ok(kw) = i.try_parse(|i2| try_parse_keyword(i2, BORDER_STYLE_KEYWORDS)) {
                        style = Some(CssValue::Keyword(kw));
                        continue;
                    }
                }
                if color.is_none() {
                    if let Ok(v) = i.try_parse(parse_color_or_currentcolor) {
                        color = Some(v);
                        continue;
                    }
                }
                break;
            }

            if width.is_none() && style.is_none() && color.is_none() {
                return Err(());
            }

            // CSS shorthand: omitted components reset to initial values.
            let w = width.unwrap_or(CssValue::Length(3.0, LengthUnit::Px)); // medium
            let s = style.unwrap_or(CssValue::Keyword("none".into()));
            let c = color.unwrap_or(CssValue::Keyword("currentcolor".into()));
            Ok(vec![
                Declaration::new("column-rule-width", w),
                Declaration::new("column-rule-style", s),
                Declaration::new("column-rule-color", c),
            ])
        })
        .unwrap_or_default()
}

/// Parse `columns` shorthand: `<column-width> || <column-count>` (CSS Multi-col Level 1 §3).
pub(super) fn parse_columns_shorthand(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let mut width = None;
            let mut count = None;

            for _ in 0..2 {
                // Try count first so bare integers and "auto" map to column-count.
                if count.is_none() {
                    if let Ok(v) = i.try_parse(parse_column_count_value) {
                        count = Some(v);
                        continue;
                    }
                }
                if width.is_none() {
                    if let Ok(v) = i.try_parse(parse_column_width_value) {
                        width = Some(v);
                        continue;
                    }
                }
                break;
            }

            if width.is_none() && count.is_none() {
                return Err(());
            }

            // CSS shorthand: omitted components reset to initial values.
            let w = width.unwrap_or(CssValue::Auto);
            let c = count.unwrap_or(CssValue::Auto);
            Ok(vec![
                Declaration::new("column-width", w),
                Declaration::new("column-count", c),
            ])
        })
        .unwrap_or_default()
}

/// Parse a border-width value: `thin` | `medium` | `thick` | non-negative length.
fn parse_border_width_value(input: &mut Parser) -> Result<CssValue, ()> {
    if let Ok(ident) = input.try_parse(|i| i.expect_ident().map(|s| s.to_ascii_lowercase())) {
        return match ident.as_str() {
            "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
            "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
            "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
            _ => Err(()),
        };
    }
    let token = input.next().map_err(|_| ())?;
    match *token {
        Token::Dimension {
            value, ref unit, ..
        } if value >= 0.0 => {
            let u = crate::values::parse_length_unit(unit)?;
            Ok(CssValue::Length(value, u))
        }
        Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Parse `column-count` value: `auto` | positive integer.
fn parse_column_count_value(input: &mut Parser) -> Result<CssValue, ()> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    let n = input.expect_integer().map_err(|_| ())?;
    if n < 1 {
        return Err(());
    }
    #[allow(clippy::cast_precision_loss)]
    Ok(CssValue::Number(n as f32))
}

/// Parse `column-width` value: `auto` | non-negative length.
fn parse_column_width_value(input: &mut Parser) -> Result<CssValue, ()> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    let token = input.next().map_err(|_| ())?;
    match *token {
        Token::Dimension {
            value, ref unit, ..
        } if value >= 0.0 => {
            let u = crate::values::parse_length_unit(unit)?;
            Ok(CssValue::Length(value, u))
        }
        Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Parse a color value, including `currentcolor` keyword.
fn parse_color_or_currentcolor(input: &mut Parser) -> Result<CssValue, ()> {
    if input
        .try_parse(|i| i.expect_ident_matching("currentcolor"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("currentcolor".to_string()));
    }
    let c = crate::color::parse_color(input)?;
    Ok(CssValue::Color(c))
}

// --- Counter properties ---

/// Parse `counter-reset`, `counter-increment`, or `counter-set`.
///
/// Syntax: `none` | `[<custom-ident> <integer>?]+`
/// `default_value` is 0 for counter-reset/counter-set, 1 for counter-increment.
/// Returns a `CssValue::List` with alternating `Keyword(name)` and `Number(value)` entries.
pub(super) fn parse_counter_list(
    input: &mut Parser,
    name: &str,
    default_value: i32,
) -> Vec<Declaration> {
    // `none` produces an empty list.
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return single_decl(name, CssValue::Keyword("none".to_string()));
    }

    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let mut items: Vec<CssValue> = Vec::new();
            // Parse one or more `<custom-ident> <integer>?` pairs.
            while let Ok(ident) = i.try_parse(|i2| {
                let tok = i2.expect_ident().map_err(|_| ())?;
                Ok::<_, ()>(tok.as_ref().to_ascii_lowercase())
            }) {
                // Reject CSS-wide keywords.
                if matches!(ident.as_str(), "inherit" | "initial" | "unset" | "revert") {
                    return Err(());
                }
                items.push(CssValue::Keyword(ident));
                // Optional integer value.
                let val = i
                    .try_parse(|i2| i2.expect_integer().map_err(|_| ()))
                    .unwrap_or(default_value);
                #[allow(clippy::cast_precision_loss)]
                items.push(CssValue::Number(val as f32));
                // If no more idents follow, we're done.
            }
            if items.is_empty() {
                return Err(());
            }
            Ok(single_decl(name, CssValue::List(items)))
        })
        .unwrap_or_default()
}
