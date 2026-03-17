use elidex_plugin::WritingMode;

use super::*;

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
fn box_sizing_border_box_percentage_width() {
    let style = ComputedStyle {
        display: Display::Block,
        width: Dimension::Percentage(50.0), // 50% of 800 = 400
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::Length(20.0),
            bottom: Dimension::ZERO,
            left: Dimension::Length(20.0),
        },
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
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::Length(20.0),
            bottom: Dimension::ZERO,
            left: Dimension::Length(20.0),
        },
        box_sizing: BoxSizing::BorderBox,
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();

    let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
    // auto: content_width = 800 - 20 - 20 = 760 (no border-box subtraction).
    assert!((lb.content.width - 760.0).abs() < f32::EPSILON);
}

#[test]
fn margin_auto_overconstrained() {
    // CSS 2.1 $10.3.3: overconstrained LTR -- margin-left=0, margin-right absorbs overflow.
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
    // CSS 2.1 $10.3.3: no auto margins, overconstrained LTR
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

// --- M3-6: min-width / max-width ---

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
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::Length(20.0),
            bottom: Dimension::ZERO,
            left: Dimension::Length(20.0),
        },
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
        padding: EdgeSizes {
            top: Dimension::ZERO,
            right: Dimension::Length(20.0),
            bottom: Dimension::ZERO,
            left: Dimension::Length(20.0),
        },
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

// --- Vertical writing mode axis swap ---

#[test]
fn vertical_rl_inline_only_swaps_axis() {
    // In vertical-rl with auto width and inline-only children,
    // the block-axis result from inline layout (total column width)
    // should become the physical width, and the inline-axis size
    // (height) should be the containing height / inline constraint.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    let text = dom.create_text("Hello");
    dom.append_child(parent, text);
    let style = ComputedStyle {
        display: Display::Block,
        writing_mode: WritingMode::VerticalRl,
        font_family: vec!["Arial".to_string(), "Helvetica".to_string()],
        font_size: 16.0,
        ..Default::default()
    };
    dom.world_mut().insert_one(parent, style);
    let font_db = FontDatabase::new();

    // Use layout_block_with_height so containing_height is known.
    let lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), 0.0, 0.0, &font_db);

    // In vertical mode with auto width, content_width should be the
    // block-axis result (column width ≈ font_size), not 800.
    // The key assertion: width should NOT be 800 (the containing width).
    // It should be much smaller — roughly one column width (≈ font_size).
    assert!(
        lb.content.width < 100.0,
        "vertical-rl auto width should shrink to column width, got {}",
        lb.content.width
    );
    // Height should be the inline-axis constraint (600.0 containing height).
    assert!(
        (lb.content.height - 600.0).abs() < 1.0,
        "vertical-rl height should be inline-axis size (600), got {}",
        lb.content.height
    );
}

#[test]
fn vertical_lr_explicit_width_unchanged() {
    // When width is explicitly set, vertical axis swap should not override it.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    let text = dom.create_text("Hello");
    dom.append_child(parent, text);
    let style = ComputedStyle {
        display: Display::Block,
        writing_mode: WritingMode::VerticalLr,
        width: Dimension::Length(200.0),
        font_family: vec!["Arial".to_string()],
        font_size: 16.0,
        ..Default::default()
    };
    dom.world_mut().insert_one(parent, style);
    let font_db = FontDatabase::new();

    let lb = layout_block_with_height(&mut dom, parent, 800.0, Some(600.0), 0.0, 0.0, &font_db);

    // Explicit width should be preserved.
    assert!(
        (lb.content.width - 200.0).abs() < 1.0,
        "vertical-lr explicit width should be 200, got {}",
        lb.content.width
    );
}

// --- Percentage padding (CSS 2.1 §8.4) ---

#[test]
fn padding_percentage_resolves_to_containing_width() {
    // padding: 10% on a child inside a 400px containing block → 40px each side
    let style = ComputedStyle {
        display: Display::Block,
        padding: EdgeSizes {
            top: Dimension::Percentage(10.0),
            right: Dimension::Percentage(10.0),
            bottom: Dimension::Percentage(10.0),
            left: Dimension::Percentage(10.0),
        },
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 400.0, 0.0, 0.0, &font_db);
    assert!(
        (lb.padding.top - 40.0).abs() < f32::EPSILON,
        "padding-top: 10% of 400 = 40, got {}",
        lb.padding.top
    );
    assert!((lb.padding.right - 40.0).abs() < f32::EPSILON);
    assert!((lb.padding.bottom - 40.0).abs() < f32::EPSILON);
    assert!((lb.padding.left - 40.0).abs() < f32::EPSILON);
    // Content width = 400 - 40*2 = 320
    assert!(
        (lb.content.width - 320.0).abs() < f32::EPSILON,
        "content width = {}",
        lb.content.width
    );
}

#[test]
fn padding_percentage_top_bottom_use_width() {
    // CSS 2.1 §8.4: padding-top/bottom percentages refer to containing block WIDTH, not height
    let style = ComputedStyle {
        display: Display::Block,
        padding: EdgeSizes {
            top: Dimension::Percentage(25.0),
            right: Dimension::ZERO,
            bottom: Dimension::Percentage(25.0),
            left: Dimension::ZERO,
        },
        ..Default::default()
    };
    let (mut dom, div) = make_dom_with_block_div(style);
    let font_db = FontDatabase::new();
    let lb = layout_block(&mut dom, div, 200.0, 0.0, 0.0, &font_db);
    // 25% of 200px width = 50px for both top and bottom
    assert!(
        (lb.padding.top - 50.0).abs() < f32::EPSILON,
        "padding-top: 25% of 200 = 50, got {}",
        lb.padding.top
    );
    assert!(
        (lb.padding.bottom - 50.0).abs() < f32::EPSILON,
        "padding-bottom: 25% of 200 = 50, got {}",
        lb.padding.bottom
    );
}
