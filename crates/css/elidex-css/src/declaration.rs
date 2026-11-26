//! CSS declaration parsing and shorthand expansion.
//!
//! Parses property-value pairs into [`Declaration`] structs, expanding
//! shorthand properties (`margin`, `padding`, `border`) into their
//! longhand equivalents.

use cssparser::{Parser, ParserInput, Token};
use elidex_plugin::{CssPropertyRegistry, CssValue};

use elidex_plugin::parse_color;

use crate::values::parse_global_keyword;
use elidex_plugin::css_resolve::strict::{
    parse_length_or_percentage, parse_length_percentage_or_auto,
    parse_non_negative_length_or_percentage,
};

mod box_model;
mod flex;
mod font;
mod grid;
mod grid_shorthand;
mod misc;
pub(crate) mod substitution;
pub use substitution::parse_var_function;

// CSSOM-facing inline-style entry points live in a submodule so this
// parser-dispatcher file stays under the 1000-line growth guard.
mod cssom;
pub use cssom::{
    parse_inline_style, parse_value_for_property, serialize_declaration_value_for_storage,
};

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
/// are dispatched to the matching [`CssPropertyHandler`](elidex_plugin::CssPropertyHandler)
/// for parsing.
///
/// **Contract:** Returns an empty `Vec` for both unknown properties and
/// known properties with unparseable values. The caller (e.g.
/// `DeclarationListParser`) treats an empty result as an error, which
/// triggers cssparser's standard error recovery (skip the declaration).
pub(crate) fn parse_property_value(
    name: &str,
    input: &mut Parser,
    registry: Option<&CssPropertyRegistry>,
) -> Vec<Declaration> {
    if name.starts_with("--") {
        return substitution::capture(substitution::source(input))
            .map(|seq| single_decl(name, CssValue::Custom(seq)))
            .unwrap_or_default();
    }

    let Some(grammar) = PropertyGrammar::select(name, registry) else {
        return Vec::new();
    };
    let start = input.state();
    let source = substitution::source(input);
    match elidex_plugin::component_values::ComponentValues::inspect_variables(source) {
        None => return Vec::new(),
        Some(true) => {
            return substitution::capture(source)
                .map(|value| single_decl(name, CssValue::Pending(value)))
                .unwrap_or_default()
        }
        Some(false) => {}
    }
    input.reset(&start);

    // Check for global keywords first.
    if let Ok(val) = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        parse_global_keyword(ident.as_ref()).ok_or(())
    }) {
        // Shorthand properties must expand global keywords into longhand declarations.
        return grammar
            .members(name)
            .into_iter()
            .map(|member| Declaration::new(member, val.clone()))
            .collect();
    }

    grammar.parse(name, input)
}

/// CSS Values §A: the longhands receiving a pending substitution's result
/// (or unset on failure), as defined by the admitting grammar.
#[must_use]
pub fn declaration_members(name: &str, registry: Option<&CssPropertyRegistry>) -> Vec<String> {
    PropertyGrammar::select(name, registry).map_or_else(Vec::new, |grammar| grammar.members(name))
}

/// Members and owning handler of a shorthand as admitted by its grammar.
pub struct ShorthandFamily<'a> {
    pub members: Vec<String>,
    pub registered_handler: Option<&'a dyn elidex_plugin::CssPropertyHandler>,
}

/// CSSOM §6.6.1 uses the admitting grammar for shorthand operations.
#[must_use]
pub fn shorthand_family<'a>(
    name: &str,
    registry: Option<&'a CssPropertyRegistry>,
) -> Option<ShorthandFamily<'a>> {
    let grammar = PropertyGrammar::select(name, registry)?;
    let members = grammar.members(name);
    if members.is_empty() || matches!(members.as_slice(), [single] if single == name) {
        return None;
    }
    let registered_handler = match grammar {
        PropertyGrammar::Builtin(_) => None,
        PropertyGrammar::Registered(handler) => Some(handler),
    };
    Some(ShorthandFamily {
        members,
        registered_handler,
    })
}

type BuiltinParser = fn(&str, &mut Parser) -> Vec<Declaration>;

enum PropertyGrammar<'a> {
    Builtin(BuiltinParser),
    Registered(&'a dyn elidex_plugin::CssPropertyHandler),
}

impl<'a> PropertyGrammar<'a> {
    fn select(name: &str, registry: Option<&'a CssPropertyRegistry>) -> Option<Self> {
        builtin_parser(name)
            .map(Self::Builtin)
            .or_else(|| registry?.resolve(name).map(Self::Registered))
    }

