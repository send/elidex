//! Tests for CSS selector parsing, matching, and specificity.

use super::*;
use cssparser::ParserInput;
use elidex_ecs::{Attributes, EcsDom, ElementState, Entity};

mod matching;
mod parse;
mod specificity;

fn parse_sel(css: &str) -> Result<Selector, ()> {
    let mut input = ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    super::parse::parse_one_selector(&mut parser)
}

fn parse_list(css: &str) -> Result<Vec<Selector>, ()> {
    let mut input = ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    parse_selector_list(&mut parser)
}

fn spec(id: u16, class: u16, tag: u16) -> Specificity {
    Specificity { id, class, tag }
}

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn elem_with_class(dom: &mut EcsDom, tag: &str, class: &str) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set("class", class);
    dom.create_element(tag, attrs)
}

fn elem_with_attr(dom: &mut EcsDom, tag: &str, attr: &str, value: &str) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set(attr, value);
    dom.create_element(tag, attrs)
}
