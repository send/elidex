use super::{parse_page_size, PageAlternative, MARGIN_BOX_NAMES};
use crate::{declaration::substitution, parser::DeclarationListParser, Declaration};
use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser,
};
use elidex_plugin::CssPropertyRegistry;

/// A page rule belongs to its containing stylesheet, including its origin and grammar.
#[derive(Clone, Debug)]
pub struct PageRule {
    pub alternatives: Vec<PageAlternative>,
    pub declarations: Vec<Declaration>,
    pub boxes: Vec<(String, Vec<Declaration>)>,
}

pub(crate) fn parse_body(
    input: &mut Parser,
    alternatives: Vec<PageAlternative>,
    registry: Option<&CssPropertyRegistry>,
) -> PageRule {
    let mut rule = PageRule {
        alternatives,
        declarations: Vec::new(),
        boxes: Vec::new(),
    };
    let mut body = Body {
        declarations: DeclarationListParser { registry },
    };
    for item in RuleBodyParser::new(input, &mut body).flatten() {
        match item {
            Item::Declarations(decls) => rule.declarations.extend(decls),
            Item::Box(name, decls) => rule.boxes.push((name, decls)),
        }
    }
    rule
}
struct Body<'a> {
    declarations: DeclarationListParser<'a>,
}
enum Item {
    Declarations(Vec<Declaration>),
    Box(String, Vec<Declaration>),
}
impl<'i> DeclarationParser<'i> for Body<'_> {
    type Declaration = Item;
    type Error = ();
    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        start: &ParserState,
    ) -> Result<Item, ParseError<'i, ()>> {
        if name.eq_ignore_ascii_case("size") {
            let value = parse_page_size(substitution::source(input))
                .ok_or_else(|| input.new_custom_error(()))?;
            let mut decl = Declaration::new("size", value);
            decl.important = input.try_parse(cssparser::parse_important).is_ok();
            Ok(Item::Declarations(vec![decl]))
        } else {
            self.declarations
                .parse_value(name, input, start)
                .map(Item::Declarations)
        }
    }
}
impl<'i> AtRuleParser<'i> for Body<'_> {
    type Prelude = String;
    type AtRule = Item;
    type Error = ();
    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<String, ParseError<'i, ()>> {
        let name = name.to_ascii_lowercase();
        if MARGIN_BOX_NAMES.contains(&name.as_str()) && input.is_exhausted() {
            Ok(name)
        } else {
            Err(input.new_custom_error(()))
        }
    }
    fn parse_block<'t>(
        &mut self,
        name: String,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Item, ParseError<'i, ()>> {
        Ok(Item::Box(
            name,
            RuleBodyParser::new(input, &mut self.declarations)
                .flatten()
                .flatten()
                .collect(),
        ))
    }
}
impl QualifiedRuleParser<'_> for Body<'_> {
    type Prelude = ();
    type QualifiedRule = Item;
    type Error = ();
}
impl RuleBodyItemParser<'_, Item, ()> for Body<'_> {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        false
    }
}
