//! Z-1b consume (Option D): a multicol mid-break **inline** formatting context — a
//! single IFC whose line boxes span a column break — persists an `InlineFlow` on
//! its run-start with **all columns' lines at their baked per-column inline
//! offsets** (one fragment, gen 0; columns coexist on one surface at absolute
//! coords), consumed by the **existing** `emit_inline_flow`. This is the text half
//! of the box+text convergence; the standalone box store stays Z-1a dark.
//!
//! Distinct from the **nested-block** mid-break (`box_fragment::add_spanning_block`,
//! whose div breaks at *block-child* boundaries) — that stays legacy/G11 (D-Z2,
//! committed-next) and gets NO carrier/`InlineFlow`. Here the div IS the IFC
//! container (`parent_entity`), its inline text wrapping past the column height.

use super::*;

/// A block child holding a long inline text string (no fixed height — the text
/// extent drives wrapping), so its IFC wraps to many lines and breaks across the
/// multicol column boundary **mid-IFC** (the Z-1b case). Returns `(div, text-node)`;
/// the text node is the IFC run-start (`run[0]`) key carrying the `InlineFlow`.
fn add_wrapping_text_block(dom: &mut EcsDom, parent: Entity, text: &str) -> (Entity, Entity) {
    let div = elem(dom, "div");
    dom.append_child(parent, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let tnode = dom.create_text(text);
    dom.append_child(div, tnode);
    (div, tnode)
}

/// `(inline_start, block_start)` of every line's first run in a run-start's flow,
/// across all fragments (Z-1b mid-break = one fragment holding all columns' lines).
fn flow_line_positions(dom: &EcsDom, run_start: Entity) -> Vec<(f32, f32)> {
    let flow = dom
        .world()
        .get::<&InlineFlow>(run_start)
        .expect("the mid-break IFC persists an InlineFlow on its run-start (Z-1b)");
    flow.fragments
        .iter()
        .flat_map(|f| f.lines.iter())
        .filter_map(|l| l.runs.first().map(|r| (r.inline_start(), l.block_start)))
        .collect()
}

// A long text that wraps to several lines in a ~300px column (6 test fonts at
// 16px) — enough to overflow a 2-line column and break across columns.
const LONG_TEXT: &str = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed \
do eiusmod tempor incididunt ut labore et dolore magna aliqua enim ad minim veniam";

/// Build a 2-column (width 300, gap 0, height 40 → mid-break) container whose
/// direct-child div is an IFC: leading `LONG_TEXT`, then an inline-block atomic
/// styled by `ib_style`, then trailing `LONG_TEXT`. The atomic is logically after
/// the leading text, so it lands in a column past column 0 (terminal-Z C-2 box
/// reposition target). Returns `(container, ib, run_start)`.
fn midbreak_ifc_with_atomic(dom: &mut EcsDom, ib_style: ComputedStyle) -> (Entity, Entity, Entity) {
    let container = elem(dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let div = elem(dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let run_start = dom.create_text(LONG_TEXT);
    dom.append_child(div, run_start);
    let ib = elem(dom, "span");
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(div, ib);
    let trailing = dom.create_text(LONG_TEXT);
    dom.append_child(div, trailing);
    (container, ib, run_start)
}

/// The default inline-block atomic style for the C-2 reposition tests (30×16,
/// no margin/border/padding → content origin == margin-box origin == reposition
/// target).
fn inline_block_style() -> ComputedStyle {
    ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(30.0),
        height: Dimension::Length(16.0),
        font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
        ..ComputedStyle::default()
    }
}

#[test]
fn multicol_midbreak_ifc_persists_flow_with_per_column_lines() {
    // The Z-1b crux: a single IFC whose wrapped lines exceed the column height
    // breaks mid-IFC across columns. BEFORE Z-1b this fell to the legacy
    // single-linear render route (no `InlineFlow`); now the run-start carries an
    // `InlineFlow` whose lines sit at their per-column inline offsets — column 0 at
    // x≈0, column 1 at x≈300 (container 600 / 2 columns, gap 0 → column width 300).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            // ~40px fits ≥2 lines (16px font, ~18px line) so the orphans=2 default
            // permits the break; the long text wraps well past it → mid-IFC break.
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let pos = flow_line_positions(&dom, tnode);
    assert!(
        pos.len() >= 2,
        "mid-break IFC persists multiple lines, got {pos:?}"
    );
    let col0 = pos.iter().filter(|(x, _)| x.abs() < 50.0).count();
    let col1 = pos.iter().filter(|(x, _)| (x - 300.0).abs() < 50.0).count();
    assert!(col0 > 0, "lines in column 0 (x≈0), got {pos:?}");
    assert!(
        col1 > 0,
        "lines in column 1 (x≈300) — the mid-break tail rendered in its column, \
         not lost (legacy) and not double-shifted to x≈600, got {pos:?}"
    );
}

#[test]
fn multicol_midbreak_ifc_column_one_lines_not_double_shifted() {
    // Double-shift guard (the highest-attention Option-D edge): the run-start's
    // `InlineFlow` is built AFTER the per-column `shift_descendants` loop, so the
    // column-1 lines carry the column offset baked EXACTLY ONCE (at commit). A
    // double-shift (the col_children `shift_descendants` ALSO moving the flow) would
    // push the column-1 lines to x≈600, leaving NO line at x≈300. So asserting a
    // line EXISTS at x≈300 is the robust double-shift guard (it survives legit
    // overflow columns at x≈600).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let pos = flow_line_positions(&dom, tnode);
    let at_col1 = pos.iter().any(|(x, _)| (x - 300.0).abs() < 50.0);
    assert!(
        at_col1,
        "a column-1 line sits at x≈300 (one column delta, baked once) — \
         not double-shifted to x≈600, got {pos:?}"
    );
}

#[test]
fn multicol_midbreak_ifc_column_one_first_line_rebased_to_column_top() {
    // Per-column continuation rebase: the column-1 portion's first line sits at the
    // column block-start (block_start ≈ 0, the multicol content top), side-by-side
    // with column 0 — NOT stacked below column 0's lines (the founding mid-IFC bug
    // the legacy single-linear pass never fixed). Among the column-1 lines (x≈300),
    // the topmost has block_start ≈ 0.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let pos = flow_line_positions(&dom, tnode);
    let col1_top = pos
        .iter()
        .filter(|(x, _)| (x - 300.0).abs() < 50.0)
        .map(|(_, y)| *y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        col1_top.is_finite(),
        "there is a column-1 line, got {pos:?}"
    );
    assert!(
        col1_top.abs() < 20.0,
        "the column-1 portion's first line is rebased to the column top (block≈0), \
         not stacked below column 0 — got block_start {col1_top}"
    );
}

#[test]
fn multicol_midbreak_ifc_spanning_three_columns_lines_in_each() {
    // A wrapping IFC that spans THREE columns exercises the middle column (column 1),
    // where the spanning child both RESUMES into the column AND BREAKS OUT of it (the
    // carry_midbreak ∪ break_out_child dedup, which fill collapses to one drain). The
    // box_fragment test pins this for box fragments; this pins it for the InlineFlow
    // LINES — one column's worth at each of x≈0, x≈200, x≈400 (container 600 / 3
    // columns, gap 0 → column width 200).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(3),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let pos = flow_line_positions(&dom, tnode);
    for (col, x) in [(0u32, 0.0_f32), (1, 200.0), (2, 400.0)] {
        assert!(
            pos.iter().any(|(px, _)| (px - x).abs() < 50.0),
            "column {col}: a line at x≈{x} (per-column lines across the 3-column span, \
             middle column included), got {pos:?}"
        );
    }
}

#[test]
fn multicol_whole_in_column_ifc_still_single_fragment() {
    // Dual-home: a whole-in-column IFC (fits one column) is NOT a mid-break — it
    // persists its 1-fragment `InlineFlow` via the IFC path + column shift
    // (I-multicol), unchanged by Z-1b. Pins that the mid-break path does not perturb
    // the whole-in-column home (each consumed by exactly one build site).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(200.0), // tall → each short block whole in its column
            ..ComputedStyle::default()
        },
    );
    let (_a, text_a) = add_text_block(&mut dom, container, "Alpha", 50.0);
    let (_b, text_b) = add_text_block(&mut dom, container, "Beta", 50.0);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    for t in [text_a, text_b] {
        let flow = dom
            .world()
            .get::<&InlineFlow>(t)
            .expect("whole-in-column IFC persists");
        assert_eq!(
            flow.fragments.len(),
            1,
            "whole-in-column stays a single-fragment flow (unchanged by Z-1b)"
        );
    }
}

