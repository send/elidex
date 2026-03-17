use super::*;

// ---------------------------------------------------------------------------
// Inline elements
// ---------------------------------------------------------------------------

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
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
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

    let dl = build_display_list(&dom, &font_db);
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    // Only "visible" — hidden span is skipped.
    assert_eq!(text_items.len(), 1);
}

// ---------------------------------------------------------------------------
// overflow: hidden -> PushClip/PopClip
// ---------------------------------------------------------------------------

#[test]
fn overflow_hidden_emits_clip() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            overflow_x: elidex_plugin::Overflow::Hidden,
            overflow_y: elidex_plugin::Overflow::Hidden,
            background_color: elidex_plugin::CssColor::WHITE,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(10.0, 10.0, 100.0, 50.0),
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
            overflow_x: elidex_plugin::Overflow::Visible,
            overflow_y: elidex_plugin::Overflow::Visible,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(10.0, 10.0, 100.0, 50.0),
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
                overflow_x: elidex_plugin::Overflow::Hidden,
                overflow_y: elidex_plugin::Overflow::Hidden,
                background_color: elidex_plugin::CssColor::WHITE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            entity,
            elidex_plugin::LayoutBox {
                content: Rect::new(0.0, 0.0, w, h),
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

// ---------------------------------------------------------------------------
// List markers
// ---------------------------------------------------------------------------

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
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
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
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
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
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
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
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
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
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
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
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
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
    if font_db
        .query(&["serif"], 400, elidex_text::FontStyle::Normal)
        .is_some()
    {
        assert!(
            has_text,
            "decimal marker should emit Text when fonts available"
        );
    }
}

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
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
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
                content: Rect::new(40.0, y_off, 760.0, 20.0),
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

// ---------------------------------------------------------------------------
// BiDi visual reorder
// ---------------------------------------------------------------------------

#[test]
fn bidi_reorder_pure_ltr() {
    let collapsed = vec![("Hello ".to_string(), 0), ("world".to_string(), 1)];
    let order = super::super::bidi_visual_order(&collapsed, elidex_plugin::Direction::Ltr);
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
    let order = super::super::bidi_visual_order(&collapsed, elidex_plugin::Direction::Ltr);
    assert!(order.is_empty());
}
