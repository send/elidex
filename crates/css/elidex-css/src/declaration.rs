//! CSS declaration parsing and shorthand expansion.
//!
//! Parses property-value pairs into [`Declaration`] structs, expanding
//! shorthand properties (`margin`, `padding`, `border`) into their
//! longhand equivalents.

use cssparser::{Parser, ParserInput, Token};
use elidex_ecs::InlineStyle;
use elidex_plugin::{CssPropertyRegistry, CssValue};

use crate::color::parse_color;
use crate::values::{
    parse_global_keyword, parse_length_or_percentage, parse_length_percentage_or_auto,
    parse_non_negative_length_or_percentage,
};

mod box_model;
mod flex;
mod font;
mod grid;
mod grid_shorthand;
mod misc;

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

/// Parse a `style` content attribute string into an [`InlineStyle`]
/// component — the canonical attribute→component derivation
/// (One-issue-one-way; CSSOM §6.6 "parse a CSS declaration block" over
/// the style content attribute, CSS Style Attributes §3). Every site
/// that materializes `InlineStyle` from CSS text funnels through here:
/// CSSOM lazy hydration on first `el.style.*` access (`ensure_inline_style`
/// in `elidex-dom-api`) and the `style.cssText` setter. The HTML parser
/// does NOT materialize `InlineStyle` — the cascade reads `attrs("style")`
/// directly.
///
/// Built on [`parse_declaration_block`], so the component holds the
/// post-parse canonical form: shorthands expand to longhands, unknown or
/// unparseable declarations drop (only supported properties are
/// reflected, matching `CSSStyleDeclaration` behaviour), and values
/// serialize via [`CssValue::to_css_string`] (e.g. color keywords
/// round-trip to hex). `!important` flags are preserved per declaration
/// (`InlineStyle::is_important`) and re-emitted by
/// `InlineStyle::css_text()` — load-bearing for the cascade, which
/// re-parses the `style` attribute that `sync_to_attribute` rewrites
/// from `css_text()` after every `el.style.*` mutation.
///
/// `registry` resolves properties not handled by the built-in parser
/// (e.g. `transform`, `transition`, and other plugin-registered
/// properties via `elidex_style::default_css_property_registry`). Pass
/// `None` only where registry-backed properties are intentionally out of
/// scope; the CSSOM materialization path passes the default registry so
/// those properties round-trip through `InlineStyle`.
#[must_use]
pub fn parse_inline_style(css: &str, registry: Option<&CssPropertyRegistry>) -> InlineStyle {
    let mut style = InlineStyle::default();
    for decl in parse_declaration_block_with_registry(css, registry) {
        style.set_with_priority(decl.property, decl.value.to_css_string(), decl.important);
    }
    style
}

/// Parse `value` as the value of CSS property `property` (CSSOM §6.6.1
/// `setProperty` steps 5–6, "parse a CSS value"). `property` must
/// already be in canonical case (ASCII-lowercased unless a custom
/// property). Returns the longhand-expanded declarations, or `None`
/// when the property is unsupported, the value fails to parse, or
/// trailing input remains after the value grammar.
///
/// The exhaustion check is load-bearing for both spec fidelity and
/// safety: a value like `red; background: url(//evil)` or
/// `red !important` parses a prefix and leaves trailing tokens — per
/// the spec it must be rejected whole (the §6.6.1 note: value cannot
/// include `!important`; priority travels as a separate argument), and
/// accepting it verbatim would fabricate declarations / priority when
/// the serialized block is re-parsed by the cascade.
///
/// Custom properties accept any `<declaration-value>` (CSS Syntax 3) —
/// which excludes *top-level* `;` and `!` delimiters, plus bad-string /
/// bad-url tokens and unmatched `)` / `]` / `}` at **any** nesting
/// level — stored as raw tokens. A raw top-level `;` or `!` would
/// fabricate a declaration boundary / priority on the serialized
/// block's re-parse; nested `;` / `!` (inside blocks/functions) are
/// legitimate and accepted.
#[must_use]
pub fn parse_value_for_property(
    property: &str,
    value: &str,
    registry: Option<&CssPropertyRegistry>,
) -> Option<Vec<Declaration>> {
    let mut pi = ParserInput::new(value);
    let mut input = Parser::new(&mut pi);
    if property.starts_with("--") {
        if !is_declaration_value(&mut input, true) {
            return None;
        }
        let raw = value.trim();
        if raw.is_empty() {
            return None;
        }
        return Some(vec![Declaration::new(
            property,
            CssValue::RawTokens(raw.to_string()),
        )]);
    }
    let decls = parse_property_value(property, &mut input, registry);
    if decls.is_empty() || !input.is_exhausted() {
        return None;
    }
    Some(decls)
}

