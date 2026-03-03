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
    parse_non_negative_length_or_percentage,
};

mod box_model;
mod flex;
mod font;
mod grid;

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

impl Declaration {
    /// Create a normal (non-important) declaration.
    #[must_use]
    pub fn new(property: impl Into<String>, value: CssValue) -> Self {
        Self {
            property: property.into(),
            value,
            important: false,
        }
    }
}

/// Parse an inline style attribute string into declarations.
///
/// Shorthand properties are expanded into their longhand equivalents.
#[must_use]
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
        "border-top" => box_model::parse_border_side_shorthand(input, "top"),
        "border-right" => box_model::parse_border_side_shorthand(input, "right"),
        "border-bottom" => box_model::parse_border_side_shorthand(input, "bottom"),
        "border-left" => box_model::parse_border_side_shorthand(input, "left"),
        "background" => parse_background_shorthand(input),

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
                "list-item",
                "grid",
                "inline-grid",
                "table",
                "inline-table",
                "table-caption",
                "table-row",
                "table-cell",
                "table-row-group",
                "table-header-group",
                "table-footer-group",
                "table-column",
                "table-column-group",
            ],
        ),
        "position" => parse_keyword_property(
            input,
            name,
            &["static", "relative", "absolute", "fixed", "sticky"],
        ),
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            parse_keyword_property(
                input,
                name,
                &[
                    "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge",
                    "inset", "outset",
                ],
            )
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

        // TODO(Phase 4): `font` shorthand property (CSS Fonts Level 3 §4).

        // --- Font properties ---
        "font-size" => font::parse_font_size(input),
        "font-weight" => font::parse_font_weight(input),
        "font-style" => parse_keyword_property(input, name, &["normal", "italic", "oblique"]),
        "font-family" => font::parse_font_family(input),

        // --- Line height ---
        "line-height" => font::parse_line_height(input),

        // --- Box model ---
        "box-sizing" => parse_keyword_property(input, name, &["content-box", "border-box"]),
        "border-radius" => parse_border_radius(input),
        "opacity" => parse_opacity(input),

        // --- Text alignment ---
        "text-align" => parse_text_align(input),

        // --- Text transform ---
        "text-transform" => parse_keyword_property(
            input,
            name,
            &["none", "uppercase", "lowercase", "capitalize"],
        ),

        // --- Text decoration ---
        "text-decoration" | "text-decoration-line" => parse_text_decoration_line(input),

        // --- White-space ---
        "white-space" => parse_keyword_property(
            input,
            name,
            &["normal", "pre", "nowrap", "pre-wrap", "pre-line"],
        ),

        // --- Overflow ---
        "overflow" => parse_overflow(input),

        // --- Min/max sizing ---
        "min-width" | "min-height" => {
            parse_value_property(input, name, parse_non_negative_length_or_percentage)
        }
        "max-width" | "max-height" => parse_max_dimension(input, name),

        // --- List style ---
        "list-style-type" => parse_keyword_property(
            input,
            name,
            &["disc", "circle", "square", "decimal", "none"],
        ),
        "list-style" => parse_list_style_shorthand(input),

        // --- Gap properties ---
        "row-gap" | "column-gap" => parse_value_property(input, name, parse_gap_value),
        "gap" => parse_gap_shorthand(input),

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
                "space-evenly",
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

        // --- Grid container properties ---
        "grid-template-columns" | "grid-template-rows" => grid::parse_grid_template(input, name),
        "grid-auto-flow" => grid::parse_grid_auto_flow(input),
        "grid-auto-columns" | "grid-auto-rows" => grid::parse_grid_auto_track(input, name),

        // --- Grid item properties ---
        "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
            grid::parse_grid_line(input, name)
        }

        // --- Grid shorthands ---
        "grid-column" | "grid-row" => grid::parse_grid_line_shorthand(input, name),
        "grid-area" => grid::parse_grid_area(input),

        // --- Writing mode / BiDi properties ---
        "direction" => parse_keyword_property(input, name, &["ltr", "rtl"]),
        "unicode-bidi" => parse_keyword_property(
            input,
            name,
            &[
                "normal",
                "embed",
                "bidi-override",
                "isolate",
                "isolate-override",
                "plaintext",
            ],
        ),
        "writing-mode" => parse_keyword_property(
            input,
            name,
            &["horizontal-tb", "vertical-rl", "vertical-lr"],
        ),
        "text-orientation" => {
            parse_keyword_property(input, name, &["mixed", "upright", "sideways"])
        }

        // --- Table properties ---
        "border-collapse" => parse_keyword_property(input, name, &["separate", "collapse"]),
        "border-spacing" => parse_border_spacing(input),
        "table-layout" => parse_keyword_property(input, name, &["auto", "fixed"]),
        "caption-side" => parse_keyword_property(input, name, &["top", "bottom"]),

        // --- Content property ---
        "content" => parse_content(input),

        // --- Unknown property: silently drop ---
        _ => Vec::new(),
    }
}

