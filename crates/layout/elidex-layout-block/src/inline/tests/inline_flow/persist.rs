use super::*;
use elidex_ecs::{InlineFlow, PseudoElementMarker, TextContent};

#[test]
fn persists_single_line_flow() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
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

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("simple horizontal text run should persist an InlineFlow");
    assert_eq!(
        flow.fragments[0].lines.len(),
        1,
        "single short word → one line"
    );
    let line = &flow.fragments[0].lines[0];
    assert_eq!(line.block_start, 0.0, "content origin is ZERO");
    assert_eq!(line.runs.len(), 1);
    assert_eq!(line.runs[0].text(), Some("Hello"));
    assert_eq!(line.runs[0].inline_start(), 0.0, "left-aligned start");
    // The run's entity is the style source (the <p>), not the text-node key.
    assert_eq!(line.runs[0].entity(), parent);
}

#[test]
fn coalesces_break_pieces_on_one_line() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Wide container: both words fit on one line.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.fragments[0].lines.len(), 1);
    assert_eq!(
        flow.fragments[0].lines[0].runs.len(),
        1,
        "contiguous same-entity break pieces coalesce into one run"
    );
    assert_eq!(
        flow.fragments[0].lines[0].runs[0].text(),
        Some("hello world")
    );
}

#[test]
fn multi_line_wrap_has_increasing_block_start() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Tiny container forces a wrap at the space.
    layout_inline_context(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.fragments[0].lines.len(), 2, "wraps into two lines");
    assert_eq!(flow.fragments[0].lines[0].block_start, 0.0);
    assert!(
        flow.fragments[0].lines[1].block_start > flow.fragments[0].lines[0].block_start,
        "second line below the first (block_start {} > {})",
        flow.fragments[0].lines[1].block_start,
        flow.fragments[0].lines[0].block_start
    );
    assert!(flow.fragments[0].lines[0].runs[0]
        .text()
        .is_some_and(|t| t.starts_with("hello")));
    assert_eq!(flow.fragments[0].lines[1].runs[0].text(), Some("world"));
}

#[test]
fn absolute_coordinates_offset_by_content_origin() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let origin = Point::new(10.0, 20.0);
    layout_inline_context(&mut dom, &children, 800.0, parent, origin, &env(&font_db));

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(
        flow.fragments[0].lines[0].block_start, 20.0,
        "block_start = origin.y"
    );
    assert_eq!(
        flow.fragments[0].lines[0].runs[0].inline_start(),
        10.0,
        "inline_start = origin.x (left-aligned)"
    );
}

#[test]
fn persists_pseudo_element_flow() {
    // Slice 3: pseudo `content` (incl. counter()) is resolved into the pseudo's
    // `TextContent` by the pre-layout generated-content pass, so layout measures
    // the real text and a plain-LTR, non-transformed pseudo run now persists like
    // any text run (the slice-1 `has_pseudo` gate is gone). Render consumes it.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let pseudo = dom.create_element("span", Attributes::default());
    let pseudo_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(pseudo, pseudo_style);
    let _ = dom.world_mut().insert_one(pseudo, PseudoElementMarker);
    let _ = dom
        .world_mut()
        .insert_one(pseudo, TextContent("AB".to_string()));
    dom.append_child(parent, pseudo);

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

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("plain-LTR pseudo run with resolved text must persist (slice 3)");
    assert!(
        flow.fragments[0]
            .lines
            .iter()
            .flat_map(|l| &l.runs)
            .any(|r| r.text().is_some_and(|t| t.contains("AB"))),
        "persisted flow should carry the pseudo's resolved generated text"
    );
}

// (Vertical writing modes now persist — slice 2. See `persists_vertical_rl_flow`,
// `persists_vertical_lr_flow`, and `vertical_absolute_coordinates_swap_axes` below;
// the slice-1 `gate_excludes_vertical_writing_mode` was removed when the gate
// dropped.)

/// F9: off the paged path `layout_generation` is constant 0, so a run that
/// becomes non-persistable must be cleared by an explicit remove — not by
/// generation comparison. Persist, then re-layout gated-out, and assert removed.
#[test]
fn stale_flow_cleared_when_run_becomes_gated_out() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Pass 1: simple run persists.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_ok(),
        "pass 1 persists"
    );

    // Make the run non-persistable: empty its text so the IFC produces no items
    // (justify no longer gates — it converged in slice 4 — so emptying the content is
    // the simplest "becomes non-persistable" flip; same generation = 0).
    let _ = dom.world_mut().insert_one(key, TextContent(String::new()));

    // Pass 2: now non-persistable → the stale flow must be removed (not consumable).
    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "stale InlineFlow must be explicitly cleared when the run becomes gated out \
         (generation is constant 0 off the paged path, so it can't signal staleness)"
    );
}

#[test]
fn rtl_text_persists_logical_order() {
    // Slice 4 / bidi inverts the old `gate_excludes_rtl_text`: an RTL run now
    // persists an `InlineFlow` in **logical** order — layout stays logical, render
    // owns the UAX #9 L2 visual reorder at paint (master §4.2). So the persisted run
    // carries the logical (source-order) text, NOT a pre-reordered string.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("שלום עולם") else {
        return;
    };
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

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("RTL run must now persist (render reorders visually, layout positions logically)");
    assert_eq!(
        flow.fragments[0].lines[0].runs[0].text(),
        Some("שלום עולם"),
        "persisted run carries the logical source-order text; visual reorder is render's job"
    );
}
