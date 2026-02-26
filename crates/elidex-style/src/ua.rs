//! Default User-Agent (UA) stylesheet.
//!
//! Provides browser default styles per the HTML specification.
//! Parsed once and cached via `OnceLock`.

use std::sync::OnceLock;

use elidex_css::{parse_stylesheet, Origin, Stylesheet};

/// The UA stylesheet CSS source.
///
/// Based on the HTML specification's rendering section, covering the
/// Phase 1 property set.
const UA_CSS: &str = r"
html, body, div, p, h1, h2, h3, h4, h5, h6,
ul, ol, li, dl, dt, dd, blockquote, pre,
form, fieldset, table, address, article, aside,
details, figcaption, figure, footer, header,
main, nav, section, summary, hr {
    display: block;
}

head, link, meta, script, style, title {
    display: none;
}

body {
    margin: 8px;
}

h1 {
    font-size: 32px;
    margin-top: 21px;
    margin-bottom: 21px;
}

h2 {
    font-size: 24px;
    margin-top: 19px;
    margin-bottom: 19px;
}

h3 {
    font-size: 19px;
    margin-top: 18px;
    margin-bottom: 18px;
}

h4 {
    font-size: 16px;
    margin-top: 21px;
    margin-bottom: 21px;
}

h5 {
    font-size: 13px;
    margin-top: 22px;
    margin-bottom: 22px;
}

h6 {
    font-size: 11px;
    margin-top: 24px;
    margin-bottom: 24px;
}

p {
    margin-top: 16px;
    margin-bottom: 16px;
}

ul, ol {
    margin-top: 16px;
    margin-bottom: 16px;
    padding-left: 40px;
}

/* Spec: display: list-item. Using block until list markers are supported. */
li {
    display: block;
}

blockquote {
    margin-top: 16px;
    margin-bottom: 16px;
    margin-left: 40px;
    margin-right: 40px;
}

pre {
    margin-top: 16px;
    margin-bottom: 16px;
}

hr {
    border-top-style: solid;
    border-top-width: 1px;
    margin-top: 8px;
    margin-bottom: 8px;
}
";

/// Returns the parsed UA stylesheet (lazily initialized, cached).
pub fn ua_stylesheet() -> &'static Stylesheet {
    static UA: OnceLock<Stylesheet> = OnceLock::new();
    UA.get_or_init(|| parse_stylesheet(UA_CSS, Origin::UserAgent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssValue, LengthUnit};

    #[test]
    fn ua_parses_without_error() {
        let ss = ua_stylesheet();
        assert!(!ss.rules.is_empty());
        assert_eq!(ss.origin, Origin::UserAgent);
    }

    #[test]
    fn body_has_margin_8px() {
        let ss = ua_stylesheet();
        // Find the body rule that has margin declarations (not the display:block rule).
        let body_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "body"))
            }) && r.declarations.iter().any(|d| d.property == "margin-top")
        });
        assert!(body_rule.is_some(), "body margin rule not found");
        let body_rule = body_rule.unwrap();
        let margin_top = body_rule
            .declarations
            .iter()
            .find(|d| d.property == "margin-top")
            .unwrap();
        assert_eq!(margin_top.value, CssValue::Length(8.0, LengthUnit::Px));
    }

    #[test]
    fn head_display_none() {
        let ss = ua_stylesheet();
        let head_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "head"))
            })
        });
        assert!(head_rule.is_some(), "head rule not found");
        let head_rule = head_rule.unwrap();
        let display = head_rule
            .declarations
            .iter()
            .find(|d| d.property == "display");
        assert!(display.is_some());
        assert_eq!(
            display.unwrap().value,
            CssValue::Keyword("none".to_string())
        );
    }
}
