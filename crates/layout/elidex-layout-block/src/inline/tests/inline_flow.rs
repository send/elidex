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
    let (atomic_inline, atomic_block) = flow.fragments[0]
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
    let (atomic_inline, atomic_block) = flow.fragments[0]
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
        .fragments[0]
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

/// Lay out `text` with `text_transform` applied to the parent, returning the
/// persisted single-line run text (the slice's payoff: text-transform now
/// persists instead of gating to render's legacy path).
fn transformed_run_text(
    text: &str,
    transform: TextTransform,
    available_inline: f32,
) -> Option<String> {
    let (mut dom, parent, mut style, font_db) = setup_inline_test(text)?;
    style.text_transform = transform;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        available_inline,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("text-transform run must now persist an InlineFlow");
    Some(
        flow.fragments[0].lines[0].runs[0]
            .text()
            .expect("persisted run must carry text")
            .to_string(),
    )
}

#[test]
fn text_transform_uppercase_persists_transformed_text() {
    // The slice payoff: layout transforms before measuring, so the run persists
    // (no gate) and the persisted text is the final, transformed glyphs.
    let Some(t) = transformed_run_text("hello", TextTransform::Uppercase, 800.0) else {
        return;
    };
    assert_eq!(t, "HELLO");
}

#[test]
fn text_transform_lowercase_persists_transformed_text() {
    let Some(t) = transformed_run_text("HELLO", TextTransform::Lowercase, 800.0) else {
        return;
    };
    assert_eq!(t, "hello");
}

#[test]
fn text_transform_capitalize_word_boundaries() {
    // CSS Text 3 §2.1.1: first typographic letter unit of each word.
    let Some(t) = transformed_run_text("hello world", TextTransform::Capitalize, 800.0) else {
        return;
    };
    assert_eq!(t, "Hello World");
}

#[test]
fn text_transform_capitalize_after_collapse() {
    // CSS Text 3 §2.1.2: transform runs AFTER §4.1.1 collapse, so word
    // boundaries are computed on the collapsed text.
    let Some(t) = transformed_run_text("  hello   world  ", TextTransform::Capitalize, 800.0)
    else {
        return;
    };
    assert!(
        t.contains("Hello World"),
        "collapsed-then-capitalized text should read 'Hello World', got {t:?}"
    );
    assert!(
        !t.contains("hello"),
        "first word must be capitalized: {t:?}"
    );
}

#[test]
fn text_transform_multi_line_each_line_transformed() {
    // Multi-line payoff: the legacy single-linear-pass mis-rendered wrapped
    // transformed runs; converged layout positions each line from the
    // transformed advances. Tiny container forces a wrap at the space.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    style.text_transform = TextTransform::Uppercase;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
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
    assert!(flow.fragments[0].lines[0].runs[0]
        .text()
        .is_some_and(|t| t.starts_with("HELLO")));
    assert_eq!(flow.fragments[0].lines[1].runs[0].text(), Some("WORLD"));
    assert!(
        flow.fragments[0].lines[1].block_start > flow.fragments[0].lines[0].block_start,
        "second line below the first"
    );
}

#[test]
fn paged_fragmented_run_now_persists() {
    // Slice 4 / I-paged inverts the old `gate_excludes_fragmented` test: its
    // reason ("flow_lines are not yet sliced per fragment") is resolved by THIS
    // slice's `slice_and_rebase_fragment`, so a *paged* fragmented run now
    // persists (per-page slice + continuation rebase + page-generation stamp).
    // `available_block` is large, so the whole short run fits in one page fragment.
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
        fragmentation_type: crate::FragmentationType::Page,
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
        dom.world().get::<&InlineFlow>(key).is_ok(),
        "a paged fragmented run now persists an InlineFlow (slice 4 / I-paged)"
    );
}

