use super::*;
use elidex_ecs::Attributes;
use elidex_layout_block::layout_block_only;

fn flex_container() -> ComputedStyle {
    ComputedStyle {
        display: Display::Flex,
        ..Default::default()
    }
}

fn flex_item(width: f32, height: f32) -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(width),
        height: Dimension::Length(height),
        ..Default::default()
    }
}

fn make_flex_dom(
    container_style: ComputedStyle,
    items: &[ComputedStyle],
) -> (EcsDom, Entity, Vec<Entity>) {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(container, container_style);

    let mut entities = Vec::new();
    for item_style in items {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(container, child);
        dom.world_mut().insert_one(child, item_style.clone());
        entities.push(child);
    }
    (dom, container, entities)
}

fn get_lb(dom: &EcsDom, entity: Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

#[test]
fn row_basic_layout() {
    let (mut dom, container, items) = make_flex_dom(
        flex_container(),
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    let lb = layout_flex(
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

    assert!((lb.content.width - 800.0).abs() < f32::EPSILON);
    assert!((lb.content.height - 50.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb0.content.width - 100.0).abs() < f32::EPSILON);
    assert!((lb1.content.width - 200.0).abs() < f32::EPSILON);
    assert!(lb1.content.x > lb0.content.x);
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
    let lb = layout_flex(
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

    assert!((lb.content.height - 120.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(lb1.content.y > lb0.content.y);
}

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
    layout_flex(
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
    layout_flex(
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
    let lb = layout_flex(
        &mut dom,
        container,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    assert!((lb.content.height - 100.0).abs() < f32::EPSILON);

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    assert!(lb1.content.y > lb0.content.y);
}

#[test]
fn justify_content_center() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::Center,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.x - 350.0).abs() < 1.0);
}

#[test]
fn justify_content_space_between() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::SpaceBetween,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!(lb0.content.x < 1.0);
    assert!((lb1.content.x + lb1.content.width - 800.0).abs() < 1.0);
}

#[test]
fn justify_content_space_around() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::SpaceAround,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.x - 150.0).abs() < 1.0);
    assert!((lb1.content.x - 550.0).abs() < 1.0);
}

#[test]
fn justify_content_space_evenly() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::SpaceEvenly,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.x - 200.0).abs() < 1.0);
    assert!((lb1.content.x - 500.0).abs() < 1.0);
}

#[test]
fn justify_content_flex_end() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::FlexEnd,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.x - 700.0).abs() < 1.0);
}

#[test]
fn align_items_center() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Center,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.y - 15.0).abs() < 1.0);
    assert!(lb1.content.y.abs() < 1.0);
}

#[test]
fn align_items_flex_start() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::FlexStart,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!(lb0.content.y.abs() < 1.0);
}