#[test]
fn multicol_balanced_midbreak_ifc_one_fragment_no_probe_leftover() {
    // Balanced fill re-lays via ≤12 probe passes (probe_total_height + binary
    // search). The run-start `InlineFlow` build in `position_column_fragments` is
    // `is_probe`-guarded (the box-push analogue) AND uses `insert_one` (replace), so
    // a mid-break IFC ends with EXACTLY ONE fragment carrying all columns' lines — no
    // probe-pass garbage accumulated, no duplicate lines. This is the InlineFlow
    // analogue of `box_fragment::multicol_balanced_midbreak_no_probe_leftover` (which
    // pins the box path) — the highest-recurrence trap class in this convergence
    // program (Z-1b-0's post-push review was a probe/upsert dedup bug).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Balance, // → binary-search probes
            ..ComputedStyle::default()        // auto block-size → always balances
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let flow = dom
        .world()
        .get::<&InlineFlow>(tnode)
        .expect("balanced mid-break IFC persists an InlineFlow");
    assert_eq!(
        flow.fragments.len(),
        1,
        "exactly one fragment — no probe-pass leftover, no duplicate accumulation \
         (is_probe-guarded build + insert_one replace)"
    );
    // The single fragment holds every column's lines at their baked offsets; lines
    // span ≥2 columns (the balance distributed the IFC across columns).
    let xs: Vec<f32> = flow.fragments[0]
        .lines
        .iter()
        .filter_map(|l| l.runs.first().map(elidex_ecs::InlineFlowRun::inline_start))
        .collect();
    let distinct_cols = xs.iter().any(|x| x.abs() < 50.0) && xs.iter().any(|x| *x > 100.0);
    assert!(
        distinct_cols,
        "the one fragment's lines span ≥2 columns (balanced mid-break), got {xs:?}"
    );
}

