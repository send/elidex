use std::sync::Arc;

use super::*;
use elidex_ecs::{Attributes, ImageData};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, Direction};
use elidex_text::FontDatabase;

mod height_replaced;
mod margin_collapse;
mod width;

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
