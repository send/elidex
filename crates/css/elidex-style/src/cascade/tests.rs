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

/// Setup a DOM with a document root, a div shadow host, and an open shadow root.
#[allow(unused_must_use)]
fn setup_shadow_host() -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);
    dom.attach_shadow(host, elidex_ecs::ShadowRootMode::Open);
    (dom, host)
}

#[test]
fn single_declaration_wins() {
    let (dom, _, div) = setup_with_element("div");
    let ss = parse_stylesheet("div { color: red; }", Origin::Author);
    let sheets: Vec<&Stylesheet> = vec![&ss];
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &inline, &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &inline, &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
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
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &[], &ShadowCascade::Outer);
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

// --- Presentational hint (extra_declarations) cascade priority tests ---

#[test]
#[allow(unused_must_use)]
fn extra_decl_beats_ua_rule() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let div = elem(&mut dom, "div");
    dom.append_child(root, div);

    let ua = parse_stylesheet("div { color: green; }", Origin::UserAgent);
    let sheets: Vec<&Stylesheet> = vec![&ua];
    let hints = [Declaration::new("color", CssValue::Color(CssColor::BLUE))];
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &hints, &ShadowCascade::Outer);
    // Hint (author origin) beats UA normal.
    assert_eq!(
        winners.get("color"),
        Some(&&CssValue::Color(CssColor::BLUE))
    );
}

#[test]
#[allow(unused_must_use)]
fn extra_decl_loses_to_author_selector() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let div = elem(&mut dom, "div");
    dom.append_child(root, div);

    let author = parse_stylesheet("div { color: red; }", Origin::Author);
    let sheets: Vec<&Stylesheet> = vec![&author];
    let hints = [Declaration::new("color", CssValue::Color(CssColor::BLUE))];
    let winners = collect_and_cascade(div, &dom, &sheets, &[], &hints, &ShadowCascade::Outer);
    // Author selector rule beats hint (same origin, higher source_order).
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

#[test]
#[allow(unused_must_use)]
fn outer_context_beats_inner_context() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let div = elem(&mut dom, "div");
    dom.append_child(root, div);

    // Inner context rule (high specificity).
    let inner = parse_stylesheet("#special.important { color: blue; }", Origin::Author);
    // Outer context rule (low specificity).
    let outer = parse_stylesheet("div { color: red; }", Origin::Author);

    let mut entries: Vec<CascadeEntry> = Vec::new();

    // Simulate inner rules at is_outer_context = false.
    let mut inner_attrs = Attributes::default();
    inner_attrs.set("id", "special");
    inner_attrs.set("class", "important");
    let inner_div = elem_with_attrs(&mut dom, "div", inner_attrs);
    dom.append_child(root, inner_div);

    let inner_sheets: Vec<&Stylesheet> = vec![&inner];
    let outer_sheets: Vec<&Stylesheet> = vec![&outer];
    collect_matching_rules(&mut entries, inner_div, &dom, &inner_sheets, None, false);
    collect_matching_rules(&mut entries, inner_div, &dom, &outer_sheets, None, true);

    entries.sort_by(|a, b| a.priority.cmp(&b.priority));
    let mut winners: HashMap<&str, &CssValue> = HashMap::new();
    for entry in &entries {
        winners.insert(entry.property, entry.value);
    }

    // Outer context (div { color: red }) wins over inner context
    // despite lower specificity.
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

#[test]
fn host_selector_skipped_in_outer_context() {
    let (dom, host) = setup_shadow_host();

    // :host selector in outer stylesheet should be skipped.
    let outer_with_host = parse_stylesheet(":host { color: red; }", Origin::Author);
    let sheets: Vec<&Stylesheet> = vec![&outer_with_host];
    let winners = collect_and_cascade(host, &dom, &sheets, &[], &[], &ShadowCascade::Outer);

    // :host in outer context should not match.
    assert!(!winners.contains_key("color"));
}

#[test]
fn shadow_cascade_host_participates() {
    let (dom, host) = setup_shadow_host();

    let shadow_sheet = parse_stylesheet(":host { color: blue; }", Origin::Author);
    let winners = collect_and_cascade(
        host,
        &dom,
        &[],
        &[],
        &[],
        &ShadowCascade::Host(&shadow_sheet),
    );

    assert_eq!(
        winners.get("color"),
        Some(&&CssValue::Color(CssColor::BLUE))
    );
}

