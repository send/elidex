//! CSS declaration parsing and shorthand expansion.
//!
//! Parses property-value pairs into [`Declaration`] structs, expanding
//! shorthand properties (`margin`, `padding`, `border`) into their
//! longhand equivalents.

use cssparser::{Parser, ParserInput, Token};
use elidex_plugin::CssValue;

use crate::color::parse_color;
use crate::values::{
    parse_global_keyword, parse_length_or_percentage, parse_length_percentage_or_auto,
};

mod box_model;
mod flex;
mod font;

#[cfg(test)]
mod tests;

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
            let mut decls = parse_property_value(&name, i);
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
/// All returned declarations have `important: false`; callers must set
/// `important` after checking for `!important`.
///
/// **Contract:** Returns an empty `Vec` for both unknown properties and
/// known properties with unparseable values. The caller (e.g.
/// `DeclarationListParser`) treats an empty result as an error, which
/// triggers cssparser's standard error recovery (skip the declaration).
#[allow(clippy::too_many_lines)]
pub(crate) fn parse_property_value(name: &str, input: &mut Parser) -> Vec<Declaration> {
    // Custom properties (--*): store the entire value as raw tokens.
    if name.starts_with("--") {
        let raw = collect_remaining_tokens(input);
        if raw.is_empty() {
            return Vec::new();
        }
        return single_decl(name, CssValue::RawTokens(raw));
    }

    // Check for global keywords first.
    if let Ok(val) = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        parse_global_keyword(ident.as_ref()).ok_or(())
    }) {
        // Shorthand properties must expand global keywords into longhand declarations.
        return expand_global_keyword(name, val);
    }

    // Check for var() function as the entire value.
    // NOTE(Phase 3): Only whole-value var() is supported. Multi-token values
    // like `margin: 0 var(--x)` or `border: var(--bw) solid var(--bc)` are
    // not handled — the var() within a compound value will cause the
    // property-specific parser to fail, silently dropping the declaration.
    // TODO: support var() in any token position (CSS Variables Level 1 §3).
    if let Ok(var_val) = input.try_parse(parse_var_function) {
        return single_decl(name, var_val);
    }

    match name {
        // --- Shorthand properties ---
        "margin" => box_model::expand_four_sides(input, "margin", parse_length_percentage_or_auto),
        "padding" => box_model::expand_four_sides(input, "padding", parse_length_or_percentage),
        "border" => box_model::parse_border_shorthand(input),

        // --- Keyword properties ---
        "display" => parse_keyword_property(
            input,
            name,
            &[
                "block",
                "inline",
                "inline-block",
                "none",
                "flex",
                "inline-flex",
            ],
        ),
        "position" => {
            parse_keyword_property(input, name, &["static", "relative", "absolute", "fixed"])
        }
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            parse_keyword_property(input, name, &["none", "solid", "dashed", "dotted"])
        }

        // --- Color properties ---
        "color"
        | "background-color"
        | "border-top-color"
        | "border-right-color"
        | "border-bottom-color"
        | "border-left-color" => parse_color_property(input, name),

        // --- Length/percentage/auto properties ---
        "width" | "height" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            parse_value_property(input, name, parse_length_percentage_or_auto)
        }

        // --- Length/percentage properties (no auto) ---
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            parse_value_property(input, name, parse_length_or_percentage)
        }

        // --- Border width ---
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            box_model::parse_border_width_property(input, name)
        }

        // --- Font properties ---
        "font-size" => font::parse_font_size(input),
        "font-weight" => font::parse_font_weight(input),
        "font-family" => font::parse_font_family(input),

        // --- Line height ---
        "line-height" => font::parse_line_height(input),

        // --- Text transform ---
        "text-transform" => parse_keyword_property(
            input,
            name,
            &["none", "uppercase", "lowercase", "capitalize"],
        ),

        // --- Text decoration ---
        "text-decoration" | "text-decoration-line" => parse_text_decoration_line(input),

        // --- Flex keyword properties ---
        "flex-direction" => parse_keyword_property(
            input,
            name,
            &["row", "row-reverse", "column", "column-reverse"],
        ),
        "flex-wrap" => parse_keyword_property(input, name, &["nowrap", "wrap", "wrap-reverse"]),
        "justify-content" => parse_keyword_property(
            input,
            name,
            &[
                "flex-start",
                "flex-end",
                "center",
                "space-between",
                "space-around",
                "space-evenly",
            ],
        ),
        "align-items" => parse_keyword_property(
            input,
            name,
            &["stretch", "flex-start", "flex-end", "center", "baseline"],
        ),
        "align-self" => parse_keyword_property(
            input,
            name,
            &[
                "auto",
                "stretch",
                "flex-start",
                "flex-end",
                "center",
                "baseline",
            ],
        ),
        "align-content" => parse_keyword_property(
            input,
            name,
            &[
                "stretch",
                "flex-start",
                "flex-end",
                "center",
                "space-between",
                "space-around",
            ],
        ),

        // --- Flex number properties ---
        "flex-grow" | "flex-shrink" => flex::parse_non_negative_number(input, name),
        "order" => flex::parse_integer_property(input, name),

        // --- Flex basis ---
        "flex-basis" => flex::parse_flex_basis(input),

        // --- Flex shorthands ---
        "flex" => flex::parse_flex_shorthand(input),
        "flex-flow" => flex::parse_flex_flow_shorthand(input),

        // --- Unknown property: silently drop ---
        _ => Vec::new(),
    }
}

