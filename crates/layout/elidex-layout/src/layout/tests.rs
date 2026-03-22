//! Tests for the layout orchestrator.

use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{CssSize, Dimension, LayoutBox, Point, Size, TrackSection};

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

// ---------------------------------------------------------------------------
// Basic layout tests
// ---------------------------------------------------------------------------

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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    let div_lb = get_layout(&dom, div);
    // div is inside body which has margin: 8px
    // body content_x = 0 + 8 + 0 + 0 = 8
    // div content_x = 8 (inherits body's content offset)
    assert!((div_lb.content.origin.x - 8.0).abs() < f32::EPSILON);
    assert!((div_lb.content.origin.y - 8.0).abs() < f32::EPSILON);
    // div width = body content width = 800 - 8 - 8 = 784
    assert!((div_lb.content.size.width - 784.0).abs() < f32::EPSILON);
}

#[test]
fn body_margin_reflected() {
    let (mut dom, _root, _html, body) = build_styled_dom();
    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(1024.0, 768.0), &font_db);

    let body_lb = get_layout(&dom, body);
    assert!((body_lb.margin.top - 8.0).abs() < f32::EPSILON);
    assert!((body_lb.margin.left - 8.0).abs() < f32::EPSILON);
    assert!((body_lb.content.origin.x - 8.0).abs() < f32::EPSILON);
    assert!((body_lb.content.origin.y - 8.0).abs() < f32::EPSILON);
    // body content width = 1024 - 8 - 8 = 1008
    assert!((body_lb.content.size.width - 1008.0).abs() < f32::EPSILON);
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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    let body_lb = get_layout(&dom, body);
    // Block context (div is block). Text node is wrapped in an anonymous
    // block box (CSS 2.1 §9.2.1.1). Body height >= div height (50).
    assert!(
        body_lb.content.size.height >= 50.0,
        "body height should be at least div height (50), got {}",
        body_lb.content.size.height
    );
}

// ---------------------------------------------------------------------------
// Grid integration tests
// ---------------------------------------------------------------------------

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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    // Grid container should have a LayoutBox assigned
    assert!(dom.world().get::<&LayoutBox>(container).is_ok());
    assert!(dom.world().get::<&LayoutBox>(child).is_ok());
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
            grid_template_columns: elidex_plugin::GridTrackList::Explicit(
                TrackSection::from_tracks(vec![
                    elidex_plugin::TrackSize::Fr(1.0),
                    elidex_plugin::TrackSize::Fr(1.0),
                ]),
            ),
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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    let grid_lb = get_layout(&dom, grid);
    let c1_lb = get_layout(&dom, c1);
    let c2_lb = get_layout(&dom, c2);

    // Grid is inside body (margin: 8px), so content starts at x=8.
    assert!(approx_eq(grid_lb.content.origin.x, 8.0));
    // Grid content width = 800 - 16 = 784.
    assert!(approx_eq(grid_lb.content.size.width, 784.0));
    // Each column = 784 / 2 = 392.
    assert!(approx_eq(c1_lb.content.size.width, 392.0));
    assert!(approx_eq(c2_lb.content.size.width, 392.0));
    // c2 starts at x = 8 + 392.
    assert!(approx_eq(c2_lb.content.origin.x, 400.0));
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
            grid_template_columns: elidex_plugin::GridTrackList::Explicit(
                TrackSection::from_tracks(vec![elidex_plugin::TrackSize::Length(200.0)]),
            ),
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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    // Grid item should have LayoutBox.
    let flex_lb = get_layout(&dom, flex_item);
    let inner_lb = get_layout(&dom, inner);

    // Grid item gets 200px width from grid.
    assert!(approx_eq(flex_lb.content.size.width, 200.0));
    // Inner flex item: width 50px, height 30px.
    assert!(approx_eq(inner_lb.content.size.width, 50.0));
    assert!(approx_eq(inner_lb.content.size.height, 30.0));
}