#[test]
fn multicol_whole_run_now_persists() {
    // Slice 4 / I-multicol: a multicol (`Column`) run that is WHOLE in its column
    // (`skip_lines == 0`, fits → no break) now persists an `InlineFlow` — it was gated
    // to legacy pre-I-multicol. And persisting changes ONLY `InlineFlow` presence: the
    // persisted geometry is byte-identical to the trusted non-fragmented layout of the
    // same content (D-mc2 — the optimistic `flow_align` for `Column` does not perturb
    // `entity_bounds`/the packer geometry, which the packer commits unconditionally).
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Reference: non-fragmented layout = the established-correct geometry.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let ref_flow: InlineFlow = {
        let g = dom
            .world()
            .get::<&InlineFlow>(key)
            .expect("non-fragmented run persists");
        (*g).clone()
    };

    // Column-whole: large available_block → the IFC fits one column, no break.
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
        fragmentation_type: crate::FragmentationType::Column,
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

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("a whole-in-column multicol run now persists (I-multicol)");
    assert_eq!(
        flow.fragments.len(),
        1,
        "whole-in-column = a single fragment (length-1 Vec, no accumulate)"
    );
    assert_eq!(
        flow.fragments[0].generation, 0,
        "plain (non-paged) multicol stamps generation 0"
    );
    assert_eq!(
        *flow, ref_flow,
        "column-whole geometry is byte-identical to non-fragmented (gate widening is geometry-neutral)"
    );
}

#[test]
fn gate_excludes_multicol_continuation() {
    // A multicol continuation (`skip_lines > 0` — the tail of a column-spanning IFC)
    // must NOT persist: render consumes the whole `InlineFlow` off the paged path, so a
    // tail-only flow would drop the prior column's lines. Mid-IFC column break stays on
    // legacy (deferred to Z with box fragments — G11).
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 1,
        widows: 1,
        skip_lines: 1, // continuation from a prior column
        fragmentation_type: crate::FragmentationType::Column,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        1.0, // narrow → "hello world" wraps to 2 lines so a continuation is meaningful
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "a multicol continuation (skip_lines>0) must stay on legacy (mid-IFC → Z)"
    );
}

#[test]
fn gate_excludes_multicol_truncation() {
    // A multicol run truncated by a fragment break (`break_after_line.is_some()`) must
    // NOT persist: its tail goes to a column the single subtree shift won't reach. Only
    // whole-in-column (skip 0 AND no break) persists.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Width 1.0 → 2 lines; tiny available_block forces a break (orphans=1 keeps line 0,
    // line 1 truncated to the next column).
    let frag = InlineFragConstraint {
        available_block: 0.001,
        orphans: 1,
        widows: 1,
        skip_lines: 0,
        fragmentation_type: crate::FragmentationType::Column,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "a truncated multicol run (break_after_line Some) must stay on legacy (mid-IFC → Z)"
    );
}

#[test]
fn multicol_nested_in_paged_stamps_page_generation() {
    // Multicol nested in paged media: a whole-in-column run's persisted fragment carries
    // the PAGE generation (`env.layout_generation`), NOT a stale generation 0. Pins the
    // D-mc3 overwrite-safety weakest link — the final fill pass stamps the page
    // generation, overwriting any gen-0 flow `probe_total_height` (whose
    // `LayoutInput::probe` hard-codes generation 0) may have left on the run-start.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let env_page2 = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 2, // multicol on page 2
    };
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
        fragmentation_type: crate::FragmentationType::Column,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env_page2,
        Some(&frag),
    );
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("whole-in-column persists");
    assert_eq!(
        flow.fragments[0].generation, 2,
        "column run stamps the page generation (env.layout_generation), not gen 0"
    );
}

// --- slice 4 / I-paged: per-page slice + continuation rebase ---

