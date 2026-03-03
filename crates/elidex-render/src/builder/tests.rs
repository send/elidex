use std::sync::Arc;

use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{EdgeSizes, Rect};

/// Font families used across tests. Covers common system fonts on
/// Linux, macOS, and Windows so that at least one is available on CI.
const TEST_FONT_FAMILIES: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];

/// Build a `Vec<String>` from [`TEST_FONT_FAMILIES`] for `ComputedStyle`.
fn test_font_family_strings() -> Vec<String> {
    TEST_FONT_FAMILIES
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Common test setup: creates a DOM with a root, one block element with a
/// [`ComputedStyle`] and [`LayoutBox`], and returns `(dom, element)`.
///
/// `style_fn` receives a default `ComputedStyle` with `display: Block` and
/// `test_font_family_strings()` pre-filled; callers can override fields.
fn setup_block_element(
    style: elidex_plugin::ComputedStyle,
    layout: elidex_plugin::LayoutBox,
) -> (elidex_ecs::EcsDom, elidex_ecs::Entity) {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let elem = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(root, elem);
    let _ = dom.world_mut().insert_one(elem, style);
    let _ = dom.world_mut().insert_one(elem, layout);
    (dom, elem)
}

/// Return `true` if test fonts are available on this system.
fn fonts_available(font_db: &elidex_text::FontDatabase) -> bool {
    font_db.query(TEST_FONT_FAMILIES, 400).is_some()
}

#[test]
fn empty_dom_empty_display_list() {
    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert!(dl.0.is_empty());
}

#[test]
fn background_color_emits_solid_rect() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(dl.0.len(), 1);
    let crate::display_list::DisplayItem::SolidRect { rect, color } = &dl.0[0] else {
        panic!("expected SolidRect");
    };
    assert_eq!(*color, elidex_plugin::CssColor::RED);
    assert!((rect.width - 100.0).abs() < f32::EPSILON);
}

#[test]
fn transparent_background_no_item() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert!(dl.0.is_empty());
}

#[test]
#[allow(unused_must_use)]
fn text_node_emits_text_item() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let text = dom.create_text("Hello");
    dom.append_child(div, text);

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    assert_eq!(text_items.len(), 1);
    let crate::display_list::DisplayItem::Text {
        glyphs, font_size, ..
    } = &text_items[0]
    else {
        unreachable!();
    };
    assert_eq!(glyphs.len(), 5); // "Hello" = 5 glyphs
    assert!((*font_size - 16.0).abs() < f32::EPSILON);
}

#[test]
#[allow(unused_must_use)]
fn nested_elements_painter_order() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("div", Attributes::default());
    dom.append_child(root, outer);
    dom.append_child(outer, inner);

    dom.world_mut().insert_one(
        outer,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        outer,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
            },
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        inner,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::BLUE,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        inner,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 180.0,
                height: 80.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    // Painter's order: outer first, inner second.
    assert_eq!(dl.0.len(), 2);
    match (&dl.0[0], &dl.0[1]) {
        (
            crate::display_list::DisplayItem::SolidRect {
                color: c1,
                rect: r1,
            },
            crate::display_list::DisplayItem::SolidRect {
                color: c2,
                rect: r2,
            },
        ) => {
            assert_eq!(*c1, elidex_plugin::CssColor::RED);
            assert_eq!(*c2, elidex_plugin::CssColor::BLUE);
            assert!((r1.width - 200.0).abs() < f32::EPSILON);
            assert!((r2.width - 180.0).abs() < f32::EPSILON);
        }
        _ => panic!("expected two SolidRects"),
    }
}

#[test]
#[allow(unused_must_use)]
fn display_none_skipped() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let visible = dom.create_element("div", Attributes::default());
    let hidden = dom.create_element("div", Attributes::default());
    dom.append_child(root, visible);
    dom.append_child(root, hidden);

    dom.world_mut().insert_one(
        visible,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        visible,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        hidden,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::None,
            background_color: elidex_plugin::CssColor::BLUE,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        hidden,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 50.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(dl.0.len(), 1);
    let crate::display_list::DisplayItem::SolidRect { color, .. } = &dl.0[0] else {
        panic!("expected SolidRect");
    };
    assert_eq!(*color, elidex_plugin::CssColor::RED);
}

