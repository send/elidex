//! CSS selector parsing, matching, and specificity.
//!
//! Supports: universal (`*`), tag, class, id, descendant (space), child (`>`),
//! adjacent sibling (`+`), general sibling (`~`), attribute selectors,
//! pseudo-classes (`:root`, `:first-child`, `:last-child`, `:only-child`,
//! `:empty`, `:hover`, `:focus`, `:active`, `:link`, `:visited`),
//! and negation (`:not()`).

mod matching;
pub(crate) mod parse;
mod traverse;
mod types;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

pub use types::*;

use cssparser::{Parser, ParserInput};
use elidex_ecs::{EcsDom, Entity};

use matching::match_components;
use parse::parse_one_selector;

/// A parsed CSS selector with its computed specificity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector {
    /// Components stored right-to-left for efficient matching.
    pub components: Vec<SelectorComponent>,
    /// Computed specificity.
    pub specificity: Specificity,
    /// Optional pseudo-element (`::before`, `::after`).
    ///
    /// Pseudo-elements must appear at the end of a selector and are not
    /// part of element matching -- they are stored separately and used
    /// during style resolution to generate content.
    pub pseudo_element: Option<PseudoElement>,
}

impl Selector {
    /// Check if this selector matches the given entity in the DOM.
    pub fn matches(&self, entity: Entity, dom: &EcsDom) -> bool {
        if self.components.is_empty() {
            // Empty components with a pseudo-element (e.g., `::before` alone)
            // is equivalent to `*::before` -- matches any element.
            return self.pseudo_element.is_some();
        }
        match_components(&self.components, 0, entity, dom)
    }
}

/// Parse a comma-separated list of selectors.
#[must_use = "parsing result should be used"]
#[allow(clippy::result_unit_err)]
pub fn parse_selector_list(input: &mut Parser) -> Result<Vec<Selector>, ()> {
    let mut selectors = vec![parse_one_selector(input)?];
    while input
        .try_parse(|i| i.expect_comma().map_err(|_| ()))
        .is_ok()
    {
        selectors.push(parse_one_selector(input)?);
    }
    Ok(selectors)
}

/// Parse a comma-separated list of selectors from a string.
///
/// Convenience wrapper around [`parse_selector_list`] that handles
/// `ParserInput` / `Parser` creation internally, so callers don't need
/// a `cssparser` dependency.
#[must_use = "parsing result should be used"]
#[allow(clippy::result_unit_err)]
pub fn parse_selector_from_str(selector: &str) -> Result<Vec<Selector>, ()> {
    let mut input = ParserInput::new(selector);
    let mut parser = Parser::new(&mut input);
    parse_selector_list(&mut parser)
}