#[test]
#[allow(unused_must_use)]
fn shadow_cascade_slotted_participates() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);

    let p = elem(&mut dom, "p");
    dom.append_child(host, p);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Mark p as slotted.
    dom.world_mut()
        .insert_one(p, elidex_ecs::SlottedMarker)
        .unwrap();

    let shadow_sheet = parse_stylesheet("::slotted(p) { font-weight: bold; }", Origin::Author);
    let winners = collect_and_cascade(
        p,
        &dom,
        &[],
        &[],
        &[],
        &ShadowCascade::Slotted(&shadow_sheet),
    );

    assert_eq!(
        winners.get("font-weight"),
        Some(&&CssValue::Keyword("bold".to_string()))
    );
}

#[test]
#[allow(unused_must_use)]
fn outer_rule_beats_slotted_rule() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);

    let p = elem(&mut dom, "p");
    dom.append_child(host, p);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    dom.world_mut()
        .insert_one(p, elidex_ecs::SlottedMarker)
        .unwrap();

    let outer = parse_stylesheet("p { font-weight: normal; }", Origin::Author);
    let shadow_sheet = parse_stylesheet("::slotted(p) { font-weight: bold; }", Origin::Author);
    let outer_sheets: Vec<&Stylesheet> = vec![&outer];
    let winners = collect_and_cascade(
        p,
        &dom,
        &outer_sheets,
        &[],
        &[],
        &ShadowCascade::Slotted(&shadow_sheet),
    );

    // Outer context wins over ::slotted().
    assert_eq!(
        winners.get("font-weight"),
        Some(&&CssValue::Keyword("normal".to_string()))
    );
}

#[test]
#[allow(unused_must_use)]
fn extra_decl_loses_to_inline_style() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: red");
    let div = elem_with_attrs(&mut dom, "div", attrs);
    dom.append_child(root, div);

    let sheets: Vec<&Stylesheet> = vec![];
    let inline = get_inline_declarations(div, &dom);
    let hints = [Declaration::new("color", CssValue::Color(CssColor::BLUE))];
    let winners = collect_and_cascade(div, &dom, &sheets, &inline, &hints, &ShadowCascade::Outer);
    // Inline style beats hint.
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

// --- CSS Cascading L4 §6.1: !important reversal for shadow context ---

#[test]
fn important_inner_beats_normal_outer() {
    // :host { color: red !important } should beat outer div { color: blue }
    let (dom, host) = setup_shadow_host();

    let outer = parse_stylesheet("div { color: blue; }", Origin::Author);
    let shadow_sheet = parse_stylesheet(":host { color: red !important; }", Origin::Author);
    let outer_sheets: Vec<&Stylesheet> = vec![&outer];
    let winners = collect_and_cascade(
        host,
        &dom,
        &outer_sheets,
        &[],
        &[],
        &ShadowCascade::Host(&shadow_sheet),
    );

    // Inner !important beats outer normal (CSS Cascading L4 §6.1).
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

#[test]
fn important_inner_beats_important_outer() {
    // CSS Cascading L4 §6.1: !important では inner が outer に勝つ。
    // :host { color: red !important } should beat outer div { color: blue !important }
    let (dom, host) = setup_shadow_host();

    let outer = parse_stylesheet("div { color: blue !important; }", Origin::Author);
    let shadow_sheet = parse_stylesheet(":host { color: red !important; }", Origin::Author);
    let outer_sheets: Vec<&Stylesheet> = vec![&outer];
    let winners = collect_and_cascade(
        host,
        &dom,
        &outer_sheets,
        &[],
        &[],
        &ShadowCascade::Host(&shadow_sheet),
    );

    // Both !important: inner context wins over outer (reversed per §6.1).
    assert_eq!(winners.get("color"), Some(&&CssValue::Color(CssColor::RED)));
}

#[test]
#[allow(unused_must_use)]
fn important_slotted_beats_normal_outer() {
    // ::slotted(p) { font-weight: bold !important } beats outer p { font-weight: normal }
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);

    let p = elem(&mut dom, "p");
    dom.append_child(host, p);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    dom.world_mut()
        .insert_one(p, elidex_ecs::SlottedMarker)
        .unwrap();

    let outer = parse_stylesheet("p { font-weight: normal; }", Origin::Author);
    let shadow_sheet = parse_stylesheet(
        "::slotted(p) { font-weight: bold !important; }",
        Origin::Author,
    );
    let outer_sheets: Vec<&Stylesheet> = vec![&outer];
    let winners = collect_and_cascade(
        p,
        &dom,
        &outer_sheets,
        &[],
        &[],
        &ShadowCascade::Slotted(&shadow_sheet),
    );

    // Inner !important beats outer normal.
    assert_eq!(
        winners.get("font-weight"),
        Some(&&CssValue::Keyword("bold".to_string()))
    );
}
