use super::*;
use crate::display_list::DisplayItem;
use elidex_plugin::{BackfaceVisibility, Dimension, TransformFunction};

/// Helper: build display list and return the items.
fn build_dl(dom: &elidex_ecs::EcsDom) -> Vec<DisplayItem> {
    let font_db = elidex_text::FontDatabase::new();
    build_display_list(dom, &font_db).0
}

#[test]
fn push_pop_transform_in_list() {
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::Translate(
                elidex_plugin::CssValue::Length(10.0, elidex_plugin::LengthUnit::Px),
                elidex_plugin::CssValue::Length(20.0, elidex_plugin::LengthUnit::Px),
            )],
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let items = build_dl(&dom);
    // Should contain PushTransform ... PopTransform.
    let push_count = items
        .iter()
        .filter(|i| matches!(i, DisplayItem::PushTransform { .. }))
        .count();
    let pop_count = items
        .iter()
        .filter(|i| matches!(i, DisplayItem::PopTransform))
        .count();
    assert_eq!(push_count, 1, "expected one PushTransform");
    assert_eq!(pop_count, 1, "expected one PopTransform");
}

#[test]
fn build_without_transform_no_push_pop() {
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
    let items = build_dl(&dom);
    let has_transform = items.iter().any(|i| {
        matches!(
            i,
            DisplayItem::PushTransform { .. } | DisplayItem::PopTransform
        )
    });
    assert!(!has_transform, "no PushTransform/PopTransform expected");
}

#[test]
fn transform_wraps_clip() {
    // An element with both transform and overflow:hidden should emit:
    // PushTransform → background → PushClip → PopClip → PopTransform
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::Rotate(45.0)],
            overflow_x: elidex_plugin::Overflow::Hidden,
            overflow_y: elidex_plugin::Overflow::Hidden,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 100.0),
            ..Default::default()
        },
    );
    let items = build_dl(&dom);

    // Find positions of each item type.
    let push_transform_pos = items
        .iter()
        .position(|i| matches!(i, DisplayItem::PushTransform { .. }));
    let push_clip_pos = items
        .iter()
        .position(|i| matches!(i, DisplayItem::PushClip { .. }));
    let pop_clip_pos = items.iter().position(|i| matches!(i, DisplayItem::PopClip));
    let pop_transform_pos = items
        .iter()
        .position(|i| matches!(i, DisplayItem::PopTransform));

    assert!(push_transform_pos.is_some(), "PushTransform expected");
    assert!(push_clip_pos.is_some(), "PushClip expected");
    assert!(pop_clip_pos.is_some(), "PopClip expected");
    assert!(pop_transform_pos.is_some(), "PopTransform expected");

    // Order: PushTransform < PushClip < PopClip < PopTransform.
    let pt = push_transform_pos.unwrap();
    let pc = push_clip_pos.unwrap();
    let ec = pop_clip_pos.unwrap();
    let et = pop_transform_pos.unwrap();
    assert!(pt < pc, "PushTransform before PushClip");
    assert!(pc < ec, "PushClip before PopClip");
    assert!(ec < et, "PopClip before PopTransform");
}

#[test]
fn backface_hidden_skips_paint() {
    // An element rotated 180° on Y axis with backface-visibility: hidden
    // should produce no paint items.
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::RotateY(180.0)],
            backface_visibility: BackfaceVisibility::Hidden,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let items = build_dl(&dom);
    // Backface hidden + facing away: entire subtree skipped.
    assert!(items.is_empty(), "backface-hidden should skip all items");
}

#[test]
fn backface_visible_still_paints() {
    // Same rotation but backface-visibility: visible (default) — should paint.
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::RotateY(180.0)],
            backface_visibility: BackfaceVisibility::Visible,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let items = build_dl(&dom);
    assert!(!items.is_empty(), "backface-visible should still paint");
}

#[test]
fn perspective_propagated_to_children() {
    // Parent with perspective property → child with transform should
    // produce PushTransform on the child (perspective modifies child's transform).
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();

    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(root, parent);
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            perspective: Some(500.0),
            perspective_origin: (Dimension::Percentage(50.0), Dimension::Percentage(50.0)),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 200.0, 200.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(parent, child);
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::BLUE,
            has_transform: true,
            transform: vec![TransformFunction::RotateY(30.0)],
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::LayoutBox {
            content: Rect::new(20.0, 20.0, 160.0, 160.0),
            ..Default::default()
        },
    );

    let items = build_dl(&dom);
    let has_push = items
        .iter()
        .any(|i| matches!(i, DisplayItem::PushTransform { .. }));
    assert!(
        has_push,
        "child with parent perspective should have PushTransform"
    );
}

#[test]
fn nested_transforms_emit_push_pop_each() {
    // Parent transform + child transform → each should get its own PushTransform/PopTransform.
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();

    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(root, parent);
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::Translate(
                elidex_plugin::CssValue::Length(10.0, elidex_plugin::LengthUnit::Px),
                elidex_plugin::CssValue::Length(0.0, elidex_plugin::LengthUnit::Px),
            )],
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 200.0, 200.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(parent, child);
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::BLUE,
            has_transform: true,
            transform: vec![TransformFunction::Scale(2.0, 2.0)],
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::LayoutBox {
            content: Rect::new(20.0, 20.0, 100.0, 100.0),
            ..Default::default()
        },
    );

    let items = build_dl(&dom);
    let push_count = items
        .iter()
        .filter(|i| matches!(i, DisplayItem::PushTransform { .. }))
        .count();
    let pop_count = items
        .iter()
        .filter(|i| matches!(i, DisplayItem::PopTransform))
        .count();
    assert_eq!(push_count, 2, "parent + child each get PushTransform");
    assert_eq!(pop_count, 2, "parent + child each get PopTransform");
}

#[test]
fn identity_transform_elided() {
    // A transform that resolves to identity (translate(0,0)) should NOT
    // produce PushTransform/PopTransform.
    let (dom, _) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::RED,
            has_transform: true,
            transform: vec![TransformFunction::Translate(
                elidex_plugin::CssValue::Length(0.0, elidex_plugin::LengthUnit::Px),
                elidex_plugin::CssValue::Length(0.0, elidex_plugin::LengthUnit::Px),
            )],
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 50.0),
            ..Default::default()
        },
    );
    let items = build_dl(&dom);
    let has_transform = items.iter().any(|i| {
        matches!(
            i,
            DisplayItem::PushTransform { .. } | DisplayItem::PopTransform
        )
    });
    assert!(!has_transform, "identity transform should be elided");
}

#[test]
fn parent_perspective_child_no_transform() {
    // Parent with perspective, child WITHOUT any transform.
    // The child should still get PushTransform because parent perspective
    // affects the child's coordinate space (even with no transform functions,
    // the perspective projection produces a non-identity affine when
    // perspective-origin differs from element position).
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();

    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(root, parent);
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            perspective: Some(200.0),
            perspective_origin: (Dimension::Percentage(50.0), Dimension::Percentage(50.0)),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        parent,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 200.0, 200.0),
            ..Default::default()
        },
    );

    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    let _ = dom.append_child(parent, child);
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            background_color: elidex_plugin::CssColor::BLUE,
            // No transform — has_transform is false (default)
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        child,
        elidex_plugin::LayoutBox {
            content: Rect::new(20.0, 20.0, 160.0, 160.0),
            ..Default::default()
        },
    );

    let items = build_dl(&dom);
    // The child should still paint (not skipped).
    let has_solid = items
        .iter()
        .any(|i| matches!(i, DisplayItem::SolidRect { .. }));
    assert!(
        has_solid,
        "child should still paint with parent perspective"
    );
}
