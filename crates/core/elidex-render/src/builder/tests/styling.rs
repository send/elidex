use super::*;

// ---------------------------------------------------------------------------
// Text alignment
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn text_align_center_offsets_text() {
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            text_align: elidex_plugin::TextAlign::Center,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 400.0, 20.0),
            ..Default::default()
        },
    );
    let txt = dom.create_text("Hi");
    dom.append_child(p, txt);

    let font_db = elidex_text::FontDatabase::new();

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    assert!(!text_items.is_empty(), "should have text items");
    // Center-aligned text: first glyph should be shifted right from 0.
    // Exact offset = (400 - text_width) / 2, which is > 0 for any short text.
    if let crate::display_list::DisplayItem::Text { glyphs, .. } = text_items[0] {
        assert!(
            glyphs[0].position.0 > 0.0 && glyphs[0].position.0 < 400.0,
            "center-aligned text should be between 0 and container width, got x={}",
            glyphs[0].position.0
        );
    }
}

#[test]
#[allow(unused_must_use)]
fn text_align_right_offsets_text() {
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            text_align: elidex_plugin::TextAlign::Right,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 400.0, 20.0),
            ..Default::default()
        },
    );
    let txt = dom.create_text("Hi");
    dom.append_child(p, txt);

    let font_db = elidex_text::FontDatabase::new();

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    assert!(!text_items.is_empty(), "should have text items");
    // Right-aligned: offset = 400 - text_width, so glyph x > center offset.
    if let crate::display_list::DisplayItem::Text { glyphs, .. } = text_items[0] {
        assert!(
            glyphs[0].position.0 > 0.0 && glyphs[0].position.0 < 400.0,
            "right-aligned text should be between 0 and container width, got x={}",
            glyphs[0].position.0
        );
        // Right offset should be > center offset for the same text.
        // "Hi" in 400px: right offset ≈ 380+, center offset ≈ 190+.
        assert!(
            glyphs[0].position.0 > 200.0,
            "right-aligned text should be in right half, got x={}",
            glyphs[0].position.0
        );
    }
}

// ---------------------------------------------------------------------------
// Border rendering
// ---------------------------------------------------------------------------

#[test]
fn emit_borders_four_sides() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_top: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_right: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_bottom: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_left: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(12.0, 12.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // 4 border SolidRects (no background since transparent).
    let rect_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
            .count();
    assert_eq!(rect_count, 4);
}

#[test]
fn emit_borders_style_none_skipped() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            // Only top border is solid; others are none (default).
            border_top: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(2.0, 2.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Only 1 border (top), others skipped because style=none.
    assert_eq!(dl.0.len(), 1);
}

#[test]
fn emit_borders_zero_width_skipped() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_top: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 0.0,
                ..Default::default()
            }, // zero width, should be skipped
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert!(dl.0.is_empty());
}

#[test]
fn background_with_border_radius_emits_rounded_rect() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            border_radii: [10.0; 4],
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(dl.0.len(), 1);
    assert!(
        matches!(&dl.0[0], crate::display_list::DisplayItem::RoundedRect { radii, .. } if (radii[0] - 10.0).abs() < f32::EPSILON)
    );
}

#[test]
fn background_without_border_radius_emits_solid_rect() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            border_radii: [0.0; 4],
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(dl.0.len(), 1);
    assert!(matches!(
        &dl.0[0],
        crate::display_list::DisplayItem::SolidRect { .. }
    ));
}

#[test]
fn opacity_cases() {
    use elidex_plugin::CssColor;
    let cases: &[(CssColor, f32, u8)] = &[
        (CssColor::new(255, 0, 0, 200), 0.5, 100),
        (CssColor::RED, 0.0, 0),
        (CssColor::RED, 1.0, 255),
    ];
    for &(color, opacity, expected_a) in cases {
        let result = apply_opacity(color, opacity);
        assert_eq!(
            result.a, expected_a,
            "opacity={opacity}, color.a={}",
            color.a
        );
        // RGB channels should be preserved.
        assert_eq!(result.r, color.r);
        assert_eq!(result.g, color.g);
        assert_eq!(result.b, color.b);
    }
}

/// When both `border-radius` and uniform `border` are set, a single
/// `RoundedBorderRing` item is emitted instead of 4 axis-aligned `SolidRect`.
#[test]
fn border_radius_with_border_emits_rounded_ring() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            border_radii: [10.0; 4],
            border_top: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLACK,
            },
            border_right: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLACK,
            },
            border_bottom: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLACK,
            },
            border_left: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLACK,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(2.0, 2.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // 1 RoundedRect (background) + 1 RoundedBorderRing (borders).
    let rounded =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }))
            .count();
    let rings =
        dl.0.iter()
            .filter(|i| {
                matches!(
                    i,
                    crate::display_list::DisplayItem::RoundedBorderRing { .. }
                )
            })
            .count();
    let rects =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
            .count();
    assert_eq!(rounded, 1, "should have 1 RoundedRect for background");
    assert_eq!(rings, 1, "should have 1 RoundedBorderRing for borders");
    assert_eq!(rects, 0, "should have no SolidRect borders");
}

