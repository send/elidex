use super::*;
use elidex_ecs::{InlineFlow, InlineFlowRun};
use elidex_plugin::{Direction, TextAlign};

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
