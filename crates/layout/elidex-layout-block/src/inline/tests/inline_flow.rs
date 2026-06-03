//! Tests for `InlineFlow` persistence — the converged inline-text geometry that
//! render consumes (slice 1 of the render↔layout inline-pipeline convergence).

use super::*;
use elidex_ecs::{InlineFlow, InlineFlowRun, PseudoElementMarker, TextContent};
use elidex_plugin::{Direction, TextAlign, TextTransform, WritingMode};

/// Build a `LayoutEnv` for the test font db. `pub(super)` so the sibling
/// `relpos_subflow` test module can reuse it.
pub(super) fn env(font_db: &FontDatabase) -> crate::LayoutEnv<'_> {
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

/// Lay out `text` under writing mode `wm` and return `(dom, parent, run-start key)`.
/// `containing_inline_size` is the inline-axis extent (height for vertical modes).
fn layout_vertical(
    text: &str,
    wm: WritingMode,
    containing_inline_size: f32,
    origin: Point,
) -> Option<(EcsDom, Entity, Entity)> {
    let (mut dom, parent, mut style, font_db) = setup_inline_test(text)?;
    style.writing_mode = wm;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        containing_inline_size,
        parent,
        origin,
        &env(&font_db),
    );
    Some((dom, parent, key))
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
    assert_eq!(flow.lines.len(), 1);
    assert_eq!(
        flow.lines[0].runs.len(),
        1,
        "contiguous same-entity break pieces coalesce into one run"
    );
    assert_eq!(flow.lines[0].runs[0].text(), Some("hello world"));
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
    assert!(flow.lines[0].runs[0]
        .text()
        .is_some_and(|t| t.starts_with("hello")));
    assert_eq!(flow.lines[1].runs[0].text(), Some("world"));
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
        flow.lines[0].runs[0].inline_start(),
        10.0,
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
        flow.lines[0].runs[0].inline_start() > 0.0,
        "centered text is offset from the line start, got {}",
        flow.lines[0].runs[0].inline_start()
    );
}

// --- gate: runs that diverge between layout IFC and render are NOT persisted
// (relpos/sticky still gated — slice 3p-b; static atomics now persist — 3p-a) ---

#[test]
fn persists_atomic_inline_as_box_member() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    // Append a static inline-block after the text → the run contains an atomic.
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

    // Slice 3p-a: a static atomic now persists as an `AtomicBox` member (was gated).
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("a run with a static atomic inline now persists an InlineFlow (slice 3p-a)");
    // Find the AtomicBox member and the block_start of the line it landed on. (The
    // test's `layout_block_only` child lays the inline-block out at full container
    // width, so it wraps below the text onto its own line — which makes the D7
    // reposition's block-axis move observable: the box moves from content_origin
    // y=0 down to the line.)
    let (atomic_inline, atomic_block) = flow
        .lines
        .iter()
        .flat_map(|line| line.runs.iter().map(move |r| (r, line.block_start)))
        .find_map(|(r, block_start)| match r {
            InlineFlowRun::AtomicBox {
                entity,
                inline_start,
            } if *entity == ib => Some((*inline_start, block_start)),
            _ => None,
        })
        .expect("the inline-block must be recorded as an AtomicBox member of the flow");
    assert!(
        atomic_block > 0.0 || atomic_inline > 0.0,
        "the atomic is placed away from the IFC origin on its line, got ({atomic_inline}, {atomic_block})"
    );

    // D7: layout repositioned the atomic's LayoutBox from content_origin (0,0) to its
    // member position (margins are 0, so margin-box origin == content origin).
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_inline).abs() < 0.5
            && (lb.content.origin.y - atomic_block).abs() < 0.5,
        "atomic LayoutBox repositioned to its member position ({atomic_inline}, {atomic_block}), \
         got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

