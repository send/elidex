use super::*;

#[test]
fn vertical_stacking_two_divs() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child1 = dom.create_element("div", Attributes::default());
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.append_child(parent, child2);

    let child_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(50.0),
        ..Default::default()
    };

    dom.world_mut().insert_one(parent, block_style());
    dom.world_mut().insert_one(child1, child_style.clone());
    dom.world_mut().insert_one(child2, child_style);

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
}

#[test]
fn display_none_excluded() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let visible = dom.create_element("div", Attributes::default());
    let hidden = dom.create_element("div", Attributes::default());
    dom.append_child(parent, visible);
    dom.append_child(parent, hidden);

    dom.world_mut().insert_one(parent, block_style());
    dom.world_mut().insert_one(
        visible,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        hidden,
        ComputedStyle {
            display: Display::None,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.height - 50.0).abs() < f32::EPSILON);
}

#[test]
fn box_sizing_border_box_height() {
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(100.0),
        padding: EdgeSizes {
            top: 10.0,
            right: 0.0,
            bottom: 10.0,
            left: 0.0,
        },
        border_top: BorderSide {
            width: 2.0,
            ..BorderSide::NONE
        },
        border_bottom: BorderSide {
            width: 2.0,
            ..BorderSide::NONE
        },
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // content height = 100 - 10 - 10 - 2 - 2 = 76
    assert!((lb.content.height - 76.0).abs() < f32::EPSILON);
}

// --- M3-4: replaced element (image) layout ---

#[test]
fn replaced_element_intrinsic_size() {
    // width:auto, height:auto -> use intrinsic dimensions.
    let style = ComputedStyle {
        display: Display::Block,
        ..Default::default()
    };
    let (mut dom, img) = make_dom_with_image(style, 200, 100);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
    assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
}

#[test]
fn replaced_element_css_width_aspect_ratio() {
    // width:300px, height:auto -> height computed from aspect ratio.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(300.0),
        ..Default::default()
    };
    let (mut dom, img) = make_dom_with_image(style, 200, 100);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 300.0).abs() < f32::EPSILON);
    // height = 300 * 100/200 = 150
    assert!((lb.content.height - 150.0).abs() < f32::EPSILON);
}

#[test]
fn replaced_element_css_height_aspect_ratio() {
    // width:auto, height:200px -> width computed from aspect ratio.
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let (mut dom, img) = make_dom_with_image(style, 300, 100);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
    // width = 200 * 300/100 = 600
    assert!((lb.content.width - 600.0).abs() < f32::EPSILON);
    assert!((lb.content.height - 200.0).abs() < f32::EPSILON);
}

#[test]
fn replaced_element_both_dimensions_specified() {
    // width:400px, height:300px -> both used as-is.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        height: Dimension::Length(300.0),
        ..Default::default()
    };
    let (mut dom, img) = make_dom_with_image(style, 200, 100);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
    assert!((lb.content.height - 300.0).abs() < f32::EPSILON);
}

#[test]
fn replaced_element_border_box() {
    // box-sizing: border-box with padding.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(220.0),
        height: Dimension::Length(120.0),
        padding: EdgeSizes {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        },
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, img) = make_dom_with_image(style, 200, 100);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
    // content = 220 - 10 - 10 = 200
    assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
    // content height = 120 - 10 - 10 = 100
    assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
}

#[test]
fn no_image_data_normal_layout() {
    // Element without ImageData -> normal block layout.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        height: Dimension::Length(200.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
    assert!((lb.content.height - 200.0).abs() < f32::EPSILON);
}

// --- M3-5: Percentage heights ---

#[test]
fn percentage_height_with_definite_parent() {
    // Parent height=200, child height=50% -> 100.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Percentage(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    let child_lb = dom
        .world()
        .get::<&LayoutBox>(child)
        .map(|lb| (*lb).clone())
        .expect("child LayoutBox");
    assert!(
        (child_lb.content.height - 100.0).abs() < f32::EPSILON,
        "expected height=100 (50% of 200), got {}",
        child_lb.content.height
    );
}