// NOTE (vertical writing modes): the per-column line offset bake in
// `position_column_fragments` (`run.inline_start_mut() += inline_offset`) is
// **axis-agnostic** — `inline_start` already holds the writing-mode-projected
// physical-per-axis coordinate (physical x for horizontal, physical y for
// vertical), and `inline_offset` is the column delta along that same inline axis,
// identical to the box snapshot's `Vector::x_only`/`y_only(inline_offset)` delta
// which IS exercised for vertical by `geometry::vertical_rl_columns` /
// `vertical_lr_columns`. So the vertical mid-break IFC bake is the composition of
// already-tested pieces (the axis-agnostic fold + the vertical box delta), not a
// new axis path — covered transitively, mirroring the I-multicol vertical-shift
// transitive-coverage rationale in `inline_flow.rs`. (A dedicated vertical
// mid-break-IFC integration test is deferred for the same reason.)

#[test]
fn multicol_midbreak_ifc_with_atomic_repositions_box_to_its_column() {
    // Terminal-Z C-2 (atomic-as-fragment): a mid-break IFC containing a static atomic
    // inline (an inline-block) repositions the atomic's `LayoutBox` to its per-column
    // on-line position. The atomic is monolithic ⇒ it lands in exactly ONE column;
    // `position_column_fragments` repositions its box (and subtree) to the
    // born-absolute target carried by its `AtomicBox` run, and PRUNES it from the
    // generic per-column shift so the column offset is applied exactly once. Pin: the
    // atomic's box origin EQUALS its `AtomicBox` run's folded `inline_start` /
    // `block_start` — pre-C-2 the box stayed at column-0 base (x≈0) while the run was
    // folded to the column, so this assertion catches both the old gap AND a
    // double-shift (which would land the box at `target + delta_col`).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    // IFC: leading wrapping text, an inline-block atomic, then more wrapping text —
    // enough to overflow the column and break mid-IFC. Run-start = the first text node.
    let div = elem(&mut dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let run_start = dom.create_text(LONG_TEXT);
    dom.append_child(div, run_start);
    let ib = elem(&mut dom, "span");
    let _ = dom.world_mut().insert_one(
        ib,
        ComputedStyle {
            display: Display::InlineBlock,
            width: Dimension::Length(30.0),
            height: Dimension::Length(16.0),
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    dom.append_child(div, ib);
    let trailing = dom.create_text(LONG_TEXT);
    dom.append_child(div, trailing);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Text deliverable holds with the atomic present: the run-start persists an
    // InlineFlow whose lines span ≥2 columns.
    let flow = dom
        .world()
        .get::<&InlineFlow>(run_start)
        .expect("mid-break IFC with an atomic still persists its text InlineFlow");
    let xs: Vec<f32> = flow
        .fragments
        .iter()
        .flat_map(|f| f.lines.iter())
        .filter_map(|l| l.runs.first().map(elidex_ecs::InlineFlowRun::inline_start))
        .collect();
    assert!(
        xs.iter().any(|x| x.abs() < 50.0) && xs.iter().any(|x| (x - 300.0).abs() < 50.0),
        "text lines span columns 0 and 1 with the atomic present, got {xs:?}"
    );
    // Find the atomic's `AtomicBox` run (render walks it) and the line it sits on.
    let (run_inline, line_block) = flow
        .fragments
        .iter()
        .flat_map(|f| f.lines.iter())
        .find_map(|l| {
            l.runs.iter().find_map(|r| match r {
                elidex_ecs::InlineFlowRun::AtomicBox {
                    entity,
                    inline_start,
                } if *entity == ib => Some((*inline_start, l.block_start)),
                _ => None,
            })
        })
        .expect("the inline-block atomic is carried as an AtomicBox run");
    // The atomic landed in a column past column 0 (so the reposition is non-trivial:
    // delta_col ≠ 0 — a col-0-only test would mask the double-shift / pre-C-2 gap).
    assert!(
        run_inline >= 300.0,
        "the atomic's run is folded to column ≥1 (x≈300+), got {run_inline}"
    );
    // C-2: the atomic's `LayoutBox` origin equals its run's folded position — the box
    // is repositioned to its column, NOT left at column-0 base nor double-shifted.
    // No margin/border/padding on the inline-block ⇒ content origin == margin-box
    // origin == the folded inline/block target.
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic has a LayoutBox");
    assert!(
        (lb.content.origin.x - run_inline).abs() < 0.01,
        "atomic box x repositioned to its column: box.x {} vs run {run_inline}",
        lb.content.origin.x
    );
    assert!(
        (lb.content.origin.y - line_block).abs() < 0.01,
        "atomic box y at its line block-start: box.y {} vs line {line_block}",
        lb.content.origin.y
    );
}

#[test]
fn multicol_midbreak_flow_survives_a_throwaway_probe() {
    // Codex PR#316 R3 (P2) + R4 (Z-1b-0.5 prereq #318): the run-start `InlineFlow` is
    // built by `position_column_fragments` (is_probe-guarded → not rebuilt during a
    // probe). A throwaway probe running AFTER a definitive layout must leave that live
    // flow BIT-FOR-BIT intact — neither erased (R3d: the do_carrier path preserves it
    // under is_probe instead of clearing) NOR shifted (R4: the is_probe-aware
    // `shift_descendants` skips the persisted-`InlineFlow` arm during a probe). Pin:
    // lay definitively (flow built at per-column coords), snapshot the coords, re-lay
    // with is_probe=true (an ancestor/intrinsic probe) — the flow survives at the SAME
    // coordinates (presence alone is too weak; a probe-shift would pass it).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    // Snapshot every line's (block_start, per-run inline_start) — the absolute coords
    // render consumes — so the post-probe assertion catches a shift, not just erasure.
    let flow_coords = |dom: &EcsDom| -> Vec<(f32, Vec<f32>)> {
        dom.world()
            .get::<&InlineFlow>(tnode)
            .ok()
            .map(|flow| {
                flow.fragments
                    .iter()
                    .flat_map(|f| f.lines.iter())
                    .map(|l| {
                        (
                            l.block_start,
                            l.runs
                                .iter()
                                .map(elidex_ecs::InlineFlowRun::inline_start)
                                .collect(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);
    let before = flow_coords(&dom);
    assert!(
        !before.is_empty(),
        "definitive layout builds the mid-break flow with lines"
    );

    // A throwaway probe over the same subtree (e.g. an ancestor's balanced search /
    // intrinsic re-measure) must NOT erase OR shift the live flow.
    let probe_input = LayoutInput {
        is_probe: true,
        ..make_input(&font_db)
    };
    layout_multicol(&mut dom, container, &probe_input, layout_child_fn);
    let after = flow_coords(&dom);
    assert!(
        !after.is_empty(),
        "a throwaway probe after the definitive layout must NOT erase the live \
         mid-break InlineFlow (else render drops to the legacy path)"
    );
    assert_eq!(
        before.len(),
        after.len(),
        "probe must not change the mid-break flow's line count"
    );
    for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
        assert!(
            (b.0 - a.0).abs() < 0.01,
            "line {i} block_start unchanged by the probe: before {} after {}",
            b.0,
            a.0
        );
        assert_eq!(
            b.1.len(),
            a.1.len(),
            "line {i} run count unchanged by the probe"
        );
        for (bx, ax) in b.1.iter().zip(a.1.iter()) {
            assert!(
                (bx - ax).abs() < 0.01,
                "line {i} run inline_start unchanged by the probe (the per-column \
                 baked offset must survive — a probe-shift would corrupt it): \
                 before {bx} after {ax}"
            );
        }
    }
}

#[test]
fn multicol_midbreak_multigroup_probe_preserves_all_subflows() {
    // Codex PR#316 R3 post-rebase (P2): a mid-break IFC is laid PER-COLUMN, so a
    // probe of column 0 sees only column-0 run groups in `persisted_keys`. The
    // `clear_inline_flows` call (now `is_probe`-gated) must NOT erase a run group
    // whose lines fall in a LATER column — a `position:relative` inline sub-flow is a
    // separate group keyed on the span's first child, and the probe deliberately does
    // not rebuild it (`position_column_fragments` is `is_probe`-guarded). The earlier
    // single-group `multicol_midbreak_flow_survives_a_throwaway_probe` cannot catch
    // this: it needs ≥2 groups in one IFC, with one group absent from column 0's
    // slice. Pin: lay definitively (both the top-level text AND the relpos sub-flow
    // get flows), probe, assert EVERY group that had a flow still has one.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    // ONE IFC (a div) with TWO run groups: a long top-level text (fills column 0 and
    // wraps into column 1) + a trailing `position:relative` inline span whose own
    // text is a SEPARATE sub-flow (keyed on the span's text node), landing in a later
    // column.
    let div = elem(&mut dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let prefix = dom.create_text(LONG_TEXT);
    dom.append_child(div, prefix);
    let span = elem(&mut dom, "span");
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let span_text = dom.create_text("relative positioned sub-flow trailing text content");
    dom.append_child(span, span_text);
    dom.append_child(div, span);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Every run-start carrying a flow after the definitive layout (presence is the
    // facet under test — the probe-clear erasure).
    let with_flow = |dom: &EcsDom| -> Vec<Entity> {
        [prefix, span_text]
            .into_iter()
            .filter(|&e| dom.world().get::<&InlineFlow>(e).is_ok())
            .collect()
    };
    let before = with_flow(&dom);
    assert_eq!(
        before.len(),
        2,
        "the definitive layout persists BOTH the top-level text flow and the relpos \
         span sub-flow (multi-group mid-break); got {before:?}"
    );

    // A throwaway probe must preserve ALL existing mid-break sub-flows, not just the
    // groups present in the first column's slice.
    let probe_input = LayoutInput {
        is_probe: true,
        ..make_input(&font_db)
    };
    layout_multicol(&mut dom, container, &probe_input, layout_child_fn);

    let after = with_flow(&dom);
    assert_eq!(
        before, after,
        "a throwaway probe must NOT clear a later-column run group's InlineFlow (the \
         multi-group clear-erasure facet) — every group that had a flow keeps it"
    );
}

#[test]
fn multicol_balanced_midbreak_flow_survives_probe_total_height() {
    // Codex PR#316 R2 (P2): `fill_columns_balanced` Step 1 (`probe_total_height`) lays
    // the IFC with NO fragmentainer (`is_probe=true`), so a mid-break IFC reaches the
    // `persist_flow` arm and (pre-fix) overwrote the live all-column mid-break flow
    // with single-column col-0 probe geometry; the probe-guarded column positioning
    // does not rebuild it, so the corruption survives to render. The earlier
    // probe-survival test uses `column-fill:auto` (no `probe_total_height`); this one
    // uses `balance` to exercise the persist_flow WRITE face. Pin: lay definitively
    // (per-column flow built), re-lay under an ancestor probe — coords unchanged.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Balance,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let (_div, tnode) = add_wrapping_text_block(&mut dom, container, LONG_TEXT);

    let coords = |dom: &EcsDom| -> Vec<(f32, f32)> {
        dom.world()
            .get::<&InlineFlow>(tnode)
            .ok()
            .map(|f| {
                f.fragments
                    .iter()
                    .flat_map(|fr| fr.lines.iter())
                    .map(|l| {
                        (
                            l.block_start,
                            l.runs
                                .first()
                                .map_or(0.0, elidex_ecs::InlineFlowRun::inline_start),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);
    let before = coords(&dom);
    assert!(
        !before.is_empty(),
        "balanced definitive layout builds the mid-break flow"
    );

    let probe_input = LayoutInput {
        is_probe: true,
        ..make_input(&font_db)
    };
    layout_multicol(&mut dom, container, &probe_input, layout_child_fn);
    let after = coords(&dom);
    assert_eq!(
        before, after,
        "a probe (incl. the balanced `probe_total_height` no-fragmentainer pass) must \
         NOT overwrite the live mid-break flow with single-column col-0 geometry"
    );
}

#[test]
fn multicol_midbreak_clipping_block_carriers_and_is_consumable() {
    // terminal-Z C-1 (retiring the #316 `midbreak_clips` legacy-fallback): a mid-break
    // IFC whose own block clips overflow now ALSO carriers (Option D) and is flagged
    // **consumable** in the box store, so render's fragment-walk paints it per-column —
    // a SEPARATE clip per column, the converged `InlineFlow` re-emitted under each
    // disjoint column clip (each line surviving in exactly one column). This fixes the
    // col-0-clipped-away regression #316 deferred (§2.6 hard invariant: the carrier +
    // the per-column box store coincide on the same entity). Pin: BOTH the clipping and
    // non-clipping mid-break block persist the run-start `InlineFlow`, have ≥2 box-store
    // fragments on the IFC container, and the container is `is_consumable`.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    // (run-start InlineFlow present, #box-store fragments, container is_consumable).
    let probe = |clip: bool| -> (bool, usize, bool) {
        let mut dom = EcsDom::new();
        let container = elem(&mut dom, "div");
        let _ = dom.world_mut().insert_one(
            container,
            ComputedStyle {
                display: Display::Block,
                column_count: Some(2),
                column_fill: ColumnFill::Auto,
                height: Dimension::Length(40.0),
                ..ComputedStyle::default()
            },
        );
        let div = elem(&mut dom, "div");
        dom.append_child(container, div);
        let mut div_style = ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        };
        if clip {
            div_style.overflow_x = elidex_plugin::Overflow::Hidden;
        }
        let _ = dom.world_mut().insert_one(div, div_style);
        let tnode = dom.create_text(LONG_TEXT);
        dom.append_child(div, tnode);
        let input = make_input(&font_db);
        layout_multicol(&mut dom, container, &input, layout_child_fn);
        let has_flow = dom.world().get::<&InlineFlow>(tnode).is_ok();
        let n_box = dom.fragment_tree().fragments_for(div).count();
        let consumable = dom.fragment_tree().is_consumable(div);
        (has_flow, n_box, consumable)
    };
    for clip in [false, true] {
        let (has_flow, n_box, consumable) = probe(clip);
        assert!(
            has_flow,
            "mid-break block (clip={clip}) persists the converged per-column InlineFlow (Option D)"
        );
        assert!(
            n_box >= 2,
            "mid-break block (clip={clip}) has a per-column box-store fragment in each \
             spanned column (≥2) — the geometry render's fragment-walk clips per column"
        );
        assert!(
            consumable,
            "a direct-child IFC mid-break (clip={clip}) is store-flagged consumable, so \
             render paints per-column chrome + clip + content"
        );
    }
}

#[test]
fn multicol_relay_collapsing_a_span_to_whole_drops_stale_store_fragments() {
    // Codex PR#321 R4-F4 + R6-F1: a definitive re-lay within one pass (no store clear
    // between) can COLLAPSE a child from spanning multiple columns to fitting whole in
    // one column. It then drops out of `box_snapshots`/`own`, so an `own`-only cleanup
    // would leave its prior per-column store fragments behind — and the render router
    // (which ORs over `fragments_for`) would paint phantom columns. The committer now
    // `remove_entity`s every direct child before re-committing, so the collapsed child
    // is de-indexed: no stale fragments, not consumable.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let set_height = |dom: &mut EcsDom, h: f32| {
        let _ = dom.world_mut().insert_one(
            container,
            ComputedStyle {
                display: Display::Block,
                column_count: Some(2),
                column_fill: ColumnFill::Auto,
                height: Dimension::Length(h),
                ..ComputedStyle::default()
            },
        );
    };
    let div = elem(&mut dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let tnode = dom.create_text(LONG_TEXT);
    dom.append_child(div, tnode);

    // Lay 1: short columns ⇒ the div spans both columns (mid-break, consumable).
    set_height(&mut dom, 40.0);
    layout_multicol(&mut dom, container, &make_input(&font_db), layout_child_fn);
    assert!(
        dom.fragment_tree().fragments_for(div).count() >= 2
            && dom.fragment_tree().is_consumable(div),
        "lay 1: the div spans ≥2 columns and is consumable"
    );

    // Lay 2 (same pass, no clear): a tall column fits all content in column 0 ⇒ the div
    // collapses to whole and no longer appears in any column's box snapshots.
    set_height(&mut dom, 4000.0);
    layout_multicol(&mut dom, container, &make_input(&font_db), layout_child_fn);
    assert_eq!(
        dom.fragment_tree().fragments_for(div).count(),
        0,
        "lay 2: the collapsed div's prior per-column store fragments are removed"
    );
    assert!(
        !dom.fragment_tree().is_consumable(div),
        "the collapsed div is no longer consumable — render uses its single LayoutBox"
    );
}

#[test]
fn multicol_with_direct_inline_midbreak_leaves_no_stale_carrier() {
    // Codex PR#316 R1 (P2): direct inline content in a multicol container makes the
    // IFC `parent_entity` BE the multicol itself, so `fill` (which drains carriers off
    // its snapshotted direct mid-break *children*) never drains the self-carrier. If
    // left on the container and the container is later an OUTER multicol's mid-break
    // direct child, the outer `fill` would fold this stale carrier at the outer column
    // offset (garbage). `layout_multicol` must clear its own container's carrier after
    // laying. Pin: a multicol whose direct inline content breaks mid-column carries NO
    // `ColumnFlowSlice` afterward.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    // Direct inline content (a text node child of the multicol container itself).
    let text = dom.create_text(LONG_TEXT);
    dom.append_child(container, text);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    assert!(
        dom.world()
            .get::<&elidex_ecs::ColumnFlowSlice>(container)
            .is_err(),
        "multicol container's own self-carrier is cleared after layout (no leak to an \
         ancestor multicol's drain)"
    );
}

#[test]
fn multicol_nested_block_midbreak_gets_no_inline_flow() {
    // D-Z2 (deferred): a div that breaks at *block-child* boundaries (nested-block
    // mid-break) is NOT an IFC container at the break — it stays legacy/G11 and gets
    // NO carrier/`InlineFlow` from Z-1b. (Its box fragments ARE populated, Z-1a, but
    // that is dark.) Pins the Z-1b scope: only IFC mid-breaks fold into `InlineFlow`.
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    // Two 50px block children → the div breaks at the block boundary (col-0/col-1).
    let span = super::box_fragment::add_spanning_block(&mut dom, container, 2, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    assert!(
        dom.world().get::<&InlineFlow>(span).is_err(),
        "nested-block mid-break (D-Z2) gets no InlineFlow — stays legacy/G11"
    );
}

/// The `(inline_start, block_start)` of `atomic`'s `AtomicBox` run in `run_start`'s
/// flow, or `None` if it is not a flow member (e.g. a relpos atomic) / not found.
fn atomic_run_position(dom: &EcsDom, run_start: Entity, atomic: Entity) -> Option<(f32, f32)> {
    let flow = dom.world().get::<&InlineFlow>(run_start).ok()?;
    flow.fragments
        .iter()
        .flat_map(|f| f.lines.iter())
        .find_map(|l| {
            l.runs.iter().find_map(|r| match r {
                elidex_ecs::InlineFlowRun::AtomicBox {
                    entity,
                    inline_start,
                } if *entity == atomic => Some((*inline_start, l.block_start)),
                _ => None,
            })
        })
}

/// An entity's `LayoutBox` content-origin (panics if absent).
fn box_origin(dom: &EcsDom, e: Entity) -> Point {
    dom.world()
        .get::<&LayoutBox>(e)
        .expect("entity has a LayoutBox")
        .content
        .origin
}

#[test]
fn multicol_midbreak_relpos_atomic_repositions_to_its_column() {
    // C-2 §2.2: a `position:relative` mid-break atomic is NOT a flow member, so its
    // reposition record (placement + un-offset basis) is carried out via the unified
    // `atomic_repositions` carrier (which holds both static and relpos atomics), built
    // from the relpos placements rather than the `AtomicBox` flow runs. The seam
    // repositions its box to the column AND preserves the baked relative offset (the
    // delta basis is the *un-offset* origin, so `+= target − un-offset` lands it at
    // `target + offset`). Pin (two layouts over the same flow): the relpos box sits
    // exactly `left` to the right of where the SAME atomic sits as a static
    // inline-block — offset preserved, applied on top of the per-column reposition.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    // Static reference layout: read the atomic's repositioned column x.
    let mut dom_s = EcsDom::new();
    let (c_s, ib_s, _) = midbreak_ifc_with_atomic(&mut dom_s, inline_block_style());
    let input = make_input(&font_db);
    layout_multicol(&mut dom_s, c_s, &input, layout_child_fn);
    let static_x = box_origin(&dom_s, ib_s).x;
    // Relpos layout: same structure + `position:relative; left:10px`.
    let mut dom_r = EcsDom::new();
    let mut style = inline_block_style();
    style.position = Position::Relative;
    style.left = Dimension::Length(10.0);
    let (c_r, ib_r, run_start_r) = midbreak_ifc_with_atomic(&mut dom_r, style);
    layout_multicol(&mut dom_r, c_r, &input, layout_child_fn);
    let relpos_x = box_origin(&dom_r, ib_r).x;
    // The relpos atomic is NOT carried as a flow member (render Layer 6 paints it).
    assert!(
        atomic_run_position(&dom_r, run_start_r, ib_r).is_none(),
        "a relpos atomic is not an AtomicBox flow member (would double-paint)"
    );
    // The atomic landed in a column past column 0 (non-trivial reposition).
    assert!(
        static_x >= 300.0,
        "static atomic repositioned to column ≥1, got {static_x}"
    );
    // The relpos atomic is repositioned to its column via the unified
    // `atomic_repositions` carrier — born-absolute to the SAME target as the static one
    // (excluded from the
    // generic per-column shift, so no double-shift accumulation; a col-shifted relpos
    // atomic in multiple columns' `frag.children` would land at a multiple of the
    // offset). Here `relpos_x == static_x` because this test's `layout_child_fn`
    // (`layout_block_inner`) does NOT bake `apply_relative_offset` (see
    // `inline/tests/relpos_subflow.rs`), so the box carries no relative offset to
    // preserve. Offset-PRESERVATION (basis = un-offset origin) is covered by the
    // `reposition_atomic_box` unit tests in the block crate, which use a baking
    // `layout_child`; here we pin the C-2-specific behavior: the relpos atomic is
    // carried out and repositioned to its column (not stranded at column-0 base, not
    // double-shifted).
    assert!(
        (relpos_x - static_x).abs() < 0.01,
        "relpos atomic repositioned to its column (born-absolute, no double-shift): \
         relpos {relpos_x} vs static {static_x}"
    );
}

#[test]
fn multicol_midbreak_atomic_subtree_follows_box_to_its_column() {
    // C-2 §2.1: `reposition_atomic_box` moves the atomic's WHOLE subtree
    // (`shift_descendants` of its composed children), and the generic per-column
    // shift PRUNES that subtree (so the descendant is not double-shifted). Pin: an
    // inline-block with a block child — after reposition the child sits at the same
    // offset INSIDE the atomic it had at column-0 base (rigid subtree move), and both
    // are in the atomic's column (not stranded at column-0).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    // Inline-block sized to hold a child; give it a child block.
    let (container, ib, run_start) = midbreak_ifc_with_atomic(&mut dom, inline_block_style());
    let child = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(10.0),
            height: Dimension::Length(10.0),
            ..ComputedStyle::default()
        },
    );
    dom.append_child(ib, child);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let (run_inline, _) =
        atomic_run_position(&dom, run_start, ib).expect("atomic carried as a flow run");
    assert!(
        run_inline >= 300.0,
        "atomic folded to column ≥1, got {run_inline}"
    );
    let ib_origin = box_origin(&dom, ib);
    let child_origin = box_origin(&dom, child);
    // The atomic box is at its column.
    assert!(
        (ib_origin.x - run_inline).abs() < 0.01,
        "atomic box repositioned to its column: {} vs run {run_inline}",
        ib_origin.x
    );
    // The child followed: it is inside the atomic's column (not at column-0 base),
    // at the SAME relative offset it had within the atomic (rigid subtree shift ⇒
    // child origin == atomic origin + the within-box offset; for a top-left child
    // with no atomic padding that offset is 0).
    assert!(
        (child_origin.x - ib_origin.x).abs() < 0.01,
        "child x followed the atomic to its column: child {} vs atomic {}",
        child_origin.x,
        ib_origin.x
    );
}

#[test]
fn multicol_midbreak_atomic_reposition_is_definitive_only_and_idempotent() {
    // C-2 §2.4: the atomic reposition runs ONLY on the definitive pass
    // (`position_column_fragments` commit block is `!is_probe`-gated), so it never
    // accumulates probe garbage. Unlike the `InlineFlow`/box-store (separate
    // probe-gated components, protected from a probe so they keep the definitive
    // value), the atomic's geometry lives in its `LayoutBox` — which EVERY pass
    // re-lays at column-0 base (it is throwaway working geometry, not a probe-gated
    // render component), so a probe re-lay + un-excluded column shift disturbs it on
    // purpose; the always-last definitive pass is what render sees. Pin the realistic
    // guarantee: a probe sandwiched between two definitive passes leaves the FINAL
    // box exactly where the first definitive pass put it (the reposition is
    // idempotent and recovers from a probe's disturbance — it does not, e.g.,
    // double-apply the column offset on the second definitive pass).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let (container, ib, run_start) = midbreak_ifc_with_atomic(&mut dom, inline_block_style());
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);
    let (run_inline, _) =
        atomic_run_position(&dom, run_start, ib).expect("atomic carried as a flow run");
    assert!(run_inline >= 300.0, "atomic folded to column ≥1");
    let first = box_origin(&dom, ib);
    // A throwaway probe (an ancestor/intrinsic pass) disturbs the LayoutBox...
    let mut probe = make_input(&font_db);
    probe.is_probe = true;
    layout_multicol(&mut dom, container, &probe, layout_child_fn);
    // ...then the next definitive pass restores it exactly (idempotent reposition).
    layout_multicol(&mut dom, container, &input, layout_child_fn);
    let again = box_origin(&dom, ib);
    assert!(
        (first.x - again.x).abs() < 0.01 && (first.y - again.y).abs() < 0.01,
        "definitive reposition is idempotent and recovers from a probe: {first:?} vs {again:?}"
    );
}

#[test]
fn multicol_midbreak_atomic_in_column0_repositions_to_on_line_position() {
    // C-2: a mid-break atomic in COLUMN 0 (delta_col = 0) is still repositioned to its
    // on-line position from the IFC top-left placement `layout_atomic_items` gave it.
    // A col-0-only test would mask the double-shift (delta_col = 0), so this complements
    // the col≥1 pins — it guards that the col-0 reposition is correct (box == run), not
    // that the bug is exercised. Atomic placed FIRST so it lands on line 0 of column 0.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let div = elem(&mut dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    // Atomic FIRST (line 0, column 0), then long text overflowing → mid-break.
    let ib = elem(&mut dom, "span");
    let _ = dom.world_mut().insert_one(ib, inline_block_style());
    dom.append_child(div, ib);
    let run_start = dom.create_text(LONG_TEXT);
    dom.append_child(div, run_start);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // The atomic's run-start key is the atomic itself if it is `run[0]`; the text node
    // otherwise. Search both for the `AtomicBox` run.
    let pos =
        atomic_run_position(&dom, ib, ib).or_else(|| atomic_run_position(&dom, run_start, ib));
    let (run_inline, line_block) = pos.expect("atomic carried as a flow run");
    assert!(
        run_inline < 300.0,
        "the leading atomic is in column 0 (x < column width), got {run_inline}"
    );
    let origin = box_origin(&dom, ib);
    assert!(
        (origin.x - run_inline).abs() < 0.01 && (origin.y - line_block).abs() < 0.01,
        "column-0 atomic box at its on-line position: box {origin:?} vs run ({run_inline}, {line_block})"
    );
}

#[test]
fn multicol_midbreak_atomic_vertical_rl_projects_inline_axis() {
    // C-2 §2.5: in a vertical writing mode the seam derives `is_vertical` from the
    // multicol element's own `wm` and `reposition_atomic_box` projects inline↔physical
    // accordingly — the inline axis is physical Y, the block axis is physical X. Pin:
    // the atomic's box origin matches its run with the axes SWAPPED (box.y ==
    // run.inline_start, box.x == line.block_start) — a horizontal-projection bug would
    // put box.x == run.inline_start instead.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            writing_mode: WritingMode::VerticalRl,
            // Vertical: block axis = horizontal = `width`; a small block extent
            // (40 / 2 cols = 20 per column ⇒ ~1 line) forces the mid-column break.
            width: Dimension::Length(40.0),
            ..ComputedStyle::default()
        },
    );
    let div = elem(&mut dom, "div");
    dom.append_child(container, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
            ..ComputedStyle::default()
        },
    );
    let run_start = dom.create_text(LONG_TEXT);
    dom.append_child(div, run_start);
    let ib = elem(&mut dom, "span");
    let _ = dom.world_mut().insert_one(ib, inline_block_style());
    dom.append_child(div, ib);
    let trailing = dom.create_text(LONG_TEXT);
    dom.append_child(div, trailing);

    // Vertical: inline axis = physical Y (height 600 available for the text flow).
    let mut input = make_input(&font_db);
    input.containing.height = Some(600.0);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let (run_inline, line_block) =
        atomic_run_position(&dom, run_start, ib).expect("atomic carried as a flow run");
    let origin = box_origin(&dom, ib);
    // Inline axis is Y, block axis is X (vertical projection).
    assert!(
        (origin.y - run_inline).abs() < 0.01,
        "vertical: atomic box Y at its inline_start: box.y {} vs run {run_inline}",
        origin.y
    );
    assert!(
        (origin.x - line_block).abs() < 0.01,
        "vertical: atomic box X at its line block-start: box.x {} vs line {line_block}",
        origin.x
    );
}
