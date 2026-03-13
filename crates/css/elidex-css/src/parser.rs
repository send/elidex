//! CSS stylesheet parser.
//!
//! Uses the `cssparser` crate for tokenization and rule-level parsing,
//! delegating property value parsing to [`crate::declaration`].

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
};

use elidex_plugin::CssPropertyRegistry;

use crate::declaration::{parse_property_value, Declaration, Origin};
use crate::selector::{parse_selector_list, Selector};

/// A parsed CSS stylesheet.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    /// The cascade origin of this stylesheet.
    pub origin: Origin,
    /// Rules in source order.
    pub rules: Vec<CssRule>,
    /// Raw `@keyframes` blocks: `(name, body_text)`.
    ///
    /// Extracted during parsing. The body text is the content between
    /// the outer `{ }` braces and must be parsed by the animation handler
    /// (e.g. `elidex_css_anim::parse::parse_keyframes`).
    pub keyframes_raw: Vec<(String, String)>,
}

/// A single CSS rule (selector list + declarations).
#[derive(Clone, Debug)]
pub struct CssRule {
    /// Selectors for this rule.
    pub selectors: Vec<Selector>,
    /// Declarations (all longhand, shorthands already expanded).
    pub declarations: Vec<Declaration>,
    /// Position in the source stylesheet (0-based).
    pub source_order: u32,
}

/// Parse a CSS string into a [`Stylesheet`].
///
/// Invalid rules are silently skipped per CSS error recovery rules.
/// `@keyframes` rules are extracted into [`Stylesheet::keyframes_raw`].
#[must_use]
pub fn parse_stylesheet(css: &str, origin: Origin) -> Stylesheet {
    parse_stylesheet_with_registry(css, origin, None)
}

/// Parse a CSS string into a [`Stylesheet`], with optional handler registry.
///
/// When a `registry` is provided, properties not handled by the built-in parser
/// (e.g. `transition-*`, `animation-*`) are dispatched to the matching
/// [`CssPropertyHandler`](elidex_plugin::CssPropertyHandler) for parsing.
///
/// `@keyframes` rules are always extracted into [`Stylesheet::keyframes_raw`].
#[must_use]
pub fn parse_stylesheet_with_registry(
    css: &str,
    origin: Origin,
    registry: Option<&CssPropertyRegistry>,
) -> Stylesheet {
    let mut pi = ParserInput::new(css);
    let mut input = Parser::new(&mut pi);
    let mut rules = Vec::new();
    let mut source_order: u32 = 0;
    let mut keyframes_raw = Vec::new();

    let mut rule_parser = RuleListParser {
        source_order: &mut source_order,
        keyframes_raw: &mut keyframes_raw,
        registry,
    };

    for rule in StyleSheetParser::new(&mut input, &mut rule_parser).flatten() {
        rules.push(rule);
    }

    Stylesheet {
        origin,
        rules,
        keyframes_raw,
    }
}

// --- cssparser trait implementations ---

struct RuleListParser<'a> {
    source_order: &'a mut u32,
    keyframes_raw: &'a mut Vec<(String, String)>,
    registry: Option<&'a CssPropertyRegistry>,
}

/// `@keyframes` rules are parsed and stored in `keyframes_raw`.
/// All other at-rules are silently dropped.
impl<'i> AtRuleParser<'i> for RuleListParser<'_> {
    type Prelude = String;
    type AtRule = CssRule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, ()>> {
        if name.eq_ignore_ascii_case("keyframes") || name.eq_ignore_ascii_case("-webkit-keyframes")
        {
            let ident = input.expect_ident().map_err(ParseError::from)?;
            Ok(ident.as_ref().to_string())
        } else {
            Err(input.new_custom_error(()))
        }
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::AtRule, ParseError<'i, ()>> {
        // Collect the raw block text by consuming all tokens (including
        // nested curly-bracket blocks) and using slice_from.
        let start_pos = input.position();
        while input.next_including_whitespace_and_comments().is_ok() {}
        let body = input.slice_from(start_pos).to_string();

        self.keyframes_raw.push((prelude, body));

        // Return Err so no CssRule is produced — @keyframes are stored
        // separately in Stylesheet::keyframes_raw.
        Err(input.new_custom_error(()))
    }
}

impl<'i> QualifiedRuleParser<'i> for RuleListParser<'_> {
    type Prelude = Vec<Selector>;
    type QualifiedRule = CssRule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, ()>> {
        parse_selector_list(input).map_err(|()| input.new_custom_error(()))
    }

    fn parse_block<'t>(
        &mut self,
        selectors: Self::Prelude,
        _location: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, ()>> {
        let mut decl_parser = DeclarationListParser {
            registry: self.registry,
        };
        let mut declarations = Vec::new();

        for decls in RuleBodyParser::new(input, &mut decl_parser).flatten() {
            declarations.extend(decls);
        }

        let order = *self.source_order;
        *self.source_order = self.source_order.saturating_add(1);

        Ok(CssRule {
            selectors,
            declarations,
            source_order: order,
        })
    }
}

