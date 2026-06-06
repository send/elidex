//! Slice 4 / I-multicol: whole-IFC-in-column `InlineFlow` persistence + column shift.

use super::*;

#[test]
fn multicol_column_run_shifted_to_column_offset() {
    // Slice 4 / I-multicol crux: a whole-in-column IFC in column 1 has its persisted
    // `InlineFlow` SHIFTED to the column's inline offset, not just its `LayoutBox`.
    // Before the shifter convergence the column shift moved only `LayoutBox`, so the
    // converged column's inline text repainted at column 0.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0), // definite → sequential fill
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);
    // Each block (height 50 == column height) fills a column: A in col 0, B in col 1.
    let (_div_a, text_a) = add_text_block(&mut dom, container, "Alpha", 50.0);
    let (div_b, text_b) = add_text_block(&mut dom, container, "Beta", 50.0);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Container 600 wide, 2 columns, gap 0 → column 0 at x=0, column 1 at x=300
    // (offset = 1 * (geom.width + gap) = 1 * (300 + 0)). No padding/margin, so the
    // run's inline_start equals the column's content-box left edge exactly.
    let a_x = flow_inline_start(&dom, text_a);
    let b_x = flow_inline_start(&dom, text_b);
    let b_box_x = dom
        .world()
        .get::<&LayoutBox>(div_b)
        .unwrap()
        .content
        .origin
        .x;
    assert!(a_x.abs() < 1.0, "column-0 flow stays at x=0, got {a_x}");
    assert!(
        (b_x - 300.0).abs() < 1.0,
        "column-1 flow shifted to the exact column offset (width 300, gap 0), got {b_x}"
    );
    assert!(
        (b_x - b_box_x).abs() < 1.0,
        "the flow moved WITH its box (flow {b_x} == box {b_box_x}) — not left behind at column 0"
    );
}

#[test]
fn multicol_balanced_persists_one_fragment_per_run() {
    // D-mc3 overwrite-safety: balanced fill re-lays via probes (probe_total_height +
    // binary search), but with no accumulate the final pass overwrites every probe
    // flow — each run-start ends with EXACTLY ONE fragment, no probe garbage.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Balance, // → binary-search probes
        ..ComputedStyle::default()        // auto block-size → always balances
    };
    let _ = dom.world_mut().insert_one(container, style);
    let texts: Vec<Entity> = (0..4)
        .map(|i| add_text_block(&mut dom, container, &format!("Item{i}"), 40.0).1)
        .collect();

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    for tnode in texts {
        let flow = dom
            .world()
            .get::<&InlineFlow>(tnode)
            .expect("each whole-in-column IFC persists");
        assert_eq!(
            flow.fragments.len(),
            1,
            "balanced-fill probes must leave exactly one fragment (overwrite-safety, no accumulate)"
        );
    }
}

#[test]
fn multicol_spanner_ifc_persists_nonfragmented() {
    // A `column-span: all` spanner's IFC is laid full-width with `fragmentainer: None`
    // → persists as a NON-fragmented InlineFlow at its real position (no column shift).
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(100.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let spanner_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(30.0),
        font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, spanner_style);
    let spanner_text = dom.create_text("Spanner");
    dom.append_child(spanner, spanner_text);

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let flow = dom
        .world()
        .get::<&InlineFlow>(spanner_text)
        .expect("spanner IFC persists (non-fragmented, fragmentainer None)");
    assert_eq!(
        flow.fragments.len(),
        1,
        "spanner = a non-fragmented single flow"
    );
    assert_eq!(flow.fragments[0].generation, 0);
    let x = flow.fragments[0].lines[0].runs[0].inline_start();
    assert!(
        x.abs() < 1.0,
        "spanner flow at the full-width origin (not column-shifted), got {x}"
    );
}

// NOTE (slice 4 / I-multicol): vertical-WM column shift is covered transitively, not by
// a dedicated integration test here. The multicol→`shift_descendants` wiring is WM-agnostic
// (proven by `multicol_column_run_shifted_to_column_offset`); the physical-delta→block/inline
// projection by the run-start's parent WM is `shift_descendants`' own responsibility, tested in
// `elidex-layout-block` (slice 3p-a/3p-b-2 vertical InlineFlow shift); and multicol's vertical
// box shift (`Vector::y_only`) is exercised by `vertical_rl_columns`/`vertical_lr_columns`. The
// vertical flow shift is the composition of these already-tested pieces.

#[test]
fn multicol_balanced_overflow_shifts_flow_to_columns_no_column_zero_ghost() {
    // Regression (I-multicol correctness review 2026-06-06): with `column-fill:
    // balance` and a `max-height` too small to fit content in `column-count`
    // columns, the spec creates overflow columns in the inline direction (CSS
    // Multicol L1 §8.2) — content is NOT dropped.
    //
    // The balanced fill's definitive pass used to cap at `column-count`, dropping
    // the overflow children. With whole-in-column persistence (#291) the bug
    // surfaces as a stale flow: the unconstrained height probe persists a gen-0
    // `InlineFlow` at column-0 on every child's run-start; a child the capped
    // definitive pass never re-laid-out kept that column-0 probe flow (never
    // overwritten by its real per-column flow), which render paints as a ghost at
    // column-0. This is the overflow complement of
    // `multicol_balanced_persists_one_fragment_per_run` (the no-overflow
    // overwrite-safety case): the definitive pass must lay out the *full* child
    // set so every overflow child's flow is overwritten and shifted to its column.
    let font_db = make_font_db();
    if !fonts_available(&font_db) {
        return;
    }
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Balance,
        // 5 blocks × 50px = 250px content; at the 60px max-height column cap only
        // one 50px block fits per column → 5 columns needed, far over `count = 2`
        // (2 in-flow + 3 overflow).
        max_height: Dimension::Length(60.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);
    let texts: Vec<Entity> = (0..5)
        .map(|i| add_text_block(&mut dom, container, &format!("Col{i}"), 50.0).1)
        .collect();

    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Container 600 wide, 2 columns, gap 0 → column width 300. Child `i` is whole
    // in column `i`, so its flow's inline_start == i × 300. An overflow child the
    // old cap dropped would keep its probe flow stranded at column-0 (x ≈ 0).
    for (i, &tnode) in texts.iter().enumerate() {
        let x = flow_inline_start(&dom, tnode);
        #[allow(clippy::cast_precision_loss)]
        let expected = i as f32 * 300.0;
        assert!(
            (x - expected).abs() < 1.0,
            "column {i} flow at x={x}, expected column offset {expected} (overflow child stranded at column-0?)"
        );
    }

    // §8.2: overflow columns actually created — all 5 children participate.
    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    let total_columns: u32 = info.segments.iter().map(|&(count, _, _)| count).sum();
    assert!(
        total_columns >= 5,
        "expected ≥5 columns (overflow per §8.2), got {total_columns}"
    );
}
