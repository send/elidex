//! Tree-level layout entry point.
//!
//! Walks the DOM tree and assigns [`LayoutBox`] components to each element.
//! The public API is [`layout_tree`], which takes a styled DOM and produces
//! layout boxes for the entire document.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::block::stack_block_children;
use elidex_layout_block::positioned;
use elidex_layout_block::LayoutInput;
use elidex_plugin::{ComputedStyle, Display, LayoutBox, Position};
use elidex_text::FontDatabase;

/// Dispatch child layout based on the element's display type.
///
/// This is the [`ChildLayoutFn`](elidex_layout_block::ChildLayoutFn) provided
/// to all layout algorithms, routing flex/grid containers to their respective
/// crates and everything else to block layout.
pub fn dispatch_layout_child(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> LayoutBox {
    let style = elidex_layout_block::get_style(dom, entity);
    let lb = match style.display {
        // display: contents — element generates no box (CSS Display Level 3 §2.8).
        // Children are promoted to the parent's formatting context via
        // flatten_contents(). Return a zero-size box at the given position.
        Display::Contents => LayoutBox {
            content: elidex_plugin::Rect::new(input.offset_x, input.offset_y, 0.0, 0.0),
            padding: elidex_plugin::EdgeSizes::default(),
            border: elidex_plugin::EdgeSizes::default(),
            margin: elidex_plugin::EdgeSizes::default(),
        },
        Display::Flex | Display::InlineFlex => {
            elidex_layout_flex::layout_flex(dom, entity, input, dispatch_layout_child)
        }
        Display::Grid | Display::InlineGrid => {
            elidex_layout_grid::layout_grid(dom, entity, input, dispatch_layout_child)
        }
        Display::Table | Display::InlineTable => {
            elidex_layout_table::layout_table(dom, entity, input, dispatch_layout_child)
        }
        _ => elidex_layout_block::block::layout_block_inner(
            dom,
            entity,
            input,
            dispatch_layout_child,
        ),
    };

    // CSS 2.1 §9.4.3: relative offset.
    // Return the original LayoutBox (without offset) so siblings use the
    // unshifted space. The ECS LayoutBox is updated with the offset.
    if style.position == Position::Relative {
        let mut offset_lb = lb.clone();
        positioned::apply_relative_offset(
            &mut offset_lb,
            &style,
            input.containing_width,
            input.containing_height,
        );
        let dx = offset_lb.content.x - lb.content.x;
        let dy = offset_lb.content.y - lb.content.y;
        let _ = dom.world_mut().insert_one(entity, offset_lb);
        if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
            let children: Vec<_> = dom.children_iter(entity).collect();
            elidex_layout_block::block::shift_descendants(dom, &children, dx, dy);
        }
    }

    lb
}

