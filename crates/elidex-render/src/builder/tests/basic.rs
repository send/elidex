use super::*;

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
