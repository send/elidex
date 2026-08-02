//! Atomic inline boxes (`display: inline-block`) participating in an IFC.

use super::*;

#[test]
fn inline_block_participates_in_ifc() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(50.0),
        height: Dimension::Length(30.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);
    let ib_text = dom.create_text("X");
    dom.append_child(ib, ib_text);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;

    let ib_lb = dom.world().get::<&LayoutBox>(ib);
    assert!(ib_lb.is_ok(), "inline-block should have a LayoutBox");
    let ib_lb = ib_lb.unwrap();
    assert!(
        (ib_lb.content.size.width - 50.0).abs() < f32::EPSILON,
        "inline-block width should be 50px, got {}",
        ib_lb.content.size.width
    );

    assert!(
        h >= 30.0,
        "line height should be >= inline-block height (30px), got {h}"
    );
}

#[test]
fn inline_block_not_block_level() {
    assert!(
        !crate::block::is_block_level(Display::InlineBlock),
        "InlineBlock should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineFlex),
        "InlineFlex should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineGrid),
        "InlineGrid should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineTable),
        "InlineTable should not be block-level"
    );
}
