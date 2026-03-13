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
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 10.0,
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
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 10.0,
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
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 10.0,
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
            top: 0.0,
            right: 20.0,
            bottom: 0.0,
            left: 20.0,
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
            top: 0.0,
            right: 20.0,
            bottom: 0.0,
            left: 20.0,
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
            top: 0.0,
            right: 20.0,
            bottom: 0.0,
            left: 20.0,
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
            top: 0.0,
            right: 20.0,
            bottom: 0.0,
            left: 20.0,
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
