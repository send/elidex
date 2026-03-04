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
form, fieldset, address, article, aside,
details, figcaption, figure, footer, header,
main, nav, section, summary, hr {
    display: block;
}

table { display: table; box-sizing: border-box; border-collapse: separate; border-spacing: 2px; }
caption { display: table-caption; text-align: center; }
thead { display: table-header-group; }
tbody { display: table-row-group; }
tfoot { display: table-footer-group; }
tr { display: table-row; }
td, th { display: table-cell; padding: 1px; }
th { font-weight: bold; text-align: center; }
colgroup { display: table-column-group; }
col { display: table-column; }

head, link, meta, script, style, title, template {
    display: none;
}

body {
    margin: 8px;
}

h1, h2, h3, h4, h5, h6 { font-weight: bold; }

h1 { font-size: 2em; margin-top: 0.67em; margin-bottom: 0.67em; }
h2 { font-size: 1.5em; margin-top: 0.83em; margin-bottom: 0.83em; }
h3 { font-size: 1.17em; margin-top: 1em; margin-bottom: 1em; }
h4 { font-size: 1em; margin-top: 1.33em; margin-bottom: 1.33em; }
h5 { font-size: 0.83em; margin-top: 1.67em; margin-bottom: 1.67em; }
h6 { font-size: 0.67em; margin-top: 2.33em; margin-bottom: 2.33em; }

p { margin-top: 1em; margin-bottom: 1em; }

ul, ol {
    margin-top: 1em;
    margin-bottom: 1em;
    padding-left: 40px;
}

li { display: list-item; }

ol { list-style-type: decimal; }

dl { margin-top: 1em; margin-bottom: 1em; }
dd { margin-left: 40px; }

pre { white-space: pre; }

pre, code, kbd, samp, tt { font-family: monospace; }

blockquote {
    margin-top: 1em;
    margin-bottom: 1em;
    margin-left: 40px;
    margin-right: 40px;
}

pre { margin-top: 1em; margin-bottom: 1em; }

hr {
    color: gray;
    border-top-style: solid;
    border-top-width: 1px;
    margin-top: 0.5em;
    margin-bottom: 0.5em;
    margin-left: auto;
    margin-right: auto;
}

a:link {
    color: #0000ee;
    text-decoration-line: underline;
}

a:visited {
    color: #551a8b;
    text-decoration-line: underline;
}

/* TODO(Phase 4): WHATWG 15.3.6 specifies `bdi { direction: ltr; }` as
   the initial direction for the bidi-isolation context. Currently omitted
   because elidex defaults to LTR, but should be explicit for correctness. */
bdi { unicode-bidi: isolate; }
bdo { unicode-bidi: bidi-override; }
bdo[dir='ltr'] { direction: ltr; }
bdo[dir='rtl'] { direction: rtl; }

