//! Tests for the layout orchestrator.

use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{CssSize, Dimension, LayoutBox, Point, Size, TrackSection};

mod basic;
mod flex_assignment;
mod fragmentation;
mod grid_alignment;
mod provenance;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct DispatchObservation {
    pub(super) entity: Entity,
    pub(super) formatter: &'static str,
    pub(super) incoming_width: f32,
    pub(super) effective_width: f32,
    pub(super) shrink_to_fit: bool,
}

thread_local! {
    pub(super) static TRACK_CHILD: std::cell::Cell<Option<Entity>> = const { std::cell::Cell::new(None) };
    pub(super) static TRACK_WIDTH: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
    pub(super) static FIRST_FORMATTER: std::cell::Cell<Option<DispatchObservation>> = const { std::cell::Cell::new(None) };
}

pub(super) fn record_flex_dispatch(
    entity: Entity,
    input: &LayoutInput<'_>,
    effective: &LayoutInput<'_>,
    formatter: &'static str,
) {
    if input.depth == 0 {
        FIRST_FORMATTER.set(Some(DispatchObservation {
            entity,
            formatter,
            incoming_width: input.containing.width,
            effective_width: effective.containing.width,
            shrink_to_fit: !std::ptr::eq(input, effective),
        }));
    }
}

fn get_layout(dom: &EcsDom, entity: Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

fn build_styled_dom() -> (EcsDom, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    let body = dom.create_element_ownerless_fixture("body", Attributes::default());
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
