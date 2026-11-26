//! CSS Paged Media 3 §§4.2–4.4: a non-forgiving list of compound page selectors.
use crate::Specificity;
use cssparser::{ParseError, Parser, Token};
use elidex_plugin::PageSelector;

/// One alternative of the selector list on a page rule.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageAlternative {
    pub name: Option<String>,
    pub pseudos: Vec<PageSelector>,
    pub specificity: Specificity,
}

impl PageAlternative {
    #[must_use]
    pub fn matches(&self, name: Option<&str>, number: usize, blank: bool) -> bool {
        self.name
            .as_deref()
            .is_none_or(|required| !required.eq_ignore_ascii_case("auto") && Some(required) == name)
            && self
                .pseudos
                .iter()
                .all(|pseudo| pseudo.matches(number, blank))
    }
}

pub(crate) fn alternatives<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<Vec<PageAlternative>, ParseError<'i, ()>> {
    if input.is_exhausted() {
        return Ok(vec![PageAlternative::default()]);
    }
    input.parse_comma_separated(|part| {
        part.skip_whitespace();
        let name = part
            .try_parse(Parser::expect_ident_cloned)
            .ok()
            .map(|ident| ident.to_string());
        let mut specificity = Specificity {
            id: u16::from(name.is_some()),
            ..Specificity::default()
        };
        let mut pseudos = Vec::new();
        // is_exhausted ignores trailing whitespace but leaves the cursor in place.
        while !part.is_exhausted() {
            if !matches!(part.next_including_whitespace()?, Token::Colon) {
                return Err(part.new_custom_error(()));
            }
            let Token::Ident(ident) = part.next_including_whitespace()? else {
                return Err(part.new_custom_error(()));
            };
            let pseudo =
                PageSelector::from_keyword(ident).ok_or_else(|| part.new_custom_error(()))?;
            match pseudo {
                PageSelector::First | PageSelector::Blank => {
                    specificity.class = specificity.class.saturating_add(1);
                }
                PageSelector::Left | PageSelector::Right => {
                    specificity.tag = specificity.tag.saturating_add(1);
                }
            }
            pseudos.push(pseudo);
        }
        if name.is_none() && pseudos.is_empty() {
            return Err(part.new_custom_error(()));
        }
        Ok(PageAlternative {
            name,
            pseudos,
            specificity,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;
    fn parse(css: &str) -> Option<Vec<PageAlternative>> {
        alternatives(&mut Parser::new(&mut ParserInput::new(css))).ok()
    }
    #[test]
    fn rejects_the_entire_invalid_list() {
        for css in [
            ":first, :unknown",
            ":left,",
            "name :first",
            ": first",
            ":right :blank",
            ",",
            "left#x",
        ] {
            assert!(parse(css).is_none(), "{css}");
        }
        assert!(parse("  ").unwrap()[0].matches(None, 1, false));
    }
    #[test]
    fn named_and_compound_matches_retain_specificity() {
        let list = parse("Invoice:first:first, :right, :blank:left").unwrap();
        assert_eq!(
            list[0].specificity,
            Specificity {
                id: 1,
                class: 2,
                tag: 0
            }
        );
        assert!(list[0].matches(Some("Invoice"), 1, false));
        assert!(!list[0].matches(None, 1, false));
        assert!(!list[0].matches(Some("invoice"), 1, false));
        assert!(list[1].matches(None, 1, false));
        assert!(!list[2].matches(None, 1, true));
        assert!(list[2].matches(None, 2, true));
        assert!(!parse("AUTO").unwrap()[0].matches(Some("AUTO"), 1, false));
    }
}
