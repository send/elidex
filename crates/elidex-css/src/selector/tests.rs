//! Tests for CSS selector parsing, matching, and specificity.

use super::*;
use cssparser::ParserInput;
use elidex_ecs::{Attributes, EcsDom, ElementState, Entity};

fn parse_sel(css: &str) -> Result<Selector, ()> {
    let mut input = ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    parse::parse_one_selector(&mut parser)
}

fn parse_list(css: &str) -> Result<Vec<Selector>, ()> {
    let mut input = ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut input);
    parse_selector_list(&mut parser)
}

#[test]
fn parse_tag() {
    let sel = parse_sel("div").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Tag("div".into())]);
}

#[test]
fn parse_class() {
    let sel = parse_sel(".foo").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Class("foo".into())]);
}

#[test]
fn parse_id() {
    let sel = parse_sel("#bar").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Id("bar".into())]);
}

#[test]
fn parse_universal() {
    let sel = parse_sel("*").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Universal]);
}

#[test]
fn parse_compound() {
    let sel = parse_sel("div.foo#bar").unwrap();
    // Stored right-to-left: Id, Class, Tag (reversed from parse order)
    assert_eq!(
        sel.components,
        vec![
            SelectorComponent::Id("bar".into()),
            SelectorComponent::Class("foo".into()),
            SelectorComponent::Tag("div".into()),
        ]
    );
}

#[test]
fn parse_descendant() {
    let sel = parse_sel("div p").unwrap();
    assert!(sel.components.contains(&SelectorComponent::Descendant));
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("div".into())));
    assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
}

#[test]
fn parse_child() {
    let sel = parse_sel("div > p").unwrap();
    assert!(sel.components.contains(&SelectorComponent::Child));
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("div".into())));
    assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
}

#[test]
fn parse_selector_list_test() {
    let sels = parse_list("div, p").unwrap();
    assert_eq!(sels.len(), 2);
}

#[test]
fn specificity_tag() {
    let sel = parse_sel("div").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 0,
            tag: 1
        }
    );
}

#[test]
fn specificity_class() {
    let sel = parse_sel(".foo").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 0
        }
    );
}

#[test]
fn specificity_id() {
    let sel = parse_sel("#bar").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 1,
            class: 0,
            tag: 0
        }
    );
}

#[test]
fn specificity_compound() {
    let sel = parse_sel("div.foo#bar").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 1,
            class: 1,
            tag: 1
        }
    );
}

#[test]
fn specificity_ordering() {
    let id = Specificity {
        id: 1,
        class: 0,
        tag: 0,
    };
    let class = Specificity {
        id: 0,
        class: 1,
        tag: 0,
    };
    let tag = Specificity {
        id: 0,
        class: 0,
        tag: 1,
    };
    assert!(id > class);
    assert!(class > tag);
}

// --- DOM matching tests ---

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

#[test]
fn match_tag_against_dom() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let sel = parse_sel("div").unwrap();
    assert!(sel.matches(div, &dom));

    let span = elem(&mut dom, "span");
    assert!(!sel.matches(span, &dom));
}

#[test]
fn match_class_against_dom() {
    let mut dom = EcsDom::new();
    let e = elem_with_class(&mut dom, "div", "foo bar");
    let sel_foo = parse_sel(".foo").unwrap();
    let sel_bar = parse_sel(".bar").unwrap();
    let sel_absent = parse_sel(".baz").unwrap();
    assert!(sel_foo.matches(e, &dom));
    assert!(sel_bar.matches(e, &dom));
    assert!(!sel_absent.matches(e, &dom));
}

#[test]
fn class_matching_is_case_sensitive() {
    let mut dom = EcsDom::new();
    let e = elem_with_class(&mut dom, "div", "foo");
    let sel = parse_sel(".Foo").unwrap();
    assert!(!sel.matches(e, &dom));
}

#[test]
fn id_matching_is_case_sensitive() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("id", "main");
    let e = dom.create_element("div", attrs);
    let sel = parse_sel("#Main").unwrap();
    assert!(!sel.matches(e, &dom));
}

