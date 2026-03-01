//! CSS cascade algorithm.
//!
//! Implements the CSS cascade for Phase 1: collects matching declarations
//! from stylesheets and inline styles, then determines the winning value
//! for each property based on origin, importance, specificity, and
//! source order.

use std::collections::HashMap;

use elidex_css::{Declaration, Origin, Specificity, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::CssValue;

/// A single declaration entry in the cascade, annotated with priority metadata.
struct CascadeEntry<'a> {
    property: &'a str,
    value: &'a CssValue,
    priority: CascadePriority,
}

/// Cascade priority for comparing declarations.
///
/// Field order determines comparison priority (derived `Ord`):
/// `importance_layer` > `is_inline` > `specificity` > `stylesheet_index` > `source_order`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CascadePriority {
    /// 0 = UA normal, 1 = Author normal, 2 = Author !important, 3 = UA !important.
    importance_layer: u8,
    /// Inline styles beat selector-based styles at the same layer.
    is_inline: bool,
    /// Selector specificity.
    specificity: Specificity,
    /// Index of the stylesheet in the list (later stylesheet = higher index = wins).
    stylesheet_index: u32,
    /// Position in source order within a stylesheet (higher = later = wins ties).
    source_order: u32,
}

fn importance_layer(origin: Origin, important: bool) -> u8 {
    match (origin, important) {
        (Origin::UserAgent, false) => 0,
        (Origin::UserAgent, true) => 3,
        // Author + any future origins default to author-level.
        (_, false) => 1,
        (_, true) => 2,
    }
}

/// Collect matching declarations and cascade to determine winners.
///
/// Returns a map from property name to the winning `CssValue` reference.
pub(crate) fn collect_and_cascade<'a>(
    entity: Entity,
    dom: &EcsDom,
    stylesheets: &'a [&'a Stylesheet],
    inline_declarations: &'a [Declaration],
) -> HashMap<&'a str, &'a CssValue> {
    let mut entries: Vec<CascadeEntry<'a>> = Vec::new();

    // Collect from stylesheets.
    for (sheet_idx, stylesheet) in stylesheets.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // Stylesheet count won't exceed u32::MAX.
        let sheet_index = sheet_idx as u32;
        for rule in &stylesheet.rules {
            // Single-pass: find max specificity among matching selectors.
            let max_specificity = rule
                .selectors
                .iter()
                .filter(|sel| sel.matches(entity, dom))
                .map(|sel| sel.specificity)
                .max();
            let Some(max_specificity) = max_specificity else {
                continue;
            };

            for decl in &rule.declarations {
                entries.push(CascadeEntry {
                    property: &decl.property,
                    value: &decl.value,
                    priority: CascadePriority {
                        importance_layer: importance_layer(stylesheet.origin, decl.important),
                        is_inline: false,
                        specificity: max_specificity,
                        stylesheet_index: sheet_index,
                        source_order: rule.source_order,
                    },
                });
            }
        }
    }

    // Collect inline styles (highest specificity, treated as author origin).
    // Inline styles use a synthetic source_order of u32::MAX to ensure they
    // come after any stylesheet declarations at the same priority.
    for decl in inline_declarations {
        entries.push(CascadeEntry {
            property: &decl.property,
            value: &decl.value,
            priority: CascadePriority {
                importance_layer: importance_layer(Origin::Author, decl.important),
                is_inline: true,
                specificity: Specificity::default(),
                stylesheet_index: u32::MAX,
                source_order: u32::MAX,
            },
        });
    }

    // Sort by priority (ascending — last entry wins).
    entries.sort_by(|a, b| a.priority.cmp(&b.priority));

    // Last-wins per property.
    let mut winners: HashMap<&str, &CssValue> = HashMap::with_capacity(entries.len());
    for entry in &entries {
        winners.insert(entry.property, entry.value);
    }

    winners
}