#[test]
fn background_uses_border_box() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::GREEN,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 20.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
            padding: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
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
    assert_eq!(dl.0.len(), 1);
    let crate::display_list::DisplayItem::SolidRect { rect, .. } = &dl.0[0] else {
        panic!("expected SolidRect");
    };
    // border box: x = 20 - 5 - 2 = 13, width = 100 + 10 + 4 = 114
    assert!((rect.x - 13.0).abs() < f32::EPSILON);
    assert!((rect.y - 13.0).abs() < f32::EPSILON);
    assert!((rect.width - 114.0).abs() < f32::EPSILON);
    assert!((rect.height - 64.0).abs() < f32::EPSILON);
}

#[test]
#[allow(unused_must_use)]
fn whitespace_only_text_node_skipped() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let ws = dom.create_text("   \n   ");
    dom.append_child(div, ws);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Whitespace-only text should produce no display items.
    assert!(dl.0.is_empty());
}

#[test]
#[allow(unused_must_use)]
fn inline_elements_text_collected() {
    // <p>Hello <strong>world</strong>!</p>
    // Should produce a single "Hello world!" text item.
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let t1 = dom.create_text("Hello ");
    let strong = dom.create_element("strong", Attributes::default());
    let t2 = dom.create_text("world");
    let t3 = dom.create_text("!");
    dom.append_child(p, t1);
    dom.append_child(p, strong);
    dom.append_child(strong, t2);
    dom.append_child(p, t3);
    // strong is inline — no LayoutBox, but has ComputedStyle.
    dom.world_mut().insert_one(
        strong,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Inline,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    // Styled inline runs: one text item per styled segment.
    // "Hello " (parent style), "world" (strong style), "!" (parent style).
    assert_eq!(text_items.len(), 3);
    let total_glyphs: usize = text_items
        .iter()
        .map(|item| {
            let crate::display_list::DisplayItem::Text { glyphs, .. } = item else {
                unreachable!();
            };
            glyphs.len()
        })
        .sum();
    // "Hello world!" = 12 glyphs total across 3 segments.
    assert_eq!(total_glyphs, 12);
}

// L9: text-align center/right in builder
#[test]
#[allow(unused_must_use)]
fn text_align_center_offsets_text() {
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            text_align: elidex_plugin::TextAlign::Center,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let txt = dom.create_text("Hi");
    dom.append_child(p, txt);

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }
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
            glyphs[0].x > 0.0 && glyphs[0].x < 400.0,
            "center-aligned text should be between 0 and container width, got x={}",
            glyphs[0].x
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
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let txt = dom.create_text("Hi");
    dom.append_child(p, txt);

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }
    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    assert!(!text_items.is_empty(), "should have text items");
    // Right-aligned: offset = 400 - text_width, so glyph x > center offset.
    if let crate::display_list::DisplayItem::Text { glyphs, .. } = text_items[0] {
        assert!(
            glyphs[0].x > 0.0 && glyphs[0].x < 400.0,
            "right-aligned text should be between 0 and container width, got x={}",
            glyphs[0].x
        );
        // Right offset should be > center offset for the same text.
        // "Hi" in 400px: right offset ≈ 380+, center offset ≈ 190+.
        assert!(
            glyphs[0].x > 200.0,
            "right-aligned text should be in right half, got x={}",
            glyphs[0].x
        );
    }
}

// --- M3-2: border rendering tests ---

#[test]
fn emit_borders_four_sides() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_top_style: elidex_plugin::BorderStyle::Solid,
            border_right_style: elidex_plugin::BorderStyle::Solid,
            border_bottom_style: elidex_plugin::BorderStyle::Solid,
            border_left_style: elidex_plugin::BorderStyle::Solid,
            border_top_color: elidex_plugin::CssColor::RED,
            border_right_color: elidex_plugin::CssColor::RED,
            border_bottom_color: elidex_plugin::CssColor::RED,
            border_left_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 12.0,
                y: 12.0,
                width: 100.0,
                height: 50.0,
            },
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
            border_top_style: elidex_plugin::BorderStyle::Solid,
            border_top_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 2.0,
                y: 2.0,
                width: 100.0,
                height: 50.0,
            },
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
            border_top_style: elidex_plugin::BorderStyle::Solid,
            border_top_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
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
            border_radius: 10.0,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(dl.0.len(), 1);
    assert!(
        matches!(&dl.0[0], crate::display_list::DisplayItem::RoundedRect { radius, .. } if (*radius - 10.0).abs() < f32::EPSILON)
    );
}

#[test]
fn background_without_border_radius_emits_solid_rect() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            border_radius: 0.0,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
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