    fn members(&self, name: &str) -> Vec<String> {
        match self {
            Self::Builtin(_) => {
                let members = shorthand_longhands(name);
                if members.is_empty() {
                    vec![name.to_owned()]
                } else {
                    members
                }
            }
            Self::Registered(handler) => handler
                .declaration_members(name)
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }

    fn parse(&self, name: &str, input: &mut Parser) -> Vec<Declaration> {
        match self {
            Self::Builtin(parse) => parse(name, input),
            Self::Registered(handler) => handler
                .parse(name, input)
                .unwrap_or_default()
                .into_iter()
                .map(|decl| Declaration::new(decl.property, decl.value))
                .collect(),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn builtin_parser(name: &str) -> Option<BuiltinParser> {
    Some(match name {
        // --- Shorthand properties ---
        "margin" => |_, input| {
            box_model::expand_four_sides(input, "margin", parse_length_percentage_or_auto)
        },
        "padding" => {
            |_, input| box_model::expand_four_sides(input, "padding", parse_length_or_percentage)
        }
        "border" => |_, input| box_model::parse_border_shorthand(input),
        "border-top" => |_, input| box_model::parse_border_side_shorthand(input, "top"),
        "border-right" => |_, input| box_model::parse_border_side_shorthand(input, "right"),
        "border-bottom" => |_, input| box_model::parse_border_side_shorthand(input, "bottom"),
        "border-left" => |_, input| box_model::parse_border_side_shorthand(input, "left"),
        "background" => |_, input| misc::parse_background_shorthand(input),

        // --- Keyword properties ---
        "display" => |name, input| {
            parse_keyword_property(
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
            )
        },
        "position" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["static", "relative", "absolute", "fixed", "sticky"],
            )
        },
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            |name, input| {
                parse_keyword_property(
                    input,
                    name,
                    &[
                        "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge",
                        "inset", "outset",
                    ],
                )
            }
        }

        // --- Color properties ---
        "color"
        | "background-color"
        | "border-top-color"
        | "border-right-color"
        | "border-bottom-color"
        | "border-left-color"
        | "text-decoration-color" => |name, input| parse_color_property(input, name),

        // --- Length/percentage/auto properties ---
        "width" | "height" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            |name, input| parse_value_property(input, name, parse_length_percentage_or_auto)
        }

        // --- Length/percentage properties (no auto) ---
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            |name, input| parse_value_property(input, name, parse_length_or_percentage)
        }

        // --- Border width ---
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            |name, input| box_model::parse_border_width_property(input, name)
        }

        // --- Font shorthand ---
        "font" => |_, input| font::parse_font_shorthand(input),

        // --- Font properties ---
        "font-size" => |_, input| font::parse_font_size(input),
        "font-weight" => |_, input| font::parse_font_weight(input),
        "font-style" => {
            |name, input| parse_keyword_property(input, name, &["normal", "italic", "oblique"])
        }
        "font-family" => |_, input| font::parse_font_family(input),

        // --- Line height ---
        "line-height" => |_, input| font::parse_line_height(input),

        // --- Box model ---
        "box-sizing" => {
            |name, input| parse_keyword_property(input, name, &["content-box", "border-box"])
        }
        "border-radius" => |_, input| misc::parse_border_radius(input),
        "border-top-left-radius"
        | "border-top-right-radius"
        | "border-bottom-right-radius"
        | "border-bottom-left-radius" => {
            |name, input| parse_value_property(input, name, parse_non_negative_length_or_percentage)
        }
        "opacity" => |_, input| misc::parse_opacity(input),

        // --- Text alignment ---
        "text-align" => |_, input| misc::parse_text_align(input),

