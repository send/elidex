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
fn multicol_midbreak_ifc_with_atomic_text_still_persists() {
    // Robustness (Codex PR#316 R1): a mid-break IFC containing a static atomic
    // inline (an inline-block) must still persist its per-column TEXT correctly and
    // carry the `AtomicBox` run (render walks it) without panicking. The atomic's
    // per-column LayoutBox POSITION is committed-next (atomic-as-fragment, plan §C/§D)
    // — a mid-break IFC re-runs `layout_atomic_items` for the whole IFC every column,
    // so correct per-column atomic positioning needs the box-store fragment model;
    // Z-1b deliberately does NOT reposition mid-break atomics (deferred whole, not
    // half-fixed), keeping the pre-Z-1b box position. This pins the text deliverable's
    // robustness to an atomic's presence.
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
    // The atomic is carried as an `AtomicBox` member somewhere in the flow (render
    // walks it; its per-column position is committed-next).
    let has_atomic = flow.fragments.iter().flat_map(|f| f.lines.iter()).any(|l| {
        l.runs
            .iter()
            .any(|r| matches!(r, elidex_ecs::InlineFlowRun::AtomicBox { .. }))
    });
    assert!(
        has_atomic,
        "the inline-block atomic is carried as an AtomicBox run"
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
