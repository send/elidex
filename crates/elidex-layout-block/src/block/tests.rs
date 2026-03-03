use std::sync::Arc;

use super::*;
use elidex_ecs::{Attributes, ImageData};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, Direction};
use elidex_text::FontDatabase;

fn block_style() -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        ..Default::default()
    }
}

fn make_dom_with_block_div(style: ComputedStyle) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(div, style);
    (dom, div)
}

#[test]
fn width_auto_fills_containing_block() {
    let (mut dom, div) = make_dom_with_block_div(block_style());
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 800.0).abs() < f32::EPSILON);
}

#[test]
fn fixed_width_with_padding_border() {
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        padding_left: 10.0,
        padding_right: 10.0,
        border_left_width: 2.0,
        border_right_width: 2.0,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
    assert!((lb.padding.left - 10.0).abs() < f32::EPSILON);
    assert!((lb.border.left - 2.0).abs() < f32::EPSILON);
    // border box width = 200 + 10 + 10 + 2 + 2 = 224
    let bb = lb.border_box();
    assert!((bb.width - 224.0).abs() < f32::EPSILON);
}

#[test]
fn margin_auto_centering() {
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
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
fn margin_collapse_adjacent_siblings() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child1 = dom.create_element("div", Attributes::default());
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.append_child(parent, child2);

    dom.world_mut().insert_one(parent, block_style());
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_bottom: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_top: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // Without collapse: 40 + 20 + 30 + 40 = 130
    // With collapse: 40 + max(20,30) + 40 = 110
    assert!((lb.content.height - 110.0).abs() < f32::EPSILON);
}

#[test]
fn margin_collapse_negative() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child1 = dom.create_element("div", Attributes::default());
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.append_child(parent, child2);

    dom.world_mut().insert_one(parent, block_style());
    // Both negative: collapsed = min(-10, -20) = -20
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_bottom: Dimension::Length(-10.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_top: Dimension::Length(-20.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // Without collapse: 40 + (-10) + (-20) + 40 = 50
    // With collapse (both neg): 40 + min(-10,-20) + 40 = 40 + (-20) + 40 = 60
    assert!((lb.content.height - 60.0).abs() < f32::EPSILON);
}

#[test]
fn margin_collapse_mixed() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child1 = dom.create_element("div", Attributes::default());
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.append_child(parent, child2);

    dom.world_mut().insert_one(parent, block_style());
    // Mixed: collapsed = 20 + (-10) = 10
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_bottom: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            margin_top: Dimension::Length(-10.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // Without collapse: 40 + 20 + (-10) + 40 = 90
    // With collapse (mixed): 40 + (20 + (-10)) + 40 = 90
    // Actually same here because sum == collapse for mixed.
    // But the key difference: old code did max(20, -10) = 20, giving 100.
    assert!((lb.content.height - 90.0).abs() < f32::EPSILON);
}

#[test]
fn margin_auto_overconstrained() {
    // CSS 2.1 §10.3.3: overconstrained LTR — margin-left=0, margin-right absorbs overflow.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(900.0),
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 900.0).abs() < f32::EPSILON);
    // margin-left = 0 (overconstrained LTR)
    assert!(lb.margin.left.abs() < f32::EPSILON);
    // margin-right = 800 - 900 = -100
    assert!((lb.margin.right - (-100.0)).abs() < f32::EPSILON);
}

#[test]
fn overconstrained_non_auto_margins() {
    // CSS 2.1 §10.3.3: no auto margins, overconstrained LTR
    // margin-right recalculated to satisfy constraint equation.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(900.0),
        margin_left: Dimension::Length(10.0),
        margin_right: Dimension::Length(10.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 900.0).abs() < f32::EPSILON);
    assert!((lb.margin.left - 10.0).abs() < f32::EPSILON);
    // margin-right = 800 - 900 - 10 = -110
    assert!((lb.margin.right - (-110.0)).abs() < f32::EPSILON);
}

#[test]
fn percentage_width_and_margin() {
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Percentage(50.0),
        margin_left: Dimension::Percentage(10.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 1000.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 500.0).abs() < f32::EPSILON);
    assert!((lb.margin.left - 100.0).abs() < f32::EPSILON);
}

// --- M3-2: box-sizing: border-box ---

#[test]
fn box_sizing_content_box_default() {
    // Default content-box: content_width = specified width.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        padding_left: 10.0,
        padding_right: 10.0,
        border_left_width: 2.0,
        border_right_width: 2.0,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
    // border box = 200 + 20 + 4 = 224
    let bb = lb.border_box();
    assert!((bb.width - 224.0).abs() < f32::EPSILON);
}

