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

head, link, meta, script, style, title {
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

    // --- M3.5-2: Table UA styles ---

    #[test]
    fn table_display_table() {
        let ss = ua_stylesheet();
        let table_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "table"))
            }) && r.declarations.iter().any(|d| {
                d.property == "display" && d.value == CssValue::Keyword("table".to_string())
            })
        });
        assert!(table_rule.is_some(), "table display: table rule not found");
    }

    #[test]
    fn tr_display_table_row() {
        let ss = ua_stylesheet();
        let tr_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "tr"))
            }) && r.declarations.iter().any(|d| {
                d.property == "display" && d.value == CssValue::Keyword("table-row".to_string())
            })
        });
        assert!(tr_rule.is_some(), "tr display: table-row rule not found");
    }

    #[test]
    fn td_th_display_table_cell() {
        let ss = ua_stylesheet();
        let td_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "td"))
            }) && r.declarations.iter().any(|d| {
                d.property == "display" && d.value == CssValue::Keyword("table-cell".to_string())
            })
        });
        assert!(td_rule.is_some(), "td display: table-cell rule not found");
    }

    #[test]
    fn th_font_weight_bold() {
        let ss = ua_stylesheet();
        let th_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "th"))
            }) && r.declarations.iter().any(|d| d.property == "font-weight")
        });
        assert!(th_rule.is_some(), "th font-weight: bold rule not found");
        let fw = th_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "font-weight")
            .unwrap();
        assert_eq!(fw.value, CssValue::Keyword("bold".to_string()));
    }

    // --- M3.5-4: BiDi UA styles ---

    #[test]
    fn bdi_unicode_bidi_isolate() {
        let ss = ua_stylesheet();
        let bdi_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "bdi"))
            }) && r.declarations.iter().any(|d| d.property == "unicode-bidi")
        });
        assert!(bdi_rule.is_some(), "bdi unicode-bidi rule not found");
        let ub = bdi_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "unicode-bidi")
            .unwrap();
        assert_eq!(ub.value, CssValue::Keyword("isolate".to_string()));
    }

    #[test]
    fn bdo_unicode_bidi_override() {
        let ss = ua_stylesheet();
        let bdo_rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "bdo"))
            }) && r.declarations.iter().any(|d| d.property == "unicode-bidi")
        });
        assert!(bdo_rule.is_some(), "bdo unicode-bidi rule not found");
        let ub = bdo_rule
            .unwrap()
            .declarations
            .iter()
            .find(|d| d.property == "unicode-bidi")
            .unwrap();
        assert_eq!(ub.value, CssValue::Keyword("bidi-override".to_string()));
    }

    #[test]
    fn bdo_dir_ltr_rule() {
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "bdo"))
            }) && r.declarations.iter().any(|d| {
                d.property == "direction" && d.value == CssValue::Keyword("ltr".to_string())
            })
        });
        assert!(rule.is_some(), "bdo[dir=ltr] direction rule not found");
    }

    #[test]
    fn bdo_dir_rtl_rule() {
        let ss = ua_stylesheet();
        let rule = ss.rules.iter().find(|r| {
            r.selectors.iter().any(|sel| {
                sel.components
                    .iter()
                    .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == "bdo"))
            }) && r.declarations.iter().any(|d| {
                d.property == "direction" && d.value == CssValue::Keyword("rtl".to_string())
            })
        });
        assert!(rule.is_some(), "bdo[dir=rtl] direction rule not found");
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