#[test]
fn match_descendant() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let span = elem(&mut dom, "span");
    let p = elem(&mut dom, "p");
    dom.append_child(div, span);
    dom.append_child(span, p);

    let sel = parse_sel("div p").unwrap();
    assert!(sel.matches(p, &dom));
}

#[test]
fn match_child_direct_only() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let span = elem(&mut dom, "span");
    let p = elem(&mut dom, "p");
    dom.append_child(div, span);
    dom.append_child(span, p);

    let sel_child = parse_sel("div > p").unwrap();
    // p's direct parent is span, not div.
    assert!(!sel_child.matches(p, &dom));

    let sel_direct = parse_sel("span > p").unwrap();
    assert!(sel_direct.matches(p, &dom));
}

// --- Pseudo-class tests (M3-0) ---

#[test]
fn parse_pseudo_class_root() {
    let sel = parse_sel(":root").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("root".into())]
    );
    // Pseudo-class has class-level specificity (0, 1, 0).
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 0
        }
    );
}

#[test]
fn parse_pseudo_class_with_tag() {
    let sel = parse_sel("html:root").unwrap();
    assert!(sel
        .components
        .contains(&SelectorComponent::PseudoClass("root".into())));
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("html".into())));
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 1
        }
    );
}

#[test]
fn match_root_pseudo_class() {
    let mut dom = EcsDom::new();
    let doc_root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(doc_root, html);
    dom.append_child(html, body);

    let sel = parse_sel(":root").unwrap();
    assert!(sel.matches(html, &dom));
    assert!(!sel.matches(body, &dom));
}

#[test]
fn root_requires_document_parent() {
    // An html element without a proper document root parent should not match :root.
    let mut dom = EcsDom::new();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(html, body);
    // html has no parent at all -- :root requires parent to be non-element.
    let sel = parse_sel(":root").unwrap();
    assert!(!sel.matches(html, &dom));
}

// --- M3-3: Sibling combinator parse tests ---

#[test]
fn parse_adjacent_sibling() {
    let sel = parse_sel("h1 + p").unwrap();
    assert!(sel.components.contains(&SelectorComponent::AdjacentSibling));
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("h1".into())));
    assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
}

#[test]
fn parse_general_sibling() {
    let sel = parse_sel("h1 ~ p").unwrap();
    assert!(sel.components.contains(&SelectorComponent::GeneralSibling));
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("h1".into())));
    assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
}

// --- M3-3: Attribute selector parse tests ---

#[test]
fn parse_attr_presence() {
    let sel = parse_sel("[href]").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "href".into(),
            matcher: None,
        }]
    );
}

#[test]
fn parse_attr_exact() {
    let sel = parse_sel(r#"[type="text"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "type".into(),
            matcher: Some(AttributeMatcher::Exact("text".into())),
        }]
    );
}

#[test]
fn parse_attr_includes() {
    let sel = parse_sel(r#"[class~="foo"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "class".into(),
            matcher: Some(AttributeMatcher::Includes("foo".into())),
        }]
    );
}

#[test]
fn parse_attr_dash_match() {
    let sel = parse_sel(r#"[lang|="en"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "lang".into(),
            matcher: Some(AttributeMatcher::DashMatch("en".into())),
        }]
    );
}

#[test]
fn parse_attr_prefix() {
    let sel = parse_sel(r#"[href^="https"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "href".into(),
            matcher: Some(AttributeMatcher::Prefix("https".into())),
        }]
    );
}

#[test]
fn parse_attr_suffix() {
    let sel = parse_sel(r#"[href$=".pdf"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "href".into(),
            matcher: Some(AttributeMatcher::Suffix(".pdf".into())),
        }]
    );
}

#[test]
fn parse_attr_substring() {
    let sel = parse_sel(r#"[title*="hello"]"#).unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Attribute {
            name: "title".into(),
            matcher: Some(AttributeMatcher::Substring("hello".into())),
        }]
    );
}

