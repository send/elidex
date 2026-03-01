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
ul, ol, dl, dt, dd, blockquote, pre,
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

li {
    display: list-item;
}

ol > li {
    list-style-type: decimal;
}

pre {
    white-space: pre;
}

code, kbd, samp, tt {
    font-family: monospace;
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

a:link {
    color: #0000ee;
    text-decoration-line: underline;
}

a:visited {
    color: #551a8b;
    text-decoration-line: underline;
}
";

/// Returns the parsed UA stylesheet (lazily initialized, cached).
#[must_use]
pub fn ua_stylesheet() -> &'static Stylesheet {
    static UA: OnceLock<Stylesheet> = OnceLock::new();
    UA.get_or_init(|| parse_stylesheet(UA_CSS, Origin::UserAgent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, CssValue, LengthUnit};

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

    #[test]
    fn li_display_list_item() {
        let ss = ua_stylesheet();
        // Find the dedicated `li` rule (not the group rule).
        // The group rule (html, body, div, ..., li, ...) has display:block;
        // the dedicated `li` rule has display:list-item.
        let li_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "li"))
            }) && r.declarations.iter().any(|d| {
                d.property == "display" && d.value == CssValue::Keyword("list-item".to_string())
            })
        });
        assert!(li_rule.is_some(), "li display: list-item rule not found");
    }

    #[test]
    fn pre_white_space_pre() {
        let ss = ua_stylesheet();
        let pre_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "pre"))
            }) && r.declarations.iter().any(|d| d.property == "white-space")
        });
        assert!(pre_rule.is_some(), "pre white-space rule not found");
        let ws = pre_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "white-space")
            .unwrap();
        assert_eq!(ws.value, CssValue::Keyword("pre".to_string()));
    }

    #[test]
    fn code_has_font_family_monospace() {
        let ss = ua_stylesheet();
        let code_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "code"))
            }) && r.declarations.iter().any(|d| d.property == "font-family")
        });
        assert!(code_rule.is_some(), "code font-family rule not found");
        let ff = code_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "font-family")
            .unwrap();
        assert_eq!(
            ff.value,
            CssValue::List(vec![CssValue::Keyword("monospace".to_string())])
        );
    }

    #[test]
    fn a_link_has_blue_color_and_underline() {
        let ss = ua_stylesheet();
        let link_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components.iter().any(
                    |c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "a"),
                ) && sel.components.iter().any(
                    |c| matches!(c, elidex_css::SelectorComponent::PseudoClass(p) if p == "link"),
                )
            }) && r.declarations.iter().any(|d| d.property == "color")
        });
        assert!(link_rule.is_some(), "a:link color rule not found");
        let color_decl = link_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "color")
            .unwrap();
        // #0000ee = rgb(0, 0, 238)
        assert_eq!(
            color_decl.value,
            CssValue::Color(CssColor::new(0, 0, 238, 255))
        );
    }

    #[test]
    fn a_visited_has_purple_color() {
        let ss = ua_stylesheet();
        let visited_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components.iter().any(
                    |c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "a"),
                ) && sel.components.iter().any(
                    |c| {
                        matches!(c, elidex_css::SelectorComponent::PseudoClass(p) if p == "visited")
                    },
                )
            }) && r.declarations.iter().any(|d| d.property == "color")
        });
        assert!(visited_rule.is_some(), "a:visited color rule not found");
        let color_decl = visited_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "color")
            .unwrap();
        // #551a8b = rgb(85, 26, 139)
        assert_eq!(
            color_decl.value,
            CssValue::Color(CssColor::new(85, 26, 139, 255))
        );
    }

    #[test]
    fn code_does_not_have_white_space_pre() {
        let ss = ua_stylesheet();
        // code should NOT have white-space: pre (only pre does).
        let code_ws_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "code"))
            }) && r.declarations.iter().any(|d| d.property == "white-space")
        });
        assert!(
            code_ws_rule.is_none(),
            "code should not have white-space in UA stylesheet"
        );
    }
}
