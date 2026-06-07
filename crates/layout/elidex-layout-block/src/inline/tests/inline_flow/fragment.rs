use super::*;
use elidex_ecs::InlineFlow;

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
        is_probe: false,
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