// --- M3-3: Structural pseudo-class parse tests ---

#[test]
fn parse_first_child() {
    let sel = parse_sel(":first-child").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("first-child".into())]
    );
}

#[test]
fn parse_last_child() {
    let sel = parse_sel(":last-child").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("last-child".into())]
    );
}

#[test]
fn parse_only_child() {
    let sel = parse_sel(":only-child").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("only-child".into())]
    );
}

#[test]
fn parse_empty() {
    let sel = parse_sel(":empty").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("empty".into())]
    );
}

// --- M3-3: :not() parse tests ---

#[test]
fn parse_not_class() {
    let sel = parse_sel(":not(.foo)").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Not(vec![SelectorComponent::Class(
            "foo".into()
        )])]
    );
}

#[test]
fn parse_not_tag() {
    let sel = parse_sel(":not(div)").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Not(vec![SelectorComponent::Tag(
            "div".into()
        )])]
    );
}

#[test]
fn parse_nested_not_rejected() {
    // CSS Selectors Level 3: :not() cannot contain :not().
    assert!(parse_sel(":not(:not(.foo))").is_err());
}

// --- M3-3: Specificity tests ---

#[test]
fn specificity_attr_presence() {
    let sel = parse_sel("[attr]").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 0
        }
    );
}

#[test]
fn specificity_attr_value() {
    let sel = parse_sel(r"[attr=val]").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 0
        }
    );
}

#[test]
fn specificity_not_id() {
    // CSS Selectors Level 3: :not() specificity = argument specificity.
    // :not(#id) -> (1, 0, 0), not (1, 1, 0).
    let sel = parse_sel(":not(#id)").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 1,
            class: 0,
            tag: 0
        }
    );
}

#[test]
fn specificity_tag_first_child() {
    let sel = parse_sel("div:first-child").unwrap();
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 1
        }
    );
}

// --- M3-3: Sibling combinator matching tests ---

#[test]
fn match_adjacent_sibling() {
    // <div><h1/><p/></div> -- `h1 + p` matches p.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let h1 = elem(&mut dom, "h1");
    let p = elem(&mut dom, "p");
    dom.append_child(div, h1);
    dom.append_child(div, p);

    let sel = parse_sel("h1 + p").unwrap();
    assert!(sel.matches(p, &dom));
    // h1 has no previous sibling that is p.
    assert!(!sel.matches(h1, &dom));
}

#[test]
fn match_adjacent_sibling_not_immediate() {
    // <div><h1/><span/><p/></div> -- `h1 + p` should NOT match p
    // because span is between h1 and p.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let h1 = elem(&mut dom, "h1");
    let span = elem(&mut dom, "span");
    let p = elem(&mut dom, "p");
    dom.append_child(div, h1);
    dom.append_child(div, span);
    dom.append_child(div, p);

    let sel = parse_sel("h1 + p").unwrap();
    assert!(!sel.matches(p, &dom));
}

#[test]
fn match_general_sibling() {
    // <div><h1/><span/><p/></div> -- `h1 ~ p` matches p.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let h1 = elem(&mut dom, "h1");
    let span = elem(&mut dom, "span");
    let p = elem(&mut dom, "p");
    dom.append_child(div, h1);
    dom.append_child(div, span);
    dom.append_child(div, p);

    let sel = parse_sel("h1 ~ p").unwrap();
    assert!(sel.matches(p, &dom));
    // p before h1 should NOT match.
    assert!(!sel.matches(h1, &dom));
}

#[test]
fn match_general_sibling_before() {
    // <div><p/><h1/></div> -- `h1 ~ p` should NOT match p (p is before h1).
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let p = elem(&mut dom, "p");
    let h1 = elem(&mut dom, "h1");
    dom.append_child(div, p);
    dom.append_child(div, h1);

    let sel = parse_sel("h1 ~ p").unwrap();
    assert!(!sel.matches(p, &dom));
}

// --- M3-3: Attribute matching tests ---