/// When border-radius is set but border colors differ, fall back to `SolidRect`.
#[test]
fn border_radius_different_colors_falls_back_to_solid_rect() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_radii: [10.0; 4],
            border_top: elidex_plugin::BorderSide {
                width: 2.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_right: elidex_plugin::BorderSide {
                width: 2.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLUE,
            },
            border_bottom: elidex_plugin::BorderSide {
                width: 2.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_left: elidex_plugin::BorderSide {
                width: 2.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::BLUE,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(2.0, 2.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Different colors → fallback to SolidRect borders.
    let rings =
        dl.0.iter()
            .filter(|i| {
                matches!(
                    i,
                    crate::display_list::DisplayItem::RoundedBorderRing { .. }
                )
            })
            .count();
    let rects =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
            .count();
    assert_eq!(
        rings, 0,
        "should not use RoundedBorderRing with mixed colors"
    );
    assert_eq!(rects, 4, "should fall back to 4 SolidRect borders");
}

#[test]
fn border_corners_no_overlap() {
    // Verify that left/right borders are inset by top/bottom widths.
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_top: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_right: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_bottom: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            border_left: elidex_plugin::BorderSide {
                width: 0.0,
                style: elidex_plugin::BorderStyle::Solid,
                color: elidex_plugin::CssColor::RED,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(5.0, 5.0, 100.0, 50.0),
            border: EdgeSizes {
                top: 3.0,
                right: 2.0,
                bottom: 3.0,
                left: 2.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let rects: Vec<_> =
        dl.0.iter()
            .filter_map(|i| match i {
                crate::display_list::DisplayItem::SolidRect { rect, .. } => Some(rect),
                _ => None,
            })
            .collect();
    assert_eq!(rects.len(), 4);
    // border-box: x=3, y=2, w=104, h=56 (content 100x50 + border 2+2 / 3+3)
    // top: full width, y=2, h=3
    let top = rects[0];
    assert!((top.y - 2.0).abs() < f32::EPSILON);
    assert!((top.height - 3.0).abs() < f32::EPSILON);
    assert!((top.width - 104.0).abs() < f32::EPSILON);
    // bottom: full width, y=55, h=3
    let bottom = rects[1];
    assert!((bottom.y - 55.0).abs() < f32::EPSILON);
    assert!((bottom.height - 3.0).abs() < f32::EPSILON);
    // right: inset by top(3)+bottom(3), height=50
    let right = rects[2];
    assert!((right.y - 5.0).abs() < f32::EPSILON); // 2 + 3
    assert!((right.height - 50.0).abs() < f32::EPSILON); // 56 - 3 - 3
                                                         // left: same inset
    let left = rects[3];
    assert!((left.y - 5.0).abs() < f32::EPSILON);
    assert!((left.height - 50.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// text-transform
// ---------------------------------------------------------------------------

#[test]
fn apply_text_transform_cases() {
    use elidex_plugin::TextTransform;
    let cases = [
        ("hello", TextTransform::Uppercase, "HELLO"),
        ("HELLO", TextTransform::Lowercase, "hello"),
        ("hello world", TextTransform::Capitalize, "Hello World"),
        // UAX #29: punctuation-adjacent word boundaries.
        ("hello-world", TextTransform::Capitalize, "Hello-World"),
        ("it's a test", TextTransform::Capitalize, "It's A Test"),
        ("Hello", TextTransform::None, "Hello"),
    ];
    for (input, transform, expected) in cases {
        assert_eq!(
            super::super::text::apply_text_transform(input, transform),
            expected,
            "input={input:?}, transform={transform:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Image rendering
// ---------------------------------------------------------------------------

#[test]
fn image_data_emits_image_item() {
    let (mut dom, img) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(10.0, 10.0, 200.0, 100.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        img,
        elidex_ecs::ImageData {
            pixels: Arc::new(vec![255u8; 4]), // 1x1 white pixel
            width: 1,
            height: 1,
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let image_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Image { .. }))
            .collect();
    assert_eq!(image_items.len(), 1);
    match &image_items[0] {
        crate::display_list::DisplayItem::Image {
            painting_area,
            image_width,
            image_height,
            ..
        } => {
            assert!((painting_area.width - 200.0).abs() < f32::EPSILON);
            assert!((painting_area.height - 100.0).abs() < f32::EPSILON);
            assert_eq!(*image_width, 1);
            assert_eq!(*image_height, 1);
        }
        _ => unreachable!(),
    }
}

#[test]
fn no_image_data_no_image_item() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let image_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Image { .. }))
            .count();
    assert_eq!(image_count, 0);
}

#[test]
fn image_opacity_zero_skipped() {
    let (mut dom, img) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            opacity: 0.0,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        img,
        elidex_ecs::ImageData {
            pixels: Arc::new(vec![255u8; 4]),
            width: 1,
            height: 1,
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let image_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Image { .. }))
            .count();
    assert_eq!(image_count, 0);
}

// ---------------------------------------------------------------------------
// Text decoration
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn text_decoration_underline_emits_solid_rect() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            text_decoration_line: elidex_plugin::TextDecorationLine {
                underline: true,
                ..elidex_plugin::TextDecorationLine::default()
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("Hello");
    dom.append_child(div, text);

    let font_db = elidex_text::FontDatabase::new();

    let dl = build_display_list(&dom, &font_db);
    // Should have: Text item + SolidRect for underline.
    let text_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .count();
    let rect_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
            .count();
    assert_eq!(text_count, 1);
    // At least 1 rect for underline (no background since transparent).
    assert!(rect_count >= 1, "expected underline rect, got {rect_count}");
}

// ---------------------------------------------------------------------------
// Styled inline runs
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn styled_span_color_preserved() {
    // <p><span style="color:red">red</span> normal</p>
    // The span text should have a different color from the parent text.
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            color: elidex_plugin::CssColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );
    let span = dom.create_element("span", Attributes::default());
    let t_red = dom.create_text("red");
    let t_normal = dom.create_text(" normal");
    dom.append_child(p, span);
    dom.append_child(span, t_red);
    dom.append_child(p, t_normal);
    dom.world_mut().insert_one(
        span,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Inline,
            font_family: test_font_family_strings(),
            color: elidex_plugin::CssColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter_map(|i| {
                if let crate::display_list::DisplayItem::Text { color, .. } = i {
                    Some(*color)
                } else {
                    None
                }
            })
            .collect();
    // Should have 2 text items (span "red" + parent "normal").
    assert_eq!(text_items.len(), 2);
    // First text item from span should be red.
    assert_eq!(
        text_items[0],
        elidex_plugin::CssColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );
    // Second text item from parent should be black.
    assert_eq!(
        text_items[1],
        elidex_plugin::CssColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        }
    );
}

// ---------------------------------------------------------------------------
// White-space collapse
// ---------------------------------------------------------------------------

#[test]
fn collapse_segments_cases() {
    use elidex_plugin::WhiteSpace;
    let cases: &[(&str, WhiteSpace, &str)] = &[
        // Normal: spaces + newlines collapse
        ("hello  \n  world", WhiteSpace::Normal, "hello world"),
        // NoWrap: same collapsing as normal
        ("hello  \n  world", WhiteSpace::NoWrap, "hello world"),
        // Pre: preserves everything
        ("hello  \n  world", WhiteSpace::Pre, "hello  \n  world"),
        // PreWrap: preserves everything
        (
            "  hello  \n  world  ",
            WhiteSpace::PreWrap,
            "  hello  \n  world  ",
        ),
        // PreLine: trailing newline preserved
        ("hello\n", WhiteSpace::PreLine, "hello\n"),
        // PreLine: trailing spaces trimmed
        ("hello   ", WhiteSpace::PreLine, "hello"),
        // PreLine: spaces before newline stripped (CSS Text §4)
        ("hello   \nworld", WhiteSpace::PreLine, "hello\nworld"),
        // CRLF normalized to LF (CSS Text §4.1)
        ("hello\r\nworld", WhiteSpace::PreLine, "hello\nworld"),
        // Bare CR normalized to LF (CSS Text §4.1)
        ("hello\rworld", WhiteSpace::Pre, "hello\nworld"),
    ];
    for (input, ws, expected) in cases {
        let segments = vec![make_segment(input)];
        let result = collapse_segments(&segments, *ws);
        assert_eq!(result.len(), 1, "input={input:?}, ws={ws:?}");
        assert_eq!(result[0].0, *expected, "input={input:?}, ws={ws:?}");
    }
}

#[test]
fn collapse_segments_pre_line_collapses_spaces_preserves_newlines() {
    let segments = vec![make_segment("hello   \n   world")];
    let result = collapse_segments(&segments, elidex_plugin::WhiteSpace::PreLine);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].0.contains('\n'),
        "pre-line should preserve newlines"
    );
    assert!(
        !result[0].0.contains("   "),
        "pre-line should collapse spaces"
    );
}
