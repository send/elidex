//! Tests for `InlineFlow` persistence — the converged inline-text geometry that
//! render consumes (slice 1 of the render↔layout inline-pipeline convergence).

use super::*;
use elidex_ecs::{InlineFlow, PseudoElementMarker, TextContent};
use elidex_plugin::{Direction, Position, TextAlign, TextTransform, WritingMode};

/// Build a `LayoutEnv` for the test font db.
fn env(font_db: &FontDatabase) -> crate::LayoutEnv<'_> {
    crate::LayoutEnv {
        font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    }
}

/// The run-start key = the first composed child of the parent (render's run[0]).
fn run_start(dom: &EcsDom, parent: Entity) -> Entity {
    dom.composed_children(parent)[0]
}

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
    assert_eq!(flow.lines.len(), 1, "single short word → one line");
    let line = &flow.lines[0];
    assert_eq!(line.block_start, 0.0, "content origin is ZERO");
    assert_eq!(line.runs.len(), 1);
    assert_eq!(line.runs[0].text, "Hello");
    assert_eq!(line.runs[0].inline_start, 0.0, "left-aligned start");
    // The run's entity is the style source (the <p>), not the text-node key.
    assert_eq!(line.runs[0].entity, parent);
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
    assert_eq!(flow.lines.len(), 1);
    assert_eq!(
        flow.lines[0].runs.len(),
        1,
        "contiguous same-entity break pieces coalesce into one run"
    );
    assert_eq!(flow.lines[0].runs[0].text, "hello world");
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
    assert_eq!(flow.lines.len(), 2, "wraps into two lines");
    assert_eq!(flow.lines[0].block_start, 0.0);
    assert!(
        flow.lines[1].block_start > flow.lines[0].block_start,
        "second line below the first (block_start {} > {})",
        flow.lines[1].block_start,
        flow.lines[0].block_start
    );
    assert!(flow.lines[0].runs[0].text.starts_with("hello"));
    assert_eq!(flow.lines[1].runs[0].text, "world");
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
    assert_eq!(flow.lines[0].block_start, 20.0, "block_start = origin.y");
    assert_eq!(
        flow.lines[0].runs[0].inline_start, 10.0,
        "inline_start = origin.x (left-aligned)"
    );
}

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
        flow.lines[0].runs[0].inline_start > 0.0,
        "centered text is offset from the line start, got {}",
        flow.lines[0].runs[0].inline_start
    );
}

// --- gate: runs that diverge between layout IFC and render are NOT persisted ---

#[test]
fn gate_excludes_atomic_inline() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    // Append an inline-block after the text → the run contains an atomic.
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "a run containing an atomic inline must not persist an InlineFlow \
         (render flattens the atomic's text — consuming the flow would drop it)"
    );
}

#[test]
fn gate_excludes_relative_positioned_inline() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        position: Position::Relative,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    let t = dom.create_text("x");
    dom.append_child(span, t);
    dom.append_child(parent, span);

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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "relative/sticky positioned inline is in-flow in layout but Layer-6 in render \
         → must not persist"
    );
}

#[test]
fn gate_excludes_pseudo_element() {
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
        .insert_one(pseudo, TextContent("•".to_string()));
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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "pseudo/generated content is resolved by render (counters) → must not persist in slice 1"
    );
}

#[test]
fn gate_excludes_vertical_writing_mode() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "vertical writing modes are slice 2 → not persisted in slice 1"
    );
}

#[test]
fn gate_excludes_justify() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi there") else {
        return;
    };
    style.text_align = TextAlign::Justify;
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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "text-align: justify falls back to render until per-line distribution moves to layout"
    );
}

/// F9: off the paged path `layout_generation` is constant 0, so a run that
/// becomes non-persistable must be cleared by an explicit remove — not by
/// generation comparison. Persist, then re-layout gated-out, and assert removed.
#[test]
fn stale_flow_cleared_when_run_becomes_gated_out() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
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

    // Flip the gate: switch the parent to justify (same generation = 0).
    let mut justified = style;
    justified.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, justified);

    // Pass 2: now gated out → the stale flow must be removed (not consumable).
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
fn gate_excludes_rtl_text() {
    // Hebrew text needs bidi visual reordering, which render applies but layout's
    // logical-order positions do not encode → must fall back (slice 4 handles bidi).
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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "RTL/bidi text must not persist — render reorders visually, layout positions logically"
    );
}

#[test]
fn gate_excludes_text_transform() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("hello") else {
        return;
    };
    style.text_transform = TextTransform::Uppercase;
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

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "text-transform must not persist — layout measures untransformed text but render \
         transforms before shaping, so baked positions would be wrong"
    );
}

#[test]
fn gate_excludes_fragmented() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "fragmented (paged) runs must not persist — flow_lines are not yet sliced per fragment"
    );
}