#[test]
fn match_attr_presence() {
    let mut dom = EcsDom::new();
    let a = elem_with_attr(&mut dom, "a", "href", "https://example.com");
    let div = elem(&mut dom, "div");

    let sel = parse_sel("[href]").unwrap();
    assert!(sel.matches(a, &dom));
    assert!(!sel.matches(div, &dom));
}

#[test]
fn match_attr_exact() {
    let mut dom = EcsDom::new();
    let input = elem_with_attr(&mut dom, "input", "type", "text");
    let checkbox = elem_with_attr(&mut dom, "input", "type", "checkbox");

    let sel = parse_sel(r#"[type="text"]"#).unwrap();
    assert!(sel.matches(input, &dom));
    assert!(!sel.matches(checkbox, &dom));
}

#[test]
fn match_attr_includes() {
    let mut dom = EcsDom::new();
    let e1 = elem_with_class(&mut dom, "div", "foo bar");
    let e2 = elem_with_class(&mut dom, "div", "foobar");

    let sel = parse_sel(r#"[class~="foo"]"#).unwrap();
    assert!(sel.matches(e1, &dom)); // "foo bar" contains word "foo"
    assert!(!sel.matches(e2, &dom)); // "foobar" does not contain word "foo"
}

#[test]
fn match_attr_dash_match() {
    let mut dom = EcsDom::new();
    let en = elem_with_attr(&mut dom, "div", "lang", "en");
    let en_us = elem_with_attr(&mut dom, "div", "lang", "en-US");
    let eng = elem_with_attr(&mut dom, "div", "lang", "eng");

    let sel = parse_sel(r#"[lang|="en"]"#).unwrap();
    assert!(sel.matches(en, &dom));
    assert!(sel.matches(en_us, &dom));
    assert!(!sel.matches(eng, &dom));
}

#[test]
fn match_attr_prefix() {
    let mut dom = EcsDom::new();
    let https = elem_with_attr(&mut dom, "a", "href", "https://example.com");
    let http = elem_with_attr(&mut dom, "a", "href", "http://example.com");

    let sel = parse_sel(r#"[href^="https"]"#).unwrap();
    assert!(sel.matches(https, &dom));
    assert!(!sel.matches(http, &dom));
}

#[test]
fn match_attr_suffix() {
    let mut dom = EcsDom::new();
    let pdf = elem_with_attr(&mut dom, "a", "href", "/doc/report.pdf");
    let html = elem_with_attr(&mut dom, "a", "href", "/page/index.html");

    let sel = parse_sel(r#"[href$=".pdf"]"#).unwrap();
    assert!(sel.matches(pdf, &dom));
    assert!(!sel.matches(html, &dom));
}

#[test]
fn match_attr_substring() {
    let mut dom = EcsDom::new();
    let has_hello = elem_with_attr(&mut dom, "div", "title", "say hello world");
    let no_hello = elem_with_attr(&mut dom, "div", "title", "goodbye");

    let sel = parse_sel(r#"[title*="hello"]"#).unwrap();
    assert!(sel.matches(has_hello, &dom));
    assert!(!sel.matches(no_hello, &dom));
}

// --- M3-3: Structural pseudo-class matching tests ---

#[test]
fn match_first_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "ul");
    let li1 = elem(&mut dom, "li");
    let li2 = elem(&mut dom, "li");
    dom.append_child(parent, li1);
    dom.append_child(parent, li2);

    let sel = parse_sel(":first-child").unwrap();
    assert!(sel.matches(li1, &dom));
    assert!(!sel.matches(li2, &dom));
}

#[test]
fn match_first_child_with_text_node_before() {
    // Text node before element -- :first-child should still match the
    // first element child (text nodes are not elements).
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let text = dom.create_text("some text");
    let span = elem(&mut dom, "span");
    dom.append_child(parent, text);
    dom.append_child(parent, span);

    let sel = parse_sel(":first-child").unwrap();
    assert!(sel.matches(span, &dom));
}

#[test]
fn match_last_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "ul");
    let li1 = elem(&mut dom, "li");
    let li2 = elem(&mut dom, "li");
    dom.append_child(parent, li1);
    dom.append_child(parent, li2);

    let sel = parse_sel(":last-child").unwrap();
    assert!(!sel.matches(li1, &dom));
    assert!(sel.matches(li2, &dom));
}

#[test]
fn match_only_child() {
    let mut dom = EcsDom::new();
    let parent1 = elem(&mut dom, "div");
    let only = elem(&mut dom, "span");
    dom.append_child(parent1, only);

    let parent2 = elem(&mut dom, "div");
    let child1 = elem(&mut dom, "span");
    let child2 = elem(&mut dom, "span");
    dom.append_child(parent2, child1);
    dom.append_child(parent2, child2);

    let sel = parse_sel(":only-child").unwrap();
    assert!(sel.matches(only, &dom));
    assert!(!sel.matches(child1, &dom));
    assert!(!sel.matches(child2, &dom));
}

#[test]
fn match_empty() {
    let mut dom = EcsDom::new();
    let empty_div = elem(&mut dom, "div");
    let non_empty_div = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    dom.append_child(non_empty_div, child);

    let sel = parse_sel(":empty").unwrap();
    assert!(sel.matches(empty_div, &dom));
    assert!(!sel.matches(non_empty_div, &dom));
}

#[test]
fn match_empty_with_text_child() {
    // :empty should NOT match if there's a text node child.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let text = dom.create_text("hello");
    dom.append_child(div, text);

    let sel = parse_sel(":empty").unwrap();
    assert!(!sel.matches(div, &dom));
}

// --- M3-3: :not() matching tests ---

#[test]
fn match_not_class() {
    let mut dom = EcsDom::new();
    let foo = elem_with_class(&mut dom, "div", "foo");
    let bar = elem_with_class(&mut dom, "div", "bar");

    let sel = parse_sel(":not(.foo)").unwrap();
    assert!(!sel.matches(foo, &dom));
    assert!(sel.matches(bar, &dom));
}

#[test]
fn match_not_tag() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let span = elem(&mut dom, "span");

    let sel = parse_sel(":not(div)").unwrap();
    assert!(!sel.matches(div, &dom));
    assert!(sel.matches(span, &dom));
}

// --- M3-3: Sibling with text node skipping ---

#[test]
fn adjacent_sibling_skips_text_nodes() {
    // <div><h1/>text<p/></div> -- `h1 + p` should match p
    // because text nodes are not elements and should be skipped.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let h1 = elem(&mut dom, "h1");
    let text = dom.create_text("between");
    let p = elem(&mut dom, "p");
    dom.append_child(div, h1);
    dom.append_child(div, text);
    dom.append_child(div, p);

    let sel = parse_sel("h1 + p").unwrap();
    assert!(sel.matches(p, &dom));
}

// --- M3-3: Complex combined selectors ---

#[test]
fn parse_compound_with_attr_and_class() {
    let sel = parse_sel(r#"input.required[type="text"]"#).unwrap();
    assert!(sel
        .components
        .contains(&SelectorComponent::Tag("input".into())));
    assert!(sel
        .components
        .contains(&SelectorComponent::Class("required".into())));
    assert!(sel.components.contains(&SelectorComponent::Attribute {
        name: "type".into(),
        matcher: Some(AttributeMatcher::Exact("text".into())),
    }));
}

#[test]
fn match_child_first_child_combined() {
    // ul > li:first-child
    let mut dom = EcsDom::new();
    let ul = elem(&mut dom, "ul");
    let li1 = elem(&mut dom, "li");
    let li2 = elem(&mut dom, "li");
    dom.append_child(ul, li1);
    dom.append_child(ul, li2);

    let sel = parse_sel("ul > li:first-child").unwrap();
    assert!(sel.matches(li1, &dom));
    assert!(!sel.matches(li2, &dom));
}

// --- M3.5-0: Pseudo-element parse tests ---

#[test]
fn parse_pseudo_element_before() {
    let sel = parse_sel("p::before").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Tag("p".into())]);
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
    // tag(p) + pseudo-element(::before) = (0,0,2)
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 0,
            tag: 2
        }
    );
}