#[test]
fn percentage_height_without_definite_parent() {
    // Parent height=auto, child height=50% -> falls back to auto (content height).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            // height: Auto (default)
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Percentage(50.0),
            // No content -> height = 0.
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

    let child_lb = dom
        .world()
        .get::<&LayoutBox>(child)
        .map(|lb| (*lb).clone())
        .expect("child LayoutBox");
    // Auto parent -> percentage height unresolvable -> auto -> content height (0).
    assert!(
        child_lb.content.height.abs() < f32::EPSILON,
        "expected height=0 (auto fallback), got {}",
        child_lb.content.height
    );
}

#[test]
fn percentage_height_nested_blocks() {
    // Grandparent height=400, parent height=50% (=200), child height=50% (=100).
    let mut dom = EcsDom::new();
    let gp = dom.create_element("div", Attributes::default());
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(gp, parent);
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        gp,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Percentage(50.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Percentage(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    layout_block(&mut dom, gp, 800.0, 0.0, 0.0, &font_db);

    let parent_lb = dom
        .world()
        .get::<&LayoutBox>(parent)
        .map(|lb| (*lb).clone())
        .expect("parent LayoutBox");
    let child_lb = dom
        .world()
        .get::<&LayoutBox>(child)
        .map(|lb| (*lb).clone())
        .expect("child LayoutBox");
    assert!(
        (parent_lb.content.height - 200.0).abs() < f32::EPSILON,
        "parent height = 50% of 400 = 200, got {}",
        parent_lb.content.height
    );
    assert!(
        (child_lb.content.height - 100.0).abs() < f32::EPSILON,
        "child height = 50% of 200 = 100, got {}",
        child_lb.content.height
    );
}

// --- M3-6: min-height / max-height ---

#[test]
fn min_height_constrains_auto() {
    // Block with no children -> auto height = 0, min-height: 200px -> height = 200.
    let style = ComputedStyle {
        display: Display::Block,
        min_height: Dimension::Length(200.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.height - 200.0).abs() < 1.0,
        "min-height should force height to 200, got {}",
        lb.content.height
    );
}

#[test]
fn max_height_constrains_explicit() {
    // height: 400px, max-height: 300px -> height = 300.
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(400.0),
        max_height: Dimension::Length(300.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.height - 300.0).abs() < 1.0,
        "max-height should limit height to 300, got {}",
        lb.content.height
    );
}

// --- M3-6: Display::ListItem is block-level ---

#[test]
fn list_item_is_block_level() {
    assert!(is_block_level(Display::ListItem));
}

// --- M3.5-4: RTL direction margin auto centering ---

#[test]
fn rtl_margin_auto_centering_centers() {
    // Both margins auto in RTL should center the element (same as LTR).
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        direction: Direction::Rtl,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
    assert!((lb.margin.left - 200.0).abs() < f32::EPSILON);
    assert!((lb.margin.right - 200.0).abs() < f32::EPSILON);
}

#[test]
fn rtl_overconstrained_both_auto_negative() {
    // Both margins auto, overconstrained (box wider than container).
    // RTL: margin-right = 0, margin-left absorbs negative overflow.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(900.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        direction: Direction::Rtl,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.margin.right - 0.0).abs() < f32::EPSILON);
    assert!((lb.margin.left - (-100.0)).abs() < f32::EPSILON);
}

#[test]
fn rtl_overconstrained_no_auto() {
    // No auto margins, overconstrained. RTL: margin-left is recalculated.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(600.0),
        margin_left: Dimension::Length(50.0),
        margin_right: Dimension::Length(50.0),
        direction: Direction::Rtl,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // RTL: margin-right is preserved (50), margin-left is recalculated.
    // margin-left = 800 - 600 - 50 = 150.
    assert!((lb.margin.right - 50.0).abs() < f32::EPSILON);
    assert!((lb.margin.left - 150.0).abs() < f32::EPSILON);
}

#[test]
fn ltr_overconstrained_no_auto() {
    // Verify LTR behavior: margin-right is recalculated when overconstrained.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(600.0),
        margin_left: Dimension::Length(50.0),
        margin_right: Dimension::Length(50.0),
        direction: Direction::Ltr,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // LTR: margin-left is preserved (50), margin-right is recalculated.
    // margin-right = 800 - 600 - 50 = 150.
    assert!((lb.margin.left - 50.0).abs() < f32::EPSILON);
    assert!((lb.margin.right - 150.0).abs() < f32::EPSILON);
}
