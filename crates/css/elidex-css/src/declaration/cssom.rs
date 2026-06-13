//! CSSOM-facing inline-style entry points: `parse a CSS declaration
//! block` over the `style` content attribute (`parse_inline_style`), the
//! `CSSStyleDeclaration.setProperty` value/name parse
//! (`parse_value_for_property`), and the round-trip-safe storage
//! serialization (`serialize_declaration_value_for_storage`). Split out of
//! `declaration.rs` to keep the parser dispatcher under the 1000-line
//! limit; these are the consumers of the parser internals, not the parser
//! itself.

use cssparser::{Parser, ParserInput, Token};
use elidex_ecs::InlineStyle;
use elidex_plugin::{CssPropertyRegistry, CssValue};

use super::{parse_declaration_block_with_registry, parse_property_value, Declaration};

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
/// Built on [`super::parse_declaration_block`], so the component holds the
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
        // CSS Cascade 4 §6.3: an important declaration takes precedence
        // over a normal one for the same property. Collapsing duplicate
        // declarations to a single `InlineStyle` entry must not let a
        // later *normal* declaration overwrite an earlier *important* one
        // — otherwise `css_text()` write-back would permanently drop the
        // cascade-winning important value (e.g.
        // `color: red !important; color: blue` ⇒ should stay `red`).
        if !decl.important && style.is_important(&decl.property) {
            continue;
        }
        let stored = serialize_declaration_value_for_storage(&decl.property, &decl.value, registry);
        style.set_with_priority(decl.property, stored, decl.important);
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
        // CSSOM §6.6.1 setProperty proceeds only for a *valid* custom
        // property name. A name carrying CSS delimiters (`--x;color`)
        // would be written back verbatim by `InlineStyle::css_text()` and
        // then split by the cascade's declaration-block re-parse,
        // injecting a fabricated declaration (`--x;color: red` ⇒ applied
        // `color: red`). Require a single `<ident-token>` spanning the
        // whole name (CSS Syntax 3 dashed-ident).
        if !is_valid_custom_property_name(property) {
            return None;
        }
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

/// Is `name` a valid CSS custom-property name (CSS Syntax 3 — a `--`
/// dashed-ident, i.e. a single `<ident-token>` spanning the whole string)?
///
/// Rejects names that merely start with `--` but carry CSS delimiters
/// (`;`, `:`, whitespace, …): those tokenize as multiple tokens, so a
/// verbatim `css_text()` write-back would let the cascade's
/// declaration-block re-parse split off an injected declaration. An
/// escaped delimiter (`--a\;b`) is a single ident token and is accepted —
/// it round-trips with the escape intact.
fn is_valid_custom_property_name(name: &str) -> bool {
    if !name.starts_with("--") {
        return false;
    }
    let mut pi = ParserInput::new(name);
    let mut parser = Parser::new(&mut pi);
    parser.expect_ident().is_ok() && parser.is_exhausted()
}

/// Serialize a parsed declaration `value` for storage in [`InlineStyle`]
/// such that it round-trips through the cascade's re-parse of
/// `attrs("style")`.
///
/// The inline-style write-back stores the *serialized* value string (the
/// CSSOM canonical form, so `el.style.color = "red"` reads back
/// `#ff0000`). [`CssValue::to_css_string`] comma-joins every
/// [`CssValue::List`] because the type does not record its separator
/// (slot `#11-cssvalue-list-separator-fidelity`); for space-separated
/// list properties (`text-decoration-line`, grid track lists) the
/// comma-joined string does NOT re-parse, so the declaration silently
/// vanishes from cascade-visible CSS. Before this single-derivation
/// unification the inline path stored raw attribute substrings (which
/// round-tripped by construction); this guard restores that guarantee on
/// the canonical-serialization path.
///
/// Only [`CssValue::List`] can serialize to a non-round-tripping form, so
/// every other value (colour / length / keyword / …) takes the canonical
/// string with no re-parse. For a list whose canonical (comma) form does
/// not re-parse, fall back to the space-joined form, then to the
/// canonical form (no worse than before).
#[must_use]
pub fn serialize_declaration_value_for_storage(
    property: &str,
    value: &CssValue,
    registry: Option<&CssPropertyRegistry>,
) -> String {
    let canonical = value.to_css_string();
    let CssValue::List(items) = value else {
        return canonical;
    };
    if reparses_to_same(property, &canonical, value, registry) {
        return canonical;
    }
    let spaced = items
        .iter()
        .map(CssValue::to_css_string)
        .collect::<Vec<_>>()
        .join(" ");
    if reparses_to_same(property, &spaced, value, registry) {
        return spaced;
    }
    canonical
}

/// Does re-parsing `serialized` for `property` reproduce exactly `value`?
/// Used by [`serialize_declaration_value_for_storage`] to detect a lossy
/// canonical serialization.
fn reparses_to_same(
    property: &str,
    serialized: &str,
    value: &CssValue,
    registry: Option<&CssPropertyRegistry>,
) -> bool {
    matches!(
        parse_value_for_property(property, serialized, registry).as_deref(),
        Some([decl]) if decl.property == property && &decl.value == value
    )
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
