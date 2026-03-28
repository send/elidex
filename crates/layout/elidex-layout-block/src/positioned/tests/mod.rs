use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{Direction, LayoutBox, Point, Size};

mod absolute_fixed;
mod resolve_relative;
mod writing_mode;

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON * 100.0
}

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn set_style(dom: &mut EcsDom, entity: Entity, pos: Position) {
    let _ = dom.world_mut().insert_one(
        entity,
        ComputedStyle {
            display: Display::Block,
            position: pos,
            ..Default::default()
        },
    );
}

fn make_lb(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
    LayoutBox {
        content: Rect::new(x, y, w, h),
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn make_inline_props(
    start: Option<f32>,
    size: Option<f32>,
    end: Option<f32>,
    margin_start_raw: f32,
    margin_end_raw: f32,
    margin_start_auto: bool,
    margin_end_auto: bool,
    pb: f32,
    containing: f32,
    static_offset: f32,
) -> InlineAxisProps {
    InlineAxisProps {
        start,
        end,
        size,
        margin_start_raw,
        margin_end_raw,
        margin_start_auto,
        margin_end_auto,
        pb,
        containing,
        static_offset,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_block_props(
    start: Option<f32>,
    end: Option<f32>,
    size: Option<f32>,
    content_size: Option<f32>,
    margin_start_raw: f32,
    margin_end_raw: f32,
    margin_start_auto: bool,
    margin_end_auto: bool,
    pb: f32,
    containing: f32,
    static_offset: f32,
) -> BlockAxisProps {
    BlockAxisProps {
        start,
        end,
        size,
        content_size,
        margin_start_raw,
        margin_end_raw,
        margin_start_auto,
        margin_end_auto,
        pb,
        containing,
        static_offset,
    }
}

fn setup_block_with_abs() -> (EcsDom, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let normal = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, normal);
    dom.append_child(root, abs_child);

    // Root: relative positioned, 800x600
    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    // Normal child: 800x100
    let _ = dom.world_mut().insert_one(
        normal,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    (dom, root, normal, abs_child)
}

fn font_db() -> elidex_text::FontDatabase {
    elidex_text::FontDatabase::new()
}
