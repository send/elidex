use super::*;

#[test]
fn left_float_positioned_at_left_edge() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 100.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    assert!((float_box.content.x - 0.0).abs() < f32::EPSILON);
    assert!((float_box.content.y - 0.0).abs() < f32::EPSILON);
    assert!((float_box.content.width - 200.0).abs() < f32::EPSILON);
    assert!((float_box.content.height - 100.0).abs() < f32::EPSILON);
}

#[test]
fn right_float_positioned_at_right_edge() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let floated = make_float_child(&mut dom, parent, Float::Right, 200.0, 100.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Right float: x = containing_width - width = 800 - 200 = 600
    assert!((float_box.content.x - 600.0).abs() < f32::EPSILON);
    assert!((float_box.content.y - 0.0).abs() < f32::EPSILON);
}

#[test]
fn float_excluded_from_normal_flow() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let _floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 100.0);

    let normal = dom.create_element("div", Attributes::default());
    dom.append_child(parent, normal);
    dom.world_mut().insert_one(
        normal,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    // Normal flow child starts at y=0 (float is out of flow).
    let normal_box = dom.world().get::<&LayoutBox>(normal).unwrap();
    assert!((normal_box.content.y - 0.0).abs() < f32::EPSILON);
}

#[test]
fn clear_left_advances_past_float() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let _floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 100.0);

    let cleared = dom.create_element("div", Attributes::default());
    dom.append_child(parent, cleared);
    dom.world_mut().insert_one(
        cleared,
        ComputedStyle {
            display: Display::Block,
            clear: Clear::Left,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    // Cleared element starts below the left float.
    let cleared_box = dom.world().get::<&LayoutBox>(cleared).unwrap();
    assert!((cleared_box.content.y - 100.0).abs() < f32::EPSILON);
}

#[test]
fn float_extends_parent_height() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let _floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 150.0);

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    // Parent height should extend to contain the float.
    assert!((parent_box.content.height - 150.0).abs() < f32::EPSILON);
}

#[test]
fn clear_both_advances_past_all_floats() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let _left_float = make_float_child(&mut dom, parent, Float::Left, 200.0, 80.0);
    let _right_float = make_float_child(&mut dom, parent, Float::Right, 200.0, 120.0);

    let cleared = dom.create_element("div", Attributes::default());
    dom.append_child(parent, cleared);
    dom.world_mut().insert_one(
        cleared,
        ComputedStyle {
            display: Display::Block,
            clear: Clear::Both,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    // Cleared element starts below the tallest float (120).
    let cleared_box = dom.world().get::<&LayoutBox>(cleared).unwrap();
    assert!((cleared_box.content.y - 120.0).abs() < f32::EPSILON);
}

#[test]
fn float_with_nonzero_parent_offset() {
    // Float X position must include the parent's content offset.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            padding_left: 20.0,
            padding_top: 10.0,
            ..Default::default()
        },
    );
    let floated = make_float_child(&mut dom, parent, Float::Left, 100.0, 50.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Float x should include parent's padding-left (20px).
    assert!(
        (float_box.content.x - 20.0).abs() < f32::EPSILON,
        "expected x=20.0, got {}",
        float_box.content.x
    );
    // Float y should include parent's padding-top (10px).
    assert!(
        (float_box.content.y - 10.0).abs() < f32::EPSILON,
        "expected y=10.0, got {}",
        float_box.content.y
    );
}