#[test]
fn box_sizing_border_box_width() {
    // border-box: specified 200px includes padding + border.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        padding_left: 10.0,
        padding_right: 10.0,
        border_left_width: 2.0,
        border_right_width: 2.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // content = 200 - 10 - 10 - 2 - 2 = 176
    assert!((lb.content.width - 176.0).abs() < f32::EPSILON);
    // border box = 176 + 20 + 4 = 200
    let bb = lb.border_box();
    assert!((bb.width - 200.0).abs() < f32::EPSILON);
}

#[test]
fn box_sizing_border_box_height() {
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(100.0),
        padding_top: 10.0,
        padding_bottom: 10.0,
        border_top_width: 2.0,
        border_bottom_width: 2.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // content height = 100 - 10 - 10 - 2 - 2 = 76
    assert!((lb.content.height - 76.0).abs() < f32::EPSILON);
}

#[test]
fn box_sizing_border_box_percentage_width() {
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Percentage(50.0), // 50% of 800 = 400
        padding_left: 20.0,
        padding_right: 20.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // content = 400 - 20 - 20 = 360
    assert!((lb.content.width - 360.0).abs() < f32::EPSILON);
}

#[test]
fn box_sizing_border_box_auto_width_unchanged() {
    // auto width should not be affected by border-box.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Auto,
        padding_left: 20.0,
        padding_right: 20.0,
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // auto: content_width = 800 - 20 - 20 = 760 (no border-box subtraction).
    assert!((lb.content.width - 760.0).abs() < f32::EPSILON);
}

// --- M3-4: replaced element (image) layout ---

fn make_dom_with_image(style: ComputedStyle, img_w: u32, img_h: u32) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let img = dom.create_element("img", Attributes::default());
    dom.world_mut().insert_one(img, style);
    dom.world_mut().insert_one(
        img,
        ImageData {
            pixels: Arc::new(vec![0u8; (img_w * img_h * 4) as usize]),
            width: img_w,
            height: img_h,
        },
    );
    (dom, img)
}

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
        padding_left: 10.0,
        padding_right: 10.0,
        padding_top: 10.0,
        padding_bottom: 10.0,
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

// --- M3-5: Parent-child margin collapse ---

#[test]
fn parent_child_first_child_margin_collapse() {
    // Parent margin-top=10, first child margin-top=20, no border/padding.
    // Collapsed margin = max(10, 20) = 20.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(20.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // Parent's margin-top should collapse with child's: max(10, 20) = 20.
    assert!(
        (lb.margin.top - 20.0).abs() < f32::EPSILON,
        "expected collapsed margin-top=20, got {}",
        lb.margin.top
    );
    // Child's content.y should reflect the collapsed margin (20), not original (10).
    let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
    assert!(
        (child_lb.content.y - 20.0).abs() < f32::EPSILON,
        "expected child content.y=20 (collapsed margin), got {}",
        child_lb.content.y
    );
}

#[test]
fn parent_child_margin_collapse_shifts_grandchildren() {
    // Parent margin-top=10, child margin-top=20, grandchild inside child.
    // After collapse, both child and grandchild must shift.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    let grandchild = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.append_child(child, grandchild);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        grandchild,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.margin.top - 20.0).abs() < f32::EPSILON,
        "collapsed margin-top should be 20, got {}",
        lb.margin.top
    );
    let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
    let grandchild_lb = dom.world().get::<&LayoutBox>(grandchild).unwrap();
    // Grandchild should be inside child, which is at content.y=20.
    assert!(
        (grandchild_lb.content.y - child_lb.content.y).abs() < f32::EPSILON,
        "grandchild should be at child's content.y={}, got {}",
        child_lb.content.y,
        grandchild_lb.content.y
    );
}

