//! Grid layout tests.

use elidex_ecs::{Attributes, EcsDom};
use elidex_layout_block::{layout_block_only, LayoutInput};
use elidex_plugin::{
    AlignItems, ComputedStyle, Dimension, Display, GridAutoFlow, GridLine, GridTrackList,
    LayoutBox, TrackBreadth, TrackSection, TrackSize,
};
use elidex_text::FontDatabase;

use crate::layout_grid;

/// Helper to call `layout_grid` with the old positional-argument pattern used by tests.
#[allow(clippy::too_many_arguments)]
fn do_layout_grid(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: elidex_layout_block::ChildLayoutFn,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height,
        containing_inline_size: containing_width,
        offset_x,
        offset_y,
        font_db,
        depth,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    layout_grid(dom, entity, &input, layout_child)
}

mod alignment_box;
mod baseline;
mod blockification;
mod placement;
mod subgrid;
mod track_sizing;
mod writing_mode;

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
