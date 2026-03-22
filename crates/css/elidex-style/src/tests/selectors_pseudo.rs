use super::*;

// --- M3-3: Selector enhancement integration tests ---

#[test]
fn attr_selector_style_integration() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("type", "text");
    let input = dom.create_element("input", attrs);
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, input);
    dom.append_child(body, div);

    let css = r#"[type="text"] { color: red; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let input_style = get_style(&dom, input);
    assert_eq!(input_style.color, CssColor::RED);
    // div should not be affected.
    let div_style = get_style(&dom, div);
    assert_ne!(div_style.color, CssColor::RED);
}

#[test]
fn adjacent_sibling_style_integration() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let h1 = dom.create_element("h1", Attributes::default());
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, h1);
    dom.append_child(body, p);

    let css = "h1 + p { color: blue; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let p_style = get_style(&dom, p);
    assert_eq!(p_style.color, CssColor::BLUE);
}

#[test]
fn first_child_style_integration() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let li1 = dom.create_element("li", Attributes::default());
    let li2 = dom.create_element("li", Attributes::default());
    dom.append_child(body, li1);
    dom.append_child(body, li2);

    // Use background-color (non-inherited) to avoid inheritance leaks.
    let css = "li:first-child { background-color: green; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let li1_style = get_style(&dom, li1);
    assert_eq!(li1_style.background_color, CssColor::new(0, 128, 0, 255));
    let li2_style = get_style(&dom, li2);
    assert_ne!(li2_style.background_color, CssColor::new(0, 128, 0, 255));
}

#[test]
fn not_selector_style_integration() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("class", "hidden");
    let hidden = dom.create_element("div", attrs);
    let visible = dom.create_element("div", Attributes::default());
    dom.append_child(body, hidden);
    dom.append_child(body, visible);

    let css = "div:not(.hidden) { color: red; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let hidden_style = get_style(&dom, hidden);
    assert_ne!(hidden_style.color, CssColor::RED);
    let visible_style = get_style(&dom, visible);
    assert_eq!(visible_style.color, CssColor::RED);
}

#[test]
fn child_first_child_combined_style_integration() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let ul = dom.create_element("ul", Attributes::default());
    let li1 = dom.create_element("li", Attributes::default());
    let li2 = dom.create_element("li", Attributes::default());
    dom.append_child(body, ul);
    dom.append_child(ul, li1);
    dom.append_child(ul, li2);

    let css = "ul > li:first-child { color: red; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let li1_style = get_style(&dom, li1);
    assert_eq!(li1_style.color, CssColor::RED);
    let li2_style = get_style(&dom, li2);
    assert_ne!(li2_style.color, CssColor::RED);
}

// --- M3.5-0: Pseudo-element tests ---

#[test]
fn pseudo_before_generates_entity() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, p);

    let css = r#"p::before { content: ">>"; color: red; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    // p should have a pseudo-element child.
    let children: Vec<Entity> = dom.children_iter(p).collect();
    let pe = children
        .iter()
        .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok());
    assert!(pe.is_some(), "pseudo-element entity not found");
    let pe = *pe.unwrap();
    // Check text content.
    let tc = dom.world().get::<&TextContent>(pe).unwrap();
    assert_eq!(tc.0, ">>");
    // Check style.
    let pe_style = get_style(&dom, pe);
    assert_eq!(pe_style.color, CssColor::RED);
}

#[test]
fn pseudo_after_generates_entity() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, p);

    let css = r#"p::after { content: "<<"; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children: Vec<Entity> = dom.children_iter(p).collect();
    let last = children.last().unwrap();
    assert!(dom.world().get::<&PseudoElementMarker>(*last).is_ok());
    let tc = dom.world().get::<&TextContent>(*last).unwrap();
    assert_eq!(tc.0, "<<");
}

#[test]
fn pseudo_content_none_no_entity() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, p);

    let css = r"p::before { content: none; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children: Vec<Entity> = dom.children_iter(p).collect();
    let has_pe = children
        .iter()
        .any(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok());
    assert!(!has_pe);
}

#[test]
fn pseudo_content_attr() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("title", "TitleText");
    let p = dom.create_element("p", attrs);
    dom.append_child(body, p);

    let css = r"p::before { content: attr(title); }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children: Vec<Entity> = dom.children_iter(p).collect();
    let pe = children
        .iter()
        .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .unwrap();
    let tc = dom.world().get::<&TextContent>(*pe).unwrap();
    assert_eq!(tc.0, "TitleText");
}

#[test]
fn pseudo_cascade_later_wins() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("class", "x");
    let p = dom.create_element("p", attrs);
    dom.append_child(body, p);

    let css = r#".x::before { content: "A"; } .x::before { content: "B"; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children: Vec<Entity> = dom.children_iter(p).collect();
    let pe = children
        .iter()
        .find(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .unwrap();
    let tc = dom.world().get::<&TextContent>(*pe).unwrap();
    assert_eq!(tc.0, "B");
}

#[test]
fn pseudo_re_resolve_removes_old() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    let text = dom.create_text("hello");
    dom.append_child(body, p);
    dom.append_child(p, text);

    let css = r#"p::before { content: ">>"; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    // First resolution: one pseudo entity + one text node.
    let pe_count1 = dom
        .children_iter(p)
        .filter(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .count();
    assert_eq!(pe_count1, 1);

    // Re-resolve: should still have exactly one PE.
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    let pe_count2 = dom
        .children_iter(p)
        .filter(|&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .count();
    assert_eq!(pe_count2, 1);
}

#[test]
fn pseudo_does_not_affect_normal_element_matching() {
    // Ensure pseudo-element selectors don't affect normal element styling.
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, p);

    let css = r#"p::before { content: ">>"; color: red; } p { color: blue; }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    // p itself should be blue, not red.
    let p_style = get_style(&dom, p);
    assert_eq!(p_style.color, CssColor::BLUE);
}