#[test]
fn parent_child_last_child_margin_collapse() {
    // Parent margin-bottom=5, last child margin-bottom=15, no border/padding,
    // height:auto -> bottom margin collapses.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_bottom: Dimension::Length(5.0),
            // height defaults to Auto
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_bottom: Dimension::Length(15.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // Parent's margin-bottom should collapse with child's: max(5, 15) = 15.
    assert!(
        (lb.margin.bottom - 15.0).abs() < f32::EPSILON,
        "expected collapsed margin-bottom=15, got {}",
        lb.margin.bottom
    );
    // Child's content.y should be at top (no top margin on either).
    let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
    assert!(
        child_lb.content.y.abs() < f32::EPSILON,
        "expected child content.y=0, got {}",
        child_lb.content.y
    );
}

#[test]
fn parent_child_no_bottom_collapse_with_explicit_height() {
    // Parent has height: 200px -> bottom margin does NOT collapse (CSS 2.1 §8.3.1).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_bottom: Dimension::Length(5.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_bottom: Dimension::Length(15.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // height is explicit -> no bottom collapse. Parent keeps its own margin-bottom.
    assert!(
        (lb.margin.bottom - 5.0).abs() < f32::EPSILON,
        "expected margin-bottom=5 (no collapse), got {}",
        lb.margin.bottom
    );
}

#[test]
fn parent_child_no_collapse_with_border() {
    // Parent has border-top > 0 -> no first-child collapse.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            border_top_width: 1.0,
            border_top_style: elidex_plugin::BorderStyle::Solid,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(20.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // border-top prevents collapse: parent keeps its own margin-top.
    assert!(
        (lb.margin.top - 10.0).abs() < f32::EPSILON,
        "expected margin-top=10 (no collapse), got {}",
        lb.margin.top
    );
}

#[test]
fn parent_child_no_collapse_with_padding() {
    // Parent has padding-top > 0 -> no first-child collapse.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            padding_top: 5.0,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(20.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
    // padding-top prevents collapse: parent keeps its own margin-top.
    assert!(
        (lb.margin.top - 10.0).abs() < f32::EPSILON,
        "expected margin-top=10 (no collapse), got {}",
        lb.margin.top
    );
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

// --- M3-6: min-width / max-width / min-height / max-height ---

#[test]
fn min_width_constrains_auto() {
    // width: auto in 800px container, min-width: 900px -> width = 900.
    let style = ComputedStyle {
        display: Display::Block,
        min_width: Dimension::Length(900.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.width - 900.0).abs() < 1.0,
        "min-width should force width to 900, got {}",
        lb.content.width
    );
}

#[test]
fn max_width_constrains_auto() {
    // width: auto in 800px container, max-width: 500px -> width = 500.
    let style = ComputedStyle {
        display: Display::Block,
        max_width: Dimension::Length(500.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.width - 500.0).abs() < 1.0,
        "max-width should limit width to 500, got {}",
        lb.content.width
    );
}

#[test]
fn min_width_overrides_max_width() {
    // CSS spec: when min > max, min wins.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(400.0),
        min_width: Dimension::Length(600.0),
        max_width: Dimension::Length(500.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.width - 600.0).abs() < 1.0,
        "min-width wins over max-width, got {}",
        lb.content.width
    );
}

#[test]
fn max_width_none_is_unconstrained() {
    // max-width: Auto (=none) should not constrain.
    let style = ComputedStyle {
        display: Display::Block,
        max_width: Dimension::Auto,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.width - 800.0).abs() < 1.0,
        "max-width: none should not constrain, got {}",
        lb.content.width
    );
}

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

#[test]
fn min_width_percentage() {
    // min-width: 50% of 800px = 400px, width: 200px -> constrained to 400.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(200.0),
        min_width: Dimension::Percentage(50.0),
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.content.width - 400.0).abs() < 1.0,
        "min-width 50% of 800 = 400, got {}",
        lb.content.width
    );
}

// --- M3-6: Display::ListItem is block-level ---

#[test]
fn list_item_is_block_level() {
    assert!(is_block_level(Display::ListItem));
}

// --- L15: border-box + min/max ---

#[test]
fn min_width_border_box_subtracts_padding() {
    // border-box: min-width: 200px with 20px padding each side.
    // Content min-width = 200 - 40 = 160. width: auto -> fills 800 - 40 = 760 > 160, no effect.
    // width: 100px -> content 100, but min-width 160 wins.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(100.0),
        min_width: Dimension::Length(200.0),
        box_sizing: BoxSizing::BorderBox,
        padding_left: 20.0,
        padding_right: 20.0,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // border-box width: 100px, padding: 40px -> content = 60px.
    // border-box min-width: 200px -> content min = 200 - 40 = 160.
    // Final content width = max(60, 160) = 160.
    assert!(
        (lb.content.width - 160.0).abs() < 1.0,
        "border-box min-width should subtract padding, got {}",
        lb.content.width
    );
}

#[test]
fn max_width_border_box_subtracts_padding() {
    // border-box: max-width: 200px with 20px padding each side.
    // Content max-width = 200 - 40 = 160.
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Length(300.0),
        max_width: Dimension::Length(200.0),
        box_sizing: BoxSizing::BorderBox,
        padding_left: 20.0,
        padding_right: 20.0,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // border-box width: 300px -> content = 260px.
    // border-box max-width: 200px -> content max = 160.
    // Final content width = min(260, 160) = 160.
    assert!(
        (lb.content.width - 160.0).abs() < 1.0,
        "border-box max-width should subtract padding, got {}",
        lb.content.width
    );
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
