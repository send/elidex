use super::*;
use elidex_plugin::{BorderSide, Dimension, EdgeSizes};

#[test]
fn flex_grow_distributes_space() {
    let items_styles = [
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            flex_grow: 1.0,
            ..Default::default()
        },
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            flex_grow: 1.0,
            ..Default::default()
        },
    ];
    let (mut dom, container, items) = make_flex_dom(flex_container(), &items_styles);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb0.content.width - 300.0).abs() < 1.0);
    assert!((lb1.content.width - 300.0).abs() < 1.0);
}

#[test]
fn flex_shrink_reduces_items() {
    let items_styles = [
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(300.0),
            height: Dimension::Length(50.0),
            flex_shrink: 1.0,
            ..Default::default()
        },
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(300.0),
            height: Dimension::Length(50.0),
            flex_shrink: 1.0,
            ..Default::default()
        },
    ];
    let (mut dom, container, items) = make_flex_dom(
        ComputedStyle {
            display: Display::Flex,
            width: Dimension::Length(400.0),
            ..Default::default()
        },
        &items_styles,
    );
    let font_db = FontDatabase::new();
    do_layout_flex(
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb0.content.width - 200.0).abs() < 1.0);
    assert!((lb1.content.width - 200.0).abs() < 1.0);
}

#[test]
fn grown_item_child_uses_flex_resolved_width() {
    // Verify that a flex item's child sees the flex-resolved width
    // (after grow), not the original style width.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(container, flex_container());

    // Single item: width 100, flex-grow 1 in 600px container -> grows to 600.
    let item = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        item,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            flex_grow: 1.0,
            ..Default::default()
        },
    );
    dom.append_child(container, item);

    // Child with auto width should fill the parent's grown width.
    let child = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(item, child);

    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        container,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let item_lb = get_lb(&dom, item);
    let child_lb = get_lb(&dom, child);

    // Item should have grown to 600px.
    assert!((item_lb.content.width - 600.0).abs() < 1.0);

    // Child with auto width should also be 600px (fills parent).
    assert!(
        (child_lb.content.width - 600.0).abs() < 1.0,
        "child width={} should be ~600 (parent's flex-resolved width)",
        child_lb.content.width,
    );
}

#[test]
fn margin_item_child_not_double_offset() {
    // Verify that a flex item with non-zero margins does NOT double-offset
    // its children (margin-box vs border-box position).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(container, flex_container());

    let item = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        item,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            height: Dimension::Length(50.0),
            margin_left: Dimension::Length(20.0),
            margin_right: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.append_child(container, item);

    let grandchild = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        grandchild,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(item, grandchild);

    let font_db = FontDatabase::new();
    do_layout_flex(
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

    let item_lb = get_lb(&dom, item);
    let grandchild_lb = get_lb(&dom, grandchild);

    // Item content starts at margin-left (20px).
    assert!((item_lb.content.x - 20.0).abs() < f32::EPSILON);

    // Grandchild should be at the same x as the item's content origin
    // (no extra margin offset).
    assert!(
        (grandchild_lb.content.x - item_lb.content.x).abs() < f32::EPSILON,
        "grandchild x={} should equal item content x={}",
        grandchild_lb.content.x,
        item_lb.content.x,
    );
}

#[test]
fn descendant_positioned_at_container_offset() {
    // Verify that flex item descendants get correct absolute coordinates
    // when the container itself is at a non-zero offset.
    let mut dom = EcsDom::new();
    let outer = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(outer, flex_container());

    let child = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(child, flex_item(200.0, 50.0));
    dom.append_child(outer, child);

    let font_db = FontDatabase::new();
    let offset_x = 100.0_f32;
    let offset_y = 200.0_f32;
    let outer_lb = do_layout_flex(
        &mut dom,
        outer,
        800.0,
        None,
        offset_x,
        offset_y,
        &font_db,
        0,
        layout_block_only,
    );

    // Container starts at (100, 200).
    assert!((outer_lb.content.x - offset_x).abs() < f32::EPSILON);
    assert!((outer_lb.content.y - offset_y).abs() < f32::EPSILON);

    // Child should be offset from the container's content origin.
    let child_lb = get_lb(&dom, child);
    assert!(child_lb.content.x >= offset_x);
    assert!(child_lb.content.y >= offset_y);
}

#[test]
fn stretch_skips_explicit_cross_size() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Stretch,
        ..Default::default()
    };
    // Both items have explicit heights -- neither should stretch.
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
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

    let lb0 = get_lb(&dom, items[0]);
    // 30px item should NOT stretch to 60px because height is explicit.
    assert!((lb0.content.height - 30.0).abs() < 1.0);
}

