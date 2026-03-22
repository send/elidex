//! Tests for root overflow propagation (CSS Overflow L3 §3.1).

use super::*;

#[test]
fn root_propagation_html_hidden() {
    let (mut dom, _root, html, _body) = build_simple_dom();
    let ss = parse_stylesheet("html { overflow: hidden; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // html's overflow should be propagated to viewport and reset to visible.
    assert_eq!(vp.overflow_x, Overflow::Hidden);
    assert_eq!(vp.overflow_y, Overflow::Hidden);
    let html_style = get_style(&dom, html);
    assert_eq!(html_style.overflow_x, Overflow::Visible);
    assert_eq!(html_style.overflow_y, Overflow::Visible);
}

#[test]
fn root_propagation_body_scroll() {
    let (mut dom, _root, html, body) = build_simple_dom();
    let ss = parse_stylesheet("body { overflow: scroll; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // html is visible → check body. body's overflow propagated to viewport.
    assert_eq!(vp.overflow_x, Overflow::Scroll);
    assert_eq!(vp.overflow_y, Overflow::Scroll);
    let html_style = get_style(&dom, html);
    assert_eq!(html_style.overflow_x, Overflow::Visible);
    assert_eq!(html_style.overflow_y, Overflow::Visible);
    let body_style = get_style(&dom, body);
    assert_eq!(body_style.overflow_x, Overflow::Visible);
    assert_eq!(body_style.overflow_y, Overflow::Visible);
}

#[test]
fn root_propagation_both_visible() {
    let (mut dom, _root, _html, _body) = build_simple_dom();
    let ss = parse_stylesheet("div { overflow: hidden; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // Both html and body are visible → default auto/auto.
    assert_eq!(vp.overflow_x, Overflow::Auto);
    assert_eq!(vp.overflow_y, Overflow::Auto);
}

#[test]
fn root_propagation_html_takes_priority() {
    let (mut dom, _root, html, body) = build_simple_dom();
    let ss = parse_stylesheet(
        "html { overflow: hidden; } body { overflow: scroll; }",
        Origin::Author,
    );
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // html has non-visible overflow → html takes priority, body is untouched.
    assert_eq!(vp.overflow_x, Overflow::Hidden);
    assert_eq!(vp.overflow_y, Overflow::Hidden);
    let html_style = get_style(&dom, html);
    assert_eq!(html_style.overflow_x, Overflow::Visible);
    let body_style = get_style(&dom, body);
    assert_eq!(body_style.overflow_x, Overflow::Scroll);
}

#[test]
fn root_propagation_per_axis() {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let ss = parse_stylesheet(
        "body { overflow-x: hidden; overflow-y: scroll; }",
        Origin::Author,
    );
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    assert_eq!(vp.overflow_x, Overflow::Hidden);
    assert_eq!(vp.overflow_y, Overflow::Scroll);
    let body_style = get_style(&dom, body);
    assert_eq!(body_style.overflow_x, Overflow::Visible);
    assert_eq!(body_style.overflow_y, Overflow::Visible);
}

#[test]
fn root_propagation_body_clip_becomes_hidden() {
    let (mut dom, _root, _html, _body) = build_simple_dom();
    let ss = parse_stylesheet("body { overflow: clip; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // clip → hidden on viewport.
    assert_eq!(vp.overflow_x, Overflow::Hidden);
    assert_eq!(vp.overflow_y, Overflow::Hidden);
}

#[test]
fn root_propagation_no_html() {
    // Fragment with no <html> element.
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(root, div);
    let ss = parse_stylesheet("div { overflow: hidden; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    assert_eq!(vp.overflow_x, Overflow::Auto);
    assert_eq!(vp.overflow_y, Overflow::Auto);
}

#[test]
fn root_propagation_body_display_none() {
    let (mut dom, _root, _html, _body) = build_simple_dom();
    let ss = parse_stylesheet("body { display: none; overflow: scroll; }", Origin::Author);
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // CSS Overflow L3 §3.1: display:none body is not a valid propagation
    // source — fall back to default auto/auto.
    assert_eq!(vp.overflow_x, Overflow::Auto);
    assert_eq!(vp.overflow_y, Overflow::Auto);
}

#[test]
fn root_propagation_html_display_none() {
    let (mut dom, _root, _html, _body) = build_simple_dom();
    let ss = parse_stylesheet(
        "html { display: none; overflow: hidden; } body { overflow: scroll; }",
        Origin::Author,
    );
    let vp = resolve_styles(&mut dom, &[&ss], Size::new(1920.0, 1080.0));
    // html display:none → no propagation at all, even from body.
    assert_eq!(vp.overflow_x, Overflow::Auto);
    assert_eq!(vp.overflow_y, Overflow::Auto);
}
