//! CSS declaration keywords and shared strict numeric grammar.
pub use elidex_plugin::css_resolve::strict::{
    parse_length, parse_length_or_percentage, parse_length_percentage_or_auto, parse_length_unit,
    parse_non_negative_length_or_percentage,
};
use elidex_plugin::CssValue;

/// Check if an identifier is a CSS global keyword (`initial`, `inherit`, `unset`).
pub fn parse_global_keyword(ident: &str) -> Option<CssValue> {
    match ident.to_ascii_lowercase().as_str() {
        "initial" => Some(CssValue::Initial),
        "inherit" => Some(CssValue::Inherit),
        "unset" => Some(CssValue::Unset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_keywords() {
        assert_eq!(parse_global_keyword("initial"), Some(CssValue::Initial));
        assert_eq!(parse_global_keyword("inherit"), Some(CssValue::Inherit));
        assert_eq!(parse_global_keyword("unset"), Some(CssValue::Unset));
        assert_eq!(parse_global_keyword("INITIAL"), Some(CssValue::Initial));
        assert_eq!(parse_global_keyword("something"), None);
    }
}
