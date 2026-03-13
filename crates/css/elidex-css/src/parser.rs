//! CSS stylesheet parser.
//!
//! Uses the `cssparser` crate for tokenization and rule-level parsing,
//! delegating property value parsing to [`crate::declaration`].

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
};

use crate::declaration::{parse_property_value, Declaration, Origin};
use crate::selector::{parse_selector_list, Selector};

/// A parsed CSS stylesheet.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    /// The cascade origin of this stylesheet.
    pub origin: Origin,
    /// Rules in source order.
    pub rules: Vec<CssRule>,
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
#[must_use]
pub fn parse_stylesheet(css: &str, origin: Origin) -> Stylesheet {
    let mut pi = ParserInput::new(css);
    let mut input = Parser::new(&mut pi);
    let mut rules = Vec::new();
    let mut source_order: u32 = 0;

    let mut rule_parser = RuleListParser {
        source_order: &mut source_order,
    };

    for rule in StyleSheetParser::new(&mut input, &mut rule_parser).flatten() {
        rules.push(rule);
    }

    Stylesheet { origin, rules }
}

// --- cssparser trait implementations ---

struct RuleListParser<'a> {
    source_order: &'a mut u32,
}

/// We don't support at-rules in Phase 1; all are silently dropped.
impl AtRuleParser<'_> for RuleListParser<'_> {
    type Prelude = ();
    type AtRule = CssRule;
    type Error = ();
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
        let mut decl_parser = DeclarationListParser;
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

struct DeclarationListParser;

impl<'i> DeclarationParser<'i> for DeclarationListParser {
    type Declaration = Vec<Declaration>;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _start: &cssparser::ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, ()>> {
        let lower_name = name.to_ascii_lowercase();
        let decls = parse_property_value(&lower_name, input);
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

impl AtRuleParser<'_> for DeclarationListParser {
    type Prelude = ();
    type AtRule = Vec<Declaration>;
    type Error = ();
}

impl QualifiedRuleParser<'_> for DeclarationListParser {
    type Prelude = ();
    type QualifiedRule = Vec<Declaration>;
    type Error = ();
}

impl RuleBodyItemParser<'_, Vec<Declaration>, ()> for DeclarationListParser {
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
