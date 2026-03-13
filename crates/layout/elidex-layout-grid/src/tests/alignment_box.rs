use super::*;
use elidex_plugin::{BorderSide, EdgeSizes};

#[test]
fn grid_gap_between_items() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
                column_gap: 20.0,
                row_gap: 10.0,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);
    let c3 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    let clb = do_layout_grid(
        &mut dom,
        container,
        420.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // 420 - 20 (gap) = 400 / 2 = 200 each.
    assert!(approx_eq(lb1.content.width, 200.0));
    assert!(approx_eq(lb2.content.width, 200.0));
    // Column gap: c2 starts at 200 + 20 = 220.
    assert!(approx_eq(lb2.content.x, 220.0));
    // Row gap: c3 at y = 50 + 10 = 60.
    assert!(approx_eq(lb3.content.y, 60.0));
    // Container height: 50 + 10 + 30 = 90.
    assert!(approx_eq(clb.content.height, 90.0));
}

#[test]
fn grid_align_items_center() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
                align_items: AlignItems::Center,
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 40.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // Centered in 100px row: (100 - 40) / 2 = 30.
    assert!(approx_eq(lb.content.y, 30.0));
}

#[test]
fn grid_with_padding_border() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                padding: EdgeSizes::uniform(10.0),
                border_top: BorderSide { width: 5.0, ..BorderSide::NONE },
                border_right: BorderSide { width: 5.0, ..BorderSide::NONE },
                border_bottom: BorderSide { width: 5.0, ..BorderSide::NONE },
                border_left: BorderSide { width: 5.0, ..BorderSide::NONE },
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let clb = do_layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // Content area starts after padding+border: 10+5=15.
    assert!(approx_eq(clb.content.x, 15.0));
    assert!(approx_eq(clb.content.y, 15.0));
    // Content width: 400 - 2*(10+5) = 370.
    assert!(approx_eq(clb.content.width, 370.0));
    // Child should fill the grid.
    assert!(approx_eq(lb.content.width, 370.0));
}

#[test]
fn grid_item_margin() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(200.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_top: Dimension::Length(10.0),
                margin_left: Dimension::Length(20.0),
                margin_right: Dimension::Length(20.0),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // Item starts at margin offset: x=20, y=10.
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
    // Width: 200 - 20 - 20 = 160.
    assert!(approx_eq(lb.content.width, 160.0));
}

#[test]
fn grid_box_sizing_border_box() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                width: Dimension::Length(400.0),
                box_sizing: elidex_plugin::BoxSizing::BorderBox,
                padding: EdgeSizes::new(0.0, 20.0, 0.0, 20.0),
                border_left: BorderSide { width: 5.0, ..BorderSide::NONE },
                border_right: BorderSide { width: 5.0, ..BorderSide::NONE },
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let clb = do_layout_grid(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // border-box: content = 400 - 2*(20+5) = 350.
    assert!(approx_eq(clb.content.width, 350.0));
    assert!(approx_eq(lb.content.width, 350.0));
}

#[test]
fn grid_align_self_stretch_with_center_container() {
    // align-self: stretch should stretch the item even when container has align-items: center.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
                align_items: AlignItems::Center,
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                // height is auto -- eligible for stretch.
                align_self: elidex_plugin::AlignSelf::Stretch,
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // align-self: stretch should override align-items: center.
    // Item should fill the 100px row (starts at y=0, height=100).
    assert!(approx_eq(lb.content.y, 0.0));
    assert!(approx_eq(lb.content.height, 100.0));
}

#[test]
fn grid_negative_track_size_clamped() {
    // Negative track sizes (from malformed CSS) should be clamped to 0.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(-50.0), TrackSize::Length(200.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // Negative track should be clamped to 0.
    assert!(lb1.content.width >= 0.0);
    assert!(lb2.content.width >= 0.0);
    // The second column (200px) should still work correctly.
    assert!(approx_eq(lb2.content.width, 200.0));
}

// ---------------------------------------------------------------------------
// M3.5-4: RTL direction support
// ---------------------------------------------------------------------------

#[test]
fn grid_rtl_reverses_column_order() {
    // direction: rtl -> columns placed right-to-left
    let mut dom = EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            grid,
            ComputedStyle {
                display: Display::Grid,
                direction: elidex_plugin::Direction::Rtl,
                grid_template_columns: vec![TrackSize::Length(100.0), TrackSize::Length(200.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c0 = make_grid_child(&mut dom, grid, 30.0);
    let c1 = make_grid_child(&mut dom, grid, 30.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        grid,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_layout(&dom, c0);
    let lb1 = get_layout(&dom, c1);

    // RTL: first column (100px) should be on the right, second (200px) on the left.
    assert!(
        lb0.content.x > lb1.content.x,
        "RTL grid: col 0 (x={}) should be right of col 1 (x={})",
        lb0.content.x,
        lb1.content.x,
    );
}
