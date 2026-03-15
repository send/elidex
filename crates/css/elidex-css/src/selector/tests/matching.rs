use super::*;

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

// ---- Dynamic pseudo-class tests ----

#[test]
fn dynamic_pseudo_class_matches_when_state_set() {
    let cases: &[(&str, &str, u16)] = &[
        ("div", ":hover", ElementState::HOVER),
        ("input", ":focus", ElementState::FOCUS),
        ("button", ":active", ElementState::ACTIVE),
        ("a", ":link", ElementState::LINK),
    ];

    for &(tag, pseudo, flag) in cases {
        let mut dom = EcsDom::new();
        let entity = elem(&mut dom, tag);
        let selector_str = format!("{tag}{pseudo}");
        let sel = parse_sel(&selector_str).unwrap();

        // No state -> no match.
        assert!(
            !sel.matches(entity, &dom),
            "{selector_str} should not match without state"
        );

        // Set the flag -> matches.
        let mut state = ElementState::default();
        state.insert(flag);
        let _ = dom.world_mut().insert_one(entity, state);
        assert!(
            sel.matches(entity, &dom),
            "{selector_str} should match with state set"
        );
    }
}

#[test]
fn visited_does_not_match_link_state() {
    // :visited should not match when only LINK flag is set.
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "a");
    let mut state = ElementState::default();
    state.insert(ElementState::LINK);
    let _ = dom.world_mut().insert_one(a, state);

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
    assert_eq!(sel.specificity, spec(0, 1, 1));
}

// --- Shadow DOM matching tests ---

#[test]
fn host_matches_shadow_host() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();

    let sel = parse_sel(":host").unwrap();
    assert!(sel.matches(host, &dom));
}

#[test]
fn host_does_not_match_non_host() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");

    let sel = parse_sel(":host").unwrap();
    assert!(!sel.matches(div, &dom));
}

#[test]
fn host_function_matches_with_class() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "active");
    let host = dom.create_element("div", attrs);
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();

    let sel = parse_sel(":host(.active)").unwrap();
    assert!(sel.matches(host, &dom));
}

#[test]
fn host_function_no_match_without_class() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();

    let sel = parse_sel(":host(.active)").unwrap();
    assert!(!sel.matches(host, &dom));
}

#[test]
fn slotted_matches_assigned_node() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "p");
    dom.append_child(host, light);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Assign light to slot.
    dom.world_mut()
        .insert_one(
            slot,
            elidex_ecs::SlotAssignment {
                assigned_nodes: vec![light],
            },
        )
        .unwrap();
    dom.world_mut()
        .insert_one(light, elidex_ecs::SlottedMarker)
        .unwrap();

    let sel = parse_sel("::slotted(p)").unwrap();
    assert!(sel.matches(light, &dom));
}

#[test]
fn slotted_no_match_unassigned() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");

    let sel = parse_sel("::slotted(div)").unwrap();
    assert!(!sel.matches(div, &dom));
}

#[test]
fn shadow_boundary_stops_descendant_selector() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let shadow_child = elem(&mut dom, "p");
    dom.append_child(sr, shadow_child);

    // "div p" should NOT match because the shadow root is a boundary.
    let sel = parse_sel("div p").unwrap();
    assert!(!sel.matches(shadow_child, &dom));
}

#[test]
fn shadow_boundary_stops_child_selector() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let shadow_child = elem(&mut dom, "p");
    dom.append_child(sr, shadow_child);

    // "div > p" should NOT match because shadow root is between them.
    let child_sel = parse_sel("div > p").unwrap();
    assert!(!child_sel.matches(shadow_child, &dom));
}

// --- :not(:host) matching tests ---

#[test]
fn not_host_matches_non_host() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let host = elem(&mut dom, "div");
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();

    let sel = parse_sel(":not(:host)").unwrap();
    // Non-host matches :not(:host).
    assert!(sel.matches(div, &dom));
    // Shadow host does NOT match :not(:host).
    assert!(!sel.matches(host, &dom));
}