/// Create a single-declaration `Vec`.
fn single_decl(name: &str, value: CssValue) -> Vec<Declaration> {
    vec![Declaration {
        property: name.to_string(),
        value,
        important: false,
    }]
}

// --- Property-specific parsers ---

/// Try to parse an identifier and match it against the given keyword list.
/// Returns the matched keyword (lowercased) or Err if no match.
pub(crate) fn try_parse_keyword<'i>(
    input: &mut Parser<'i, '_>,
    allowed: &[&str],
) -> Result<String, cssparser::ParseError<'i, ()>> {
    let location = input.current_source_location();
    let ident = input.expect_ident()?.clone();
    let lower = ident.to_ascii_lowercase();
    if allowed.contains(&lower.as_str()) {
        Ok(lower)
    } else {
        Err(location.new_unexpected_token_error(Token::Ident(ident)))
    }
}

fn parse_keyword_property(input: &mut Parser, name: &str, allowed: &[&str]) -> Vec<Declaration> {
    input
        .try_parse(
            |i| -> Result<Vec<Declaration>, cssparser::ParseError<'_, ()>> {
                let kw = try_parse_keyword(i, allowed)?;
                Ok(single_decl(name, CssValue::Keyword(kw)))
            },
        )
        .unwrap_or_default()
}

fn parse_color_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
    // Try `currentcolor` keyword first (case-insensitive).
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("currentcolor") {
            Ok(single_decl(name, CssValue::Keyword("currentcolor".into())))
        } else {
            Err(())
        }
    }) {
        return decls;
    }

    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let color = parse_color(i)?;
            Ok(single_decl(name, CssValue::Color(color)))
        })
        .unwrap_or_default()
}

/// Parse a single-value property using the given value parser function.
fn parse_value_property(
    input: &mut Parser,
    name: &str,
    value_parser: fn(&mut Parser) -> Result<CssValue, ()>,
) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = value_parser(i)?;
            Ok(single_decl(name, val))
        })
        .unwrap_or_default()
}

// --- Text decoration parsing ---

