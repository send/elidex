//! Tests for the elidex-dom-compat crate.

use elidex_css::{Declaration, Origin, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{CssColor, CssValue, LengthUnit};

use crate::legacy_ua::legacy_ua_stylesheet;
use crate::presentational::get_presentational_hints;
use crate::vendor_prefix::strip_vendor_prefixes;

// --- Helper functions ---

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn elem_with_attrs(dom: &mut EcsDom, tag: &str, attrs: Attributes) -> Entity {
    dom.create_element(tag, attrs)
}

#[allow(unused_must_use)]
fn setup_element(tag: &str) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let el = elem(&mut dom, tag);
    dom.append_child(root, el);
    (dom, el)
}

#[allow(unused_must_use)]
fn setup_element_with_attrs(tag: &str, attrs: Attributes) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let el = elem_with_attrs(&mut dom, tag, attrs);
    dom.append_child(root, el);
    (dom, el)
}

fn find_decl<'a>(decls: &'a [Declaration], prop: &str) -> Option<&'a CssValue> {
    decls.iter().find(|d| d.property == prop).map(|d| &d.value)
}

fn has_tag_selector(rule: &elidex_css::CssRule, tag: &str) -> bool {
    rule.selectors.iter().any(|sel| {
        sel.components
            .iter()
            .any(|c| matches!(c, elidex_css::SelectorComponent::Tag(t) if t == tag))
    })
}

fn find_rule_for_tag(ss: &Stylesheet, tag: &str) -> bool {
    ss.rules.iter().any(|r| has_tag_selector(r, tag))
}

fn find_decl_in_rules(ss: &Stylesheet, tag: &str, prop: &str) -> Option<CssValue> {
    ss.rules
        .iter()
        .filter(|r| has_tag_selector(r, tag))
        .flat_map(|r| &r.declarations)
        .find(|d| d.property == prop)
        .map(|d| d.value.clone())
}

// === Legacy UA stylesheet tests ===

#[test]
fn legacy_ua_single_declarations() {
    let ss = legacy_ua_stylesheet();
    for (tag, prop, expected_kw) in [
        ("b", "font-weight", "bolder"),
        ("i", "font-style", "italic"),
        ("u", "text-decoration-line", "underline"),
        ("input", "display", "inline-block"),
        ("address", "font-style", "italic"),
    ] {
        let val = find_decl_in_rules(ss, tag, prop);
        assert_eq!(
            val,
            Some(CssValue::Keyword(expected_kw.to_string())),
            "{tag} should have {prop}: {expected_kw}"
        );
    }
}

#[test]
fn legacy_ua_center_block_center() {
    let ss = legacy_ua_stylesheet();
    let display = find_decl_in_rules(ss, "center", "display");
    assert_eq!(display, Some(CssValue::Keyword("block".to_string())));
    let align = find_decl_in_rules(ss, "center", "text-align");
    assert_eq!(align, Some(CssValue::Keyword("center".to_string())));
}

#[test]
fn legacy_ua_mark_yellow() {
    let ss = legacy_ua_stylesheet();
    let bg = find_decl_in_rules(ss, "mark", "background-color");
    assert_eq!(bg, Some(CssValue::Color(CssColor::new(255, 255, 0, 255))));
    let color = find_decl_in_rules(ss, "mark", "color");
    assert_eq!(color, Some(CssValue::Color(CssColor::BLACK)));
}

#[test]
fn legacy_ua_parses_without_error() {
    let ss = legacy_ua_stylesheet();
    assert!(!ss.rules.is_empty());
    assert_eq!(ss.origin, Origin::UserAgent);
}

#[test]
fn legacy_ua_tag_rules_exist() {
    let ss = legacy_ua_stylesheet();
    for tag in ["strong", "em"] {
        assert!(find_rule_for_tag(ss, tag), "rule for {tag} not found");
    }
}

// === Presentational hints tests ===

