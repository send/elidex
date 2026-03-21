use super::*;
use elidex_plugin::background::PositionEdge;

fn area(w: f32, h: f32) -> Rect {
    Rect::new(0.0, 0.0, w, h)
}

// --- resolve_bg_size ---

#[test]
fn bg_size_auto_auto_uses_intrinsic() {
    let size = resolve_bg_size(&BgSize::default(), &area(400.0, 300.0), 100, 50);
    assert_eq!(size, (100.0, 50.0));
}

#[test]
fn bg_size_cover() {
    // 100x50 image in 400x300 area → scale by max(4.0, 6.0) = 6.0
    let size = resolve_bg_size(&BgSize::Cover, &area(400.0, 300.0), 100, 50);
    assert!((size.0 - 600.0).abs() < 0.1);
    assert!((size.1 - 300.0).abs() < 0.1);
}

#[test]
fn bg_size_contain() {
    // 100x50 image in 400x300 area → scale by min(4.0, 6.0) = 4.0
    let size = resolve_bg_size(&BgSize::Contain, &area(400.0, 300.0), 100, 50);
    assert!((size.0 - 400.0).abs() < 0.1);
    assert!((size.1 - 200.0).abs() < 0.1);
}

#[test]
fn bg_size_explicit_both() {
    let size = resolve_bg_size(
        &BgSize::Explicit(
            Some(BgSizeDimension::Length(200.0)),
            Some(BgSizeDimension::Length(100.0)),
        ),
        &area(400.0, 300.0),
        100,
        50,
    );
    assert_eq!(size, (200.0, 100.0));
}

#[test]
fn bg_size_explicit_width_auto_height() {
    // width=200px, height=auto → height = 200 * 50 / 100 = 100
    let size = resolve_bg_size(
        &BgSize::Explicit(Some(BgSizeDimension::Length(200.0)), None),
        &area(400.0, 300.0),
        100,
        50,
    );
    assert_eq!(size, (200.0, 100.0));
}

#[test]
fn bg_size_percentage() {
    let size = resolve_bg_size(
        &BgSize::Explicit(
            Some(BgSizeDimension::Percentage(50.0)),
            Some(BgSizeDimension::Percentage(50.0)),
        ),
        &area(400.0, 300.0),
        100,
        50,
    );
    assert_eq!(size, (200.0, 150.0));
}

// --- resolve_bg_position ---

#[test]
fn bg_position_default_zero() {
    let pos = resolve_bg_position(&BgPosition::default(), &area(400.0, 300.0), (100.0, 50.0));
    // 0% of (400-100) = 0.0, 0% of (300-50) = 0.0
    assert_eq!(pos, (0.0, 0.0));
}

#[test]
fn bg_position_center() {
    let pos = resolve_bg_position(
        &BgPosition {
            x: BgPositionAxis::Percentage(50.0),
            y: BgPositionAxis::Percentage(50.0),
        },
        &area(400.0, 300.0),
        (100.0, 50.0),
    );
    // 50% of (400-100) = 150.0, 50% of (300-50) = 125.0
    assert!((pos.0 - 150.0).abs() < 0.1);
    assert!((pos.1 - 125.0).abs() < 0.1);
}

#[test]
fn bg_position_length() {
    let pos = resolve_bg_position(
        &BgPosition {
            x: BgPositionAxis::Length(10.0),
            y: BgPositionAxis::Length(20.0),
        },
        &area(400.0, 300.0),
        (100.0, 50.0),
    );
    assert_eq!(pos, (10.0, 20.0));
}

#[test]
fn bg_position_right_bottom_edge() {
    let pos = resolve_bg_position(
        &BgPosition {
            x: BgPositionAxis::Edge(PositionEdge::Right, 10.0),
            y: BgPositionAxis::Edge(PositionEdge::Bottom, 20.0),
        },
        &area(400.0, 300.0),
        (100.0, 50.0),
    );
    // right 10px → 400 - 100 - 10 = 290
    // bottom 20px → 300 - 50 - 20 = 230
    assert!((pos.0 - 290.0).abs() < 0.1);
    assert!((pos.1 - 230.0).abs() < 0.1);
}

// --- compute_inner_radii ---

