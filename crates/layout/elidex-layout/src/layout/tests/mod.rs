//! Tests for the layout orchestrator.

use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{CssSize, Dimension, LayoutBox, Point, Size, TrackSection};

mod basic;
mod fragmentation;

fn get_layout(dom: &EcsDom, entity: Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

fn build_styled_dom() -> (EcsDom, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(root, html);
    dom.append_child(html, body);

    dom.world_mut().insert_one(
        html,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        body,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(8.0),
            margin_right: Dimension::Length(8.0),
            margin_bottom: Dimension::Length(8.0),
            margin_left: Dimension::Length(8.0),
            ..Default::default()
        },
    );

    (dom, root, html, body)
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}