// --- M3-2: box-sizing: border-box in flex ---

#[test]
fn flex_item_border_box_width() {
    let container = flex_container();
    let item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        height: Dimension::Length(50.0),
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::Length(10.0),
            bottom: Dimension::ZERO,
            left: Dimension::Length(10.0),
        },
        border_left: BorderSide {
            width: 2.0,
            ..BorderSide::NONE
        },
        border_right: BorderSide {
            width: 2.0,
            ..BorderSide::NONE
        },
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, cont, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        cont,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // border-box: content = 200 - 10 - 10 - 2 - 2 = 176
    assert!((lb.content.width - 176.0).abs() < f32::EPSILON);
    // border_box() = 200
    let bb = lb.border_box();
    assert!((bb.width - 200.0).abs() < f32::EPSILON);
}

#[test]
fn flex_container_border_box_height() {
    let container = ComputedStyle {
        display: Display::Flex,
        height: Dimension::Length(200.0),
        padding: EdgeSizes {
            top: Dimension::Length(15.0),
            right: Dimension::ZERO,
            bottom: Dimension::Length(15.0),
            left: Dimension::ZERO,
        },
        border_top: BorderSide {
            width: 5.0,
            ..BorderSide::NONE
        },
        border_bottom: BorderSide {
            width: 5.0,
            ..BorderSide::NONE
        },
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let item = flex_item(100.0, 50.0);
    let (mut dom, cont, _) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    let lb = do_layout_flex(
        &mut dom,
        cont,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    // content height = 200 - 15 - 15 - 5 - 5 = 160
    assert!((lb.content.height - 160.0).abs() < f32::EPSILON);
}

// --- M3-5: Flex container with percentage height child ---

#[test]
fn flex_child_percentage_height() {
    let container = ComputedStyle {
        display: Display::Flex,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let child_style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Percentage(50.0),
        ..Default::default()
    };
    let (mut dom, cont, items) = make_flex_dom(container, &[child_style]);
    let font_db = FontDatabase::new();
    do_layout_flex(
        &mut dom,
        cont,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_lb(&dom, items[0]);
    // height: 50% of 200 = 100.
    assert!((lb.content.height - 100.0).abs() < 1.0);
}

// --- M3-6: min/max width in flex items ---

#[test]
fn flex_item_min_width_prevents_shrink() {
    // Two items each 300px in 400px container. Normal shrink would give 200 each.
    // Item 0 has min-width: 250px -> frozen at 250, item 1 gets 150.
    let item0 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(300.0),
        height: Dimension::Length(50.0),
        min_width: Dimension::Length(250.0),
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(flex_container(), &[item0, flex_item(300.0, 50.0)]);
    let font_db = FontDatabase::new();
    do_layout_flex(
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(
        lb0.content.width >= 249.0,
        "item 0 should respect min-width 250, got {}",
        lb0.content.width
    );
    assert!(
        lb1.content.width < lb0.content.width,
        "item 1 should be smaller than item 0"
    );
}

#[test]
fn flex_item_max_width_prevents_grow() {
    // Two items each 100px, flex-grow: 1 in 800px container. Normal would give 400 each.
    // Item 0 has max-width: 200px -> frozen at 200, item 1 gets remainder.
    let item0 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        flex_grow: 1.0,
        max_width: Dimension::Length(200.0),
        ..Default::default()
    };
    let item1 = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        height: Dimension::Length(50.0),
        flex_grow: 1.0,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(flex_container(), &[item0, item1]);
    let font_db = FontDatabase::new();
    do_layout_flex(
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

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(
        lb0.content.width <= 201.0,
        "item 0 should respect max-width 200, got {}",
        lb0.content.width
    );
    assert!(
        lb1.content.width > lb0.content.width,
        "item 1 should get remaining space, got {}",
        lb1.content.width
    );
}