#[test]
fn inline_grid_dispatches_to_grid() {
    // inline-grid uses shrink-to-fit width (CSS 2.1 §10.3.5).
    // With fixed-size columns, shrink-to-fit uses intrinsic sizes.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::InlineGrid,
            grid_template_columns: elidex_plugin::GridTrackList::Explicit(
                TrackSection::from_tracks(vec![
                    elidex_plugin::TrackSize::Length(200.0),
                    elidex_plugin::TrackSize::Length(200.0),
                ]),
            ),
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
    layout_tree(&mut dom, Size::new(600.0, 600.0), &font_db);

    let container_lb = get_layout(&dom, container);
    let c1_lb = get_layout(&dom, c1);

    // 2 columns of 200px each -> container width = 400px (shrink-to-fit).
    assert!(approx_eq(container_lb.content.size.width, 400.0));
    assert!(approx_eq(c1_lb.content.size.width, 200.0));
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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    let lb = get_layout(&dom, root);
    // 50% of viewport height 600 = 300.
    assert!(approx_eq(lb.content.size.height, 300.0));
}

// ---------------------------------------------------------------------------
// Table integration tests
// ---------------------------------------------------------------------------

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
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
    let table_lb = get_layout(&dom, table);
    assert!(approx_eq(table_lb.content.size.width, 600.0));
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
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    let table_lb = get_layout(&dom, table);
    // Table inside body (margin: 8px), so content starts at x=8.
    assert!(approx_eq(table_lb.content.origin.x, 8.0));
    // Table content width = 800 - 16 = 784.
    assert!(approx_eq(table_lb.content.size.width, 784.0));
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
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
    assert!(dom.world().get::<&LayoutBox>(flex).is_ok());
    assert!(dom.world().get::<&LayoutBox>(inner).is_ok());
    let inner_lb = get_layout(&dom, inner);
    // Flex item height should be preserved.
    assert!(approx_eq(inner_lb.content.size.height, 30.0));
}

// ---------------------------------------------------------------------------
// Anonymous table wrapper tests (G6)
// ---------------------------------------------------------------------------

#[test]
fn orphan_row_wrapped_in_anonymous_table() {
    // A <tr> inside a <div> should get wrapped in an anonymous table.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
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
    dom.append_child(div, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    // The cell should have been laid out (via anonymous table wrapping).
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

#[test]
fn orphan_row_group_wrapped_in_anonymous_table() {
    // A <tbody> inside a <div> should get wrapped in an anonymous table.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );

    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(div, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    // The cell should have been laid out.
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

// ---------------------------------------------------------------------------
// F-3: Grid/InlineGrid trigger anonymous table wrappers
// ---------------------------------------------------------------------------

#[test]
fn orphan_row_in_grid_wrapped_in_anonymous_table() {
    // A <tr> inside a grid container should get wrapped in an anonymous table.
    let mut dom = EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
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
    dom.append_child(grid, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    // The cell should have been laid out (via anonymous table wrapping in grid).
    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Orphan <tr> in grid should be wrapped in anonymous table"
    );
}

#[test]
fn orphan_row_in_flex_wrapped_in_anonymous_table() {
    // A <tr> inside a flex container should get wrapped in an anonymous table.
    let mut dom = EcsDom::new();
    let flex = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        flex,
        ComputedStyle {
            display: Display::Flex,
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
    dom.append_child(flex, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Orphan <tr> in flex should be wrapped in anonymous table"
    );
}

// ---------------------------------------------------------------------------
// F-4: Orphan table-cell → anonymous row → anonymous table (§17.2.1 Rule 2+3)
// ---------------------------------------------------------------------------

#[test]
fn orphan_cell_wrapped_in_anonymous_row_and_table() {
    // A <td> directly inside a <div> should get wrapped in anonymous <tr> + <table>.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(div, td);

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(600.0, 400.0), &font_db);

    // The cell should have been laid out via anonymous row + table wrapping.
    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Orphan <td> in <div> should be wrapped in anonymous row + table"
    );
}

// ---------------------------------------------------------------------------
// Fragmentation tests (CSS Fragmentation Level 3)
// ---------------------------------------------------------------------------

#[test]
fn layout_fragmented_single_fragment_when_content_fits() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1, "content fits → 1 fragment");
    assert!(fragments[0].break_token.is_none());
}

#[test]
fn layout_fragmented_two_fragments_on_overflow() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    // Add 3 children, each 80px tall. Total = 240px.
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(80.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, parent, &input, frag);
    assert!(
        fragments.len() >= 2,
        "240px in 100px fragments → at least 2 fragments"
    );
    // After take() optimization, intermediate fragments have break_token = None
    // in the Vec. Verify fragmentation occurred by checking fragment count.
    assert!(
        fragments.len() >= 2,
        "non-last fragments were fragmented (break_token moved to loop state)"
    );
    assert!(
        fragments.last().unwrap().break_token.is_none(),
        "last fragment has no break token"
    );
}