slot { display: contents; }
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
    use elidex_css::{CssRule, SelectorComponent};
    use elidex_plugin::{CssColor, CssValue, LengthUnit};

    /// Find a rule that matches a tag and has a declaration for the given property.
    fn find_tag_rule<'a>(ss: &'a Stylesheet, tag: &str, property: &str) -> Option<&'a CssRule> {
        ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Tag(t) if t == tag))
            }) && r.declarations.iter().any(|d| d.property == property)
        })
    }

    /// Assert a UA rule exists for `tag` with `property: expected_value`.
    fn assert_ua_rule(tag: &str, property: &str, expected: &CssValue) {
        let ss = ua_stylesheet();
        let rule = find_tag_rule(ss, tag, property)
            .unwrap_or_else(|| panic!("{tag} {property} rule not found"));
        let decl = rule
            .declarations
            .iter()
            .find(|d| d.property == property)
            .unwrap();
        assert_eq!(&decl.value, expected, "{tag} {property}");
    }

    /// Assert a UA rule exists for `tag` with `property: keyword_value`.
    fn assert_ua_keyword(tag: &str, property: &str, keyword: &str) {
        assert_ua_rule(tag, property, &CssValue::Keyword(keyword.to_string()));
    }

    #[test]
    fn ua_parses_without_error() {
        let ss = ua_stylesheet();
        assert!(!ss.rules.is_empty());
        assert_eq!(ss.origin, Origin::UserAgent);
    }

    #[test]
    fn body_has_margin_8px() {
        assert_ua_rule("body", "margin-top", &CssValue::Length(8.0, LengthUnit::Px));
    }

    #[test]
    fn head_display_none() {
        assert_ua_keyword("head", "display", "none");
    }

    #[test]
    fn li_display_list_item() {
        // Find the dedicated `li` rule (not the group rule with display:block).
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Tag(t) if t == "li"))
            }) && r.declarations.iter().any(|d| {
                d.property == "display" && d.value == CssValue::Keyword("list-item".to_string())
            })
        });
        assert!(rule.is_some(), "li display: list-item rule not found");
    }

    #[test]
    fn pre_white_space_pre() {
        assert_ua_keyword("pre", "white-space", "pre");
    }

    #[test]
    fn code_has_font_family_monospace() {
        assert_ua_rule(
            "code",
            "font-family",
            &CssValue::List(vec![CssValue::Keyword("monospace".to_string())]),
        );
    }

    #[test]
    fn a_link_has_blue_color_and_underline() {
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Tag(t) if t == "a"))
                    && sel
                        .components
                        .iter()
                        .any(|c| matches!(c, SelectorComponent::PseudoClass(p) if p == "link"))
            }) && r.declarations.iter().any(|d| d.property == "color")
        });
        assert!(rule.is_some(), "a:link color rule not found");
        let decl = rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "color")
            .unwrap();
        assert_eq!(decl.value, CssValue::Color(CssColor::new(0, 0, 238, 255)));
    }

    #[test]
    fn a_visited_has_purple_color() {
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Tag(t) if t == "a"))
                    && sel
                        .components
                        .iter()
                        .any(|c| matches!(c, SelectorComponent::PseudoClass(p) if p == "visited"))
            }) && r.declarations.iter().any(|d| d.property == "color")
        });
        assert!(rule.is_some(), "a:visited color rule not found");
        let decl = rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "color")
            .unwrap();
        assert_eq!(decl.value, CssValue::Color(CssColor::new(85, 26, 139, 255)));
    }

    // --- Table UA styles ---

    #[test]
    fn table_display_table() {
        assert_ua_keyword("table", "display", "table");
    }

    #[test]
    fn tr_display_table_row() {
        assert_ua_keyword("tr", "display", "table-row");
    }

    #[test]
    fn td_th_display_table_cell() {
        assert_ua_keyword("td", "display", "table-cell");
    }

    #[test]
    fn th_font_weight_bold() {
        assert_ua_keyword("th", "font-weight", "bold");
    }

    // --- BiDi UA styles ---

    #[test]
    fn bdi_unicode_bidi_isolate() {
        assert_ua_keyword("bdi", "unicode-bidi", "isolate");
    }

    #[test]
    fn bdo_unicode_bidi_override() {
        assert_ua_keyword("bdo", "unicode-bidi", "bidi-override");
    }

    #[test]
    fn bdo_dir_ltr_rule() {
        assert_ua_keyword("bdo", "direction", "ltr");
    }

    #[test]
    fn bdo_dir_rtl_rule() {
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Tag(t) if t == "bdo"))
            }) && r.declarations.iter().any(|d| {
                d.property == "direction" && d.value == CssValue::Keyword("rtl".to_string())
            })
        });
        assert!(rule.is_some(), "bdo direction: rtl rule not found");
    }

    #[test]
    fn template_display_none() {
        assert_ua_keyword("template", "display", "none");
    }

    #[test]
    fn slot_display_contents() {
        assert_ua_keyword("slot", "display", "contents");
    }

    #[test]
    fn code_does_not_have_white_space_pre() {
        let ss = ua_stylesheet();
        assert!(
            find_tag_rule(ss, "code", "white-space").is_none(),
            "code should not have white-space in UA stylesheet"
        );
    }
}
