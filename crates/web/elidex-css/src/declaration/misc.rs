//! Miscellaneous property parsers (border-radius, opacity, text-decoration,
//! text-align, gap, overflow, max-dimension, list-style, content,
//! border-spacing, background).

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::values::{parse_length_or_percentage, parse_non_negative_length_or_percentage};

use super::{parse_color_property, parse_value_property, try_keyword_value, try_parse_keyword};
use super::{single_decl, Declaration};

// --- Border radius parsing ---

/// Parse `border-radius` as a non-negative `<length>`.
///
/// TODO(Phase 4): Support multi-value shorthand (per-corner radii) and
/// percentage values (CSS Backgrounds and Borders Level 3 §5.3).
/// Percentages require box dimensions for resolution.
/// Negative values are rejected per spec.
pub(super) fn parse_border_radius(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = parse_non_negative_length_or_percentage(i)?;
            // Reject percentages — cannot resolve without box dimensions.
            if matches!(val, CssValue::Percentage(_)) {
                return Err(());
            }
            Ok(single_decl("border-radius", val))
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

/// Parse `text-decoration-line` (or `text-decoration` shorthand).
///
/// Accepts `none`, or one or more of `underline`, `line-through` (space-separated).
/// The `text-decoration` shorthand is treated as an alias for `text-decoration-line`
/// (color/style are Phase 3 scope-out).
pub(super) fn parse_text_decoration_line(input: &mut Parser) -> Vec<Declaration> {
    // Try "none" first.
    if let Ok(val) = try_keyword_value(input, "none", &CssValue::Keyword("none".to_string())) {
        return single_decl("text-decoration-line", val);
    }

    // Collect one or more of: underline, line-through.
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
    let style_keywords = ["solid", "double", "dotted", "dashed", "wavy"];
    let line_keywords = ["underline", "overline", "line-through"];

    // Parse tokens in any order.
    loop {
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                let ident = i.expect_ident().map_err(|_| ())?;
                let lower = ident.to_ascii_lowercase();
                if lower == "none" {
                    has_none = true;
                    Ok(())
                } else if line_keywords.contains(&lower.as_str()) {
                    let kw = CssValue::Keyword(lower);
                    if !line_values.contains(&kw) {
                        line_values.push(kw);
                    }
                    Ok(())
                } else if style_val.is_none() && style_keywords.contains(&lower.as_str()) {
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
// TODO: preserve Keyword("normal") instead of converting to Length(0, Px)
// so that `getComputedStyle` can serialize "normal" per CSS Text L3 §4.2/§4.3.
pub(super) fn parse_spacing(input: &mut Parser, name: &str) -> Vec<Declaration> {
    if let Ok(val) = try_keyword_value(
        input,
        "normal",
        &CssValue::Length(0.0, elidex_plugin::LengthUnit::Px),
    ) {
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
///
// TODO(Phase 4): `justify` is mapped to `start` because full inter-word
// justification (CSS Text 3 §7.1) is not yet implemented.
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
            (&["justify"], "start"),
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

/// Parse `overflow`. Maps `scroll`/`auto` to `hidden` (Phase 3 simplification).
pub(super) fn parse_overflow(input: &mut Parser) -> Vec<Declaration> {
    parse_mapped_keyword(
        input,
        "overflow",
        &[
            (&["visible"], "visible"),
            (&["hidden", "scroll", "auto"], "hidden"),
        ],
    )
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

    // Collect one or more content items: quoted strings or attr() functions.
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

/// Parse the `background` shorthand, extracting only `background-color`.
///
/// TODO(Phase 4): Support full `background` shorthand (CSS Backgrounds Level 3 §3.10):
/// background-image, background-position, background-size, background-repeat,
/// background-origin, background-clip, background-attachment, and multi-layer values.
/// For now, try to parse the value as a color and emit `background-color`.
pub(super) fn parse_background_shorthand(input: &mut Parser) -> Vec<Declaration> {
    parse_color_property(input, "background-color")
}