#[test]
fn align_items_flex_end() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::FlexEnd,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    assert!((lb0.content.y - 30.0).abs() < 1.0);
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
    layout_flex(
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

    let lb0 = get_lb(&dom, items[0]); // order=2
    let lb1 = get_lb(&dom, items[1]); // order=1
    assert!(lb1.content.x < lb0.content.x);
    assert!((lb1.content.width - 200.0).abs() < f32::EPSILON);
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
    let lb = layout_flex(
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

    assert!((lb.content.height - 50.0).abs() < f32::EPSILON);
}

#[test]
fn empty_flex_container() {
    let (mut dom, container, _) = make_flex_dom(flex_container(), &[]);
    let font_db = FontDatabase::new();
    let lb = layout_flex(
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

    assert!((lb.content.width - 800.0).abs() < f32::EPSILON);
    assert!((lb.content.height).abs() < f32::EPSILON);
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
    let outer_lb = layout_flex(
        &mut dom,
        outer,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    assert!((outer_lb.content.width - 800.0).abs() < f32::EPSILON);

    let inner_lb = get_lb(&dom, inner);
    assert!((inner_lb.content.width - 400.0).abs() < 1.0);
}

#[test]
fn align_items_stretch() {
    let style = ComputedStyle {
        display: Display::Flex,
        align_items: AlignItems::Stretch,
        ..Default::default()
    };
    // First item has auto height (should stretch), second has explicit 60px height.
    let auto_height_item = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        // height: Auto (default) — eligible for stretch
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[auto_height_item, flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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

    // Auto-height item should stretch to line cross size (60).
    let lb0 = get_lb(&dom, items[0]);
    assert!((lb0.content.height - 60.0).abs() < 1.0);

    // Explicit-height item should NOT stretch (remains 60).
    let lb1 = get_lb(&dom, items[1]);
    assert!((lb1.content.height - 60.0).abs() < 1.0);
}

#[test]
fn grown_item_child_uses_flex_resolved_width() {
    // Verify that a flex item's child sees the flex-resolved width
    // (after grow), not the original style width.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(container, flex_container());

    // Single item: width 100, flex-grow 1 in 600px container → grows to 600.
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
    layout_flex(
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
    layout_flex(
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
    let outer_lb = layout_flex(
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
    // Both items have explicit heights — neither should stretch.
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 30.0), flex_item(100.0, 60.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
        padding_left: 10.0,
        padding_right: 10.0,
        border_left_width: 2.0,
        border_right_width: 2.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, cont, items) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    layout_flex(
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
        padding_top: 15.0,
        padding_bottom: 15.0,
        border_top_width: 5.0,
        border_bottom_width: 5.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let item = flex_item(100.0, 50.0);
    let (mut dom, cont, _) = make_flex_dom(container, &[item]);
    let font_db = FontDatabase::new();
    let lb = layout_flex(
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

// --- M3-5: Flexbox gap ---

#[test]
fn column_gap_row_direction() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: 20.0,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    // Item 0 at x=0, width=100. Gap=20. Item 1 at x=120.
    assert!((lb0.content.x).abs() < f32::EPSILON);
    assert!((lb1.content.x - 120.0).abs() < 1.0);
}

#[test]
fn row_gap_column_direction() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        row_gap: 10.0,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 40.0), flex_item(100.0, 40.0)]);
    let font_db = FontDatabase::new();
    let lb = layout_flex(
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
    // Item 0 at y=0, height=40. Gap=10. Item 1 at y=50.
    assert!((lb0.content.y).abs() < f32::EPSILON);
    assert!((lb1.content.y - 50.0).abs() < 1.0);
    // Container height = 40 + 10 + 40 = 90.
    assert!((lb.content.height - 90.0).abs() < 1.0);
}

#[test]
fn gap_affects_flex_grow() {
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
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: 100.0,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &items_styles);
    let font_db = FontDatabase::new();
    layout_flex(
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
    // Available = 600 - 100 (gap) = 500. Each grows to 250.
    assert!((lb0.content.width - 250.0).abs() < 1.0);
    assert!((lb1.content.width - 250.0).abs() < 1.0);
}

#[test]
fn gap_zero_default_unchanged() {
    // Default gap is 0 — layout should be identical to pre-gap behavior.
    let (mut dom, container, items) = make_flex_dom(
        flex_container(),
        &[flex_item(100.0, 50.0), flex_item(200.0, 50.0)],
    );
    let font_db = FontDatabase::new();
    layout_flex(
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
    // No gap: item1 starts right after item0.
    assert!((lb1.content.x - 100.0).abs() < f32::EPSILON);
    assert!((lb0.content.x).abs() < f32::EPSILON);
}

#[test]
fn gap_with_wrap_cross_axis() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Wrap,
        width: Dimension::Length(300.0),
        column_gap: 10.0,
        row_gap: 20.0,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    let lb = layout_flex(
        &mut dom,
        container,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_lb(&dom, items[0]);
    let lb1 = get_lb(&dom, items[1]);
    // Items wrap: line 0 has item0 (height 50), gap_cross=20, line 1 has item1.
    assert!((lb1.content.y - 70.0).abs() < 1.0);
    // Container height = 50 + 20 + 50 = 120.
    assert!((lb.content.height - 120.0).abs() < 1.0);
    // Both items at x=0 (different lines).
    assert!((lb0.content.x).abs() < f32::EPSILON);
    assert!((lb1.content.x).abs() < f32::EPSILON);
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
    layout_flex(
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

// L5: single item + gap (gap should not affect layout with only one item)
#[test]
fn gap_single_item_no_effect() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: 20.0,
        ..Default::default()
    };
    let (mut dom, container, items) = make_flex_dom(style, &[flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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

    let lb = get_lb(&dom, items[0]);
    assert!((lb.content.x).abs() < f32::EPSILON);
    assert!((lb.content.width - 100.0).abs() < 1.0);
}

// L6: gap + justify-content: space-between
#[test]
fn gap_with_justify_space_between() {
    let style = ComputedStyle {
        display: Display::Flex,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: 10.0,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    // space-between distributes free space between items, gap adds on top.
    // Effective gap = max(justify_gap, column_gap) → items should be well-separated.
    assert!((lb0.content.x).abs() < f32::EPSILON);
    // Item 1 should be at right edge: 800 - 100 = 700.
    assert!((lb1.content.x - 700.0).abs() < 1.0);
}

// L7: gap + flex-direction: row-reverse
#[test]
fn gap_with_row_reverse() {
    let style = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::RowReverse,
        column_gap: 20.0,
        ..Default::default()
    };
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(100.0, 50.0), flex_item(100.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    // Row-reverse: item 0 at right, item 1 to its left with gap.
    // Item 0 at x = 800 - 100 = 700.
    assert!((lb0.content.x - 700.0).abs() < 1.0);
    // Item 1 at x = 700 - 20 (gap) - 100 = 580.
    assert!((lb1.content.x - 580.0).abs() < 1.0);
}

// L8: gap + flex-shrink (items shrink, gap is preserved)
#[test]
fn gap_with_flex_shrink() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_gap: 20.0,
        ..Default::default()
    };
    // Two items each 200px wide + 20px gap = 420px. Container = 400px.
    // Items should shrink to fit, gap preserved.
    let (mut dom, container, items) =
        make_flex_dom(style, &[flex_item(200.0, 50.0), flex_item(200.0, 50.0)]);
    let font_db = FontDatabase::new();
    layout_flex(
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
    // Both items shrink equally. Total available = 400 - 20 (gap) = 380. Each gets 190.
    assert!((lb0.content.width - 190.0).abs() < 1.0);
    assert!((lb1.content.width - 190.0).abs() < 1.0);
    // Gap between items is maintained.
    let gap = lb1.content.x - (lb0.content.x + lb0.content.width);
    assert!((gap - 20.0).abs() < 1.0);
}

// --- M3-6: min/max width in flex items ---

#[test]
fn flex_item_min_width_prevents_shrink() {
    // Two items each 300px in 400px container. Normal shrink would give 200 each.
    // Item 0 has min-width: 250px → frozen at 250, item 1 gets 150.
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
    layout_flex(
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
    // Item 0 has max-width: 200px → frozen at 200, item 1 gets remainder.
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
    layout_flex(
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
