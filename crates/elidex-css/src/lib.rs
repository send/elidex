//! CSS parser, value types, and selector engine for elidex.
//!
//! Provides CSS tokenization, property parsing, selector matching,
//! and typed CSS value representations.

mod color;
mod declaration;
mod parser;
mod selector;
mod values;

pub use declaration::{parse_declaration_block, parse_var_function, Declaration, Origin};
pub use parser::{parse_stylesheet, CssRule, Stylesheet};
pub use selector::{
    parse_selector_from_str, parse_selector_list, Selector, SelectorComponent, Specificity,
};

use cssparser::{Parser, ParserInput};
use elidex_plugin::CssValue;

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
    if let Ok(c) = try_parse_exhaustive(&mut parser, color::parse_color) {
        return CssValue::Color(c);
    }

    // Try length/percentage/auto.
    if let Ok(val) = try_parse_exhaustive(&mut parser, values::parse_length_percentage_or_auto) {
        return val;
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
}