#[test]
fn inner_radii_uniform_border() {
    use elidex_plugin::EdgeSizes;
    let border = EdgeSizes {
        top: 3.0,
        right: 3.0,
        bottom: 3.0,
        left: 3.0,
    };
    let inner = compute_inner_radii([10.0; 4], &border);
    // Per-axis: (10-3, 10-3) = (7, 7) → min = 7
    assert_eq!(inner, [7.0, 7.0, 7.0, 7.0]);
}

#[test]
fn inner_radii_asymmetric_border() {
    use elidex_plugin::EdgeSizes;
    let border = EdgeSizes {
        top: 5.0,
        right: 2.0,
        bottom: 3.0,
        left: 8.0,
    };
    let inner = compute_inner_radii([10.0, 10.0, 10.0, 10.0], &border);
    // top-left: h=10-8=2, v=10-5=5 → min=2
    // top-right: h=10-2=8, v=10-5=5 → min=5
    // bottom-right: h=10-2=8, v=10-3=7 → min=7
    // bottom-left: h=10-8=2, v=10-3=7 → min=2
    assert_eq!(inner, [2.0, 5.0, 7.0, 2.0]);
}

#[test]
fn inner_radii_per_axis_asymmetric() {
    use elidex_plugin::EdgeSizes;
    let border = EdgeSizes {
        top: 2.0,
        right: 2.0,
        bottom: 2.0,
        left: 10.0,
    };
    let per_axis = compute_inner_radii_per_axis([10.0; 4], &border);
    // top-left: h=10-10=0, v=10-2=8
    assert_eq!(per_axis[0], (0.0, 8.0));
    // top-right: h=10-2=8, v=10-2=8
    assert_eq!(per_axis[1], (8.0, 8.0));
    // bottom-right: h=10-2=8, v=10-2=8
    assert_eq!(per_axis[2], (8.0, 8.0));
    // bottom-left: h=10-10=0, v=10-2=8
    assert_eq!(per_axis[3], (0.0, 8.0));
    // min(h,v) used for RoundedRect
    let inner = compute_inner_radii([10.0; 4], &border);
    assert_eq!(inner, [0.0, 8.0, 8.0, 0.0]);
}

#[test]
fn inner_radii_clamped_to_zero() {
    use elidex_plugin::EdgeSizes;
    let border = EdgeSizes {
        top: 15.0,
        right: 15.0,
        bottom: 15.0,
        left: 15.0,
    };
    let inner = compute_inner_radii([10.0; 4], &border);
    // 10 - 15 = -5, clamped to 0
    assert_eq!(inner, [0.0, 0.0, 0.0, 0.0]);
}

// --- emit_borders: dashed/dotted ---

