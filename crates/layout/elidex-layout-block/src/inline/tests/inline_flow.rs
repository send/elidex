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
fn justify_persists_with_word_spacing() {
    // Slice 4 PR-3 inverts the old `gate_excludes_justify`: `text-align: justify` is
    // the 4th alignment (CSS Text 3 §6), layout-baked like start/center/end. It now
    // persists an `InlineFlow`. A WRAPPED (multi-line) justified paragraph distributes
    // free space on each soft-wrapped line (`justify_word_spacing > 0`) but NOT on the
    // block's last line (§6.3 `text-align-last: auto` → start). Container width is
    // measured so "cc" wraps with real free space on line 0.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("aa bb cc") else {
        return;
    };
    style.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Between the prefix that fits on line 0 and the full text → "cc" wraps, line 0
    // under-fills (positive free space to justify).
    let prefix = measure_width(&font_db, "aa bb");
    let full = measure_width(&font_db, "aa bb cc");
    let container = f32::midpoint(prefix, full);
    layout_inline_context(
        &mut dom,
        &children,
        container,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("text-align: justify now persists an InlineFlow (converged, slice 4)");
    let lines = &flow.fragments[0].lines;
    assert_eq!(
        lines.len(),
        2,
        "container chosen so 'cc' wraps to a 2nd line"
    );
    assert!(
        lines[0].justify_word_spacing > 0.0,
        "the soft-wrapped first line distributes free space, got {}",
        lines[0].justify_word_spacing
    );
    assert_eq!(
        lines[1].justify_word_spacing, 0.0,
        "the block's last line is NOT justified (CSS Text 3 §6.3 text-align-last:auto)"
    );
}

#[test]
fn justify_single_line_is_last_line_not_justified() {
    // A justified paragraph that fits on ONE line: that line is the block's last line
    // (§6.3) → start-aligned, `justify_word_spacing == 0` (legacy wrongly justified it).
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("aa bb cc") else {
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

    let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
    let lines = &flow.fragments[0].lines;
    assert_eq!(lines.len(), 1, "fits on one line");
    assert_eq!(
        lines[0].justify_word_spacing, 0.0,
        "the only line IS the last line → start-aligned, not justified"
    );
    assert_eq!(
        lines[0].runs[0].inline_start(),
        0.0,
        "start-aligned: the run sits at the line origin (no justify shift)"
    );
}

#[test]
fn justify_suppressed_line_rtl_is_start_aligned() {
    // A suppressed justify line (here a single = last line) is start-aligned per
    // CSS Text 3 §6.3 (`text-align-last: auto` → `start`), NOT left-aligned. The
    // start edge in an RTL block is the RIGHT edge, so the run is offset to
    // `inline_start = free > 0` — regression for the bug where `align_offset(Justify)`
    // returned 0 and pinned the RTL last line to the left edge.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("aa bb") else {
        return;
    };
    style.text_align = TextAlign::Justify;
    style.direction = Direction::Rtl;
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

    {
        let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
        let line = &flow.fragments[0].lines[0];
        assert_eq!(line.justify_word_spacing, 0.0, "last line not justified");
        assert!(
            line.runs[0].inline_start() > 0.0,
            "RTL last line is start(right)-aligned (inline_start = free > 0), not left-pinned; got {}",
            line.runs[0].inline_start()
        );
    }
    // Sanity: an LTR last line stays at the left (start) edge — offset 0.
    let mut ltr = crate::get_style(&dom, parent);
    ltr.direction = Direction::Ltr;
    let _ = dom.world_mut().insert_one(parent, ltr);
    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let inline_start = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("persists")
        .fragments[0]
        .lines[0]
        .runs[0]
        .inline_start();
    assert_eq!(inline_start, 0.0, "LTR last line start edge = left = 0");
}