struct DeclarationListParser<'a> {
    registry: Option<&'a CssPropertyRegistry>,
}

impl<'i> DeclarationParser<'i> for DeclarationListParser<'_> {
    type Declaration = Vec<Declaration>;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _start: &cssparser::ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, ()>> {
        let lower_name = name.to_ascii_lowercase();
        let decls = parse_property_value(&lower_name, input, self.registry);
        if decls.is_empty() {
            Err(input.new_custom_error(()))
        } else {
            // Check for !important after successfully parsing the value.
            let important = input.try_parse(cssparser::parse_important).is_ok();
            if important {
                Ok(decls
                    .into_iter()
                    .map(|mut d| {
                        d.important = true;
                        d
                    })
                    .collect())
            } else {
                Ok(decls)
            }
        }
    }
}

impl AtRuleParser<'_> for DeclarationListParser<'_> {
    type Prelude = ();
    type AtRule = Vec<Declaration>;
    type Error = ();
}

impl QualifiedRuleParser<'_> for DeclarationListParser<'_> {
    type Prelude = ();
    type QualifiedRule = Vec<Declaration>;
    type Error = ();
}

impl RuleBodyItemParser<'_, Vec<Declaration>, ()> for DeclarationListParser<'_> {
    fn parse_qualified(&self) -> bool {
        false
    }

    fn parse_declarations(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::parse_declaration_block;
    use elidex_plugin::{CssColor, CssValue};

    #[test]
    fn parse_single_rule() {
        let ss = parse_stylesheet("div { color: red; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors.len(), 1);
        assert_eq!(ss.rules[0].declarations.len(), 1);
        assert_eq!(ss.rules[0].declarations[0].property, "color");
        assert_eq!(
            ss.rules[0].declarations[0].value,
            CssValue::Color(CssColor::RED)
        );
    }

    #[test]
    fn parse_multiple_rules() {
        let css = "div { color: red; } p { display: block; }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 2);
    }

    #[test]
    fn at_rule_silently_dropped() {
        let css = "@media screen { div { color: red; } } p { display: block; }";
        let ss = parse_stylesheet(css, Origin::Author);
        // @media is skipped; p rule survives.
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].declarations[0].property, "display");
    }

    #[test]
    fn keyframes_extracted() {
        let css = r"
            @keyframes fadeIn {
                from { opacity: 0; }
                to { opacity: 1; }
            }
            p { color: red; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 1, "only the p rule should be in rules");
        assert_eq!(ss.keyframes_raw.len(), 1);
        assert_eq!(ss.keyframes_raw[0].0, "fadeIn");
        assert!(ss.keyframes_raw[0].1.contains("opacity"));
    }

    #[test]
    fn webkit_keyframes_extracted() {
        let css = "@-webkit-keyframes slide { from { left: 0; } to { left: 100px; } }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.keyframes_raw.len(), 1);
        assert_eq!(ss.keyframes_raw[0].0, "slide");
    }

    #[test]
    fn multiple_keyframes() {
        let css = r"
            @keyframes a { from { opacity: 0; } to { opacity: 1; } }
            @keyframes b { 0% { width: 0; } 100% { width: 100px; } }
            div { color: red; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.keyframes_raw.len(), 2);
        assert_eq!(ss.keyframes_raw[0].0, "a");
        assert_eq!(ss.keyframes_raw[1].0, "b");
    }

    #[test]
    fn parse_inline_declarations() {
        let decls = parse_declaration_block("color: red; margin: 10px");
        // color: 1 decl + margin: 4 decls = 5
        assert_eq!(decls.len(), 5);
        assert_eq!(decls[0].property, "color");
        assert_eq!(decls[1].property, "margin-top");
    }

    #[test]
    fn source_order_preserved() {
        let css = "a { color: red; } b { color: blue; } c { color: green; }";
        let ss = parse_stylesheet(css, Origin::Author);
        assert_eq!(ss.rules.len(), 3);
        assert_eq!(ss.rules[0].source_order, 0);
        assert_eq!(ss.rules[1].source_order, 1);
        assert_eq!(ss.rules[2].source_order, 2);
    }

    #[test]
    fn selector_list_rule() {
        let ss = parse_stylesheet("h1, h2 { color: blue; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors.len(), 2);
    }

    #[test]
    fn empty_stylesheet() {
        let ss = parse_stylesheet("", Origin::Author);
        assert!(ss.rules.is_empty());
    }

    #[test]
    fn important_declaration() {
        let ss = parse_stylesheet("div { color: red !important; }", Origin::Author);
        assert_eq!(ss.rules.len(), 1);
        assert!(ss.rules[0].declarations[0].important);
    }
}