#[test]
fn presentational_single_attr_hints() {
    for (tag, attr_name, attr_val, prop, expected) in [
        (
            "body",
            "bgcolor",
            "red",
            "background-color",
            CssValue::Color(CssColor::RED),
        ),
        (
            "font",
            "color",
            "#ff0000",
            "color",
            CssValue::Color(CssColor::RED),
        ),
        (
            "div",
            "align",
            "center",
            "text-align",
            CssValue::Keyword("center".to_string()),
        ),
        (
            "tbody",
            "bgcolor",
            "blue",
            "background-color",
            CssValue::Color(CssColor::BLUE),
        ),
        (
            "p",
            "align",
            "justify",
            "text-align",
            CssValue::Keyword("justify".to_string()),
        ),
        (
            "font",
            "size",
            "5",
            "font-size",
            CssValue::Keyword("x-large".to_string()),
        ),
        (
            "font",
            "size",
            "10",
            "font-size",
            CssValue::Keyword("xxx-large".to_string()),
        ),
        (
            "font",
            "size",
            "0",
            "font-size",
            CssValue::Keyword("x-small".to_string()),
        ),
    ] {
        let mut attrs = Attributes::default();
        attrs.set(attr_name, attr_val);
        let (dom, el) = setup_element_with_attrs(tag, attrs);
        let hints = get_presentational_hints(el, &dom);
        let val = find_decl(&hints, prop);
        assert_eq!(
            val,
            Some(&expected),
            "{tag}[{attr_name}={attr_val}] → {prop}"
        );
    }
}

#[test]
fn presentational_table_border() {
    let mut attrs = Attributes::default();
    attrs.set("border", "1");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    let val = find_decl(&hints, "border-top-width");
    assert_eq!(val, Some(&CssValue::Length(1.0, LengthUnit::Px)));
    // WHATWG §15.3.10: table border-style is outset (not solid).
    let style_val = find_decl(&hints, "border-top-style");
    assert_eq!(style_val, Some(&CssValue::Keyword("outset".to_string())));
}

#[test]
fn presentational_font_face() {
    let mut attrs = Attributes::default();
    attrs.set("face", "Arial, Helvetica");
    let (dom, el) = setup_element_with_attrs("font", attrs);
    let hints = get_presentational_hints(el, &dom);
    let val = find_decl(&hints, "font-family");
    assert_eq!(
        val,
        Some(&CssValue::List(vec![
            CssValue::Keyword("Arial".to_string()),
            CssValue::Keyword("Helvetica".to_string()),
        ]))
    );
}

#[test]
fn presentational_img_width_height() {
    let mut attrs = Attributes::default();
    attrs.set("width", "100");
    attrs.set("height", "50");
    let (dom, el) = setup_element_with_attrs("img", attrs);
    let hints = get_presentational_hints(el, &dom);
    let w = find_decl(&hints, "width");
    assert_eq!(w, Some(&CssValue::Length(100.0, LengthUnit::Px)));
    let h = find_decl(&hints, "height");
    assert_eq!(h, Some(&CssValue::Length(50.0, LengthUnit::Px)));
}

#[test]
fn presentational_img_width_percentage() {
    let mut attrs = Attributes::default();
    attrs.set("width", "50%");
    let (dom, el) = setup_element_with_attrs("img", attrs);
    let hints = get_presentational_hints(el, &dom);
    let w = find_decl(&hints, "width");
    assert_eq!(w, Some(&CssValue::Percentage(50.0)));
}

#[test]
fn presentational_cellspacing() {
    let mut attrs = Attributes::default();
    attrs.set("cellspacing", "5");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    let h = find_decl(&hints, "border-spacing-h");
    assert_eq!(h, Some(&CssValue::Length(5.0, LengthUnit::Px)));
    let v = find_decl(&hints, "border-spacing-v");
    assert_eq!(v, Some(&CssValue::Length(5.0, LengthUnit::Px)));
}