#[test]
fn not_host_function_matches_host_without_class() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();

    // :not(:host(.active)) should match host without .active class.
    let sel = parse_sel(":not(:host(.active))").unwrap();
    assert!(sel.matches(host, &dom));

    // Host with .active should NOT match :not(:host(.active)).
    let mut attrs = Attributes::default();
    attrs.set("class", "active");
    let host2 = dom.create_element("div", attrs);
    dom.attach_shadow(host2, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    assert!(!sel.matches(host2, &dom));
}

// --- Form pseudo-class matching tests (M4-3.5 R2) ---

#[test]
fn read_only_matches_div_without_contenteditable() {
    // Non-form elements without contenteditable are :read-only per HTML spec.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    let sel = parse_sel(":read-only").unwrap();
    assert!(sel.matches(div, &dom));
}

#[test]
fn read_write_matches_div_with_contenteditable() {
    // contenteditable elements are :read-write per HTML spec.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("contenteditable", "true");
    let div = dom.create_element("div", attrs);
    let read_write = parse_sel(":read-write").unwrap();
    let read_only = parse_sel(":read-only").unwrap();
    assert!(read_write.matches(div, &dom));
    assert!(!read_only.matches(div, &dom));
}

#[test]
fn checked_matches_option_with_selected() {
    // :checked matches <option> elements with the selected attribute.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("selected", "");
    let option = dom.create_element("option", attrs);
    let sel = parse_sel(":checked").unwrap();
    assert!(sel.matches(option, &dom));
}

#[test]
fn checked_does_not_match_option_without_selected() {
    let mut dom = EcsDom::new();
    let option = elem(&mut dom, "option");
    let sel = parse_sel(":checked").unwrap();
    assert!(!sel.matches(option, &dom));
}

#[test]
fn required_does_not_match_fieldset() {
    // :required excludes <fieldset> per HTML spec §4.10.15.2.4.
    let mut dom = EcsDom::new();
    let fs = dom.create_element("fieldset", Attributes::default());
    let mut es = ElementState::default();
    es.insert(ElementState::REQUIRED);
    let _ = dom.world_mut().insert_one(fs, es);
    let sel = parse_sel(":required").unwrap();
    assert!(!sel.matches(fs, &dom));
}

#[test]
fn optional_does_not_match_fieldset() {
    // :optional excludes <fieldset> per HTML spec §4.10.15.2.4.
    let mut dom = EcsDom::new();
    let fs = dom.create_element("fieldset", Attributes::default());
    let _ = dom.world_mut().insert_one(fs, ElementState::default());
    let sel = parse_sel(":optional").unwrap();
    assert!(!sel.matches(fs, &dom));
}

#[test]
fn required_does_not_match_button() {
    // :required only applies to input/select/textarea, not <button>.
    let mut dom = EcsDom::new();
    let btn = dom.create_element("button", Attributes::default());
    let mut es = ElementState::default();
    es.insert(ElementState::REQUIRED);
    let _ = dom.world_mut().insert_one(btn, es);
    let sel = parse_sel(":required").unwrap();
    assert!(!sel.matches(btn, &dom));
}

#[test]
fn read_write_inherited_contenteditable() {
    // :read-write should match descendants of contenteditable elements.
    let mut dom = EcsDom::new();
    let doc = dom.create_element("html", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("contenteditable", "true");
    let parent = dom.create_element("div", attrs);
    let child = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(doc, parent));
    assert!(dom.append_child(parent, child));
    let sel = parse_sel(":read-write").unwrap();
    assert!(sel.matches(child, &dom));
    // And :read-only should NOT match it.
    let sel_ro = parse_sel(":read-only").unwrap();
    assert!(!sel_ro.matches(child, &dom));
}

#[test]
fn valid_matches_output_element() {
    // <output> is always valid per HTML §4.10.18.7.
    let mut dom = EcsDom::new();
    let output = dom.create_element("output", Attributes::default());
    let sel_valid = parse_sel(":valid").unwrap();
    let sel_invalid = parse_sel(":invalid").unwrap();
    assert!(sel_valid.matches(output, &dom));
    assert!(!sel_invalid.matches(output, &dom));
}