#[test]
fn parse_pseudo_element_after() {
    let sel = parse_sel("p::after").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::After));
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 0,
            tag: 2
        }
    );
}

#[test]
fn parse_pseudo_element_legacy_single_colon() {
    // CSS2 legacy: `:before` -> PseudoElement::Before
    let sel = parse_sel(":before").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
}

#[test]
fn parse_pseudo_element_legacy_after() {
    let sel = parse_sel(":after").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::After));
}

#[test]
fn parse_pseudo_element_class_before() {
    let sel = parse_sel(".foo::before").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Class("foo".into())]);
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
    // class(.foo) + pseudo-element(::before) = (0,1,1)
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 1
        }
    );
}

#[test]
fn parse_pseudo_element_alone() {
    // `::before` alone is valid (implies universal selector)
    let sel = parse_sel("::before").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 0,
            tag: 1
        }
    );
}

#[test]
fn parse_regular_selector_no_pseudo_element() {
    let sel = parse_sel("div").unwrap();
    assert_eq!(sel.pseudo_element, None);
}

#[test]
fn parse_pseudo_element_matches_element() {
    // Selector `p::before` should still match the `p` element itself.
    let mut dom = EcsDom::new();
    let p = elem(&mut dom, "p");
    let sel = parse_sel("p::before").unwrap();
    assert!(sel.matches(p, &dom));
}