#[test]
#[allow(unused_must_use)]
fn presentational_cellpadding_on_td() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();

    let mut table_attrs = Attributes::default();
    table_attrs.set("cellpadding", "10");
    let table = elem_with_attrs(&mut dom, "table", table_attrs);
    dom.append_child(root, table);

    let tr = elem(&mut dom, "tr");
    dom.append_child(table, tr);

    let td = elem(&mut dom, "td");
    dom.append_child(tr, td);

    let hints = get_presentational_hints(td, &dom);
    let val = find_decl(&hints, "padding-top");
    assert_eq!(val, Some(&CssValue::Length(10.0, LengthUnit::Px)));
    let val = find_decl(&hints, "padding-left");
    assert_eq!(val, Some(&CssValue::Length(10.0, LengthUnit::Px)));
}

#[test]
fn presentational_no_hints_for_span() {
    let (dom, el) = setup_element("span");
    let hints = get_presentational_hints(el, &dom);
    assert!(hints.is_empty());
}

// === Vendor prefix tests ===

#[test]
fn vendor_prefix_strip_basic() {
    for (input, must_contain, must_not_contain) in [
        (
            "div { -webkit-display: flex; }",
            vec!["display: flex"],
            vec!["-webkit-"],
        ),
        (
            "div { -webkit-border-radius: 5px; -moz-border-radius: 5px; }",
            vec!["border-radius: 5px"],
            vec!["-webkit-", "-moz-"],
        ),
        (
            ".box { -ms-transform: rotate(45deg); }",
            vec!["transform: rotate(45deg)"],
            vec!["-ms-"],
        ),
        (
            "div { -o-transition: all 0.3s; }",
            vec!["transition: all 0.3s"],
            vec!["-o-"],
        ),
    ] {
        let output = strip_vendor_prefixes(input);
        for s in &must_contain {
            assert!(output.contains(s), "expected {s:?} in output for {input:?}");
        }
        for s in &must_not_contain {
            assert!(
                !output.contains(s),
                "unexpected {s:?} in output for {input:?}"
            );
        }
    }
}

#[test]
fn vendor_prefix_preserves_custom_properties() {
    let input = "div { --my-color: red; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("--my-color: red"));
}

#[test]
fn vendor_prefix_preserves_values() {
    let input = "div { background: -webkit-linear-gradient(top, red, blue); }";
    let output = strip_vendor_prefixes(input);
    // Value should be preserved (we only strip property-name prefixes)
    assert!(output.contains("background: -webkit-linear-gradient"));
}

#[test]
fn vendor_prefix_preserves_url_contents() {
    let input = "div { background: url(-webkit-something.png); }";
    let output = strip_vendor_prefixes(input);
    // url() contents must not be corrupted.
    assert!(
        output.contains("url(-webkit-something.png)"),
        "url() contents corrupted: {output}"
    );
}

// === Vendor prefix edge case tests ===

#[test]
fn vendor_prefix_skips_css_comments() {
    // The semicolon and vendor prefix inside the comment should not affect stripping.
    let input = "div { /* ;-webkit-foo: bar */ -webkit-border-radius: 5px; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("border-radius: 5px"));
    // Comment content preserved verbatim.
    assert!(output.contains("/* ;-webkit-foo: bar */"));
}

#[test]
fn vendor_prefix_handles_string_escapes() {
    let input = r"div { content: 'it\'s here'; -webkit-transform: none; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("transform: none"));
    assert!(output.contains(r"'it\'s here'"));
}

#[test]
fn vendor_prefix_drops_nonstandard_properties() {
    let input = "div { -webkit-font-smoothing: antialiased; color: red; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("font-smoothing"));
    assert!(!output.contains("antialiased"));
    assert!(output.contains("color: red"));
}

#[test]
fn vendor_prefix_drops_text_size_adjust() {
    let input =
        "body { -webkit-text-size-adjust: 100%; -moz-text-size-adjust: 100%; font-size: 16px; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("text-size-adjust"));
    assert!(output.contains("font-size: 16px"));
}