/// Create a single-declaration `Vec`.
fn single_decl(name: &str, value: CssValue) -> Vec<Declaration> {
    vec![Declaration::new(name, value)]
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
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let kw = try_parse_keyword(i, allowed).map_err(|_| ())?;
            Ok(single_decl(name, CssValue::Keyword(kw)))
        })
        .unwrap_or_default()
}

fn parse_color_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
    // Try `currentcolor` keyword first (case-insensitive).
    if let Ok(val) = try_keyword_value(
        input,
        "currentcolor",
        &CssValue::Keyword("currentcolor".into()),
    ) {
        return single_decl(name, val);
    }

    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let color = parse_color(i)?;
            Ok(single_decl(name, CssValue::Color(color)))
        })
        .unwrap_or_default()
}

/// Try to match a single case-insensitive keyword, returning the given `CssValue` on match.
///
/// Used as an early-return before a fallback parser (e.g. `currentcolor` before color
/// parsing, `none` before length parsing).
fn try_keyword_value(input: &mut Parser, keyword: &str, value: &CssValue) -> Result<CssValue, ()> {
    input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case(keyword) {
            Ok(value.clone())
        } else {
            Err(())
        }
    })
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

// --- Border radius parsing ---

/// Parse `border-radius` as a non-negative `<length>`.
///
/// TODO(Phase 4): Support multi-value shorthand (per-corner radii) and
/// percentage values (CSS Backgrounds and Borders Level 3 §5.3).
/// Percentages require box dimensions for resolution.
/// Negative values are rejected per spec.
fn parse_border_radius(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_opacity(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_text_decoration_line(input: &mut Parser) -> Vec<Declaration> {
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

// --- var() function parsing ---

/// Parse a `var(--name)` or `var(--name, fallback)` function call.
#[must_use = "parsing result should be used"]
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
                        Some(Box::new(crate::parse_raw_token_value(&raw)))
                    }
                }
            } else {
                None
            };

            Ok(CssValue::Var(name, fallback))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Collect all remaining tokens from a parser into a trimmed string.
fn collect_remaining_tokens(input: &mut Parser) -> String {
    let start = input.position();
    // Consume all remaining tokens.
    while input.next().is_ok() {}
    let slice = input.slice_from(start);
    slice.trim().to_string()
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
fn parse_text_align(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_gap_value(input: &mut Parser) -> Result<CssValue, ()> {
    // `normal` keyword → 0px for flex containers (CSS Box Alignment §8).
    if let Ok(val) = try_keyword_value(input, "normal", &CssValue::Length(0.0, LengthUnit::Px)) {
        return Ok(val);
    }
    // Reject negative gap values (CSS Box Alignment §8).
    parse_non_negative_length_or_percentage(input)
}

/// Parse the `gap` shorthand: 1 value → both row-gap and column-gap,
/// 2 values → row-gap then column-gap.
fn parse_gap_shorthand(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_overflow(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_max_dimension(input: &mut Parser, name: &str) -> Vec<Declaration> {
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
fn parse_list_style_shorthand(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_content(input: &mut Parser) -> Vec<Declaration> {
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

// --- Border-spacing parsing ---

/// Parse `border-spacing`: 1 length → both h and v, 2 lengths → h then v.
///
/// CSS 2.1 §17.6.1: both values must be non-negative lengths (no percentages).
fn parse_border_spacing(input: &mut Parser) -> Vec<Declaration> {
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
fn parse_background_shorthand(input: &mut Parser) -> Vec<Declaration> {
    parse_color_property(input, "background-color")
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
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let side = &name["border-".len()..];
            ["width", "style", "color"]
                .iter()
                .map(|prop| format!("border-{side}-{prop}"))
                .collect()
        }
        "flex" => vec![
            "flex-grow".to_string(),
            "flex-shrink".to_string(),
            "flex-basis".to_string(),
        ],
        "flex-flow" => vec!["flex-direction".to_string(), "flex-wrap".to_string()],
        "text-decoration" => vec!["text-decoration-line".to_string()],
        "gap" => vec!["row-gap".to_string(), "column-gap".to_string()],
        "list-style" => vec!["list-style-type".to_string()],
        "background" => vec!["background-color".to_string()],
        "border-spacing" => vec![
            "border-spacing-h".to_string(),
            "border-spacing-v".to_string(),
        ],
        "grid-column" => vec![
            "grid-column-start".to_string(),
            "grid-column-end".to_string(),
        ],
        "grid-row" => vec!["grid-row-start".to_string(), "grid-row-end".to_string()],
        "grid-area" => vec![
            "grid-row-start".to_string(),
            "grid-column-start".to_string(),
            "grid-row-end".to_string(),
            "grid-column-end".to_string(),
        ],
        // Longhand properties: single declaration.
        _ => return single_decl(name, val),
    };
    longhands
        .iter()
        .map(|p| Declaration::new(p.clone(), val.clone()))
        .collect()
}
