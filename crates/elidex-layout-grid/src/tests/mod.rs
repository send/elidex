//! Grid layout tests.

use elidex_ecs::{Attributes, EcsDom};
use elidex_layout_block::layout_block_only;
use elidex_plugin::{
    AlignItems, ComputedStyle, Dimension, Display, GridAutoFlow, GridLine, LayoutBox, TrackBreadth,
    TrackSize,
};
use elidex_text::FontDatabase;

use crate::layout_grid;

mod alignment_box;
mod placement;
mod track_sizing;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_layout(dom: &EcsDom, entity: elidex_ecs::Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

fn make_grid_child(
    dom: &mut EcsDom,
    parent: elidex_ecs::Entity,
    height: f32,
) -> elidex_ecs::Entity {
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(height),
                ..Default::default()
            },
        )
        .unwrap();
    child
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

// (description, tracks, container_width, expected (x, width) per child)
type TrackSizingCase = (
    &'static str,
    &'static [TrackSize],
    f32,
    &'static [(f32, f32)],
);
