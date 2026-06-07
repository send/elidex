use super::*;
use elidex_ecs::InlineFlow;
use elidex_plugin::{Direction, TextAlign};

#[test]
fn text_align_center_bakes_offset_into_inline_start() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    style.text_align = TextAlign::Center;
    style.direction = Direction::Ltr;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert!(
        flow.fragments[0].lines[0].runs[0].inline_start() > 0.0,
        "centered text is offset from the line start, got {}",
        flow.fragments[0].lines[0].runs[0].inline_start()
    );
}

// --- gate: runs that diverge between layout IFC and render are NOT persisted
// (relpos/sticky still gated — slice 3p-b; static atomics now persist — 3p-a) ---