#[test]
fn justify_excludes_trailing_hang_from_opportunities() {
    // A soft-wrapped justify line ending in a collapsible space: that trailing space
    // HANGS (CSS Text 3 §4.1.2) and is NOT a justification opportunity, so the single
    // interior gap ("aa␠bb") absorbs ALL the (trimmed) free space and the visible words
    // reach the line-box edge. `justify_word_spacing` therefore equals the full trimmed
    // free (1 opportunity) — NOT half of it (which counting the trailing space would
    // give, under-filling the visible line).
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("aa bb cccc") else {
        return;
    };
    style.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Fits "aa bb " (trailing space hangs) but wraps "cccc".
    let prefix = measure_width(&font_db, "aa bb");
    let container = prefix + measure_width(&font_db, "aa");
    layout_inline_context(
        &mut dom,
        &children,
        container,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
    let lines = &flow.fragments[0].lines;
    assert_eq!(lines.len(), 2, "'cccc' wraps");
    let jws = lines[0].justify_word_spacing;
    let trimmed_free = container - prefix;
    assert!(
        (jws - trimmed_free).abs() < 3.0,
        "1 interior opportunity → jws == full trimmed free ({trimmed_free}), trailing space \
         excluded; got {jws} (≈half would mean the trailing hang was counted)"
    );
}

#[test]
fn justify_unexpandable_line_rtl_is_start_aligned() {
    // A soft-wrapped (NON-last) justify line with NO interior opportunities — a single
    // word whose trailing space hangs — is "unexpandable" (CSS Text 3 §6.4.3): it falls
    // back to `text-align-last: auto` → start. In RTL that is the RIGHT edge, so the
    // word lands at `inline_start = free > 0`, not pinned to the left.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("wwwwwwww xxxxxxxx") else {
        return;
    };
    style.text_align = TextAlign::Justify;
    style.direction = Direction::Rtl;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Fits one word per line → line 0 = "wwwwwwww " (0 interior opportunities), soft-wrap.
    let container = measure_width(&font_db, "wwwwwwww") + measure_width(&font_db, "ww");
    layout_inline_context(
        &mut dom,
        &children,
        container,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
    let lines = &flow.fragments[0].lines;
    assert_eq!(lines.len(), 2, "one word per line");
    assert_eq!(
        lines[0].justify_word_spacing, 0.0,
        "no interior opportunity → unexpandable, no distribution"
    );
    assert!(
        lines[0].runs[0].inline_start() > 0.0,
        "RTL unexpandable soft-wrap line is start(right)-aligned, not left-pinned; got {}",
        lines[0].runs[0].inline_start()
    );
}

#[test]
fn justify_overflow_line_no_negative_stretch() {
    // A line wider than the container (free < 0, clamped to 0) → no stretch, no
    // div-by-zero, no negative spacing. Tiny container forces a wrap; every line
    // over-fills so `justify_word_spacing == 0` throughout.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    style.text_align = TextAlign::Justify;
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

    let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
    for line in &flow.fragments[0].lines {
        assert_eq!(
            line.justify_word_spacing, 0.0,
            "an overflowing line (free clamped to 0) gets no stretch"
        );
        assert!(
            line.justify_word_spacing.is_finite(),
            "no NaN/inf from zero free or zero opportunities"
        );
    }
}

#[test]
fn justify_bakes_between_run_offset() {
    // Differential test (metric-independent): a justified soft-wrapped line bakes the
    // accumulated word-separator expansion into each run's `inline_start` (the
    // between-run part of justification). `<p>xx <em>yy</em> wwww…</p>`: a long unbreakable
    // tail word forces a wrap, so "xx " + "yy" land on a justified line 0 and the tail
    // wraps to line 1. The `em` run is preceded on its line by the "xx " run (one
    // word-separator), so under justify its `inline_start` shifts by exactly
    // `justify_word_spacing × 1` relative to the left-aligned natural position (the
    // metric unknowns cancel in the left-vs-justify difference).
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let head = dom.create_text("xx ");
    dom.append_child(parent, head);
    let em = dom.create_element("em", Attributes::default());
    // The em needs a ComputedStyle or `collect_inline_items` skips its subtree.
    let _ = dom.world_mut().insert_one(
        em,
        ComputedStyle {
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let em_text = dom.create_text("yy");
    dom.append_child(em, em_text);
    dom.append_child(parent, em);
    // A long, unbreakable tail word — wider than the container alone, so it always
    // wraps to its own line, leaving "xx yy" (plus a hung separator) on a justified
    // line 0. The pad over `measure("xx yy")` is the free space that line 0 distributes.
    let tail = dom.create_text(" wwwwwwwwwwwwwwwwwwww");
    dom.append_child(parent, tail);
    let container = measure_width(&font_db, "xx yy") + measure_width(&font_db, "xx");

    // Helper: lay out under `align`, find the em run wherever it landed, and return
    // (em inline_start, em's line justify_word_spacing, word-separators before em on its line).
    let mut run_layout = |dom: &mut EcsDom, align: TextAlign| -> (f32, f32, usize) {
        style.text_align = align;
        let _ = dom.world_mut().insert_one(parent, style.clone());
        let children = dom.composed_children(parent);
        layout_inline_context(
            dom,
            &children,
            container,
            parent,
            Point::ZERO,
            &env(&font_db),
        );
        let key = run_start(dom, parent);
        let flow = dom.world().get::<&InlineFlow>(key).expect("persists");
        for line in &flow.fragments[0].lines {
            if let Some(idx) = line.runs.iter().position(|r| r.entity() == em) {
                let preceding: usize = line.runs[..idx]
                    .iter()
                    .filter_map(InlineFlowRun::text)
                    .map(|t| {
                        t.chars()
                            .filter(|c| elidex_text::is_word_separator(*c))
                            .count()
                    })
                    .sum();
                return (
                    line.runs[idx].inline_start(),
                    line.justify_word_spacing,
                    preceding,
                );
            }
        }
        panic!("em run not found in any line");
    };

    let (natural_em, _, _) = run_layout(&mut dom, TextAlign::Left);
    let (baked_em, jws, preceding) = run_layout(&mut dom, TextAlign::Justify);
    assert!(
        jws > 0.0,
        "em's line is justified (free distributed), jws = {jws}"
    );
    assert_eq!(
        preceding, 1,
        "the em run is preceded by exactly one separator ('xx ')"
    );
    #[allow(clippy::cast_precision_loss)]
    let expected_shift = jws * preceding as f32;
    assert!(
        (baked_em - (natural_em + expected_shift)).abs() < 0.01,
        "em run shifted by jws×{preceding}: baked {baked_em} vs natural {natural_em} + {expected_shift}"
    );
}

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

// --- text-align/justify offset is applied to inline-element getClientRects/LayoutBox
// (entity_bounds), not only the persisted runs — so paint and CSSOM geometry agree.
// `commit_aligned_entity_rects` (CSSOM VIEW 1 §6 / CSS Text 3 §6.4). ---

/// Build `<p style="text-align:{align}"><span>{text}</span></p>`, lay it out at
/// `width`, and return `(dom, parent, span, font_db)`. The inner text's run is owned
/// by the span (its nearest styled ancestor), so the span gets an `entity_bounds`
/// rect → `LayoutBox` / `InlineClientRects`. `InlineFlow` persists on the span (the
/// run-start key) so a test can compare painted run geometry to the span's box.
fn setup_span_align(
    text: &str,
    align: TextAlign,
    width: f32,
) -> Option<(EcsDom, Entity, Entity, FontDatabase)> {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let span_text = dom.create_text(text);
    dom.append_child(span, span_text);
    dom.append_child(parent, span);
    let font_db = FontDatabase::new();
    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: ComputedStyle::default().font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    measure_text(&font_db, &params, "x")?;
    let style = ComputedStyle {
        font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
        text_align: align,
        direction: Direction::Ltr,
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let _ = dom.world_mut().insert_one(span, style);
    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        width,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    Some((dom, parent, span, font_db))
}

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

#[test]
fn justify_widens_inline_element_client_rect() {
    // A justified (non-last) line distributes free space at word separators, filling
    // the line to the box edge. The span's per-line client rect must WIDEN with the
    // painted text (CSS Text 3 §6.4) — reach the container's inline-end — not stay at
    // its natural (shorter) width.
    let text = "aaa bbb ccc ddd eee fff ggg hhh";
    let Some(full) = (|| {
        let (_d, _p, _s, fd) = setup_span_align(text, TextAlign::Left, 100_000.0)?;
        Some(measure_width(&fd, text))
    })() else {
        return;
    };
    // ~45% of the natural width → wraps into several lines; the first is justified.
    let width = full * 0.45;

    // Read the first-line client-rect inline-end of the span under an alignment.
    let first_line_rect_end = |align: TextAlign| -> Option<f32> {
        let (dom, _parent, span, _fd) = setup_span_align(text, align, width)?;
        let line_count = dom.world().get::<&InlineFlow>(span).ok()?.fragments[0]
            .lines
            .len();
        assert!(
            line_count >= 2,
            "text wraps into ≥2 lines, got {line_count}"
        );
        let rects = dom
            .world()
            .get::<&elidex_plugin::InlineClientRects>(span)
            .expect("a multi-line span gets per-line InlineClientRects");
        let r0 = rects.0[0];
        assert!(
            r0.origin.x < 1.0,
            "first-line rect starts at the line start"
        );
        Some(r0.origin.x + r0.size.width)
    };

    let Some(left_end) = first_line_rect_end(TextAlign::Left) else {
        return;
    };
    let Some(justify_end) = first_line_rect_end(TextAlign::Justify) else {
        return;
    };

    // Under left-align the first line stops at its natural content end (free space to
    // its right); under justify the same words spread to fill the box, so the span's
    // first-line client rect WIDENS to (at least) the container inline-end. Without the
    // fix the justified rect would stay at the un-expanded left-aligned width.
    assert!(
        justify_end > left_end + 1.0,
        "justified first-line rect ({justify_end}) widens past the left-aligned one ({left_end})"
    );
    assert!(
        justify_end >= width - 1.0,
        "justified first-line rect fills to the box edge {width}, got end {justify_end}"
    );
}