#[test]
fn vendor_prefix_drops_tap_highlight_color() {
    let input = "a { -webkit-tap-highlight-color: transparent; text-decoration: none; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("tap-highlight"));
    assert!(!output.contains("transparent"));
    assert!(output.contains("text-decoration: none"));
}

#[test]
fn vendor_prefix_drops_osx_font_smoothing() {
    let input = "div { -moz-osx-font-smoothing: grayscale; margin: 0; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("osx-font-smoothing"));
    assert!(output.contains("margin: 0"));
}

#[test]
fn vendor_prefix_drop_at_end_of_block() {
    // Drop property is the last declaration before `}`.
    let input = "div { color: red; -webkit-font-smoothing: antialiased }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("font-smoothing"));
    assert!(output.contains("color: red"));
}

#[test]
fn vendor_prefix_drop_only_declaration() {
    // Drop property is the only declaration in the block.
    let input = "div { -webkit-font-smoothing: antialiased; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("font-smoothing"));
    assert!(output.contains("div {"));
}

#[test]
fn presentational_ignored_values() {
    for (tag, attr_name, attr_val, prop) in [
        ("img", "width", "", "width"),
        ("img", "width", "abc", "width"),
        ("img", "width", "-10", "width"),
        ("table", "border", "0", "border-top-color"),
    ] {
        let mut attrs = Attributes::default();
        attrs.set(attr_name, attr_val);
        let (dom, el) = setup_element_with_attrs(tag, attrs);
        let hints = get_presentational_hints(el, &dom);
        assert!(
            find_decl(&hints, prop).is_none(),
            "{tag}[{attr_name}={attr_val:?}] should not generate {prop}"
        );
    }
}

#[test]
fn presentational_border_zero_emits_none() {
    let mut attrs = Attributes::default();
    attrs.set("border", "0");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    // border="0" should emit border-*-style: none and border-*-width: 0.
    let style_val = find_decl(&hints, "border-top-style");
    assert_eq!(
        style_val,
        Some(&CssValue::Keyword("none".to_string())),
        "border=0 should emit border-top-style: none"
    );
    let width_val = find_decl(&hints, "border-top-width");
    assert_eq!(
        width_val,
        Some(&CssValue::Length(0.0, LengthUnit::Px)),
        "border=0 should emit border-top-width: 0"
    );
}

#[test]
fn presentational_border_empty_defaults_to_1px() {
    let mut attrs = Attributes::default();
    attrs.set("border", "");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    // WHATWG: empty/invalid border defaults to 1px.
    let val = find_decl(&hints, "border-top-width");
    assert_eq!(val, Some(&CssValue::Length(1.0, LengthUnit::Px)));
}

#[test]
fn presentational_table_align_center_margins() {
    let mut attrs = Attributes::default();
    attrs.set("align", "center");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    // table[align=center] → margin-left/right: auto (not text-align).
    assert_eq!(find_decl(&hints, "margin-left"), Some(&CssValue::Auto));
    assert_eq!(find_decl(&hints, "margin-right"), Some(&CssValue::Auto));
    assert!(find_decl(&hints, "text-align").is_none());
}

#[test]
fn presentational_table_align_left_right() {
    // table[align=left] → float: left.
    let mut attrs = Attributes::default();
    attrs.set("align", "left");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert_eq!(
        find_decl(&hints, "float"),
        Some(&CssValue::Keyword("left".to_string()))
    );

    // table[align=right] → float: right.
    let mut attrs = Attributes::default();
    attrs.set("align", "right");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert_eq!(
        find_decl(&hints, "float"),
        Some(&CssValue::Keyword("right".to_string()))
    );
}

#[test]
#[allow(unused_must_use)]
fn presentational_cellpadding_zero_emits_padding() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let mut table_attrs = Attributes::default();
    table_attrs.set("cellpadding", "0");
    let table = elem_with_attrs(&mut dom, "table", table_attrs);
    dom.append_child(root, table);
    let tr = elem(&mut dom, "tr");
    dom.append_child(table, tr);
    let td = elem(&mut dom, "td");
    dom.append_child(tr, td);

    let hints = get_presentational_hints(td, &dom);
    // cellpadding="0" should emit padding: 0 to override UA default 1px.
    let val = find_decl(&hints, "padding-top");
    assert_eq!(val, Some(&CssValue::Length(0.0, LengthUnit::Px)));
}

