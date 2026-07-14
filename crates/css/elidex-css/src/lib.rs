//! CSS parser, value types, and selector engine for elidex.
//!
//! Provides CSS tokenization, property parsing, selector matching,
//! and typed CSS value representations.

mod declaration;
pub mod escape;
pub mod media;
pub mod page;
mod parser;
mod selector;
mod serialize;
mod values;

// `parse_color` (+ the `CssColor` grammar) lives in `elidex-plugin`, co-located
// with the `CssColor` type, so CSS and `elidex-form` share one color-parse home.
// Re-exported here so existing `elidex_css::parse_color` callers stay unchanged.
pub use declaration::{
    parse_declaration_block, parse_declaration_block_with_registry, parse_inline_style,
    parse_value_for_property, parse_var_function, serialize_declaration_value_for_storage,
    shorthand_longhands, Declaration, Origin,
};
pub use escape::escape_ident;
pub use page::{parse_page_rule, parse_page_rules, parse_page_selectors, parse_page_size};
pub use parser::{
    parse_single_rule, parse_stylesheet, parse_stylesheet_with_registry, CssRule, Stylesheet,
};
pub use selector::{
    parse_selector_from_str, parse_selector_list, AttributeMatcher, PseudoElement, Selector,
    SelectorComponent, Specificity,
};
// Shorthand *serialization* (the read-side twin of the parser's shorthand
// expansion) lives in `elidex-style`: it dispatches the per-family collapse to
// each property's own `CssPropertyHandler::serialize_shorthand`, which needs the
// populated `CssPropertyRegistry` — assembled a tier above this crate. Only the
// shorthand→longhand *name* table (`shorthand_longhands`, re-exported above)
// stays here, because the parser needs it too.
pub use serialize::serialize_stylesheet;

use cssparser::{Parser, ParserInput};
use elidex_plugin::{CssValue, ParseError};
// The color parser lives in `elidex-plugin` (co-located with `CssColor`); this
// `pub use` both re-exports it as `elidex_css::parse_color` (external callers
// unchanged) and brings it into scope for the internal callers below.
pub use elidex_plugin::parse_color;

/// Parse a CSS color value, handling `currentcolor` before attempting a full
/// color parse.
///
/// Returns:
/// - `CssValue::Keyword("currentcolor")` for the `currentcolor` keyword.
/// - `CssValue::Color(c)` for any other valid CSS color.
/// - `Err(ParseError)` for invalid input.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input is not a valid CSS color value.
pub fn parse_color_with_currentcolor(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input
        .try_parse(|i| i.expect_ident_matching("currentcolor"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("currentcolor".to_string()));
    }
    parse_color(input)
        .map(CssValue::Color)
        .map_err(|()| ParseError::simple("invalid color value"))
}

/// Parse a raw CSS token string into a typed [`CssValue`].
///
/// Tries `var()`, color, length/percentage/auto, keyword in order;
/// falls back to `CssValue::RawTokens`.
#[must_use]
pub fn parse_raw_token_value(raw: &str) -> CssValue {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return CssValue::RawTokens(String::new());
    }

    let mut pi = ParserInput::new(trimmed);
    let mut parser = Parser::new(&mut pi);

    // Try var() function first.
    if let Ok(var_val) = try_parse_exhaustive(&mut parser, parse_var_function) {
        return var_val;
    }

    // Try color.
    if let Ok(c) = try_parse_exhaustive(&mut parser, parse_color) {
        return CssValue::Color(c);
    }

    // Try length/percentage/auto.
    if let Ok(val) = try_parse_exhaustive(&mut parser, values::parse_length_percentage_or_auto) {
        return val;
    }

    // Try quoted string.
    if let Ok(s) = parser.try_parse(|i| -> Result<String, ()> {
        let tok = i.next().map_err(|_| ())?;
        if let cssparser::Token::QuotedString(val) = tok {
            let s = val.to_string();
            if i.is_exhausted() {
                return Ok(s);
            }
        }
        Err(())
    }) {
        return CssValue::String(s);
    }

    // Try keyword (single ident).
    if let Ok(kw) = parser.try_parse(|i| -> Result<String, ()> {
        let tok = i.next().map_err(|_| ())?;
        if let cssparser::Token::Ident(ident) = tok {
            let s = ident.to_string();
            if i.is_exhausted() {
                return Ok(s);
            }
        }
        Err(())
    }) {
        return CssValue::Keyword(kw);
    }

    CssValue::RawTokens(trimmed.to_string())
}