#[test]
fn continuation_fragment_rebases_to_fragmentainer_top() {
    // "hello world" wraps into 2 lines at width 1.0. A continuation (page 2)
    // fragment skips line 0, keeps line 1 ("world"), and rebases it to the
    // fragmentainer block-start (0.0) — NOT line 1's original second-line offset.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Baseline: the un-skipped flow puts line 1 below line 0.
    layout_inline_context(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let original_line1 = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("two-line flow persists")
        .fragments[0]
        .lines[1]
        .block_start;
    assert!(
        original_line1 > 0.0,
        "line 1 sits below line 0 pre-fragmentation"
    );

    // Continuation fragment (page 2) skipping the first line.
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 1,
        widows: 1,
        skip_lines: 1,
        fragmentation_type: crate::FragmentationType::Page,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("continuation fragment persists (I-paged)");
    assert_eq!(
        flow.fragments[0].lines.len(),
        1,
        "only the kept line 'world' is in this fragment"
    );
    assert_eq!(
        flow.fragments[0].lines[0].block_start, 0.0,
        "kept line rebased to fragmentainer top (was {original_line1} pre-rebase)"
    );
    assert_eq!(flow.fragments[0].lines[0].runs[0].text(), Some("world"));
    assert_eq!(
        flow.fragments[0].generation, 0,
        "non-paged-builder generation stamp is 0 (env default)"
    );
}

#[test]
fn paged_fragment_clientrects_single_source_with_flow() {
    // F2 single-source rebase: the persisted fragment lines and the inline
    // element's `InlineClientRects` derive from the SAME packer block offset, so a
    // continuation fragment's flow-line `block_start`s equal its clientRects tops
    // (and the first is rebased to the fragmentainer top, 0.0).
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let span_text = dom.create_text("aaa bbb ccc ddd");
    dom.append_child(span, span_text);
    dom.append_child(parent, span);
    let font_db = FontDatabase::new();
    let style = ComputedStyle {
        font_family: vec![
            "Arial".to_string(),
            "Helvetica".to_string(),
            "Liberation Sans".to_string(),
            "DejaVu Sans".to_string(),
            "Noto Sans".to_string(),
            "Hiragino Sans".to_string(),
        ],
        ..Default::default()
    };
    let params = TextMeasureParams {
        families: &[
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
        ],
        font_size: style.font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    if measure_text(&font_db, &params, "x").is_none() {
        return;
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let _ = dom.world_mut().insert_one(span, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Width 1.0 → four single-word lines; skip line 0 → keep [1,2,3] on page 2.
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 1,
        widows: 1,
        skip_lines: 1,
        fragmentation_type: crate::FragmentationType::Page,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("paged continuation persists");
    assert_eq!(
        flow.fragments[0].lines.len(),
        3,
        "three kept lines (bbb/ccc/ddd) after skipping line 0"
    );
    assert_eq!(
        flow.fragments[0].lines[0].block_start, 0.0,
        "first kept line rebased to fragmentainer top"
    );
    let rects = dom
        .world()
        .get::<&elidex_plugin::InlineClientRects>(span)
        .expect("the multi-line span gets per-line client rects");
    assert_eq!(
        rects.0.len(),
        flow.fragments[0].lines.len(),
        "one client rect per kept line (sliced consistently with the flow)"
    );
    for (i, line) in flow.fragments[0].lines.iter().enumerate() {
        assert!(
            (line.block_start - rects.0[i].origin.y).abs() < 0.01,
            "flow line {i} block_start {} == clientRects top {} (single source)",
            line.block_start,
            rects.0[i].origin.y
        );
    }
}

#[test]
fn paged_fragment_drops_tail_after_break() {
    // When orphans/widows force a break, the persisted fragment holds only the
    // lines up to the break point — the tail (next page's lines) is dropped from
    // this fragment's `lines` (and its box geometry), not left overshooting.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("aaa bbb ccc ddd") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Measure one line's block size from the un-fragmented 4-line flow.
    layout_inline_context(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let measured = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("four-line flow persists");
    assert_eq!(
        measured.fragments[0].lines.len(),
        4,
        "four single-word lines"
    );
    let line_h = measured.fragments[0].lines[1].block_start;
    drop(measured);
    assert!(line_h > 0.0);

    // Available block fits ~2 lines → break after line 2 (orphans/widows = 1).
    let frag = InlineFragConstraint {
        available_block: line_h * 2.5,
        orphans: 1,
        widows: 1,
        skip_lines: 0,
        fragmentation_type: crate::FragmentationType::Page,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("first paged fragment persists");
    assert_eq!(
        flow.fragments[0].lines.len(),
        2,
        "only the two lines that fit this page; the tail moved to page 2"
    );
    assert_eq!(
        flow.fragments[0].lines[0].block_start, 0.0,
        "first fragment starts at the fragmentainer top (skip 0 → no rebase)"
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
