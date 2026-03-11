#![allow(unused_must_use)]

use super::*;
use elidex_css::{parse_stylesheet, Declaration, Origin};
use elidex_ecs::{Attributes, ElementState, PseudoElementMarker, ShadowRootMode, TextContent};
use elidex_plugin::{BorderStyle, CssColor, CssValue, Dimension, Display, Position};

mod cascade;
mod properties;
mod selectors_pseudo;
mod shadow_compat;

fn build_simple_dom() -> (EcsDom, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(root, html);
    dom.append_child(html, body);
    (dom, root, html, body)
}

fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
    let r = dom
        .world()
        .get::<&ComputedStyle>(entity)
        .expect("ComputedStyle not found");
    (*r).clone()
}

/// Resolve styles for a single `div` child of `body`.
fn resolve_single(css: &str) -> (EcsDom, Entity, ComputedStyle) {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);
    let style = get_style(&dom, div);
    (dom, div, style)
}

/// Resolve styles with a `div` > `span` hierarchy.
fn resolve_with_child(css: &str) -> (EcsDom, Entity, Entity, ComputedStyle, ComputedStyle) {
    let (mut dom, _root, _html, body) = build_simple_dom();
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    dom.append_child(body, div);
    dom.append_child(div, span);
    let ss = parse_stylesheet(css, Origin::Author);
    resolve_styles(&mut dom, &[&ss], 1920.0, 1080.0);
    let div_style = get_style(&dom, div);
    let span_style = get_style(&dom, span);
    (dom, div, span, div_style, span_style)
}

/// Helper: create shadow tree with <style> text and return shadow root.
fn setup_shadow_with_style(dom: &mut EcsDom, host: Entity, css: &str) -> Entity {
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let style_el = dom.create_element("style", Attributes::default());
    let style_text = dom.create_text(css);
    dom.append_child(sr, style_el);
    dom.append_child(style_el, style_text);
    sr
}
