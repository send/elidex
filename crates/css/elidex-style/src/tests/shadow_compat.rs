use super::*;

// --- resolve_styles_with_compat integration tests ---

#[test]
fn compat_extra_ua_and_hints_combined() {
    // Verify that resolve_styles_with_compat applies both extra UA sheets
    // and presentational hints from the hint_generator.
    let (mut dom, _root, _html, body) = build_simple_dom();

    // Create a <b> element (needs legacy UA for font-weight: bolder)
    let b = dom.create_element("b", Attributes::default());
    dom.append_child(body, b);

    // Create an img with bgcolor (needs hint_generator for background-color)
    let mut attrs = Attributes::default();
    attrs.set("bgcolor", "red");
    let div = dom.create_element("body", attrs);
    dom.append_child(body, div);

    // Extra UA sheet with b { font-weight: bolder; }
    let extra_ua = parse_stylesheet("b { font-weight: bolder; }", Origin::UserAgent);

    // Hint generator: emit background-color for bgcolor attribute
    let hint_gen = |entity: Entity, dom: &EcsDom| -> Vec<Declaration> {
        let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
            return Vec::new();
        };
        if let Some(val) = attrs.get("bgcolor") {
            if val == "red" {
                return vec![Declaration::new(
                    "background-color",
                    CssValue::Color(CssColor::RED),
                )];
            }
        }
        Vec::new()
    };

    resolve_styles_with_compat(&mut dom, &[], &[&extra_ua], &hint_gen, 1920.0, 1080.0, None);

    // <b> should pick up font-weight: bolder from extra UA sheet.
    let b_style = get_style(&dom, b);
    // bolder from 400 (default) -> 700
    assert_eq!(b_style.font_weight, 700);

    // div with bgcolor="red" should have background-color from hint.
    let div_style = get_style(&dom, div);
    assert_eq!(div_style.background_color, CssColor::RED);
}

#[test]
fn compat_hint_loses_to_author_selector() {
    // Hint (author origin, specificity (0,0,0)) should lose to author rule.
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);

    let author = parse_stylesheet("div { background-color: blue; }", Origin::Author);

    let hint_gen = |_entity: Entity, _dom: &EcsDom| -> Vec<Declaration> {
        vec![Declaration::new(
            "background-color",
            CssValue::Color(CssColor::RED),
        )]
    };

    resolve_styles_with_compat(&mut dom, &[&author], &[], &hint_gen, 1920.0, 1080.0, None);

    let style = get_style(&dom, div);
    assert_eq!(style.background_color, CssColor::BLUE);
}

// --- Shadow DOM style scoping tests (A1, A2, A3) ---

#[test]
fn a2_host_rule_applied_to_shadow_host() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let sr = setup_shadow_with_style(&mut dom, host, ":host { color: red; }");
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    resolve_styles(&mut dom, &[], 1920.0, 1080.0);

    let style = get_style(&dom, host);
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn a2_outer_rule_beats_host_rule() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let sr = setup_shadow_with_style(&mut dom, host, ":host { color: red; }");
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    let outer = parse_stylesheet("div { color: blue; }", Origin::Author);
    resolve_styles(&mut dom, &[&outer], 1920.0, 1080.0);

    let style = get_style(&dom, host);
    assert_eq!(style.color, CssColor::BLUE);
}

#[test]
fn a3_slotted_rule_applied_to_slotted_node() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let light_p = dom.create_element("p", Attributes::default());
    dom.append_child(host, light_p);

    let sr = setup_shadow_with_style(&mut dom, host, "::slotted(p) { color: red; }");
    let slot = dom.create_element("slot", Attributes::default());
    dom.append_child(sr, slot);

    resolve_styles(&mut dom, &[], 1920.0, 1080.0);

    let style = get_style(&dom, light_p);
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn a3_outer_rule_beats_slotted_rule() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let light_p = dom.create_element("p", Attributes::default());
    dom.append_child(host, light_p);

    let sr = setup_shadow_with_style(&mut dom, host, "::slotted(p) { color: red; }");
    let slot = dom.create_element("slot", Attributes::default());
    dom.append_child(sr, slot);

    let outer = parse_stylesheet("p { color: blue; }", Origin::Author);
    resolve_styles(&mut dom, &[&outer], 1920.0, 1080.0);

    let style = get_style(&dom, light_p);
    assert_eq!(style.color, CssColor::BLUE);
}

#[test]
fn a1_nested_slot_gets_styled() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let light = dom.create_element("span", Attributes::default());
    dom.append_child(host, light);

    let sr = setup_shadow_with_style(&mut dom, host, "::slotted(span) { color: green; }");
    let wrapper = dom.create_element("div", Attributes::default());
    let slot = dom.create_element("slot", Attributes::default());
    dom.append_child(sr, wrapper);
    dom.append_child(wrapper, slot);

    resolve_styles(&mut dom, &[], 1920.0, 1080.0);

    let style = get_style(&dom, light);
    assert_eq!(style.color, CssColor::GREEN);
}

#[test]
fn shadow_inherited_props_cross_boundary() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let sr = setup_shadow_with_style(&mut dom, host, "");
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    let outer = parse_stylesheet("div { font-size: 24px; }", Origin::Author);
    resolve_styles(&mut dom, &[&outer], 1920.0, 1080.0);

    let host_style = get_style(&dom, host);
    assert_eq!(host_style.font_size, 24.0);

    let inner_style = get_style(&dom, inner);
    assert_eq!(inner_style.font_size, 24.0);
}

#[test]
fn b2_host_not_matched_outside_shadow_context() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let host = dom.create_element("div", Attributes::default());
    dom.append_child(body, host);

    let _sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();

    let outer_with_host = parse_stylesheet(":host { color: red; }", Origin::Author);
    resolve_styles(&mut dom, &[&outer_with_host], 1920.0, 1080.0);

    let style = get_style(&dom, host);
    assert_ne!(style.color, CssColor::RED);
}

#[test]
fn host_function_rule_matches_host() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("class", "special");
    let host = dom.create_element("div", attrs);
    dom.append_child(body, host);

    let sr = setup_shadow_with_style(&mut dom, host, ":host(.special) { color: red; }");
    let inner = dom.create_element("span", Attributes::default());
    dom.append_child(sr, inner);

    resolve_styles(&mut dom, &[], 1920.0, 1080.0);

    let style = get_style(&dom, host);
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn nested_shadow_hosts_have_isolated_styles() {
    let (mut dom, _root, _html, body) = build_simple_dom();

    // Outer shadow host with :host { color: red }.
    let outer_host = dom.create_element("div", Attributes::default());
    dom.append_child(body, outer_host);
    let outer_sr = setup_shadow_with_style(&mut dom, outer_host, ":host { color: red; }");

    // Inner shadow host inside the outer shadow tree.
    let inner_host = dom.create_element("div", Attributes::default());
    dom.append_child(outer_sr, inner_host);
    let inner_sr = setup_shadow_with_style(&mut dom, inner_host, ":host { color: blue; }");
    let leaf = dom.create_element("span", Attributes::default());
    dom.append_child(inner_sr, leaf);

    resolve_styles(&mut dom, &[], 1920.0, 1080.0);

    // Each shadow context is isolated: outer gets red, inner gets blue.
    let outer_style = get_style(&dom, outer_host);
    assert_eq!(outer_style.color, CssColor::RED, "outer host should be red");

    let inner_style = get_style(&dom, inner_host);
    assert_eq!(
        inner_style.color,
        CssColor::BLUE,
        "inner host should be blue"
    );
}
