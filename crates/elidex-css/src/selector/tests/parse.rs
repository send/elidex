use super::*;

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

// --- Pseudo-class tests (M3-0) ---

#[test]
fn parse_pseudo_class_root() {
    let sel = parse_sel(":root").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::PseudoClass("root".into())]
    );
    // Pseudo-class has class-level specificity (0, 1, 0).
    assert_eq!(sel.specificity, spec(0, 1, 0));
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
    assert_eq!(sel.specificity, spec(0, 1, 1));
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

// --- M3.5-0: Pseudo-element parse tests ---

#[test]
fn parse_pseudo_element_before() {
    let sel = parse_sel("p::before").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Tag("p".into())]);
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
    // tag(p) + pseudo-element(::before) = (0,0,2)
    assert_eq!(sel.specificity, spec(0, 0, 2));
}

#[test]
fn parse_pseudo_element_after() {
    let sel = parse_sel("p::after").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::After));
    assert_eq!(sel.specificity, spec(0, 0, 2));
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
    assert_eq!(sel.specificity, spec(0, 1, 1));
}

#[test]
fn parse_pseudo_element_alone() {
    // `::before` alone is valid (implies universal selector)
    let sel = parse_sel("::before").unwrap();
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
    assert_eq!(sel.specificity, spec(0, 0, 1));
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

// --- Shadow DOM selector parse tests ---

#[test]
fn parse_host_selector() {
    let sel = parse_sel(":host").unwrap();
    assert_eq!(sel.components, vec![SelectorComponent::Host]);
    // :host specificity = (0, 1, 0).
    assert_eq!(sel.specificity, spec(0, 1, 0));
}

#[test]
fn parse_host_function_selector() {
    let sel = parse_sel(":host(.active)").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::HostFunction(vec![
            SelectorComponent::Class("active".to_string())
        ])]
    );
    // :host(.active) specificity = (0, 1, 0) + (0, 1, 0) = (0, 2, 0).
    assert_eq!(sel.specificity, spec(0, 2, 0));
}

#[test]
fn parse_slotted_selector() {
    let sel = parse_sel("::slotted(div)").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Slotted(vec![SelectorComponent::Tag(
            "div".to_string()
        )])]
    );
    // ::slotted(div) specificity = (0, 0, 1) + (0, 0, 1) = (0, 0, 2).
    assert_eq!(sel.specificity, spec(0, 0, 2));
}

// --- :not(:host) and :not(:host()) tests (CSS Selectors L4 §4.3) ---

#[test]
fn parse_not_host() {
    // :not(:host) is valid per CSS Selectors L4.
    let sel = parse_sel(":not(:host)").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Not(vec![SelectorComponent::Host])]
    );
}

#[test]
fn parse_not_host_function() {
    // :not(:host(.active)) is valid per CSS Selectors L4.
    let sel = parse_sel(":not(:host(.active))").unwrap();
    assert_eq!(
        sel.components,
        vec![SelectorComponent::Not(vec![
            SelectorComponent::HostFunction(vec![SelectorComponent::Class("active".to_string())])
        ])]
    );
}

// --- M3: ::slotted() trailing selector tests ---

#[test]
fn parse_slotted_alone_ok() {
    // ::slotted(div) is valid — no trailing selectors.
    let sel = parse_sel("::slotted(div)").unwrap();
    assert!(sel
        .components
        .iter()
        .any(|c| matches!(c, SelectorComponent::Slotted(_))));
}

#[test]
fn parse_slotted_with_before_ok() {
    // ::slotted(div)::before is valid per CSS Scoping §6.1.
    let sel = parse_sel("::slotted(div)::before").unwrap();
    assert!(sel
        .components
        .iter()
        .any(|c| matches!(c, SelectorComponent::Slotted(_))));
    assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
}

#[test]
fn parse_slotted_with_class_rejected() {
    // ::slotted(div).foo is invalid — trailing simple selectors not allowed.
    // The parser should consume ::slotted(div) but stop before .foo,
    // resulting in the input not being fully consumed.
    let mut input = cssparser::ParserInput::new("::slotted(div).foo");
    let mut parser = cssparser::Parser::new(&mut input);
    let result = parse_sel_from_parser(&mut parser);
    // The selector parses ok (::slotted(div)) but ".foo" remains unconsumed.
    assert!(result.is_ok());
    assert!(
        !parser.is_exhausted(),
        "trailing .foo should remain unconsumed"
    );
}

fn parse_sel_from_parser(parser: &mut cssparser::Parser) -> Result<Selector, ()> {
    super::super::parse::parse_one_selector(parser)
}