// NOTE: the slice-3p-b-2 inversion of the old `gate_excludes_relative_positioned_atomic`
// test (a relpos/sticky atomic now persists + repositions instead of gating) lives in
// the sibling `relpos_subflow` module alongside the other positioned-inline tests.

#[test]
fn vertical_atomic_repositions_with_axis_swap() {
    // Vertical WM (slice 2 persists it): the D7 reposition projects inline-axis →
    // physical y and block-axis → physical x (the `is_vertical` swap). Assert the
    // static atomic persists as an `AtomicBox` and its `LayoutBox` lands at the
    // swapped member position.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());
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

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical static atomic persists (slice 2 + 3p-a)");
    let (atomic_inline, atomic_block) = flow
        .lines
        .iter()
        .flat_map(|line| line.runs.iter().map(move |r| (r, line.block_start)))
        .find_map(|(r, block_start)| match r {
            InlineFlowRun::AtomicBox {
                entity,
                inline_start,
            } if *entity == ib => Some((*inline_start, block_start)),
            _ => None,
        })
        .expect("the inline-block is an AtomicBox member in vertical mode");

    // is_vertical projection: inline-axis → physical y, block-axis → physical x.
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_block).abs() < 0.5
            && (lb.content.origin.y - atomic_inline).abs() < 0.5,
        "vertical atomic box at the swapped position (x=block {atomic_block}, \
         y=inline {atomic_inline}), got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

#[test]
fn atomic_inner_text_inline_flow_shifts_with_box() {
    // An inline-block CONTAINING converged text: when the atomic is repositioned to
    // its line, the inner text's persisted `InlineFlow` (absolute coords, consumed
    // directly by render) must shift with the box — else render repaints the inner
    // text at the pre-reposition (content_origin) position. Root fix in
    // `shift_descendants` (covers relpos/abspos subtree shifts too).
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(40.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    let inner = dom.create_text("hi");
    dom.append_child(ib, inner);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // The atomic was repositioned to its line (the stub lays it out full-width, so it
    // wraps below "a") → its box is off content_origin.
    let box_y = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic has a LayoutBox")
        .content
        .origin
        .y;
    // The inner text persisted its own InlineFlow (the atomic's inner IFC); its
    // block_start must have tracked the box, not stayed at the pre-shift origin.
    let inner_block = dom
        .world()
        .get::<&InlineFlow>(inner)
        .expect("the inner text persists an InlineFlow")
        .lines[0]
        .block_start;
    assert!(
        inner_block > 0.5,
        "the inner text's InlineFlow moved off content_origin with the box, got {inner_block}"
    );
    assert!(
        (inner_block - box_y).abs() < 0.5,
        "inner text InlineFlow tracks the repositioned box (box y {box_y}, inner block_start {inner_block})"
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
        flow.lines
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

// --- slice 2: vertical writing modes now persist (the `!is_vertical` gate dropped) ---

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
    assert_eq!(flow.lines.len(), 1, "short text → one line/column");
    assert_eq!(flow.lines[0].runs.len(), 1);
    assert_eq!(flow.lines[0].runs[0].text(), Some("Hi"));
    assert_eq!(flow.lines[0].runs[0].entity(), parent);
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
        flow.lines[0].block_start, 10.0,
        "vertical: block-axis maps to physical x = origin.x"
    );
    assert_eq!(
        flow.lines[0].runs[0].inline_start(),
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
    assert_eq!(flow.lines.len(), 2, "wraps into two columns");
    assert_eq!(flow.lines[0].block_start, 0.0);
    assert!(
        flow.lines[1].block_start > flow.lines[0].block_start,
        "second column is offset along the block axis (x): block_start {} > {}",
        flow.lines[1].block_start,
        flow.lines[0].block_start
    );
}

#[test]
fn vertical_justify_still_gated() {
    // Slice 2 dropped ONLY the is_vertical exclusion; every other gate still applies.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
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
        "vertical + justify must not persist (justify is still render's fallback)"
    );
}