// === WHATWG §15.4.3: img border handling ===

#[test]
fn presentational_img_border_zero_no_hints() {
    // WHATWG §15.4.3: <img border="0"> produces no presentational hints.
    let mut attrs = Attributes::default();
    attrs.set("border", "0");
    let (dom, el) = setup_element_with_attrs("img", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert!(
        find_decl(&hints, "border-top-style").is_none(),
        "img border=0 should produce no border hints"
    );
    assert!(find_decl(&hints, "border-top-width").is_none());
}

#[test]
fn presentational_img_border_empty_no_hints() {
    // WHATWG §15.4.3: <img border=""> produces no hints.
    let mut attrs = Attributes::default();
    attrs.set("border", "");
    let (dom, el) = setup_element_with_attrs("img", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert!(find_decl(&hints, "border-top-style").is_none());
}

#[test]
fn presentational_img_border_positive_emits_solid() {
    let mut attrs = Attributes::default();
    attrs.set("border", "2");
    let (dom, el) = setup_element_with_attrs("img", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert_eq!(
        find_decl(&hints, "border-top-width"),
        Some(&CssValue::Length(2.0, LengthUnit::Px))
    );
    assert_eq!(
        find_decl(&hints, "border-top-style"),
        Some(&CssValue::Keyword("solid".to_string()))
    );
}

// === WHATWG ignoring-zero dimension tests ===

#[test]
fn presentational_table_width_zero_ignored() {
    let mut attrs = Attributes::default();
    attrs.set("width", "0");
    let (dom, el) = setup_element_with_attrs("table", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert!(
        find_decl(&hints, "width").is_none(),
        "table width=0 should be ignored per WHATWG"
    );
}

#[test]
fn presentational_td_height_zero_ignored() {
    let mut attrs = Attributes::default();
    attrs.set("height", "0");
    let (dom, el) = setup_element_with_attrs("td", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert!(
        find_decl(&hints, "height").is_none(),
        "td height=0 should be ignored per WHATWG"
    );
}

// === WHATWG §15.3.11: hr presentational hints ===

#[test]
fn presentational_hr_no_height_hint() {
    let mut attrs = Attributes::default();
    attrs.set("height", "10");
    let (dom, el) = setup_element_with_attrs("hr", attrs);
    let hints = get_presentational_hints(el, &dom);
    // WHATWG: hr maps only width, not height.
    assert!(
        find_decl(&hints, "height").is_none(),
        "hr should not generate height hint"
    );
}

#[test]
fn presentational_hr_width_hint() {
    let mut attrs = Attributes::default();
    attrs.set("width", "80%");
    let (dom, el) = setup_element_with_attrs("hr", attrs);
    let hints = get_presentational_hints(el, &dom);
    assert_eq!(
        find_decl(&hints, "width"),
        Some(&CssValue::Percentage(80.0))
    );
}

// === -webkit-box-* legacy flexbox mapping tests ===

#[test]
fn webkit_box_orient_horizontal() {
    let input = "div { -webkit-box-orient: horizontal; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-direction: row;"), "got: {output}");
    assert!(!output.contains("-webkit-box-orient"));
}

#[test]
fn webkit_box_orient_vertical() {
    let input = "div { -webkit-box-orient: vertical; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-direction: column;"), "got: {output}");
}

#[test]
fn webkit_box_direction_reverse() {
    let input = "div { -webkit-box-direction: reverse; }";
    let output = strip_vendor_prefixes(input);
    assert!(
        output.contains("flex-direction: row-reverse;"),
        "got: {output}"
    );
}

#[test]
fn webkit_box_pack_values() {
    for (val, expected) in [
        ("start", "flex-start"),
        ("end", "flex-end"),
        ("center", "center"),
        ("justify", "space-between"),
    ] {
        let input = format!("div {{ -webkit-box-pack: {val}; }}");
        let output = strip_vendor_prefixes(&input);
        assert!(
            output.contains(&format!("justify-content: {expected};")),
            "-webkit-box-pack: {val} → got: {output}"
        );
    }
}

#[test]
fn webkit_box_align_values() {
    for (val, expected) in [
        ("start", "flex-start"),
        ("end", "flex-end"),
        ("center", "center"),
        ("stretch", "stretch"),
        ("baseline", "baseline"),
    ] {
        let input = format!("div {{ -webkit-box-align: {val}; }}");
        let output = strip_vendor_prefixes(&input);
        assert!(
            output.contains(&format!("align-items: {expected};")),
            "-webkit-box-align: {val} → got: {output}"
        );
    }
}

#[test]
fn webkit_box_flex_numeric() {
    let input = "div { -webkit-box-flex: 1; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-grow: 1;"), "got: {output}");

    let input = "div { -webkit-box-flex: 2.5; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-grow: 2.5;"), "got: {output}");
}