/// Helper to build a `LayoutBox` + `ComputedStyle` with uniform border for testing.
fn make_bordered_box(
    border_width: f32,
    border_style: BorderStyle,
    border_color: CssColor,
) -> (LayoutBox, ComputedStyle) {
    use elidex_plugin::{BorderSide, EdgeSizes};
    let lb = LayoutBox {
        content: Rect::new(10.0, 10.0, 100.0, 50.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes {
            top: border_width,
            right: border_width,
            bottom: border_width,
            left: border_width,
        },
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let side = BorderSide {
        width: border_width,
        style: border_style,
        color: border_color,
    };
    let style = ComputedStyle {
        border_top: side,
        border_right: side,
        border_bottom: side,
        border_left: side,
        ..ComputedStyle::default()
    };
    (lb, style)
}

#[test]
fn emit_borders_dashed_produces_styled_segments() {
    let (lb, style) = make_bordered_box(2.0, BorderStyle::Dashed, CssColor::RED);
    let mut dl = DisplayList::default();
    emit_borders(&lb, &style, &mut dl);
    // 4 sides → 4 StyledBorderSegment items
    assert_eq!(dl.len(), 4);
    for item in dl.iter() {
        match item {
            DisplayItem::StyledBorderSegment {
                width,
                dashes,
                round_caps,
                ..
            } => {
                assert!((width - 2.0).abs() < f32::EPSILON);
                assert_eq!(dashes.len(), 2);
                // Dash pattern: [3*width, width] = [6.0, 2.0]
                assert!((dashes[0] - 6.0).abs() < f32::EPSILON);
                assert!((dashes[1] - 2.0).abs() < f32::EPSILON);
                assert!(!round_caps);
            }
            other => panic!("Expected StyledBorderSegment, got {other:?}"),
        }
    }
}

#[test]
fn emit_borders_dotted_produces_round_cap_segments() {
    let (lb, style) = make_bordered_box(3.0, BorderStyle::Dotted, CssColor::BLUE);
    let mut dl = DisplayList::default();
    emit_borders(&lb, &style, &mut dl);
    assert_eq!(dl.len(), 4);
    for item in dl.iter() {
        match item {
            DisplayItem::StyledBorderSegment {
                width,
                dashes,
                round_caps,
                ..
            } => {
                assert!((width - 3.0).abs() < f32::EPSILON);
                assert_eq!(dashes.len(), 2);
                assert!(dashes[0] < 0.01); // near-zero dash
                assert!((dashes[1] - 6.0).abs() < f32::EPSILON); // 2*width gap
                assert!(round_caps);
            }
            other => panic!("Expected StyledBorderSegment, got {other:?}"),
        }
    }
}

#[test]
fn emit_borders_solid_still_produces_solid_rect() {
    let (lb, style) = make_bordered_box(2.0, BorderStyle::Solid, CssColor::BLACK);
    let mut dl = DisplayList::default();
    emit_borders(&lb, &style, &mut dl);
    // All 4 sides should be SolidRect
    assert_eq!(dl.len(), 4);
    for item in dl.iter() {
        assert!(
            matches!(item, DisplayItem::SolidRect { .. }),
            "Expected SolidRect, got {item:?}"
        );
    }
}

#[test]
fn emit_borders_none_produces_nothing() {
    let (lb, style) = make_bordered_box(2.0, BorderStyle::None, CssColor::BLACK);
    let mut dl = DisplayList::default();
    emit_borders(&lb, &style, &mut dl);
    assert!(dl.is_empty());
}

// --- emit_column_rules ---

fn make_column_rule_fixtures(
    rule_style: BorderStyle,
    rule_width: f32,
    rule_color: CssColor,
    segments: Vec<(u32, f32, f32)>,
) -> (LayoutBox, ComputedStyle, elidex_plugin::MulticolInfo) {
    use elidex_plugin::EdgeSizes;
    let lb = LayoutBox {
        content: Rect::new(10.0, 10.0, 600.0, 200.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        column_rule_style: rule_style,
        column_rule_width: rule_width,
        column_rule_color: rule_color,
        ..ComputedStyle::default()
    };
    let info = elidex_plugin::MulticolInfo {
        column_width: 290.0,
        column_gap: 20.0,
        writing_mode: elidex_plugin::WritingMode::HorizontalTb,
        segments,
    };
    (lb, style, info)
}

#[test]
fn column_rule_solid_two_columns() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::Solid,
        2.0,
        CssColor::BLACK,
        vec![(2, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    // 2 columns → 1 rule
    assert_eq!(dl.len(), 1);
}

#[test]
fn column_rule_none_style_no_output() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::None,
        2.0,
        CssColor::BLACK,
        vec![(2, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    assert!(dl.is_empty());
}

#[test]
fn column_rule_zero_width_no_output() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::Solid,
        0.0,
        CssColor::BLACK,
        vec![(2, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    assert!(dl.is_empty());
}

#[test]
fn column_rule_three_columns_two_rules() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::Solid,
        1.0,
        CssColor::BLACK,
        vec![(3, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    // 3 columns → 2 rules
    assert_eq!(dl.len(), 2);
}

#[test]
fn column_rule_dashed() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::Dashed,
        2.0,
        CssColor::BLACK,
        vec![(2, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    // Dashed renders as multiple segments
    assert!(!dl.is_empty());
}

#[test]
fn column_rule_single_column_no_rule() {
    let (lb, style, info) = make_column_rule_fixtures(
        BorderStyle::Solid,
        2.0,
        CssColor::BLACK,
        vec![(1, 0.0, 200.0)],
    );
    let mut dl = DisplayList::default();
    emit_column_rules(&lb, &style, &info, &mut dl);
    assert!(dl.is_empty());
}