        // --- Text transform ---
        "text-transform" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["none", "uppercase", "lowercase", "capitalize"],
            )
        },

        // --- Text decoration ---
        "text-decoration-line" => |_, input| misc::parse_text_decoration_line(input),
        "text-decoration" => |_, input| misc::parse_text_decoration_shorthand(input),
        "text-decoration-style" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["solid", "double", "dotted", "dashed", "wavy"],
            )
        },
        // --- Letter/word spacing ---
        "letter-spacing" => |_, input| misc::parse_spacing(input, "letter-spacing"),
        "word-spacing" => |_, input| misc::parse_spacing(input, "word-spacing"),

        // --- White-space ---
        "white-space" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["normal", "pre", "nowrap", "pre-wrap", "pre-line"],
            )
        },

        // --- Overflow ---
        "overflow" => |_, input| misc::parse_overflow(input),
        "overflow-x" | "overflow-y" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["visible", "hidden", "scroll", "auto", "clip"],
            )
        },

        // --- Min/max sizing ---
        "min-width" | "min-height" => |name, input| {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            parse_value_property(input, name, parse_non_negative_length_or_percentage)
        },
        "max-width" | "max-height" => |name, input| misc::parse_max_dimension(input, name),

        // --- List style ---
        "list-style-type" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["disc", "circle", "square", "decimal", "none"],
            )
        },
        "list-style" => |_, input| misc::parse_list_style_shorthand(input),

        // --- Gap properties ---
        "row-gap" | "column-gap" => {
            |name, input| parse_value_property(input, name, misc::parse_gap_value)
        }
        "gap" => |_, input| misc::parse_gap_shorthand(input),

        // --- Flex keyword properties ---
        "flex-direction" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["row", "row-reverse", "column", "column-reverse"],
            )
        },
        "flex-wrap" => {
            |name, input| parse_keyword_property(input, name, &["nowrap", "wrap", "wrap-reverse"])
        }
        "justify-content" => |name, input| {
            parse_keyword_property(
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
            )
        },
        "align-items" | "align-self" | "justify-items" | "justify-self" => |name, input| {
            elidex_plugin::css_resolve::parse_item_alignment(name, input)
                .map(|value| single_decl(name, value))
                .unwrap_or_default()
        },
        "align-content" => |name, input| {
            parse_keyword_property(
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
            )
        },

        // --- Flex number properties ---
        "flex-grow" | "flex-shrink" => |name, input| {
            parse_value_property(input, name, elidex_plugin::css_resolve::parse_flex_factor)
        },
        "order" => |name, input| flex::parse_integer_property(input, name),

        // --- Flex basis ---
        "flex-basis" => |name, input| {
            parse_value_property(input, name, elidex_plugin::css_resolve::parse_flex_basis)
        },

        // --- Flex shorthands ---
        "flex" => |_, input| flex::parse_flex_shorthand(input),
        "flex-flow" => |_, input| flex::parse_flex_flow_shorthand(input),

        // --- Grid container properties ---
        "grid-template-columns" | "grid-template-rows" => {
            |name, input| grid::parse_grid_template(input, name)
        }
        "grid-template-areas" => |_, input| grid_shorthand::parse_grid_template_areas(input),
        "grid-auto-flow" => |_, input| grid_shorthand::parse_grid_auto_flow(input),
        "grid-auto-columns" | "grid-auto-rows" => {
            |name, input| grid_shorthand::parse_grid_auto_track(input, name)
        }

        // --- Grid item properties ---
        "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
            |name, input| grid_shorthand::parse_grid_line(input, name)
        }

        // --- Grid shorthands ---
        "grid-column" | "grid-row" => {
            |name, input| grid_shorthand::parse_grid_line_shorthand(input, name)
        }
        "grid-area" => |_, input| grid_shorthand::parse_grid_area(input),
        "grid-template" => |_, input| grid_shorthand::parse_grid_template_shorthand(input),
        "grid" => |_, input| grid_shorthand::parse_grid_shorthand(input),

        // --- Multi-column shorthands ---
        "column-rule" => |_, input| misc::parse_column_rule_shorthand(input),
        "columns" => |_, input| misc::parse_columns_shorthand(input),

        // --- Writing mode / BiDi properties ---
        "direction" => |name, input| parse_keyword_property(input, name, &["ltr", "rtl"]),
        "unicode-bidi" => |name, input| {
            parse_keyword_property(
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
            )
        },
        "writing-mode" => |name, input| {
            parse_keyword_property(
                input,
                name,
                &["horizontal-tb", "vertical-rl", "vertical-lr"],
            )
        },
        "text-orientation" => {
            |name, input| parse_keyword_property(input, name, &["mixed", "upright", "sideways"])
        }

        // --- Position offsets ---
        "top" | "right" | "bottom" | "left" => |name, input| {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            parse_value_property(input, name, parse_length_or_percentage)
        },
        "z-index" => |name, input| {
            if let Ok(val) = try_keyword_value(input, "auto", &CssValue::Auto) {
                return single_decl(name, val);
            }
            flex::parse_integer_property(input, name)
        },

        // --- Float/clear/visibility ---
        "float" => |name, input| parse_keyword_property(input, name, &["none", "left", "right"]),
        "clear" => {
            |name, input| parse_keyword_property(input, name, &["none", "left", "right", "both"])
        }
        "visibility" => {
            |name, input| parse_keyword_property(input, name, &["visible", "hidden", "collapse"])
        }
        "vertical-align" => |_, input| misc::parse_vertical_align(input),

        // --- Table properties ---
        "border-collapse" => {
            |name, input| parse_keyword_property(input, name, &["separate", "collapse"])
        }
        "border-spacing" => |_, input| misc::parse_border_spacing(input),
        "table-layout" => |name, input| parse_keyword_property(input, name, &["auto", "fixed"]),
        "caption-side" => |name, input| {
            parse_keyword_property(input, name, &["top", "bottom", "block-start", "block-end"])
        },

        // --- Content property ---
        "content" => |_, input| misc::parse_content(input),

        // --- Counter properties ---
        "counter-reset" | "counter-set" => |name, input| misc::parse_counter_list(input, name, 0),
        "counter-increment" => |name, input| misc::parse_counter_list(input, name, 1),

        _ => return None,
    })
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