/// Walk `input` checking the CSS Syntax 3 `<declaration-value>`
/// production: `;` and `!` are excluded at the top level only;
/// bad-string / bad-url tokens and unmatched close brackets are
/// excluded at any nesting level (a yielded close token IS unmatched —
/// cssparser consumes a block's matching closer itself). Descends into
/// nested blocks/functions recursively.
fn is_declaration_value(input: &mut Parser, top_level: bool) -> bool {
    loop {
        let token = match input.next() {
            Ok(t) => t.clone(),
            Err(_) => return true,
        };
        match token {
            Token::Semicolon | Token::Delim('!') if top_level => return false,
            Token::BadString(_)
            | Token::BadUrl(_)
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket => return false,
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                let ok = input
                    .parse_nested_block(|nested| {
                        Ok::<bool, cssparser::ParseError<'_, ()>>(is_declaration_value(
                            nested, false,
                        ))
                    })
                    .unwrap_or(false);
                if !ok {
                    return false;
                }
            }
            _ => {}
        }
    }
}

/// Parse an inline style attribute string into declarations
/// (registry-less; registry-backed properties such as `transform` drop).
/// Most callers want [`parse_declaration_block_with_registry`].
///
/// Shorthand properties are expanded into their longhand equivalents.
#[must_use]
pub fn parse_declaration_block(css: &str) -> Vec<Declaration> {
    parse_declaration_block_with_registry(css, None)
}

