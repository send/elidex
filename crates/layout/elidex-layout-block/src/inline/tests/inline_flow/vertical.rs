use super::*;
use elidex_ecs::InlineFlow;
use elidex_plugin::TextAlign;

#[test]
fn persists_vertical_rl_flow() {
    let Some((dom, parent, key)) =
        layout_vertical("Hi", WritingMode::VerticalRl, 800.0, Point::ZERO)
    else {
        return;
    };
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical-rl text now persists an InlineFlow (slice 2)");
    assert_eq!(
        flow.fragments[0].lines.len(),
        1,
        "short text → one line/column"
    );
    assert_eq!(flow.fragments[0].lines[0].runs.len(), 1);
    assert_eq!(flow.fragments[0].lines[0].runs[0].text(), Some("Hi"));
    assert_eq!(flow.fragments[0].lines[0].runs[0].entity(), parent);
}

#[test]
fn persists_vertical_lr_flow() {
    let Some((dom, _parent, key)) =
        layout_vertical("Hi", WritingMode::VerticalLr, 800.0, Point::ZERO)
    else {
        return;
    };
    assert!(
        dom.world().get::<&InlineFlow>(key).is_ok(),
        "vertical-lr text persists too (slice 2 dropped only the is_vertical gate)"
    );
}

#[test]
fn vertical_absolute_coordinates_swap_axes() {
    // The persist fold applies the is_vertical projection rule: inline-axis maps to
    // physical y, block-axis to physical x (the swap, mirroring static_positions).
    // With origin (10, 20): block_start = origin.x = 10, inline_start = origin.y = 20
    // — the OPPOSITE of the horizontal case (block_start = y, inline_start = x).
    let origin = Point::new(10.0, 20.0);
    let Some((dom, _parent, key)) = layout_vertical("Hi", WritingMode::VerticalRl, 800.0, origin)
    else {
        return;
    };
    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(
        flow.fragments[0].lines[0].block_start, 10.0,
        "vertical: block-axis maps to physical x = origin.x"
    );
    assert_eq!(
        flow.fragments[0].lines[0].runs[0].inline_start(),
        20.0,
        "vertical: inline-axis maps to physical y = origin.y (start-aligned)"
    );
}

#[test]
fn vertical_multi_line_increasing_block_start() {
    // Tiny inline-axis (vertical) extent forces a wrap at the space → two columns
    // stacking along the block axis (physical x), so block_start increases.
    let Some((dom, _parent, key)) =
        layout_vertical("hello world", WritingMode::VerticalRl, 1.0, Point::ZERO)
    else {
        return;
    };
    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.fragments[0].lines.len(), 2, "wraps into two columns");
    assert_eq!(flow.fragments[0].lines[0].block_start, 0.0);
    assert!(
        flow.fragments[0].lines[1].block_start > flow.fragments[0].lines[0].block_start,
        "second column is offset along the block axis (x): block_start {} > {}",
        flow.fragments[0].lines[1].block_start,
        flow.fragments[0].lines[0].block_start
    );
}

#[test]
fn vertical_justify_persists_start_aligned() {
    // Vertical writing modes persist (slice 2) AND justify converged (slice 4 PR-3),
    // so vertical + justify now persists — but there is NO inter-word justification on
    // the block axis (CSS Text 3 §6.4 is the horizontal inline axis), so every line is
    // start-aligned: `justify_word_spacing == 0` (matching legacy's vertical behavior).
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("aa bb cc") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    style.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Narrow inline-axis (height) so it wraps — proves even a soft-wrapped vertical
    // line stays start-aligned (the is_vertical suppression, not just the last-line one).
    let prefix = measure_width(&font_db, "aa bb");
    let full = measure_width(&font_db, "aa bb cc");
    layout_inline_context(
        &mut dom,
        &children,
        f32::midpoint(prefix, full),
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical + justify now persists (start-aligned, no vertical justify)");
    for line in &flow.fragments[0].lines {
        assert_eq!(
            line.justify_word_spacing, 0.0,
            "vertical text is never inter-word justified"
        );
    }
}