/// Try a parser function and accept only if it consumed all remaining tokens.
///
/// Resets the parser position if the parse fails or if trailing tokens remain.
fn try_parse_exhaustive<T>(
    parser: &mut Parser,
    parse_fn: fn(&mut Parser) -> Result<T, ()>,
) -> Result<T, ()> {
    parser.try_parse(|i| {
        let val = parse_fn(i)?;
        if i.is_exhausted() {
            Ok(val)
        } else {
            Err(())
        }
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    /// Helper: create an element with optional class and id.
    fn elem(
        dom: &mut EcsDom,
        tag: &str,
        class: Option<&str>,
        id: Option<&str>,
    ) -> elidex_ecs::Entity {
        let mut attrs = Attributes::default();
        if let Some(c) = class {
            attrs.set("class", c);
        }
        if let Some(i) = id {
            attrs.set("id", i);
        }
        dom.create_element(tag, attrs)
    }

    #[test]
    #[allow(unused_must_use)]
    fn parse_and_match() {
        // Parse a stylesheet.
        let ss = parse_stylesheet("div.highlight { color: red; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);

        // Build a small DOM.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div_yes = elem(&mut dom, "div", Some("highlight"), None);
        let div_no = elem(&mut dom, "div", Some("other"), None);
        let span = elem(&mut dom, "span", Some("highlight"), None);
        dom.append_child(root, div_yes);
        dom.append_child(root, div_no);
        dom.append_child(root, span);

        let rule = &ss.rules[0];
        let sel = &rule.selectors[0];

        // div.highlight matches.
        assert!(sel.matches(div_yes, &dom));
        // div.other does not.
        assert!(!sel.matches(div_no, &dom));
        // span.highlight does not (wrong tag).
        assert!(!sel.matches(span, &dom));
    }

    #[test]
    fn specificity_cascade_order() {
        let css = r"
            div { color: red; }
            .highlight { color: blue; }
            #main { color: green; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 3);

        // Specificity ordering: #main > .highlight > div
        let spec_tag = ss.rules[0].selectors[0].specificity;
        let spec_class = ss.rules[1].selectors[0].specificity;
        let spec_id = ss.rules[2].selectors[0].specificity;
        assert!(spec_id > spec_class);
        assert!(spec_class > spec_tag);
    }

    #[test]
    fn important_vs_normal() {
        let css = r"
            div { color: red !important; }
            div { color: blue; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 2);

        // First rule: important.
        assert!(ss.rules[0].declarations[0].important);
        // Second rule: normal.
        assert!(!ss.rules[1].declarations[0].important);
    }

    #[test]
    fn hwb_resolves_through_declared_value_path() {
        // CSS Color 4 §8: hwb() resolves to sRGB via the shared `parse_color`
        // chokepoint, so a declaration value parses to `CssValue::Color`.
        // hwb(150 20% 10%) == rgb(20% 90% 55%) ≈ (51, 229/230, 140).
        match parse_raw_token_value("hwb(150 20% 10%)") {
            CssValue::Color(c) => {
                assert_eq!(c.r, 51);
                assert!((229..=230).contains(&c.g), "green channel {}", c.g);
                assert_eq!(c.b, 140);
                assert_eq!(c.a, 255);
            }
            other => panic!("expected CssValue::Color, got {other:?}"),
        }
    }
}
