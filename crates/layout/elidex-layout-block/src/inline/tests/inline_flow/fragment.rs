use super::*;
use elidex_ecs::InlineFlow;

#[test]
fn paged_fragmented_run_now_persists() {
    // Slice 4 / I-paged inverts the old `gate_excludes_fragmented` test: its
    // reason ("flow_lines are not yet sliced per fragment") is resolved by THIS
    // slice's `truncate_fragment`, so a *paged* fragmented run now
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
        progress: None,
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
    // (fresh run, fits → no break) now persists an `InlineFlow` — it was gated
    // to legacy pre-I-multicol. And persisting changes ONLY `InlineFlow` presence: the
    // persisted geometry is byte-identical to the trusted non-fragmented layout of the
    // same content (D-mc2 — the packer's optimistic recording for `Column` does not
    // perturb `entity_bounds`/the packer geometry, which it commits unconditionally).
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
        progress: None,
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
    // A multicol continuation (the tail of a column-spanning IFC)
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
        progress: Some(
            layout_inline_context_fragmented(
                &mut dom,
                &children,
                1.0,
                parent,
                Point::ZERO,
                &env(&font_db),
                Some(&InlineFragConstraint {
                    available_block: 0.0,
                    orphans: 1,
                    widows: 1,
                    progress: None,
                    fragmentation_type: crate::FragmentationType::Column,
                }),
            )
            .continuation
            .expect("first fragment must split after one line"),
        ),
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
        "a multicol continuation with incoming source coverage must stay on legacy (mid-IFC → Z)"
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
        progress: None,
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
        progress: None,
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
        progress: Some(
            layout_inline_context_fragmented(
                &mut dom,
                &children,
                1.0,
                parent,
                Point::ZERO,
                &env(&font_db),
                Some(&InlineFragConstraint {
                    available_block: 0.0,
                    orphans: 1,
                    widows: 1,
                    progress: None,
                    fragmentation_type: crate::FragmentationType::Column,
                }),
            )
            .continuation
            .expect("first fragment must split after one line"),
        ),
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
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let span = dom.create_element_ownerless_fixture("span", Attributes::default());
    let span_text = dom.create_text_ownerless_fixture("aaa bbb ccc ddd");
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
        progress: Some({
            // This fixture isolates fresh CSSOM geometry for the continuation.
            let mut progress = crate::source_progress::SourceProgress::default();
            progress.consume(
                parent,
                span_text,
                crate::source_progress::SourcePosition::byte(0)
                    ..crate::source_progress::SourcePosition::byte(4),
            );
            progress
        }),
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
        progress: None,
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

fn cursor_fixture(text: &str) -> (EcsDom, Entity, Entity, ComputedStyle, FontDatabase) {
    let font_db = FontDatabase::from_fixture(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../core/elidex-render/tests/fixtures/digits.ttf"
    )));
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("p", Attributes::default());
    let item = dom.create_text_ownerless_fixture(text);
    assert!(dom.append_child(parent, item));
    let style = ComputedStyle {
        font_family: vec!["Elidex Test Digits".into()],
        ..Default::default()
    };
    dom.world_mut().insert_one(parent, style.clone()).unwrap();
    (dom, parent, item, style, font_db)
}

