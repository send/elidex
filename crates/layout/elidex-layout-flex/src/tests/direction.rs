use super::*;

#[test]
fn row_basic_layout() {
    let (mut dom, container, items) = make_flex_dom(
        flex_container(),
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.width - 800.0).abs() < f32::EPSILON);
    assert!((lb.content.size.height - 50.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb0.content.size.width - 100.0).abs() < f32::EPSILON);
    assert!((lb1.content.size.width - 200.0).abs() < f32::EPSILON);
    assert!(lb1.content.origin.x > lb0.content.origin.x);
}

#[test]
fn column_layout() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 70.0)]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.height - 120.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(lb1.content.origin.y > lb0.content.origin.y);
}

#[test]
fn column_reverse_layout() {
    // Column-reverse needs explicit height to define the main-axis size.
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::ColumnReverse,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 70.0)]);
    let font_db = FontDatabase::new();
    let _lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // In column-reverse, first item appears below the second.
    assert!(
        lb0.content.origin.y > lb1.content.origin.y,
        "column-reverse: item[0] y={} should be below item[1] y={}",
        lb0.content.origin.y,
        lb1.content.origin.y,
    );
}

#[test]
fn wrap_splits_lines() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Wrap,
        width: Dimension::Length(300.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        300.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.height - 100.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(lb1.content.origin.y > lb0.content.origin.y);
}

#[test]
fn wrap_reverse_layout() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::WrapReverse,
        width: Dimension::Length(300.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        300.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.height - 100.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // In wrap-reverse, later line appears above earlier line.
    assert!(
        lb0.content.origin.y > lb1.content.origin.y,
        "wrap-reverse: line 1 y={} should be above line 0 y={}",
        lb1.content.origin.y,
        lb0.content.origin.y,
    );
}

#[test]
fn display_none_skipped() {
    let items_styles = [
        flex_item(100.0, 50.0),
        ComputedStyle {
            display: Display::None,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
        flex_item(100.0, 50.0),
    ];
    let (mut dom, container, _items) = make_flex_dom(flex_container(), &items_styles);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.height - 50.0).abs() < f32::EPSILON);
}

#[test]
fn empty_flex_container() {
    let (mut dom, container, _) = make_flex_dom(flex_container(), &[]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.size.width - 800.0).abs() < f32::EPSILON);
    assert!((lb.content.size.height).abs() < f32::EPSILON);
}

#[test]
fn nested_flex_containers() {
    let mut dom = EcsDom::new();
    let outer = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(outer, flex_container());

    let inner = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        inner,
        ComputedStyle {
            display: Display::Flex,
            width: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    dom.append_child(outer, inner);

    let child = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(child, flex_item(100.0, 30.0));
    dom.append_child(inner, child);

    let font_db = FontDatabase::new();
    let outer_lb = do_layout_flex(
        &mut dom,
        outer,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );
    assert!((outer_lb.content.size.width - 800.0).abs() < f32::EPSILON);

    let inner_lb = get_lb(&dom, inner);
    assert!((inner_lb.content.size.width - 400.0).abs() < 1.0);
}

#[test]
fn order_sorting() {
    let items_styles = [
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            order: 2,
            ..Default::default()
        },
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            height: Dimension::Length(50.0),
            order: 1,
            ..Default::default()
        },
    ];
    let (mut dom, container, items) = make_flex_dom(flex_container(), &items_styles);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        800.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]); // order=2
    let lb1 = get_lb(&dom, items[1]); // order=1
    assert!(lb1.content.origin.x < lb0.content.origin.x);
    assert!((lb1.content.size.width - 200.0).abs() < f32::EPSILON);
}
