use std::sync::Arc;

use super::*;
use elidex_ecs::{Attributes, ImageData};
use elidex_plugin::{
    BorderSide, BorderStyle, BoxSizing, Clear, ComputedStyle, Dimension, Direction, EdgeSizes,
    Float,
};
use elidex_text::FontDatabase;

mod float_layout;
mod fragmentation;
mod height_replaced;
mod margin_collapse;
mod width;
mod writing_mode;

fn block_style() -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        ..Default::default()
    }
}

fn make_dom_with_block_div(style: ComputedStyle) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(div, style);
    (dom, div)
}

/// Create a floated child element and append it to the parent.
fn make_float_child(dom: &mut EcsDom, parent: Entity, float: Float, w: f32, h: f32) -> Entity {
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            float,
            width: Dimension::Length(w),
            height: Dimension::Length(h),
            ..Default::default()
        },
    );
    child
}

fn make_dom_with_image(style: ComputedStyle, img_w: u32, img_h: u32) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let img = dom.create_element("img", Attributes::default());
    dom.world_mut().insert_one(img, style);
    dom.world_mut().insert_one(
        img,
        ImageData {
            pixels: Arc::new(vec![0u8; (img_w * img_h * 4) as usize]),
            width: img_w,
            height: img_h,
        },
    );
    (dom, img)
}
