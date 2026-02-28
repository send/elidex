//! Tree-level layout entry point.
//!
//! Walks the DOM tree and assigns [`LayoutBox`] components to each element.
//! The public API is [`layout_tree`], which takes a styled DOM and produces
//! layout boxes for the entire document.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, Display};
use elidex_text::FontDatabase;

use crate::block::{layout_block, stack_block_children};

/// Layout the entire DOM tree.
///
/// Each element that participates in layout receives a [`LayoutBox`](elidex_plugin::LayoutBox) ECS
/// component. Elements with `display: none` are skipped entirely.
///
/// # Prerequisites
///
/// `elidex_style::resolve_styles()` must have been called first so that
/// every element has a [`ComputedStyle`] component.
// TODO(Phase 2): use viewport_height for vh units and root percentage heights.
pub fn layout_tree(
    dom: &mut EcsDom,
    viewport_width: f32,
    _viewport_height: f32,
    font_db: &FontDatabase,
) {
    let roots = find_roots(dom);
    for root in roots {
        layout_root(dom, root, viewport_width, font_db);
    }
}

/// Find root entities for layout: parentless entities with styles or children.
fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    dom.root_entities()
        .into_iter()
        .filter(|&e| {
            dom.world().get::<&ComputedStyle>(e).is_ok() || dom.get_first_child(e).is_some()
        })
        .collect()
}

/// Layout starting from a root entity.
///
/// If the root has a `ComputedStyle` (is an element), layout it directly.
/// Otherwise (document root), layout its children as block-level elements.
fn layout_root(dom: &mut EcsDom, root: Entity, viewport_width: f32, font_db: &FontDatabase) {
    let root_display = dom
        .world()
        .get::<&ComputedStyle>(root)
        .map(|s| s.display)
        .ok();

    if let Some(display) = root_display {
        if display == Display::None {
            return;
        }
        if matches!(display, Display::Flex | Display::InlineFlex) {
            crate::flex::layout_flex(dom, root, viewport_width, 0.0, 0.0, font_db, 0);
        } else {
            layout_block(dom, root, viewport_width, 0.0, 0.0, font_db);
        }
        return;
    }

    // Document root: layout children as top-level blocks with margin collapse.
    let children = dom.children(root);
    stack_block_children(dom, &children, viewport_width, 0.0, 0.0, font_db, 0);
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{Dimension, LayoutBox};

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

    #[test]
    fn layout_tree_assigns_layout_box() {
        let (mut dom, _root, html, body) = build_styled_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);
        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        // All elements should have LayoutBox
        assert!(dom.world().get::<&LayoutBox>(html).is_ok());
        assert!(dom.world().get::<&LayoutBox>(body).is_ok());
        assert!(dom.world().get::<&LayoutBox>(div).is_ok());
    }

    #[test]
    fn nested_divs_position() {
        let (mut dom, _root, _html, body) = build_styled_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);
        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        let div_lb = get_layout(&dom, div);
        // div is inside body which has margin: 8px
        // body content_x = 0 + 8 + 0 + 0 = 8
        // div content_x = 8 (inherits body's content offset)
        assert!((div_lb.content.x - 8.0).abs() < f32::EPSILON);
        assert!((div_lb.content.y - 8.0).abs() < f32::EPSILON);
        // div width = body content width = 800 - 8 - 8 = 784
        assert!((div_lb.content.width - 784.0).abs() < f32::EPSILON);
    }

    #[test]
    fn body_margin_reflected() {
        let (mut dom, _root, _html, body) = build_styled_dom();
        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 1024.0, 768.0, &font_db);

        let body_lb = get_layout(&dom, body);
        assert!((body_lb.margin.top - 8.0).abs() < f32::EPSILON);
        assert!((body_lb.margin.left - 8.0).abs() < f32::EPSILON);
        assert!((body_lb.content.x - 8.0).abs() < f32::EPSILON);
        assert!((body_lb.content.y - 8.0).abs() < f32::EPSILON);
        // body content width = 1024 - 8 - 8 = 1008
        assert!((body_lb.content.width - 1008.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mixed_block_text_content() {
        let (mut dom, _root, _html, body) = build_styled_dom();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("Hello world");
        dom.append_child(body, div);
        dom.append_child(body, text);

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        let body_lb = get_layout(&dom, body);
        // Block context (div is block). Text node skipped in block context.
        // Body height = div height (50) only.
        assert!((body_lb.content.height - 50.0).abs() < f32::EPSILON);
    }
}