/// Parse an inline style attribute string into declarations, resolving
/// registry-backed properties (e.g. `transform`) through `registry`.
///
/// Shorthand properties are expanded into their longhand equivalents.
#[must_use]
pub fn parse_declaration_block_with_registry(
    css: &str,
    registry: Option<&CssPropertyRegistry>,
) -> Vec<Declaration> {
    let mut pi = ParserInput::new(css);
    let mut input = Parser::new(&mut pi);
    let mut declarations = Vec::new();

    while !input.is_exhausted() {
        let result: Result<Vec<Declaration>, ()> = input.try_parse(|i| {
            // CSS Variables Level 1 §2 — custom properties (`--*`) are
            // case-sensitive; lowercase the ident only when it is NOT
            // a custom property so `--MyVar` and `--myvar` remain
            // distinct declarations after re-parsing
            // (CSSStyleDeclaration `cssText` setter round-trip etc.).
            // Allocate exactly once per branch — `to_owned()` for the
            // case-preserving custom-property arm, `to_ascii_lowercase()`
            // for the canonical-name arm.
            let ident = i.expect_ident().map_err(|_| ())?;
            let name = if ident.starts_with("--") {
                ident.as_ref().to_owned()
            } else {
                ident.as_ref().to_ascii_lowercase()
            };
            i.expect_colon().map_err(|_| ())?;
            let mut decls = parse_property_value(&name, i, registry);
            // Check for !important (browsers support this in inline styles).
            if i.try_parse(cssparser::parse_important).is_ok() {
                for d in &mut decls {
                    d.important = true;
                }
            }
            // CSS Syntax 3 §5.4.4: a declaration's value is everything up
            // to the `;`. If tokens remain after the value (and the
            // optional `!important`) before the next top-level `;`/EOF,
            // the declaration is malformed and dropped whole — notably a
            // bare top-level `!` that is not `!important` (excluded from
            // `<declaration-value>`), so `--x: foo ! bar` does not leak
            // `--x: foo` into the block.
            if !i.is_exhausted()
                && i.try_parse(|p| p.expect_semicolon().map_err(|_| ()))
                    .is_err()
            {
                return Err(());
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
/// When `registry` is `Some`, properties not handled by the built-in match
/// are dispatched to the matching [`CssPropertyHandler`] for parsing.
///
/// **Contract:** Returns an empty `Vec` for both unknown properties and
/// known properties with unparseable values. The caller (e.g.
/// `DeclarationListParser`) treats an empty result as an error, which
/// triggers cssparser's standard error recovery (skip the declaration).
#[allow(clippy::too_many_lines)]
// Single match dispatcher over token/AST variants.
pub(crate) fn parse_property_value(
    name: &str,
    input: &mut Parser,
    registry: Option<&CssPropertyRegistry>,
) -> Vec<Declaration> {
    // Custom properties (--*): store the value as raw tokens —
    // scoped to the <declaration-value> run so a following declaration
    // or `!important` is not swallowed into the value.
    if name.starts_with("--") {
        let raw = collect_declaration_value_tokens(input);
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
    if let Ok(var_val) = input.try_parse(|i| {
        let v = parse_var_function(i)?;
        // Only match when var() is the entire value (exhaustive).
        if !i.is_exhausted() {
            return Err(());
        }
        Ok(v)
    }) {
        return single_decl(name, var_val);
    }

    // CSS Variables Level 1 §3: If the value contains var() references mixed
    // with other tokens, store the value as raw tokens for deferred
    // substitution at computed-value time — scoped to the
    // <declaration-value> run (stop before a top-level `;`/`!`) so the
    // block parser still sees a following declaration / `!important`,
    // and the setProperty trailing-input injection guard still fires.
    if let Ok(raw) = input.try_parse(|i| -> Result<String, ()> {
        let raw = collect_declaration_value_tokens(i);
        if raw.contains("var(") {
            Ok(raw)
        } else {
            Err(())
        }
    }) {
        return single_decl(name, CssValue::RawTokens(raw));
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
        "background" => misc::parse_background_shorthand(input),

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
                "contents",
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
        | "border-left-color"
        | "text-decoration-color" => parse_color_property(input, name),

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

        // --- Font shorthand ---
        "font" => font::parse_font_shorthand(input),

        // --- Font properties ---
        "font-size" => font::parse_font_size(input),
        "font-weight" => font::parse_font_weight(input),
        "font-style" => parse_keyword_property(input, name, &["normal", "italic", "oblique"]),
        "font-family" => font::parse_font_family(input),

        // --- Line height ---
        "line-height" => font::parse_line_height(input),

        // --- Box model ---
        "box-sizing" => parse_keyword_property(input, name, &["content-box", "border-box"]),
        "border-radius" => misc::parse_border_radius(input),
        "border-top-left-radius"
        | "border-top-right-radius"
        | "border-bottom-right-radius"
        | "border-bottom-left-radius" => {
            parse_value_property(input, name, parse_non_negative_length_or_percentage)
        }
        "opacity" => misc::parse_opacity(input),

        // --- Text alignment ---
        "text-align" => misc::parse_text_align(input),

        // --- Text transform ---
        "text-transform" => parse_keyword_property(
            input,
            name,
            &["none", "uppercase", "lowercase", "capitalize"],
        ),

        // --- Text decoration ---
        "text-decoration-line" => misc::parse_text_decoration_line(input),
        "text-decoration" => misc::parse_text_decoration_shorthand(input),
        "text-decoration-style" => parse_keyword_property(
            input,
            name,
            &["solid", "double", "dotted", "dashed", "wavy"],
        ),
        // --- Letter/word spacing ---
        "letter-spacing" => misc::parse_spacing(input, "letter-spacing"),
        "word-spacing" => misc::parse_spacing(input, "word-spacing"),

        // --- White-space ---
        "white-space" => parse_keyword_property(
            input,
            name,
            &["normal", "pre", "nowrap", "pre-wrap", "pre-line"],
        ),

        // --- Overflow ---
        "overflow" => misc::parse_overflow(input),
        "overflow-x" | "overflow-y" => parse_keyword_property(
            input,
            name,
            &["visible", "hidden", "scroll", "auto", "clip"],
        ),

        // --- Min/max sizing ---
        "min-width" | "min-height" => {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            parse_value_property(input, name, parse_non_negative_length_or_percentage)
        }
        "max-width" | "max-height" => misc::parse_max_dimension(input, name),

        // --- List style ---
        "list-style-type" => parse_keyword_property(
            input,
            name,
            &["disc", "circle", "square", "decimal", "none"],
        ),
        "list-style" => misc::parse_list_style_shorthand(input),

        // --- Gap properties ---
        "row-gap" | "column-gap" => parse_value_property(input, name, misc::parse_gap_value),
        "gap" => misc::parse_gap_shorthand(input),

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
                "normal",
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
                "normal",
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
        "grid-template-areas" => grid_shorthand::parse_grid_template_areas(input),
        "grid-auto-flow" => grid_shorthand::parse_grid_auto_flow(input),
        "grid-auto-columns" | "grid-auto-rows" => {
            grid_shorthand::parse_grid_auto_track(input, name)
        }

        // --- Grid item properties ---
        "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
            grid_shorthand::parse_grid_line(input, name)
        }

        // --- Grid shorthands ---
        "grid-column" | "grid-row" => grid_shorthand::parse_grid_line_shorthand(input, name),
        "grid-area" => grid_shorthand::parse_grid_area(input),
        "grid-template" => grid_shorthand::parse_grid_template_shorthand(input),
        "grid" => grid_shorthand::parse_grid_shorthand(input),

        // --- Multi-column shorthands ---
        "column-rule" => misc::parse_column_rule_shorthand(input),
        "columns" => misc::parse_columns_shorthand(input),

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

        // --- Position offsets ---
        "top" | "right" | "bottom" | "left" => {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            parse_value_property(input, name, parse_length_or_percentage)
        }
        "z-index" => {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            flex::parse_integer_property(input, name)
        }

        // --- Float/clear/visibility ---
        "float" => parse_keyword_property(input, name, &["none", "left", "right"]),
        "clear" => parse_keyword_property(input, name, &["none", "left", "right", "both"]),
        "visibility" => parse_keyword_property(input, name, &["visible", "hidden", "collapse"]),
        "vertical-align" => misc::parse_vertical_align(input),

        // --- Table properties ---
        "border-collapse" => parse_keyword_property(input, name, &["separate", "collapse"]),
        "border-spacing" => misc::parse_border_spacing(input),
        "table-layout" => parse_keyword_property(input, name, &["auto", "fixed"]),
        "caption-side" => {
            parse_keyword_property(input, name, &["top", "bottom", "block-start", "block-end"])
        }

        // --- Content property ---
        "content" => misc::parse_content(input),

        // --- Counter properties ---
        "counter-reset" | "counter-set" => misc::parse_counter_list(input, name, 0),
        "counter-increment" => misc::parse_counter_list(input, name, 1),

        // --- Fallback: dispatch to plugin handler registry ---
        _ => {
            if let Some(reg) = registry {
                if let Some(handler) = reg.resolve(name) {
                    if let Ok(decls) = handler.parse(name, input) {
                        return decls
                            .into_iter()
                            .map(|d| Declaration::new(&d.property, d.value))
                            .collect();
                    }
                }
            }
            Vec::new()
        }
    }
}

/// Create a single-declaration `Vec`.
pub(super) fn single_decl(name: &str, value: CssValue) -> Vec<Declaration> {
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

pub(super) fn parse_color_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
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
pub(super) fn try_keyword_value(
    input: &mut Parser,
    keyword: &str,
    value: &CssValue,
) -> Result<CssValue, ()> {
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
pub(super) fn parse_value_property(
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

// --- var() function parsing ---

/// Parse a `var(--name)` or `var(--name, fallback)` function call.
#[must_use = "parsing result should be used"]
#[allow(clippy::result_unit_err)] // cssparser convention: Parser methods return Result<T, ()>.
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

/// Collect a `<declaration-value>` token run into a trimmed string:
/// consumes tokens up to — but NOT including — a *top-level* `;` or `!`
/// (nested blocks/functions are skipped whole, so `;`/`!` inside them
/// are collected). Backs the custom-property and var()-carrying raw
/// values inside a declaration block: an unscoped collector would
/// swallow the following declaration (`--x: 1; color: red` losing
/// `color`) or the `!important` suffix, and on the `setProperty` path
/// would defeat the trailing-input injection guard.
fn collect_declaration_value_tokens(input: &mut Parser) -> String {
    let start = input.position();
    loop {
        let state = input.state();
        let token = match input.next() {
            Ok(t) => t.clone(),
            Err(_) => break,
        };
        match token {
            Token::Semicolon | Token::Delim('!') => {
                input.reset(&state);
                break;
            }
            // Eagerly consume a nested block: cssparser skips an
            // unentered block only on the NEXT `next()` call, so a
            // saved state taken after the opening token would point
            // inside the block and a later `reset` would truncate the
            // slice mid-block. `parse_nested_block` always positions
            // the parser after the matching closer on return.
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                let _ = input.parse_nested_block(|_| Ok::<(), cssparser::ParseError<'_, ()>>(()));
            }
            _ => {}
        }
    }
    input.slice_from(start).trim().to_string()
}

// --- Shorthand expansion helpers ---

/// The longhand properties a shorthand expands to, in canonical order
/// (empty when `name` is not a shorthand). Single source for both the
/// global-keyword expansion below and CSSOM §6.6.1 `removeProperty`
/// step 2 ("If property is a shorthand property, for each longhand
/// property longhand that property maps to…") in `elidex-dom-api`.
#[must_use]
pub fn shorthand_longhands(name: &str) -> Vec<String> {
    match name {
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
        "text-decoration" => vec![
            "text-decoration-line".to_string(),
            "text-decoration-style".to_string(),
            "text-decoration-color".to_string(),
        ],
        "gap" => vec!["row-gap".to_string(), "column-gap".to_string()],
        "list-style" => vec!["list-style-type".to_string()],
        "font" => vec![
            "font-style".to_string(),
            "font-weight".to_string(),
            "font-size".to_string(),
            "line-height".to_string(),
            "font-family".to_string(),
        ],
        "background" => vec![
            "background-color".to_string(),
            "background-image".to_string(),
            "background-position".to_string(),
            "background-size".to_string(),
            "background-repeat".to_string(),
            "background-origin".to_string(),
            "background-clip".to_string(),
            "background-attachment".to_string(),
        ],
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
        "grid-template" => vec![
            "grid-template-rows".to_string(),
            "grid-template-columns".to_string(),
            "grid-template-areas".to_string(),
        ],
        "grid" => vec![
            "grid-template-rows".to_string(),
            "grid-template-columns".to_string(),
            "grid-template-areas".to_string(),
            "grid-auto-flow".to_string(),
            "grid-auto-rows".to_string(),
            "grid-auto-columns".to_string(),
        ],
        "column-rule" => vec![
            "column-rule-width".to_string(),
            "column-rule-style".to_string(),
            "column-rule-color".to_string(),
        ],
        "columns" => vec!["column-width".to_string(), "column-count".to_string()],
        "overflow" => vec!["overflow-x".to_string(), "overflow-y".to_string()],
        "border-radius" => vec![
            "border-top-left-radius".to_string(),
            "border-top-right-radius".to_string(),
            "border-bottom-right-radius".to_string(),
            "border-bottom-left-radius".to_string(),
        ],
        // Longhand properties: not a shorthand.
        _ => Vec::new(),
    }
}

/// Expand a global keyword (inherit/initial/unset) for shorthand properties into
/// their longhand equivalents. Longhand properties produce a single declaration.
fn expand_global_keyword(name: &str, val: CssValue) -> Vec<Declaration> {
    let longhands = shorthand_longhands(name);
    if longhands.is_empty() {
        return single_decl(name, val);
    }
    longhands
        .iter()
        .map(|p| Declaration::new(p.clone(), val.clone()))
        .collect()
}