/// Retrieve inline style declarations from an element's `style` attribute.
pub(crate) fn get_inline_declarations(entity: Entity, dom: &EcsDom) -> Vec<Declaration> {
    let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
        return Vec::new();
    };
    let Some(style_str) = attrs.get("style") else {
        return Vec::new();
    };
    elidex_css::parse_declaration_block(style_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_css::parse_stylesheet;
    use elidex_ecs::Attributes;
    use elidex_plugin::CssColor;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    fn elem_with_attrs(dom: &mut EcsDom, tag: &str, attrs: Attributes) -> Entity {
        dom.create_element(tag, attrs)
    }

    /// Setup a DOM with a document root and a single element child.
    #[allow(unused_must_use)]
    fn setup_with_element(tag: &str) -> (EcsDom, Entity, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let el = elem(&mut dom, tag);
        dom.append_child(root, el);
        (dom, root, el)
    }

    #[test]
    fn single_declaration_wins() {
        let (dom, _, div) = setup_with_element("div");
        let ss = parse_stylesheet("div { color: red; }", Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
    }

    #[test]
    #[allow(unused_must_use)]
    fn specificity_wins() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("class", "highlight");
        let div = elem_with_attrs(&mut dom, "div", attrs);
        dom.append_child(root, div);

        let css = "div { color: red; } .highlight { color: blue; }";
        let ss = parse_stylesheet(css, Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(
            winners.get("color"),
            Some(&&CssValue::Color(CssColor::BLUE))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn source_order_tiebreak() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = elem(&mut dom, "div");
        dom.append_child(root, div);

        let css = "div { color: red; } div { color: blue; }";
        let ss = parse_stylesheet(css, Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(
            winners.get("color"),
            Some(&&CssValue::Color(CssColor::BLUE))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn important_beats_normal() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = elem(&mut dom, "div");
        dom.append_child(root, div);

        let css = "div { color: red !important; } div { color: blue; }";
        let ss = parse_stylesheet(css, Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
    }

    #[test]
    #[allow(unused_must_use)]
    fn ua_important_beats_author_important() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = elem(&mut dom, "div");
        dom.append_child(root, div);

        let ua = parse_stylesheet("div { color: green !important; }", Origin::UserAgent);
        let author = parse_stylesheet("div { color: red !important; }", Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ua, &author];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(
            winners.get("color"),
            Some(&&CssValue::Color(CssColor::GREEN))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn inline_beats_selector() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("style", "color: blue");
        let div = elem_with_attrs(&mut dom, "div", attrs);
        dom.append_child(root, div);

        let ss = parse_stylesheet("div { color: red; }", Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let inline = get_inline_declarations(div, &dom);
        let winners = collect_and_cascade(div, &dom, &sheets, &inline);
        assert_eq!(
            winners.get("color"),
            Some(&&CssValue::Color(CssColor::BLUE))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn important_inline_is_strongest_author() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("style", "color: blue !important");
        attrs.set("class", "highlight");
        attrs.set("id", "main");
        let div = elem_with_attrs(&mut dom, "div", attrs);
        dom.append_child(root, div);

        let css = "#main { color: red !important; }";
        let ss = parse_stylesheet(css, Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let inline = get_inline_declarations(div, &dom);
        let winners = collect_and_cascade(div, &dom, &sheets, &inline);
        assert_eq!(
            winners.get("color"),
            Some(&&CssValue::Color(CssColor::BLUE))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn independent_property_resolution() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("class", "highlight");
        let div = elem_with_attrs(&mut dom, "div", attrs);
        dom.append_child(root, div);

        let css = r"
            .highlight { color: red; display: block; }
            div { color: blue; }
        ";
        let ss = parse_stylesheet(css, Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        // color: .highlight (class specificity) beats div (tag specificity)
        assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
        // display: only .highlight declares it
        assert_eq!(
            winners.get("display"),
            Some(&&CssValue::Keyword("block".to_string()))
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn no_matching_rules_empty_winners() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = elem(&mut dom, "div");
        dom.append_child(root, div);

        let ss = parse_stylesheet("p { color: red; }", Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ss];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert!(winners.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn author_normal_beats_ua_normal() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = elem(&mut dom, "div");
        dom.append_child(root, div);

        let ua = parse_stylesheet("div { color: green; }", Origin::UserAgent);
        let author = parse_stylesheet("div { color: red; }", Origin::Author);
        let sheets: Vec<&Stylesheet> = vec![&ua, &author];
        let winners = collect_and_cascade(div, &dom, &sheets, &[]);
        assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
    }
}
