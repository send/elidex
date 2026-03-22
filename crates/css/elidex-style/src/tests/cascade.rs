use super::*;

#[test]
fn basic_style_resolution() {
    let (_dom, _div, style) = resolve_single("div { color: red; display: block; }");
    assert_eq!(style.color, CssColor::RED);
    assert_eq!(style.display, Display::Block);
}

#[test]
fn color_inheritance() {
    let (_dom, _div, _span, div_style, span_style) = resolve_with_child("div { color: red; }");
    assert_eq!(div_style.color, CssColor::RED);
    // span inherits color from div
    assert_eq!(span_style.color, CssColor::RED);
}

#[test]
fn font_size_inheritance() {
    let (_dom, _div, _span, _div_style, span_style) =
        resolve_with_child("div { font-size: 24px; }");
    assert_eq!(span_style.font_size, 24.0);
}

#[test]
fn inline_style_overrides_author() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: blue");
    let div = dom.create_element("div", attrs);
    dom.append_child(body, div);

    let css = "div { color: red; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, div);
    assert_eq!(style.color, CssColor::BLUE);
}

#[test]
fn author_important_overrides_inline() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("style", "color: blue");
    let div = dom.create_element("div", attrs);
    dom.append_child(body, div);

    let css = "div { color: red !important; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, div);
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn ua_stylesheet_body_margin() {
    let (mut dom, _root, _html, body) = build_simple_dom();

    resolve_styles(&mut dom, &[], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, body);
    assert_eq!(style.margin_top, Dimension::Length(8.0));
    assert_eq!(style.margin_right, Dimension::Length(8.0));
    assert_eq!(style.margin_bottom, Dimension::Length(8.0));
    assert_eq!(style.margin_left, Dimension::Length(8.0));
}

#[test]
fn inherit_keyword() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    dom.append_child(body, div);
    dom.append_child(div, span);

    // display is non-inherited; inherit keyword forces inheritance
    let css = "div { display: flex; } span { display: inherit; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let span_style = get_style(&dom, span);
    assert_eq!(span_style.display, Display::Flex);
}

#[test]
fn initial_keyword() {
    // div has UA display: block, but initial should reset to inline
    let (_dom, _div, style) = resolve_single("div { display: initial; }");
    assert_eq!(style.display, Display::Inline);
}

#[test]
fn unset_keyword() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    dom.append_child(body, div);
    dom.append_child(div, span);

    // color is inherited: unset -> inherit from parent
    // display is non-inherited: unset -> initial
    let css = "div { color: red; display: flex; } span { color: unset; display: unset; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let span_style = get_style(&dom, span);
    assert_eq!(span_style.color, CssColor::RED); // inherited
    assert_eq!(span_style.display, Display::Inline); // initial
}

#[test]
fn mixed_ua_author_inline() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let mut attrs = Attributes::default();
    attrs.set("style", "margin-left: 20px");
    let div = dom.create_element("div", attrs);
    dom.append_child(body, div);

    let css = "div { margin-top: 10px; }";
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, div);
    assert_eq!(style.display, Display::Block); // UA
    assert_eq!(style.margin_top, Dimension::Length(10.0)); // Author
    assert_eq!(style.margin_left, Dimension::Length(20.0)); // Inline
}

#[test]
fn empty_element_non_inherited_initial() {
    let (_dom, _div, _span, div_style, span_style) =
        resolve_with_child("div { background-color: red; }");
    // background-color is non-inherited
    assert_eq!(div_style.background_color, CssColor::RED);
    assert_eq!(span_style.background_color, CssColor::TRANSPARENT); // initial
}

#[test]
fn border_style_width_interaction() {
    let (_dom, _div, style) =
        resolve_single("div { border-top-style: solid; border-top-width: 5px; }");
    assert_eq!(style.border_top.style, BorderStyle::Solid);
    assert_eq!(style.border_top.width, 5.0);
    // Other sides have style: none -> width should be 0
    assert_eq!(style.border_right.width, 0.0);
}

#[test]
fn currentcolor_border() {
    let (_dom, _div, style) = resolve_single("div { color: red; }");
    // border-*-color initial = currentcolor -> element's color
    assert_eq!(style.border_top.color, CssColor::RED);
}

#[test]
fn currentcolor_background() {
    let (_dom, _div, style) =
        resolve_single("div { color: blue; background-color: currentcolor; }");
    assert_eq!(style.background_color, CssColor::BLUE);
}

#[test]
fn head_display_none() {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(root, html);
    dom.append_child(html, head);
    dom.append_child(html, body);

    resolve_styles(&mut dom, &[], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, head);
    assert_eq!(style.display, Display::None);
}

#[test]
fn position_property() {
    let (_dom, _div, style) = resolve_single("div { position: fixed; }");
    assert_eq!(style.position, Position::Fixed);
}

#[test]
fn rem_uses_html_font_size() {
    let (_dom, _div, style) = resolve_single("html { font-size: 20px; } div { width: 2rem; }");
    assert_eq!(style.width, Dimension::Length(40.0));
}

#[test]
fn multiple_author_stylesheets_ordering() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);

    // ss1 has extra rules so its source_order for div{color:red} > ss2's source_order.
    // Later stylesheet (ss2) must still win.
    let ss1 = parse_stylesheet("p { display: block; } div { color: red; }", Origin::Author);
    let ss2 = parse_stylesheet("div { color: blue; }", Origin::Author);
    resolve_styles(&mut dom, &[&ss1, &ss2], Size::new(1920.0, 1080.0));

    let style = get_style(&dom, div);
    // Later stylesheet wins at same specificity.
    assert_eq!(style.color, CssColor::BLUE);
}