/// Parse `text-decoration-line` (or `text-decoration` shorthand).
///
/// Accepts `none`, or one or more of `underline`, `line-through` (space-separated).
/// The `text-decoration` shorthand is treated as an alias for `text-decoration-line`
/// (color/style are Phase 3 scope-out).
fn parse_text_decoration_line(input: &mut Parser) -> Vec<Declaration> {
    // Try "none" first.
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("none") {
            Ok(single_decl(
                "text-decoration-line",
                CssValue::Keyword("none".to_string()),
            ))
        } else {
            Err(())
        }
    }) {
        return decls;
    }

    // Collect one or more of: underline, line-through.
    let mut values = Vec::new();
    loop {
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                let ident = i.expect_ident().map_err(|_| ())?;
                let lower = ident.to_ascii_lowercase();
                match lower.as_str() {
                    "underline" | "line-through" => {
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
        return single_decl("text-decoration-line", values.into_iter().next().unwrap());
    }

    single_decl("text-decoration-line", CssValue::List(values))
}

// --- var() function parsing ---

/// Parse a `var(--name)` or `var(--name, fallback)` function call.
#[allow(clippy::result_unit_err)]
pub fn parse_var_function(input: &mut Parser) -> Result<CssValue, ()> {
    input.expect_function_matching("var").map_err(|_| ())?;
    input
        .parse_nested_block(|args| -> Result<CssValue, cssparser::ParseError<'_, ()>> {
            // Expect the custom property name (an ident starting with --).
            let name_token = args.expect_ident().map_err(cssparser::ParseError::from)?;
            let name = name_token.as_ref().to_string();
            if !name.starts_with("--") {
                return Err(args.new_custom_error(()));
            }

            // Optional comma + fallback.
            let fallback = if args.try_parse(|i| i.expect_comma().map_err(|_| ())).is_ok() {
                // Try to parse fallback as a nested var().
                if let Ok(nested_var) = args.try_parse(parse_var_function) {
                    Some(Box::new(nested_var))
                } else {
                    // Collect remaining tokens as raw fallback string and re-parse.
                    let raw = collect_remaining_tokens(args);
                    if raw.is_empty() {
                        None
                    } else {
                        Some(Box::new(parse_fallback_value(&raw)))
                    }
                }
            } else {
                None
            };

            Ok(CssValue::Var(name, fallback))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Try to parse a fallback value string as a typed CSS value.
///
/// Delegates to [`crate::parse_raw_token_value`] which handles `var()`, color,
/// length/percentage/auto, and keyword parsing with `RawTokens` fallback.
fn parse_fallback_value(raw: &str) -> CssValue {
    crate::parse_raw_token_value(raw)
}

/// Collect all remaining tokens from a parser into a trimmed string.
fn collect_remaining_tokens(input: &mut Parser) -> String {
    let start = input.position();
    // Consume all remaining tokens.
    while input.next().is_ok() {}
    let slice = input.slice_from(start);
    slice.trim().to_string()
}

// --- Shorthand expansion helpers ---

/// Expand a global keyword (inherit/initial/unset) for shorthand properties into
/// their longhand equivalents. Longhand properties produce a single declaration.
fn expand_global_keyword(name: &str, val: CssValue) -> Vec<Declaration> {
    let longhands: Vec<String> = match name {
        "margin" => box_model::SIDES
            .iter()
            .map(|s| format!("margin-{s}"))
            .collect(),
        "padding" => box_model::SIDES
            .iter()
            .map(|s| format!("padding-{s}"))
            .collect(),
        "border" => box_model::SIDES
            .iter()
            .flat_map(|s| {
                ["width", "style", "color"]
                    .iter()
                    .map(move |prop| format!("border-{s}-{prop}"))
            })
            .collect(),
        "flex" => vec![
            "flex-grow".to_string(),
            "flex-shrink".to_string(),
            "flex-basis".to_string(),
        ],
        "flex-flow" => vec!["flex-direction".to_string(), "flex-wrap".to_string()],
        "text-decoration" => vec!["text-decoration-line".to_string()],
        // Longhand properties: single declaration.
        _ => return single_decl(name, val),
    };
    longhands
        .iter()
        .map(|p| Declaration {
            property: p.clone(),
            value: val.clone(),
            important: false,
        })
        .collect()
}