/// Known Phase 4 limitation: when both `border-radius` and `border` are
/// set, the background is a `RoundedRect` but borders are axis-aligned
/// `SolidRect` items. Borders do not follow rounded corners.
#[test]
fn border_radius_with_border_known_limitation() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            border_radius: 10.0,
            border_top_style: elidex_plugin::BorderStyle::Solid,
            border_right_style: elidex_plugin::BorderStyle::Solid,
            border_bottom_style: elidex_plugin::BorderStyle::Solid,
            border_left_style: elidex_plugin::BorderStyle::Solid,
            border_top_color: elidex_plugin::CssColor::BLACK,
            border_right_color: elidex_plugin::CssColor::BLACK,
            border_bottom_color: elidex_plugin::CssColor::BLACK,
            border_left_color: elidex_plugin::CssColor::BLACK,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 2.0,
                y: 2.0,
                width: 100.0,
                height: 50.0,
            },
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
    // 1 RoundedRect (background) + 4 SolidRect (borders).
    let rounded =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }))
            .count();
    let rects =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
            .count();
    assert_eq!(rounded, 1);
    assert_eq!(rects, 4);
}

#[test]
fn border_corners_no_overlap() {
    // Verify that left/right borders are inset by top/bottom widths.
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            border_top_style: elidex_plugin::BorderStyle::Solid,
            border_right_style: elidex_plugin::BorderStyle::Solid,
            border_bottom_style: elidex_plugin::BorderStyle::Solid,
            border_left_style: elidex_plugin::BorderStyle::Solid,
            border_top_color: elidex_plugin::CssColor::RED,
            border_right_color: elidex_plugin::CssColor::RED,
            border_bottom_color: elidex_plugin::CssColor::RED,
            border_left_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 5.0,
                y: 5.0,
                width: 100.0,
                height: 50.0,
            },
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

// --- M3-1: text-transform tests ---

#[test]
fn apply_text_transform_cases() {
    use elidex_plugin::TextTransform;
    let cases = [
        ("hello", TextTransform::Uppercase, "HELLO"),
        ("HELLO", TextTransform::Lowercase, "hello"),
        ("hello world", TextTransform::Capitalize, "Hello World"),
        ("Hello", TextTransform::None, "Hello"),
    ];
    for (input, transform, expected) in cases {
        assert_eq!(
            super::apply_text_transform(input, transform),
            expected,
            "input={input:?}, transform={transform:?}"
        );
    }
}

// --- M3-4: image rendering tests ---

#[test]
fn image_data_emits_image_item() {
    let (mut dom, img) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::TRANSPARENT,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 200.0,
                height: 100.0,
            },
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
            rect,
            image_width,
            image_height,
            ..
        } => {
            assert!((rect.width - 200.0).abs() < f32::EPSILON);
            assert!((rect.height - 100.0).abs() < f32::EPSILON);
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
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
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
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
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

#[test]
#[allow(unused_must_use)]
fn text_decoration_underline_emits_solid_rect() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            text_decoration_line: elidex_plugin::TextDecorationLine {
                underline: true,
                line_through: false,
            },
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let text = dom.create_text("Hello");
    dom.append_child(div, text);

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }

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

// --- M3-5: styled inline runs ---

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
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
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
    if !fonts_available(&font_db) {
        return;
    }

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

#[test]
#[allow(unused_must_use)]
fn display_none_inline_skipped() {
    // <p>visible <span style="display:none">hidden</span></p>
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let span = dom.create_element("span", Attributes::default());
    let t1 = dom.create_text("visible ");
    let t2 = dom.create_text("hidden");
    dom.append_child(p, t1);
    dom.append_child(p, span);
    dom.append_child(span, t2);
    dom.world_mut().insert_one(
        span,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::None,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    // Only "visible" — hidden span is skipped.
    assert_eq!(text_items.len(), 1);
}

#[test]
#[allow(unused_must_use)]
fn styled_segments_x_consecutive() {
    // <p><span>A</span><span>B</span></p>
    // Two segments: A and B should have consecutive x positions.
    let (mut dom, p) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );
    let s1 = dom.create_element("span", Attributes::default());
    let s2 = dom.create_element("span", Attributes::default());
    let t1 = dom.create_text("A");
    let t2 = dom.create_text("B");
    dom.append_child(p, s1);
    dom.append_child(s1, t1);
    dom.append_child(p, s2);
    dom.append_child(s2, t2);
    for &span in &[s1, s2] {
        dom.world_mut().insert_one(
            span,
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::Inline,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
        );
    }

    let font_db = elidex_text::FontDatabase::new();
    if !fonts_available(&font_db) {
        return;
    }

    let dl = build_display_list(&dom, &font_db);
    let text_first_x: Vec<f32> =
        dl.0.iter()
            .filter_map(|i| {
                if let crate::display_list::DisplayItem::Text { glyphs, .. } = i {
                    glyphs.first().map(|g| g.x)
                } else {
                    None
                }
            })
            .collect();
    // Two text items, second starts after first.
    assert_eq!(text_first_x.len(), 2);
    assert!(
        text_first_x[1] > text_first_x[0],
        "second segment x={} should be > first x={}",
        text_first_x[1],
        text_first_x[0]
    );
}

