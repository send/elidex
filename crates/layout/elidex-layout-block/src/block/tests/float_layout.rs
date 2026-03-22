use super::*;
use elidex_plugin::Overflow;

#[test]
fn left_float_positioned_at_left_edge() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 100.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    assert!((float_box.content.origin.x - 0.0).abs() < f32::EPSILON);
    assert!((float_box.content.origin.y - 0.0).abs() < f32::EPSILON);
    assert!((float_box.content.size.width - 200.0).abs() < f32::EPSILON);
    assert!((float_box.content.size.height - 100.0).abs() < f32::EPSILON);
}

#[test]
fn right_float_positioned_at_right_edge() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let floated = make_float_child(&mut dom, parent, Float::Right, 200.0, 100.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Right float: x = containing_width - width = 800 - 200 = 600
    assert!((float_box.content.origin.x - 600.0).abs() < f32::EPSILON);
    assert!((float_box.content.origin.y - 0.0).abs() < f32::EPSILON);
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
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Normal flow child starts at y=0 (float is out of flow).
    let normal_box = dom.world().get::<&LayoutBox>(normal).unwrap();
    assert!((normal_box.content.origin.y - 0.0).abs() < f32::EPSILON);
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
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Cleared element starts below the left float.
    let cleared_box = dom.world().get::<&LayoutBox>(cleared).unwrap();
    assert!((cleared_box.content.origin.y - 100.0).abs() < f32::EPSILON);
}

#[test]
fn float_extends_bfc_parent_height() {
    // CSS 2.1 §10.6.7: BFC roots expand to contain floats.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            overflow_x: elidex_plugin::Overflow::Hidden, // establishes BFC
            overflow_y: elidex_plugin::Overflow::Hidden,
            ..Default::default()
        },
    );
    let _floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 150.0);

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // BFC parent height should extend to contain the float.
    assert!((parent_box.content.size.height - 150.0).abs() < f32::EPSILON);
}

#[test]
fn float_overflows_non_bfc_parent() {
    // CSS 2.1 §10.6.7: Non-BFC blocks do NOT expand to contain floats.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let _floated = make_float_child(&mut dom, parent, Float::Left, 200.0, 150.0);

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Non-BFC parent height is 0 (float overflows).
    assert!((parent_box.content.size.height - 0.0).abs() < f32::EPSILON);
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
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    // Cleared element starts below the tallest float (120).
    let cleared_box = dom.world().get::<&LayoutBox>(cleared).unwrap();
    assert!((cleared_box.content.origin.y - 120.0).abs() < f32::EPSILON);
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
            padding: EdgeSizes {
                top: Dimension::Length(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::Length(20.0),
            },
            ..Default::default()
        },
    );
    let floated = make_float_child(&mut dom, parent, Float::Left, 100.0, 50.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Float x should include parent's padding-left (20px).
    assert!(
        (float_box.content.origin.x - 20.0).abs() < f32::EPSILON,
        "expected x=20.0, got {}",
        float_box.content.origin.x
    );
    // Float y should include parent's padding-top (10px).
    assert!(
        (float_box.content.origin.y - 10.0).abs() < f32::EPSILON,
        "expected y=10.0, got {}",
        float_box.content.origin.y
    );
}

#[test]
fn float_auto_width_shrinks_to_content() {
    // A float with auto width should shrink to fit its text content,
    // not expand to the full available width.
    let font_db = FontDatabase::new();

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            overflow_x: Overflow::Hidden, // BFC
            overflow_y: Overflow::Hidden,
            ..Default::default()
        },
    );

    // Create float with auto width containing short text.
    let floated = dom.create_element("div", Attributes::default());
    dom.append_child(parent, floated);
    dom.world_mut().insert_one(
        floated,
        ComputedStyle {
            display: Display::Block,
            float: Float::Left,
            width: Dimension::Auto,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("Hi");
    dom.append_child(floated, text);

    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    // Auto-width float should be narrower than the 800px containing block.
    assert!(
        float_box.content.size.width < 780.0,
        "expected shrink-to-fit width < 780, got {}",
        float_box.content.size.width
    );
    // Positive width requires a font for text shaping — skip on fontless CI.
    if font_db.has_fonts() {
        assert!(
            float_box.content.size.width > 0.0,
            "expected positive width, got {}",
            float_box.content.size.width
        );
    }
}

#[test]
fn float_explicit_width_unchanged() {
    // A float with explicit width should use that width, not shrink.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(parent, block_style());
    let floated = make_float_child(&mut dom, parent, Float::Left, 300.0, 50.0);

    let font_db = FontDatabase::new();
    let _parent_box = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);

    let float_box = dom.world().get::<&LayoutBox>(floated).unwrap();
    assert!(
        (float_box.content.size.width - 300.0).abs() < f32::EPSILON,
        "expected explicit width 300, got {}",
        float_box.content.size.width
    );
}

#[test]
fn float_propagates_through_non_bfc() {
    // A float inside a non-BFC child should be visible to the ancestor
    // BFC for height containment.
    let mut dom = EcsDom::new();
    let bfc_parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        bfc_parent,
        ComputedStyle {
            display: Display::Block,
            overflow_x: Overflow::Hidden, // establishes BFC
            overflow_y: Overflow::Hidden,
            ..Default::default()
        },
    );

    // Non-BFC intermediate block.
    let wrapper = dom.create_element("div", Attributes::default());
    dom.append_child(bfc_parent, wrapper);
    dom.world_mut().insert_one(wrapper, block_style());

    // Float inside the non-BFC wrapper.
    let _floated = make_float_child(&mut dom, wrapper, Float::Left, 200.0, 150.0);

    let font_db = FontDatabase::new();
    let parent_box = layout_block(&mut dom, bfc_parent, 800.0, Point::ZERO, &font_db);

    // The BFC parent should expand to contain the float (150px)
    // even though the float is inside the non-BFC wrapper.
    assert!(
        parent_box.content.size.height >= 150.0 - f32::EPSILON,
        "expected BFC parent height >= 150 (float containment), got {}",
        parent_box.content.size.height
    );
}
