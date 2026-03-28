use super::*;

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
    assert!((div_lb.content.origin.x - 8.0).abs() < f32::EPSILON);
    assert!((div_lb.content.origin.y - 8.0).abs() < f32::EPSILON);
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

    assert!(dom.world().get::<&LayoutBox>(container).is_ok());
    assert!(dom.world().get::<&LayoutBox>(child).is_ok());
}

#[test]
fn grid_nested_in_block() {
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

    assert!(approx_eq(grid_lb.content.origin.x, 8.0));
    assert!(approx_eq(grid_lb.content.size.width, 784.0));
    assert!(approx_eq(c1_lb.content.size.width, 392.0));
    assert!(approx_eq(c2_lb.content.size.width, 392.0));
    assert!(approx_eq(c2_lb.content.origin.x, 400.0));
}

#[test]
fn grid_item_is_flex_container() {
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

    let flex_lb = get_layout(&dom, flex_item);
    let inner_lb = get_layout(&dom, inner);

    assert!(approx_eq(flex_lb.content.size.width, 200.0));
    assert!(approx_eq(inner_lb.content.size.width, 50.0));
    assert!(approx_eq(inner_lb.content.size.height, 30.0));
}

#[test]
fn inline_grid_dispatches_to_grid() {
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

    assert!(approx_eq(container_lb.content.size.width, 400.0));
    assert!(approx_eq(c1_lb.content.size.width, 200.0));
}

#[test]
fn viewport_height_enables_root_percentage_height() {
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
    assert!(approx_eq(table_lb.content.origin.x, 8.0));
    assert!(approx_eq(table_lb.content.size.width, 784.0));
}

#[test]
fn table_is_block_level() {
    assert!(elidex_layout_block::block::is_block_level(Display::Table));
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
    assert!(approx_eq(inner_lb.content.size.height, 30.0));
}

// ---------------------------------------------------------------------------
// Anonymous table wrapper tests (G6)
// ---------------------------------------------------------------------------

#[test]
fn orphan_row_wrapped_in_anonymous_table() {
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

    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

#[test]
fn orphan_row_group_wrapped_in_anonymous_table() {
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

    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

// ---------------------------------------------------------------------------
// F-3: Grid/InlineGrid trigger anonymous table wrappers
// ---------------------------------------------------------------------------

#[test]
fn orphan_row_in_grid_wrapped_in_anonymous_table() {
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

    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Orphan <tr> in grid should be wrapped in anonymous table"
    );
}

#[test]
fn orphan_row_in_flex_wrapped_in_anonymous_table() {
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

    assert!(
        dom.world().get::<&LayoutBox>(td).is_ok(),
        "Orphan <td> in <div> should be wrapped in anonymous row + table"
    );
}
