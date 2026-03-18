use super::*;
use elidex_ecs::Attributes;
use elidex_layout_block::{layout_block_only, LayoutInput};
use elidex_text::FontDatabase;

mod alignment_gap;
mod direction;
mod grow_shrink;

/// Helper to call `layout_flex` with the old positional-argument pattern used by tests.
#[allow(clippy::too_many_arguments)]
fn do_layout_flex(
    dom: &mut EcsDom,
    entity: Entity,
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
        offset_x,
        offset_y,
        font_db,
        depth,
        float_ctx: None,
        viewport: None,
    };
    layout_flex(dom, entity, &input, layout_child)
}

fn flex_container() -> ComputedStyle {
    ComputedStyle {
        display: Display::Flex,
        ..Default::default()
    }
}

fn flex_item(width: f32, height: f32) -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(width),
        height: Dimension::Length(height),
        ..Default::default()
    }
}

fn make_flex_dom(
    container_style: ComputedStyle,
    items: &[ComputedStyle],
) -> (EcsDom, Entity, Vec<Entity>) {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(container, container_style);

    let mut entities = Vec::new();
    for item_style in items {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(container, child);
        dom.world_mut().insert_one(child, item_style.clone());
        entities.push(child);
    }
    (dom, container, entities)
}

fn get_lb(dom: &EcsDom, entity: Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}