#[test]
fn varying_width_pages_resume_at_source_on_count_and_replay() {
    let (mut dom, parent, _, _, font_db) = cursor_fixture("A B C D");
    let children = dom.composed_children(parent);
    let key = children[0];
    let params = TextMeasureParams {
        families: &["Elidex Test Digits"],
        font_size: crate::get_style(&dom, parent).font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    let width = |text| measure_text(&font_db, &params, text).unwrap().width;
    let narrow = ["A ", "B ", "C ", "D "]
        .map(width)
        .into_iter()
        .fold(0.0, f32::max)
        + 0.5;
    let wide = ["A B ", "B C ", "C D "]
        .map(width)
        .into_iter()
        .fold(0.0, f32::max)
        + 0.5;
    for (widths, expected) in [
        (vec![narrow, wide, wide], vec!["A", "B C", "D"]),
        (vec![wide, narrow, narrow], vec!["A B", "C", "D"]),
        (vec![wide, wide], vec!["A B", "C D"]),
    ] {
        let mut starts: Vec<Option<crate::source_progress::SourceProgress>> = Vec::new();
        let mut progress = None;
        for replay in [false, true] {
            for (i, &page_width) in widths.iter().enumerate() {
                if replay {
                    progress = starts[i].clone();
                } else {
                    starts.push(progress.clone());
                }
                let frag = InlineFragConstraint {
                    available_block: 0.0,
                    orphans: 1,
                    widows: 1,
                    progress,
                    fragmentation_type: crate::FragmentationType::Page,
                };
                progress = layout_inline_context_fragmented(
                    &mut dom,
                    &children,
                    page_width,
                    parent,
                    Point::ZERO,
                    &env(&font_db),
                    Some(&frag),
                )
                .continuation;
                let flow = dom.world().get::<&InlineFlow>(key).unwrap();
                let lines = &flow.fragments[0].lines;
                assert_eq!(lines.len(), 1);
                let text: String = lines[0].runs.iter().filter_map(|r| r.text()).collect();
                assert_eq!(text.trim_end(), expected[i]);
                assert_eq!(lines[0].runs[0].inline_start(), 0.0);
                assert_eq!(progress.is_some(), i + 1 < widths.len());
                if let Some(cursor) = &progress {
                    let next = "A B C D".find(expected[i + 1]).unwrap();
                    assert!(cursor.covers(
                        parent,
                        key,
                        crate::source_progress::SourcePosition::byte(0)
                            ..crate::source_progress::SourcePosition::byte(next)
                    ));
                    assert!(!cursor.contains(
                        parent,
                        key,
                        crate::source_progress::SourcePosition::byte(next)
                    ));
                    if replay {
                        assert_eq!(starts[i + 1].as_ref(), Some(cursor));
                    }
                }
            }
        }
    }
}

#[test]
fn fragment_cursor_keeps_original_nodes_across_collapse_transform_and_atomics() {
    use crate::source_progress::SourcePosition;
    let (mut dom, parent, first, style, font_db) = cursor_fixture("A   ");
    let empty = dom.create_element_ownerless_fixture("span", Attributes::default());
    let nested = dom.create_element_ownerless_fixture("span", Attributes::default());
    let word = dom.create_text_ownerless_fixture("   ß  C ");
    let atomic = dom.create_element_ownerless_fixture("img", Attributes::default());
    let last = dom.create_text_ownerless_fixture("D");
    dom.append_child(parent, empty);
    dom.append_child(parent, nested);
    dom.append_child(nested, word);
    dom.append_child(parent, atomic);
    dom.append_child(parent, last);
    dom.world_mut()
        .insert_one(
            empty,
            ComputedStyle {
                padding: elidex_plugin::EdgeSizes {
                    left: elidex_plugin::Dimension::Length(1.0),
                    ..Default::default()
                },
                ..style.clone()
            },
        )
        .unwrap();
    dom.world_mut()
        .insert_one(
            nested,
            ComputedStyle {
                text_transform: elidex_plugin::TextTransform::Uppercase,
                ..style.clone()
            },
        )
        .unwrap();
    dom.world_mut()
        .insert_one(
            atomic,
            ComputedStyle {
                display: elidex_plugin::Display::InlineBlock,
                width: elidex_plugin::Dimension::Length(10.0),
                height: elidex_plugin::Dimension::Length(10.0),
                ..style
            },
        )
        .unwrap();
    let children = dom.composed_children(parent);
    let mut progress = None;
    for (i, expected) in ["A", "SS", "C", "", "D"].into_iter().enumerate() {
        let frag = InlineFragConstraint {
            available_block: 0.0,
            orphans: 1,
            widows: 1,
            progress,
            fragmentation_type: crate::FragmentationType::Page,
        };
        progress = layout_inline_context_fragmented(
            &mut dom,
            &children,
            1.0,
            parent,
            Point::ZERO,
            &env(&font_db),
            Some(&frag),
        )
        .continuation;
        let flow = dom.world().get::<&InlineFlow>(first).unwrap();
        let runs = &flow.fragments[0].lines[0].runs;
        let text: String = runs.iter().filter_map(|r| r.text()).collect();
        assert_eq!(text.trim_end(), expected);
        if i == 3 {
            assert!(
                matches!(runs[0],elidex_ecs::InlineFlowRun::AtomicBox {entity,..} if entity==atomic)
            );
        }
        if let Some(cursor) = &progress {
            assert!(cursor.is_finished(parent, first));
            assert!(cursor.is_finished(parent, empty));
            assert!(cursor.covers(
                parent,
                word,
                crate::source_progress::SourcePosition::byte(0)..SourcePosition::byte(if i == 0 {
                    3
                } else {
                    7
                })
            ));
            assert_eq!(cursor.is_finished(parent, atomic), i >= 3);
        }
    }
    assert!(progress.is_none());
}

#[test]
fn fragment_cursor_keeps_zero_height_committed_line_and_geometry() {
    let (mut dom, parent, _, style, font_db) = cursor_fixture("");
    let mut nodes = Vec::new();
    for (word, height) in [("A", 16.0), ("B", 0.0), ("C", 16.0)] {
        let span = dom.create_element_ownerless_fixture("span", Attributes::default());
        let text = dom.create_text_ownerless_fixture(word);
        dom.append_child(parent, span);
        dom.append_child(span, text);
        dom.world_mut()
            .insert_one(
                span,
                ComputedStyle {
                    line_height: elidex_plugin::LineHeight::Px(height),
                    ..style.clone()
                },
            )
            .unwrap();
        nodes.push((span, text));
    }
    let children = dom.composed_children(parent);
    let key = children[0];
    let mut progress = None;
    for expected in ["AB", "C"] {
        let frag = InlineFragConstraint {
            available_block: 16.0,
            orphans: 1,
            widows: 1,
            progress,
            fragmentation_type: crate::FragmentationType::Page,
        };
        progress = layout_inline_context_fragmented(
            &mut dom,
            &children,
            1.0,
            parent,
            Point::ZERO,
            &env(&font_db),
            Some(&frag),
        )
        .continuation;
        let flow = dom.world().get::<&InlineFlow>(key).unwrap();
        let lines = &flow.fragments[0].lines;
        let words: String = lines
            .iter()
            .flat_map(|l| &l.runs)
            .filter_map(|r| r.text())
            .collect();
        assert_eq!(words, expected);
        if expected == "AB" {
            let cursor = progress.as_ref().unwrap();
            assert!(cursor.is_finished(parent, nodes[1].1));
            assert!(!cursor.is_finished(parent, nodes[2].1));
            assert_eq!((lines[1].block_start, lines[1].block_size), (16.0, 0.0));
            assert_eq!(
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(nodes[1].0)
                    .unwrap()
                    .content
                    .origin
                    .y,
                16.0
            );
            assert!(dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(nodes[2].0)
                .is_err());
        }
    }
    assert!(progress.is_none());
}

#[test]
fn fragment_cursor_tracks_forced_then_soft_break() {
    let (mut dom, parent, key, mut style, font_db) = cursor_fixture("a\nß c");
    style.white_space = elidex_plugin::WhiteSpace::PreWrap;
    style.text_transform = elidex_plugin::TextTransform::Uppercase;
    dom.world_mut().insert_one(parent, style).unwrap();
    let mut progress = None;
    for expected in ["A", "SS", "C"] {
        let frag = InlineFragConstraint {
            available_block: 1.0,
            orphans: 1,
            widows: 1,
            progress,
            fragmentation_type: crate::FragmentationType::Page,
        };
        progress = layout_inline_context_fragmented(
            &mut dom,
            &[key],
            1.0,
            parent,
            Point::ZERO,
            &env(&font_db),
            Some(&frag),
        )
        .continuation;
        let flow = dom.world().get::<&InlineFlow>(key).unwrap();
        let text: String = flow.fragments[0]
            .lines
            .iter()
            .flat_map(|l| &l.runs)
            .filter_map(|r| r.text())
            .collect();
        assert_eq!(text.trim_end(), expected);
    }
    assert!(progress.is_none());
}