#[test]
fn link_element_gets_link_state() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("href", "https://example.com");
    let a = dom.create_element("a", attrs);
    dom.append_child(body, a);

    resolve_styles(&mut dom, &[], Size::new(1920.0, 1080.0));

    let state = dom
        .world()
        .get::<&ElementState>(a)
        .ok()
        .map(|s| *s)
        .unwrap_or_default();
    assert!(state.contains(ElementState::LINK));
}

#[test]
fn ua_link_gets_blue_color() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("href", "https://example.com");
    let a = dom.create_element("a", attrs);
    let text = dom.create_text("Link");
    dom.append_child(body, a);
    dom.append_child(a, text);

    resolve_styles(&mut dom, &[], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, a);
    // UA a:link color is #0000ee = rgb(0, 0, 238)
    assert_eq!(style.color, CssColor::new(0, 0, 238, 255));
}

#[test]
fn pseudo_before_after_full_pipeline() {
    // Full pipeline: parse CSS -> resolve styles -> verify pseudo entities.
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    let text = dom.create_text("Hello");
    dom.append_child(body, p);
    dom.append_child(p, text);

    let css =
        "p::before { content: \">> \"; color: red; } p::after { content: \" <<\"; color: blue; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children = dom.children(p);
    // Should have: ::before PE, text node, ::after PE = 3 children
    assert_eq!(
        children.len(),
        3,
        "expected 3 children (::before, text, ::after)"
    );

    // First child: ::before
    let before_pe = children[0];
    assert!(dom.world().get::<&PseudoElementMarker>(before_pe).is_ok());
    let before_tc = dom.world().get::<&TextContent>(before_pe).unwrap();
    assert_eq!(before_tc.0, ">> ");
    let before_style = get_style(&dom, before_pe);
    assert_eq!(before_style.color, CssColor::new(255, 0, 0, 255));

    // Last child: ::after
    let after_pe = children[2];
    assert!(dom.world().get::<&PseudoElementMarker>(after_pe).is_ok());
    let after_tc = dom.world().get::<&TextContent>(after_pe).unwrap();
    assert_eq!(after_tc.0, " <<");
    let after_style = get_style(&dom, after_pe);
    assert_eq!(after_style.color, CssColor::new(0, 0, 255, 255));

    // Middle child: original text node (no PseudoElementMarker)
    let text_node = children[1];
    assert!(dom.world().get::<&PseudoElementMarker>(text_node).is_err());
}

#[test]
fn hover_pseudo_class_applies_style() {
    use elidex_ecs::ElementState as DomState;
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);

    let css = "div { color: black; } div:hover { color: red; }";
    let ss = parse_stylesheet(css, Origin::Author);

    // Without hover: color is black.
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    let style_no_hover = get_style(&dom, div);
    assert_eq!(style_no_hover.color, CssColor::new(0, 0, 0, 255));

    // Set hover state and re-resolve.
    let mut state = DomState::default();
    state.insert(DomState::HOVER);
    dom.world_mut().insert_one(div, state);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let style_hover = get_style(&dom, div);
    assert_eq!(style_hover.color, CssColor::new(255, 0, 0, 255));
}

#[test]
fn pseudo_content_var_resolution() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let p = dom.create_element("p", Attributes::default());
    let text = dom.create_text("Hello");
    dom.append_child(body, p);
    dom.append_child(p, text);

    let css = r#":root { --icon: ">>"; } p::before { content: var(--icon); }"#;
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    // The ::before pseudo-element should have content from var(--icon).
    let children = dom.children(p);
    let pe_children: Vec<_> = children
        .iter()
        .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .collect();
    assert_eq!(pe_children.len(), 1, "expected 1 pseudo-element");
    let tc = dom.world().get::<&TextContent>(*pe_children[0]).unwrap();
    assert_eq!(tc.0, ">>");
}

#[test]
fn hover_pseudo_element_combined() {
    use elidex_ecs::ElementState as DomState;
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    let text = dom.create_text("Hello");
    dom.append_child(body, div);
    dom.append_child(div, text);

    let css = r#"div:hover::before { content: ">>"; color: green; }"#;
    let ss = parse_stylesheet(css, Origin::Author);

    // Without hover: no pseudo-element generated.
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    let children = dom.children(div);
    let pe_count = children
        .iter()
        .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .count();
    assert_eq!(pe_count, 0, "no PE without hover");

    // Set hover and re-resolve.
    let mut state = DomState::default();
    state.insert(DomState::HOVER);
    dom.world_mut().insert_one(div, state);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let children = dom.children(div);
    let pe_children: Vec<_> = children
        .iter()
        .filter(|&&c| dom.world().get::<&PseudoElementMarker>(c).is_ok())
        .collect();
    assert_eq!(pe_children.len(), 1, "1 PE with hover");
    let tc = dom.world().get::<&TextContent>(*pe_children[0]).unwrap();
    assert_eq!(tc.0, ">>");
    let pe_style = get_style(&dom, *pe_children[0]);
    assert_eq!(pe_style.color, CssColor::new(0, 128, 0, 255));
}
