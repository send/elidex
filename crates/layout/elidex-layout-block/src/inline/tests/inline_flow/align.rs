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

// --- text-align offset is applied to inline-element getClientRects/LayoutBox
// (entity_bounds), not only the persisted runs — so paint and CSSOM geometry agree.
// `commit_aligned_entity_rects` (CSSOM VIEW 1 §6 / CSS Text 3 §6.4.1). ---

#[test]
fn center_aligns_inline_element_client_rect() {
    // A centered single-line `<span>`: its box (getClientRects fallback = LayoutBox
    // border box) must sit at the painted, centered position — == the persisted run's
    // start — not at the un-aligned line start (the pre-existing gap).
    let Some((dom, _parent, span, _fd)) = setup_span_align("Hi", TextAlign::Center, 800.0) else {
        return;
    };
    let flow = dom
        .world()
        .get::<&InlineFlow>(span)
        .expect("centered span persists an InlineFlow");
    let run_start_x = flow.fragments[0].lines[0].runs[0].inline_start();
    assert!(
        run_start_x > 0.0,
        "centered run is offset from the line start, got {run_start_x}"
    );
    let lb = dom
        .world()
        .get::<&LayoutBox>(span)
        .expect("the inline span gets a LayoutBox from entity_bounds");
    assert!(
        (lb.content.origin.x - run_start_x).abs() < 0.5,
        "span LayoutBox/getClientRects start ({}) tracks the painted centered run start ({run_start_x})",
        lb.content.origin.x
    );
    // Single-line span → merged to one fragment → no per-line InlineClientRects (the
    // getClientRects fallback to the LayoutBox border box is exercised).
    assert!(
        dom.world()
            .get::<&elidex_plugin::InlineClientRects>(span)
            .is_err(),
        "a single-line span exposes one rect via LayoutBox, not InlineClientRects"
    );
}

#[test]
fn right_aligns_inline_element_client_rect() {
    let Some((dom, _parent, span, _fd)) = setup_span_align("Hi", TextAlign::Right, 800.0) else {
        return;
    };
    let flow = dom
        .world()
        .get::<&InlineFlow>(span)
        .expect("right-aligned span persists an InlineFlow");
    let run_start_x = flow.fragments[0].lines[0].runs[0].inline_start();
    let lb = dom
        .world()
        .get::<&LayoutBox>(span)
        .expect("the inline span gets a LayoutBox");
    assert!(
        (lb.content.origin.x - run_start_x).abs() < 0.5,
        "right-aligned span box ({}) tracks the painted run start ({run_start_x})",
        lb.content.origin.x
    );
    // Right edge of the box reaches (near) the container's right edge.
    assert!(
        lb.content.origin.x + lb.content.size.width > 700.0,
        "right-aligned box ends near the container right edge, got {}",
        lb.content.origin.x + lb.content.size.width
    );
}

#[test]
fn multi_word_single_line_span_merges_to_one_rect() {
    // Two words on one line are placed as two break-segment rects, but getClientRects
    // returns ONE box fragment per line per inline element (CSSOM VIEW 1 §6): the
    // segment rects merge, so a single-line span has no multi-rect InlineClientRects.
    let Some((dom, _parent, span, _fd)) = setup_span_align("hello world", TextAlign::Left, 800.0)
    else {
        return;
    };
    assert!(
        dom.world()
            .get::<&elidex_plugin::InlineClientRects>(span)
            .is_err(),
        "a single-line two-word span merges to one rect (LayoutBox), not N InlineClientRects"
    );
    let lb = dom
        .world()
        .get::<&LayoutBox>(span)
        .expect("the span gets a LayoutBox spanning the whole line");
    assert_eq!(lb.content.origin.x, 0.0, "left-aligned starts at 0");
    assert!(
        lb.content.size.width > 0.0,
        "the merged rect spans the whole word run, got width {}",
        lb.content.size.width
    );
}