// --- M3-6: overflow: hidden → PushClip/PopClip ---

#[test]
fn overflow_hidden_emits_clip() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            overflow: elidex_plugin::Overflow::Hidden,
            background_color: elidex_plugin::CssColor::WHITE,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let has_push_clip =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::PushClip { .. }));
    let has_pop_clip =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::PopClip));
    assert!(has_push_clip, "overflow:hidden should emit PushClip");
    assert!(has_pop_clip, "overflow:hidden should emit PopClip");
}

#[test]
fn overflow_visible_no_clip() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            overflow: elidex_plugin::Overflow::Visible,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let has_push_clip =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::PushClip { .. }));
    assert!(!has_push_clip, "overflow:visible should not emit PushClip");
}

// --- M3-6: list markers ---

#[test]
#[allow(unused_must_use)]
fn list_item_disc_emits_marker() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ul = dom.create_element("ul", Attributes::default());
    dom.append_child(root, ul);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ul, li);

    dom.world_mut().insert_one(
        ul,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ul,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 100.0,
            },
            padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: elidex_plugin::ListStyleType::Disc,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 40.0,
                y: 0.0,
                width: 760.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Disc marker should emit a RoundedRect.
    let has_marker =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }));
    assert!(has_marker, "disc list marker should emit RoundedRect");
}

#[test]
#[allow(unused_must_use)]
fn list_item_square_emits_solid_rect_marker() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ul = dom.create_element("ul", Attributes::default());
    dom.append_child(root, ul);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ul, li);

    dom.world_mut().insert_one(
        ul,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ul,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 100.0,
            },
            padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: elidex_plugin::ListStyleType::Square,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 40.0,
                y: 0.0,
                width: 760.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // The first SolidRect with a very small width is the square marker.
    let small_rects: Vec<_> = dl
        .0
        .iter()
        .filter(
            |i| matches!(i, crate::display_list::DisplayItem::SolidRect { rect, .. } if rect.width < 10.0),
        )
        .collect();
    assert!(
        !small_rects.is_empty(),
        "square list marker should emit small SolidRect"
    );
}

#[test]
#[allow(unused_must_use)]
fn list_item_none_no_marker() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ul = dom.create_element("ul", Attributes::default());
    dom.append_child(root, ul);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ul, li);

    dom.world_mut().insert_one(
        ul,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ul,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 100.0,
            },
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: elidex_plugin::ListStyleType::None,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // list-style-type: none should not emit any marker shapes.
    let has_rounded =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }));
    assert!(!has_rounded, "list-style-type:none should not emit marker");
}

#[test]
#[allow(unused_must_use)]
fn list_item_circle_emits_stroked_marker() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ul = dom.create_element("ul", Attributes::default());
    dom.append_child(root, ul);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ul, li);

    dom.world_mut().insert_one(
        ul,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ul,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 100.0,
            },
            padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: elidex_plugin::ListStyleType::Circle,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 40.0,
                y: 0.0,
                width: 760.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Circle marker should emit StrokedRoundedRect (outline), not filled RoundedRect.
    let has_stroked = dl.0.iter().any(|i| {
        matches!(
            i,
            crate::display_list::DisplayItem::StrokedRoundedRect { .. }
        )
    });
    assert!(
        has_stroked,
        "circle list marker should emit StrokedRoundedRect"
    );
    let has_filled =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }));
    assert!(
        !has_filled,
        "circle list marker should not emit filled RoundedRect"
    );
}