#[test]
fn optional_does_not_match_button() {
    // :optional only applies to input/select/textarea, not <button>.
    let mut dom = EcsDom::new();
    let btn = dom.create_element("button", Attributes::default());
    let _ = dom.world_mut().insert_one(btn, ElementState::default());
    let sel = parse_sel(":optional").unwrap();
    assert!(!sel.matches(btn, &dom));
}

#[test]
fn enabled_disabled_does_not_match_fieldset() {
    // Per HTML §4.10.18.5: <fieldset> is NOT in the set of "actually disableable"
    // elements. :enabled/:disabled do not match <fieldset>.
    let mut dom = EcsDom::new();
    let fs = dom.create_element("fieldset", Attributes::default());
    let _ = dom.world_mut().insert_one(fs, ElementState::default());
    let sel_enabled = parse_sel(":enabled").unwrap();
    let sel_disabled = parse_sel(":disabled").unwrap();
    assert!(!sel_enabled.matches(fs, &dom));
    assert!(!sel_disabled.matches(fs, &dom));

    // Disabled fieldset still doesn't match :enabled/:disabled.
    let mut es = ElementState::default();
    es.insert(ElementState::DISABLED);
    let _ = dom.world_mut().insert_one(fs, es);
    assert!(!sel_enabled.matches(fs, &dom));
    assert!(!sel_disabled.matches(fs, &dom));
}

#[test]
fn checked_does_not_match_non_checkbox_radio() {
    // :checked only matches <input type=checkbox|radio> and <option selected>,
    // not arbitrary form elements with CHECKED flag.
    let mut dom = EcsDom::new();
    let btn = dom.create_element("button", Attributes::default());
    let mut es = ElementState::default();
    es.insert(ElementState::CHECKED);
    let _ = dom.world_mut().insert_one(btn, es);
    let sel = parse_sel(":checked").unwrap();
    assert!(!sel.matches(btn, &dom));

    // <input type=text> with CHECKED also should not match.
    let text_input = dom.create_element("input", Attributes::default());
    let _ = dom.world_mut().insert_one(text_input, es);
    assert!(!sel.matches(text_input, &dom));
}

#[test]
fn checked_matches_checkbox_input() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    let cb = dom.create_element("input", attrs);
    let mut es = ElementState::default();
    es.insert(ElementState::CHECKED);
    let _ = dom.world_mut().insert_one(cb, es);
    let sel = parse_sel(":checked").unwrap();
    assert!(sel.matches(cb, &dom));
}

#[test]
fn indeterminate_does_not_match_non_candidate() {
    // :indeterminate only matches checkbox, radio, and progress per HTML §4.10.18.3.
    let mut dom = EcsDom::new();
    let btn = dom.create_element("button", Attributes::default());
    let mut es = ElementState::default();
    es.insert(ElementState::INDETERMINATE);
    let _ = dom.world_mut().insert_one(btn, es);
    let sel = parse_sel(":indeterminate").unwrap();
    assert!(!sel.matches(btn, &dom));

    // <input type=text> with INDETERMINATE should not match.
    let text_input = dom.create_element("input", Attributes::default());
    let _ = dom.world_mut().insert_one(text_input, es);
    assert!(!sel.matches(text_input, &dom));
}

#[test]
fn indeterminate_matches_checkbox() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    let cb = dom.create_element("input", attrs);
    let mut es = ElementState::default();
    es.insert(ElementState::INDETERMINATE);
    let _ = dom.world_mut().insert_one(cb, es);
    let sel = parse_sel(":indeterminate").unwrap();
    assert!(sel.matches(cb, &dom));
}

#[test]
fn indeterminate_matches_progress() {
    let mut dom = EcsDom::new();
    let progress = dom.create_element("progress", Attributes::default());
    let mut es = ElementState::default();
    es.insert(ElementState::INDETERMINATE);
    let _ = dom.world_mut().insert_one(progress, es);
    let sel = parse_sel(":indeterminate").unwrap();
    assert!(sel.matches(progress, &dom));
}