// ---- Dynamic pseudo-class tests ----

#[test]
fn hover_matches_when_state_set() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let sel = parse_sel("div:hover").unwrap();

    // No state -> no match.
    assert!(!sel.matches(div, &dom));

    // Set HOVER -> matches.
    let mut state = ElementState::default();
    state.insert(ElementState::HOVER);
    let _ = dom.world_mut().insert_one(div, state);
    assert!(sel.matches(div, &dom));
}

#[test]
fn focus_matches_when_state_set() {
    let mut dom = EcsDom::new();
    let input = elem(&mut dom, "input");
    let sel = parse_sel("input:focus").unwrap();

    assert!(!sel.matches(input, &dom));

    let mut state = ElementState::default();
    state.insert(ElementState::FOCUS);
    let _ = dom.world_mut().insert_one(input, state);
    assert!(sel.matches(input, &dom));
}

#[test]
fn active_matches_when_state_set() {
    let mut dom = EcsDom::new();
    let btn = elem(&mut dom, "button");
    let sel = parse_sel("button:active").unwrap();

    assert!(!sel.matches(btn, &dom));

    let mut state = ElementState::default();
    state.insert(ElementState::ACTIVE);
    let _ = dom.world_mut().insert_one(btn, state);
    assert!(sel.matches(btn, &dom));
}

#[test]
fn link_matches_when_state_set() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "a");
    let sel = parse_sel("a:link").unwrap();

    assert!(!sel.matches(a, &dom));

    let mut state = ElementState::default();
    state.insert(ElementState::LINK);
    let _ = dom.world_mut().insert_one(a, state);
    assert!(sel.matches(a, &dom));

    // :visited should not match (only LINK set).
    let visited_sel = parse_sel("a:visited").unwrap();
    assert!(!visited_sel.matches(a, &dom));
}

#[test]
fn combined_tag_and_hover_selector() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "a");
    let sel = parse_sel("a:hover").unwrap();

    // Set both LINK and HOVER.
    let mut state = ElementState::default();
    state.insert(ElementState::LINK);
    state.insert(ElementState::HOVER);
    let _ = dom.world_mut().insert_one(a, state);
    assert!(sel.matches(a, &dom));

    // Specificity: :hover is (0,1,0) + tag a is (0,0,1) = (0,1,1).
    assert_eq!(
        sel.specificity,
        Specificity {
            id: 0,
            class: 1,
            tag: 1
        }
    );
}
