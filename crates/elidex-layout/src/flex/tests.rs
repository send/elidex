use super::*;
use elidex_ecs::Attributes;

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
    let lb = layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    let lb = layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 600.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 400.0, 0.0, 0.0, &font_db, 0);

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
    let lb = layout_flex(&mut dom, container, 300.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    let lb = layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

    assert!((lb.content.height - 50.0).abs() < f32::EPSILON);
}

#[test]
fn empty_flex_container() {
    let (mut dom, container, _) = make_flex_dom(flex_container(), &[]);
    let font_db = FontDatabase::new();
    let lb = layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    let outer_lb = layout_flex(&mut dom, outer, 800.0, 0.0, 0.0, &font_db, 0);
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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 600.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    let outer_lb = layout_flex(&mut dom, outer, 800.0, offset_x, offset_y, &font_db, 0);

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
    layout_flex(&mut dom, container, 800.0, 0.0, 0.0, &font_db, 0);

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
    layout_flex(&mut dom, cont, 800.0, 0.0, 0.0, &font_db, 0);

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
    let lb = layout_flex(&mut dom, cont, 800.0, 0.0, 0.0, &font_db, 0);

    // content height = 200 - 15 - 15 - 5 - 5 = 160
    assert!((lb.content.height - 160.0).abs() < f32::EPSILON);
}
