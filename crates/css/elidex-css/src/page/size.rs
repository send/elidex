//! Specified page size (CSS Paged Media 3 §7.1).
use crate::declaration::substitution;
use cssparser::{Parser, ParserInput, Token};
use elidex_plugin::{
    component_values::ComponentValues, css_resolve::strict, CssValue, NamedPageSize,
};

/// The same receiving grammar is used at admission and after variable substitution.
#[must_use]
pub fn parse_page_size(source: &str) -> Option<CssValue> {
    if ComponentValues::inspect_variables(source)? {
        return substitution::capture(source).map(CssValue::Pending);
    }
    let mut tokens = ParserInput::new(source);
    let mut input = Parser::new(&mut tokens);
    let mut result = Vec::new();
    if let Ok(ident) = input.try_parse(|p| p.expect_ident_cloned()) {
        let mut name = None;
        let mut orientation = None;
        let mut word = ident.to_ascii_lowercase();
        if input.is_exhausted() {
            if let Some(global) = crate::values::parse_global_keyword(&word) {
                return Some(global);
            }
        }
        if word == "auto" && input.is_exhausted() {
            return Some(CssValue::Keyword(word));
        }
        loop {
            if NamedPageSize::from_keyword(&word).is_some() && name.is_none() {
                name = Some(CssValue::Keyword(word));
            } else if matches!(word.as_str(), "portrait" | "landscape") && orientation.is_none() {
                orientation = Some(CssValue::Keyword(word));
            } else {
                return None;
            }
            if input.is_exhausted() {
                break;
            }
            word = input.expect_ident().ok()?.to_ascii_lowercase();
        }
        result.extend(name);
        result.extend(orientation);
    } else {
        while result.len() < 2 && !input.is_exhausted() {
            if input
                .try_parse(|p| match p.next()? {
                    Token::Dimension { value, .. } if *value < 0.0 => Ok(()),
                    _ => Err(p.new_custom_error::<(), ()>(())),
                })
                .is_ok()
            {
                return None;
            }
            result.push(strict::parse_length(&mut input).ok()?);
        }
    }
    (!result.is_empty() && input.is_exhausted()).then_some(CssValue::List(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consumes_the_whole_descriptor() {
        for source in [
            "auto",
            "landscape",
            "portrait A4",
            "letter landscape",
            "0",
            "1em 10vw",
            "calc(-1px)",
            "calc(1in + 12pt)",
            "var(--sheet)",
        ] {
            assert!(parse_page_size(source).is_some(), "{source}");
        }
        for source in [
            "",
            "auto 1px",
            "A4 letter",
            "portrait landscape",
            "2px 3px 4px",
            "8px 5%",
            "1px 3foo",
            "-1px",
            "portrait 5px",
            "5px landscape",
        ] {
            assert!(parse_page_size(source).is_none(), "{source}");
        }
    }
}