#[test]
fn webkit_box_ordinal_group() {
    let input = "div { -webkit-box-ordinal-group: 2; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("order: 2;"), "got: {output}");
}

#[test]
fn webkit_box_lines() {
    let input = "div { -webkit-box-lines: multiple; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-wrap: wrap;"), "got: {output}");

    let input = "div { -webkit-box-lines: single; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("flex-wrap: nowrap;"), "got: {output}");
}

#[test]
fn display_webkit_box() {
    let input = "div { display: -webkit-box; }";
    let output = strip_vendor_prefixes(input);
    assert!(
        output.contains("display: flex;"),
        "display: -webkit-box → got: {output}"
    );
    assert!(!output.contains("-webkit-box"));
}

#[test]
fn display_webkit_inline_box() {
    let input = "div { display: -webkit-inline-box; }";
    let output = strip_vendor_prefixes(input);
    assert!(
        output.contains("display: inline-flex;"),
        "display: -webkit-inline-box → got: {output}"
    );
}

#[test]
fn webkit_box_combined_declarations() {
    let input = "div { display: -webkit-box; -webkit-box-orient: vertical; -webkit-box-pack: center; color: red; }";
    let output = strip_vendor_prefixes(input);
    assert!(output.contains("display: flex;"), "display → got: {output}");
    assert!(
        output.contains("flex-direction: column;"),
        "orient → got: {output}"
    );
    assert!(
        output.contains("justify-content: center;"),
        "pack → got: {output}"
    );
    assert!(
        output.contains("color: red;"),
        "normal prop preserved → got: {output}"
    );
}

#[test]
fn webkit_box_invalid_value_dropped() {
    // Invalid values should be silently dropped (no output for that declaration).
    let input = "div { -webkit-box-orient: diagonal; color: red; }";
    let output = strip_vendor_prefixes(input);
    assert!(!output.contains("flex-direction"));
    assert!(output.contains("color: red"));
}

// === Additional presentational hints tests ===

#[test]
#[allow(unused_must_use)]
fn presentational_cellpadding_with_tbody() {
    // Real-world: table > tbody > tr > td (parsers insert implicit <tbody>)
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();

    let mut table_attrs = Attributes::default();
    table_attrs.set("cellpadding", "8");
    let table = elem_with_attrs(&mut dom, "table", table_attrs);
    dom.append_child(root, table);

    let tbody = elem(&mut dom, "tbody");
    dom.append_child(table, tbody);

    let tr = elem(&mut dom, "tr");
    dom.append_child(tbody, tr);

    let td = elem(&mut dom, "td");
    dom.append_child(tr, td);

    let hints = get_presentational_hints(td, &dom);
    let val = find_decl(&hints, "padding-top");
    assert_eq!(val, Some(&CssValue::Length(8.0, LengthUnit::Px)));
}