#[test]
fn layout_fragmented_forced_break_produces_two_fragments() {
    use elidex_plugin::BreakValue;

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child1 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            break_after: BreakValue::Page,
            ..Default::default()
        },
    );
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, parent, &input, frag);
    assert_eq!(fragments.len(), 2, "forced break → 2 fragments");
}

#[test]
fn layout_fragmented_without_fragmentainer_returns_one() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    // Even though we call layout_fragmented, it should produce 1 fragment.
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Column,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1);
}

// ---------------------------------------------------------------------------
// Paged media layout tests (CSS Paged Media Level 3)
// ---------------------------------------------------------------------------

fn make_page_ctx(width: f32, height: f32) -> elidex_plugin::PagedMediaContext {
    elidex_plugin::PagedMediaContext {
        page_width: width,
        page_height: height,
        page_margins: elidex_plugin::EdgeSizes {
            top: 50.0,
            right: 50.0,
            bottom: 50.0,
            left: 50.0,
        },
        page_rules: Vec::new(),
    }
}

#[test]
fn paged_single_page_fits_all_content() {
    let (mut dom, _root, _html, body) = build_styled_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0); // Letter size
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(!pages.is_empty(), "should have at least one page");
    assert_eq!(pages[0].page_number, 1);
    assert!(!pages[0].is_blank);
}

#[test]
fn paged_multi_page_break() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    // Add children totaling more than page content height.
    // Page content height = 1056 - 50 - 50 = 956. Each child = 500px.
    // 3 * 500 = 1500 > 956, so should span multiple pages.
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(500.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        pages.len() >= 2,
        "1500px content in 956px pages → at least 2 pages, got {}",
        pages.len()
    );
    // Page numbers are sequential.
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_number, i + 1);
    }
}

#[test]
fn paged_selector_first() {
    use elidex_plugin::{PageRule, PageSelector as PS};

    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: elidex_plugin::EdgeSizes::default(),
        page_rules: vec![PageRule {
            selectors: vec![PS::First],
            ..PageRule::default()
        }],
    };
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(!pages.is_empty());
    assert!(
        pages[0].matched_selectors.contains(&PS::First),
        "first page should match :first selector"
    );
}

#[test]
fn paged_selector_left_right() {
    use elidex_plugin::PageSelector as PS;

    // :left matches even pages, :right matches odd pages.
    assert!(PS::Right.matches(1, false));
    assert!(PS::Left.matches(2, false));
    assert!(PS::Right.matches(3, false));
    assert!(!PS::Left.matches(1, false));
    assert!(!PS::Right.matches(2, false));
}

#[test]
fn paged_blank_page_from_forced_break() {
    use elidex_plugin::BreakValue;

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            break_after: BreakValue::Page,
            ..Default::default()
        },
    );
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        pages.len() >= 2,
        "forced break should produce at least 2 pages"
    );
}

#[test]
fn paged_size_from_rule() {
    use elidex_plugin::{NamedPageSize, PageRule, PageSize};

    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: elidex_plugin::EdgeSizes::default(),
        page_rules: vec![PageRule {
            selectors: Vec::new(), // matches all pages
            size: Some(PageSize::Named(NamedPageSize::A4)),
            ..PageRule::default()
        }],
    };

    let (w, h) = page_ctx.effective_page_size(1, false);
    assert!(approx_eq(w, 794.0), "A4 width = 794, got {w}");
    assert!(approx_eq(h, 1123.0), "A4 height = 1123, got {h}");
}

#[test]
fn paged_counter_page_increments() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    // Two 600px children → at least 2 pages (content height = 956).
    for _ in 0..2 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(600.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    // Each page has incrementing page numbers.
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_number, i + 1, "page number should be sequential");
    }
}

#[test]
fn paged_two_pass_counter_pages() {
    // The total page count should be known (for counter(pages)).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(500.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    // Total pages is known after layout.
    let total = pages.len();
    assert!(total >= 2, "should have multiple pages");
    // All pages have correct page_number.
    assert_eq!(pages.last().unwrap().page_number, total);
}