/// Layout the entire DOM tree.
///
/// Each element that participates in layout receives a [`LayoutBox`] ECS
/// component. Elements with `display: none` are skipped entirely.
///
/// # Prerequisites
///
/// `elidex_style::resolve_styles()` must have been called first so that
/// every element has a [`ComputedStyle`] component.
pub fn layout_tree(
    dom: &mut EcsDom,
    viewport_width: f32,
    viewport_height: f32,
    font_db: &FontDatabase,
) {
    let roots = find_roots(dom);
    for root in roots {
        layout_root(dom, root, viewport_width, viewport_height, font_db);
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
/// If the root has a `ComputedStyle` (is an element), layout it directly
/// via the display-type dispatcher. Otherwise (document root), layout its
/// children as block-level elements.
fn layout_root(
    dom: &mut EcsDom,
    root: Entity,
    viewport_width: f32,
    viewport_height: f32,
    font_db: &FontDatabase,
) {
    let root_display = dom
        .world()
        .get::<&ComputedStyle>(root)
        .map(|s| s.display)
        .ok();

    let root_input = LayoutInput {
        containing_width: viewport_width,
        containing_height: Some(viewport_height),
        offset_x: 0.0,
        offset_y: 0.0,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some((viewport_width, viewport_height)),
    };

    if let Some(display) = root_display {
        if display == Display::None {
            return;
        }
        if display == Display::Contents {
            // display: contents at root — skip box, layout children directly.
            let children = elidex_layout_block::composed_children_flat(dom, root);
            // Root-level always establishes a BFC.
            let _ = stack_block_children(
                dom,
                &children,
                &root_input,
                dispatch_layout_child,
                true,
                root,
            );
            return;
        }
        dispatch_layout_child(dom, root, &root_input);
        return;
    }

    // Document root: layout children as top-level blocks with margin collapse.
    // Root always establishes a BFC.
    let children = elidex_layout_block::composed_children_flat(dom, root);
    let _ = stack_block_children(
        dom,
        &children,
        &root_input,
        dispatch_layout_child,
        true,
        root,
    );
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
        // Block context (div is block). Text node is wrapped in an anonymous
        // block box (CSS 2.1 §9.2.1.1). Body height ≥ div height (50).
        assert!(
            body_lb.content.height >= 50.0,
            "body height should be at least div height (50), got {}",
            body_lb.content.height
        );
    }

    #[test]
    fn grid_container_dispatches() {
        let mut dom = EcsDom::new();
        let container = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                ..Default::default()
            },
        );
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(container, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        // Grid container should have a LayoutBox assigned
        assert!(dom.world().get::<&LayoutBox>(container).is_ok());
        assert!(dom.world().get::<&LayoutBox>(child).is_ok());
    }

    // --- M3.5-1: Grid integration tests ---

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.5
    }

    #[test]
    fn grid_nested_in_block() {
        // A grid container inside a block container (body with margins).
        let (mut dom, _root, _html, body) = build_styled_dom();
        let grid = dom.create_element("div", Attributes::default());
        dom.append_child(body, grid);
        dom.world_mut().insert_one(
            grid,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: elidex_plugin::GridTrackList::Explicit(vec![
                    elidex_plugin::TrackSize::Fr(1.0),
                    elidex_plugin::TrackSize::Fr(1.0),
                ]),
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        dom.append_child(grid, c1);
        dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        let c2 = dom.create_element("div", Attributes::default());
        dom.append_child(grid, c2);
        dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        let grid_lb = get_layout(&dom, grid);
        let c1_lb = get_layout(&dom, c1);
        let c2_lb = get_layout(&dom, c2);

        // Grid is inside body (margin: 8px), so content starts at x=8.
        assert!(approx_eq(grid_lb.content.x, 8.0));
        // Grid content width = 800 - 16 = 784.
        assert!(approx_eq(grid_lb.content.width, 784.0));
        // Each column = 784 / 2 = 392.
        assert!(approx_eq(c1_lb.content.width, 392.0));
        assert!(approx_eq(c2_lb.content.width, 392.0));
        // c2 starts at x = 8 + 392.
        assert!(approx_eq(c2_lb.content.x, 400.0));
    }

    #[test]
    fn grid_item_is_flex_container() {
        // A grid item that is itself a flex container.
        let mut dom = EcsDom::new();
        let grid = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            grid,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: elidex_plugin::GridTrackList::Explicit(vec![
                    elidex_plugin::TrackSize::Length(200.0),
                ]),
                ..Default::default()
            },
        );
        let flex_item = dom.create_element("div", Attributes::default());
        dom.append_child(grid, flex_item);
        dom.world_mut().insert_one(
            flex_item,
            ComputedStyle {
                display: Display::Flex,
                ..Default::default()
            },
        );
        let inner = dom.create_element("div", Attributes::default());
        dom.append_child(flex_item, inner);
        dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        // Grid item should have LayoutBox.
        let flex_lb = get_layout(&dom, flex_item);
        let inner_lb = get_layout(&dom, inner);

        // Grid item gets 200px width from grid.
        assert!(approx_eq(flex_lb.content.width, 200.0));
        // Inner flex item: width 50px, height 30px.
        assert!(approx_eq(inner_lb.content.width, 50.0));
        assert!(approx_eq(inner_lb.content.height, 30.0));
    }

    // --- M3.5-2: Table integration tests ---

    #[test]
    fn table_dispatches_to_table_layout() {
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );

        let tr = dom.create_element("tr", Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);

        let td = dom.create_element("td", Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 600.0, 400.0, &font_db);

        assert!(dom.world().get::<&LayoutBox>(table).is_ok());
        assert!(dom.world().get::<&LayoutBox>(td).is_ok());
        let table_lb = get_layout(&dom, table);
        assert!(approx_eq(table_lb.content.width, 600.0));
    }

    #[test]
    fn table_nested_in_block() {
        let (mut dom, _root, _html, body) = build_styled_dom();
        let table = dom.create_element("table", Attributes::default());
        dom.append_child(body, table);
        dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );

        let tr = dom.create_element("tr", Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);

        let td = dom.create_element("td", Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(tr, td);

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        let table_lb = get_layout(&dom, table);
        // Table inside body (margin: 8px), so content starts at x=8.
        assert!(approx_eq(table_lb.content.x, 8.0));
        // Table content width = 800 - 16 = 784.
        assert!(approx_eq(table_lb.content.width, 784.0));
    }

    #[test]
    fn table_is_block_level() {
        assert!(elidex_layout_block::block::is_block_level(Display::Table));
        // InlineTable is an atomic inline-level box, not block-level.
        assert!(!elidex_layout_block::block::is_block_level(
            Display::InlineTable
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableCaption
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableRow
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableCell
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableRowGroup
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableHeaderGroup
        ));
        assert!(elidex_layout_block::block::is_block_level(
            Display::TableFooterGroup
        ));
    }

    #[test]
    fn table_item_is_flex_container() {
        // A table cell that contains a flex container.
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );

        let tr = dom.create_element("tr", Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);

        let td = dom.create_element("td", Attributes::default());
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                ..Default::default()
            },
        );
        dom.append_child(tr, td);

        let flex = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                ..Default::default()
            },
        );
        dom.append_child(td, flex);

        let inner = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        dom.append_child(flex, inner);

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 600.0, 400.0, &font_db);

        assert!(dom.world().get::<&LayoutBox>(table).is_ok());
        assert!(dom.world().get::<&LayoutBox>(flex).is_ok());
        assert!(dom.world().get::<&LayoutBox>(inner).is_ok());
        let inner_lb = get_layout(&dom, inner);
        // Flex item height should be preserved.
        assert!(approx_eq(inner_lb.content.height, 30.0));
    }

    #[test]
    fn inline_grid_dispatches_to_grid() {
        // inline-grid should be treated as block-level grid.
        let mut dom = EcsDom::new();
        let container = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            container,
            ComputedStyle {
                display: Display::InlineGrid,
                grid_template_columns: elidex_plugin::GridTrackList::Explicit(vec![
                    elidex_plugin::TrackSize::Fr(1.0),
                    elidex_plugin::TrackSize::Fr(1.0),
                ]),
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        dom.append_child(container, c1);
        dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 600.0, 600.0, &font_db);

        let container_lb = get_layout(&dom, container);
        let c1_lb = get_layout(&dom, c1);

        // Should be laid out as grid (2 columns of 300px each).
        assert!(approx_eq(container_lb.content.width, 600.0));
        assert!(approx_eq(c1_lb.content.width, 300.0));
    }

    #[test]
    fn viewport_height_enables_root_percentage_height() {
        // Root element with height: 50% should resolve against viewport height.
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(
            root,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Percentage(50.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        layout_tree(&mut dom, 800.0, 600.0, &font_db);

        let lb = get_layout(&dom, root);
        // 50% of viewport height 600 = 300.
        assert!(approx_eq(lb.content.height, 300.0));
    }
}