#[test]
#[allow(unused_must_use)]
fn list_item_decimal_emits_text_marker() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, ol);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li);

    dom.world_mut().insert_one(
        ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 100.0,
            },
            padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: elidex_plugin::ListStyleType::Decimal,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 40.0,
                y: 0.0,
                width: 760.0,
                height: 20.0,
            },
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Decimal marker emits Text items (if fonts are available) or nothing
    // (graceful fallback). It should never emit shape-based markers.
    let has_shape = dl.0.iter().any(|i| {
        matches!(
            i,
            crate::display_list::DisplayItem::RoundedRect { .. }
                | crate::display_list::DisplayItem::StrokedRoundedRect { .. }
        )
    });
    assert!(
        !has_shape,
        "decimal list marker should not emit shape-based markers"
    );
    // If system fonts are available, a Text item should be emitted.
    let has_text =
        dl.0.iter()
            .any(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }));
    if font_db.query(&["serif"], 400).is_some() {
        assert!(
            has_text,
            "decimal marker should emit Text when fonts available"
        );
    }
}

// --- M3-6: white-space collapse tests ---

fn make_segment(text: &str) -> StyledTextSegment {
    StyledTextSegment {
        text: text.to_string(),
        color: elidex_plugin::CssColor::BLACK,
        font_family: vec!["serif".to_string()],
        font_size: 16.0,
        font_weight: 400,
        text_transform: elidex_plugin::TextTransform::None,
        text_decoration_line: elidex_plugin::TextDecorationLine::default(),
        opacity: 1.0,
    }
}

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

// --- L11: nested overflow:hidden ---

#[test]
#[allow(unused_must_use)]
fn nested_overflow_hidden_balanced_clips() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let outer = dom.create_element("div", Attributes::default());
    dom.append_child(root, outer);
    let inner = dom.create_element("div", Attributes::default());
    dom.append_child(outer, inner);

    for (entity, w, h) in [(outer, 200.0, 100.0), (inner, 100.0, 50.0)] {
        dom.world_mut().insert_one(
            entity,
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::Block,
                overflow: elidex_plugin::Overflow::Hidden,
                background_color: elidex_plugin::CssColor::WHITE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            entity,
            elidex_plugin::LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: w,
                    height: h,
                },
                ..Default::default()
            },
        );
    }

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let push_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::PushClip { .. }))
            .count();
    let pop_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::PopClip))
            .count();
    assert_eq!(push_count, 2, "should have 2 PushClip for nested overflow");
    assert_eq!(pop_count, 2, "should have 2 PopClip for nested overflow");
    assert_eq!(push_count, pop_count, "PushClip/PopClip must be balanced");
}

// --- L14: multi-item list counter ---

#[test]
#[allow(unused_must_use)]
fn list_item_counter_increments() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, ol);
    let li1 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li1);
    let li2 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li2);

    dom.world_mut().insert_one(
        ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::LayoutBox {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 200.0,
            },
            padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    for (li, y_off) in [(li1, 0.0), (li2, 20.0)] {
        dom.world_mut().insert_one(
            li,
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::ListItem,
                list_style_type: elidex_plugin::ListStyleType::Disc,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            elidex_plugin::LayoutBox {
                content: Rect {
                    x: 40.0,
                    y: y_off,
                    width: 760.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
    }

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Both list items should emit a marker (RoundedRect for disc).
    let marker_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }))
            .count();
    assert_eq!(marker_count, 2, "two disc markers for two list items");
}

// --- M3.5-4: BiDi visual reorder ---

#[test]
fn bidi_reorder_pure_ltr() {
    let collapsed = vec![("Hello ".to_string(), 0), ("world".to_string(), 1)];
    let order = super::bidi_visual_order(&collapsed, elidex_plugin::Direction::Ltr);
    assert_eq!(order, vec![0, 1]);
}

#[test]
fn bidi_reorder_indices() {
    let cases: &[(&[u8], &[usize])] = &[
        // All LTR
        (&[0, 0, 0], &[0, 1, 2]),
        // LTR with single embedded RTL (no visual change)
        (&[0, 1, 0], &[0, 1, 2]),
        // All RTL — reversed
        (&[1, 1, 1], &[2, 1, 0]),
        // RTL with embedded LTR (level 2) — all reversed
        (&[1, 2, 1], &[2, 1, 0]),
    ];
    for (levels, expected) in cases {
        let order = elidex_text::reorder_by_levels(levels);
        assert_eq!(order, *expected, "levels: {levels:?}");
    }
}

#[test]
fn bidi_reorder_empty() {
    let collapsed: Vec<(String, usize)> = Vec::new();
    let order = super::bidi_visual_order(&collapsed, elidex_plugin::Direction::Ltr);
    assert!(order.is_empty());
}
