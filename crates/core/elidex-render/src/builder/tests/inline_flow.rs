//! Render-side tests for consuming a layout-produced `InlineFlow` (the converged
//! inline-text path) vs falling back to the legacy single-line emit.

use super::*;
use crate::display_list::DisplayItem;
use elidex_ecs::{InlineFlow, InlineFlowLine, InlineFlowRun, InlineFragment};

/// Collect the glyph vectors of every `Text` display item, in order.
fn text_item_glyphs(
    dl: &crate::display_list::DisplayList,
) -> Vec<&Vec<crate::display_list::GlyphEntry>> {
    dl.0.iter()
        .filter_map(|i| match i {
            DisplayItem::Text { glyphs, .. } => Some(glyphs),
            _ => None,
        })
        .collect()
}

/// When the run-start entity carries an `InlineFlow`, render consumes it: one
/// `Text` item per line, positioned at the line's `block_start` (multi-line —
/// the correctness fix the legacy single-line path could not produce).
#[test]
#[allow(unused_must_use)]
fn consumes_inline_flow_per_line() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 40.0),
            ..Default::default()
        },
    );
    // The text node is the run-start (render's run[0]) and carries the flow.
    let text = dom.create_text("HelloWorld");
    dom.append_child(div, text);

    // Simulate layout output: two lines, runs styled by the div.
    let flow = InlineFlow::single(
        0,
        vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "Hello".to_string(),
                    inline_start: 5.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "World".to_string(),
                    inline_start: 0.0,
                }],
            },
        ],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    assert_eq!(items.len(), 2, "two lines → two Text items from the flow");
    assert_eq!(items[0].len(), 5, "line 1 = \"Hello\"");
    assert_eq!(items[1].len(), 5, "line 2 = \"World\"");

    // Per-line baselines differ by the block_start delta (20) — proves multi-line
    // positioning (the legacy path put everything on one baseline).
    let y0 = items[0][0].position.y;
    let y1 = items[1][0].position.y;
    assert!(
        (y1 - y0 - 20.0).abs() < 0.5,
        "line 2 baseline is one line below line 1; got y0={y0}, y1={y1}"
    );
    // inline_start is honoured: line 1 starts near x = 5.0 (glyph x_offset ~ 0).
    let x0 = items[0][0].position.x;
    assert!(
        (x0 - 5.0).abs() < 2.0,
        "line 1 inline_start = 5.0, got x={x0}"
    );
}

/// An `AtomicBox` member makes render paint the atomic inline-level box: it walks
/// the entity at its `LayoutBox`, emitting the box chrome (here a background). This
/// is the slice-3p-a correctness fix — a static inline-block was previously
/// flattened to its text and its box (background/border/descendants) never painted.
#[test]
#[allow(unused_must_use)]
fn consumes_atomic_box_member_paints_box() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 40.0),
            ..Default::default()
        },
    );
    // The text node is the run-start (carries the flow); the inline-block is the
    // atomic member painted by walk().
    let text = dom.create_text("x");
    dom.append_child(div, text);
    let ib = dom.create_element("span", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        ib,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::InlineBlock,
            background_color: elidex_plugin::CssColor::RED,
            ..Default::default()
        },
    );
    // The LayoutBox layout repositioned to the atomic's on-line position.
    let ib_rect = Rect::new(10.0, 0.0, 30.0, 20.0);
    dom.world_mut().insert_one(
        ib,
        elidex_plugin::LayoutBox {
            content: ib_rect,
            ..Default::default()
        },
    );
    dom.append_child(div, ib);

    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "x".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::AtomicBox {
                    entity: ib,
                    inline_start: 10.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    // walk(atomic) emitted the inline-block's red background at its box rect — and
    // exactly once (the box is painted only via the flow member, not also as a
    // separately-walked child).
    let red_boxes: Vec<&Rect> = dl
        .0
        .iter()
        .filter_map(|i| match i {
            DisplayItem::SolidRect { rect, color } if *color == elidex_plugin::CssColor::RED => {
                Some(rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        red_boxes.len(),
        1,
        "the atomic inline-block's box must be painted exactly once via walk(), got {:?}",
        dl.0
    );
    assert!(
        (red_boxes[0].origin.x - 10.0).abs() < 0.5,
        "atomic box painted at its repositioned x=10, got {}",
        red_boxes[0].origin.x
    );
}

/// Without an `InlineFlow` the run falls back to the legacy path: the whole text
/// node is one segment on a single baseline. This proves the gate routes by
/// component presence.
#[test]
#[allow(unused_must_use)]
fn falls_back_without_inline_flow() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("HelloWorld");
    dom.append_child(div, text);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    assert_eq!(items.len(), 1, "no flow → single legacy segment");
    assert_eq!(items[0].len(), 10, "\"HelloWorld\" = 10 glyphs on one line");
}

/// The converged path must NOT re-apply `text-transform`: layout already
/// transformed the persisted run text, so render paints it verbatim. We persist
/// the same run text `"abc"` under a div whose `text-transform` is `None` vs
/// `Uppercase`; the painted glyph ids must be identical (if render re-transformed,
/// the `Uppercase` build would shape "ABC" — different glyph ids). CSS Text 3 §2.1.
#[test]
#[allow(unused_must_use)]
fn converged_path_does_not_re_transform() {
    fn glyph_ids(transform: elidex_plugin::TextTransform) -> Vec<u32> {
        let (mut dom, div) = setup_block_element(
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::Block,
                font_family: test_font_family_strings(),
                text_transform: transform,
                ..Default::default()
            },
            elidex_plugin::LayoutBox {
                content: Rect::new(0.0, 0.0, 800.0, 20.0),
                ..Default::default()
            },
        );
        let text = dom.create_text("abc");
        dom.append_child(div, text);
        // Layout already applied the transform: the persisted text is final ("abc").
        let flow = InlineFlow::single(
            0,
            vec![InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "abc".to_string(),
                    inline_start: 0.0,
                }],
            }],
        );
        dom.world_mut().insert_one(text, flow);
        let font_db = elidex_text::FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        text_item_glyphs(&dl)
            .first()
            .map(|g| g.iter().map(|e| e.glyph_id).collect())
            .unwrap_or_default()
    }

    let baseline = glyph_ids(elidex_plugin::TextTransform::None);
    assert!(!baseline.is_empty(), "verbatim 'abc' should paint glyphs");
    assert_eq!(
        glyph_ids(elidex_plugin::TextTransform::Uppercase),
        baseline,
        "converged path re-transformed the already-transformed run text (double-transform)"
    );
}

/// Regression: an absolutely-positioned child interspersed between inline text
/// under a stacking-context parent (here the root) must not cause the second
/// text run to be painted twice. The stacking-context paint path (Layer 5) must
/// skip the positioned child WITHOUT splitting the inline run, so the whole run
/// keys on the same run-start `InlineFlow` layout persisted (matching
/// `paint_non_sc` and layout's IFC grouping).
#[test]
#[allow(unused_must_use)]
fn interspersed_abspos_does_not_double_paint() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let _ = dom.world_mut().insert_one(
        root,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        root,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );

    // Inline text — absolutely positioned span — inline text.
    let text1 = dom.create_text("AAA");
    dom.append_child(root, text1);
    let abspos = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        abspos,
        elidex_plugin::ComputedStyle {
            position: elidex_plugin::Position::Absolute,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );
    dom.append_child(root, abspos);
    let text2 = dom.create_text("BBB");
    dom.append_child(root, text2);

    // Layout persists one InlineFlow on the run-start (text1) spanning both runs
    // (the abspos is out-of-flow and contributes no run).
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: root,
                    text: "AAA".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: root,
                    text: "BBB".to_string(),
                    inline_start: 40.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text1, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    // Both runs painted exactly once, from the flow (not text2 a second time via
    // a split-off legacy run).
    assert_eq!(
        items.len(),
        2,
        "two flow runs painted once each (no double-paint of the post-abspos run)"
    );
    let total_glyphs: usize = items.iter().map(|g| g.len()).sum();
    assert_eq!(
        total_glyphs, 6,
        "AAA + BBB = 6 glyphs total, none duplicated"
    );
}

/// Slice 2: a vertical-writing-mode parent dispatches to the vertical consume path.
/// Layout stored the flow with the writing-mode projection already applied, so each
/// line's `block_start` is the column's physical x and `inline_start` is the run's
/// physical y. Render reads `block_start` as x (the swap) and advances glyphs DOWN
/// the page — multi-column vertical text the legacy single-column pass could not
/// produce.
#[test]
#[allow(unused_must_use)]
fn consumes_inline_flow_vertical_columns() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 40.0, 800.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("ABAB");
    dom.append_child(div, text);

    // Vertical layout output: two columns. block_start = physical x of the column's
    // block-start edge, block_size = column width → center_x = block_start +
    // block_size/2 (= 10 and 30). inline_start = physical y (pen top).
    // Both columns use the SAME text so their leading glyphs are identical — the
    // x comparison below then isolates the column-center delta without depending on
    // per-glyph metrics (which vary by the CI runner's first available font).
    let flow = InlineFlow::single(
        0,
        vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "AB".to_string(),
                    inline_start: 0.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "AB".to_string(),
                    inline_start: 0.0,
                }],
            },
        ],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    assert_eq!(
        items.len(),
        2,
        "two columns → two Text items from the vertical flow"
    );

    // Columns are separated along physical x by the block_start delta (20): the
    // vertical consume reads block_start as x (the projection swap), not y. With
    // identical leading glyphs the per-glyph x-offset cancels, so the delta is the
    // exact column-center difference (tight tolerance, not font-dependent).
    let x0 = items[0][0].position.x;
    let x1 = items[1][0].position.x;
    assert!(
        (x1 - x0 - 20.0).abs() < 0.5,
        "second column sits one column to the right (center_x delta = 20); got x0={x0}, x1={x1}"
    );

    // Within a column, glyphs advance DOWN the page (inline axis = y).
    assert!(items[0].len() >= 2, "column 0 has multiple glyphs");
    assert!(
        items[0][1].position.y > items[0][0].position.y,
        "glyphs advance downward within a column; got y0={}, y1={}",
        items[0][0].position.y,
        items[0][1].position.y
    );
}

/// Slice 3p-b: a `position:relative` inline `<span>` converges as a per-positioned-
/// subtree SUB-FLOW. Render (unchanged) consumes it end-to-end: Layer 5 paints the
/// top-level flow (`a`,`c`) keyed on the run-start `a`, leaving the in-flow GAP where
/// the span sits (CSS 2 §9.4.3); Layer 6 `walk(span)` consumes the sub-flow keyed on
/// the span's first child (`b`). `b` is painted EXACTLY ONCE (the legacy path would
/// either drop the gap or — for a nested relpos — double-paint).
#[test]
#[allow(unused_must_use)]
fn consumes_relpos_inline_subflow_with_gap() {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let _ = dom.world_mut().insert_one(
        root,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        root,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );

    // `a` — relpos span(`b`) — `c`.
    let a_text = dom.create_text("a");
    dom.append_child(root, a_text);
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        elidex_plugin::ComputedStyle {
            position: elidex_plugin::Position::Relative,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        span,
        elidex_plugin::LayoutBox {
            content: Rect::new(24.0, 0.0, 16.0, 20.0),
            ..Default::default()
        },
    );
    let b_text = dom.create_text("b");
    dom.append_child(span, b_text);
    dom.append_child(root, span);
    let c_text = dom.create_text("c");
    dom.append_child(root, c_text);

    // Top-level flow on `a` (render's Layer-5 run[0]): `a` at x=0, `c` at x=40 —
    // the GAP (24..40) is where the span's `b` sits; `c` is NOT at a's end.
    dom.world_mut().insert_one(
        a_text,
        InlineFlow::single(
            0,
            vec![InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![
                    InlineFlowRun::Text {
                        entity: root,
                        text: "a".to_string(),
                        inline_start: 0.0,
                    },
                    InlineFlowRun::Text {
                        entity: root,
                        text: "c".to_string(),
                        inline_start: 40.0,
                    },
                ],
            }],
        ),
    );
    // Sub-flow on the span's first child `b` (render's `walk(span)` run[0]): `b` in
    // the gap at x=24.
    dom.world_mut().insert_one(
        b_text,
        InlineFlow::single(
            0,
            vec![InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun::Text {
                    entity: span,
                    text: "b".to_string(),
                    inline_start: 24.0,
                }],
            }],
        ),
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    // a (Layer 5), c (Layer 5), b (Layer 6 walk(span)) — each ONCE, b not duplicated.
    assert_eq!(
        items.len(),
        3,
        "a + c (top-level) + b (sub-flow) = 3 Text items, b painted exactly once"
    );
    let total_glyphs: usize = items.iter().map(|g| g.len()).sum();
    assert_eq!(total_glyphs, 3, "1 glyph each (a, b, c), none duplicated");

    // The painted leading-glyph x positions reflect the gap: ~0 (a), ~24 (b), ~40 (c).
    let mut xs: Vec<f32> = items.iter().map(|g| g[0].position.x).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((xs[0] - 0.0).abs() < 2.0, "`a` at x≈0, got {}", xs[0]);
    assert!(
        (xs[1] - 24.0).abs() < 2.0,
        "`b` in the gap at x≈24, got {}",
        xs[1]
    );
    assert!(
        (xs[2] - 40.0).abs() < 2.0,
        "`c` past the span at x≈40 (in-flow gap preserved, NOT x≈a's end), got {}",
        xs[2]
    );
}

/// On the paged path (`expected_generation == Some(g)`), a flow carrying a
/// fragment per page paints ONLY the fragment whose `generation` matches the
/// page being walked — the render half of the I-paged per-page consume (D4).
#[test]
#[allow(unused_must_use)]
fn paged_consume_paints_only_matching_generation() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 40.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("PageOnePageTwo");
    dom.append_child(div, text);

    // The run-start carries two fragments — one per page it spans.
    dom.world_mut().insert_one(
        text,
        InlineFlow {
            fragments: vec![
                InlineFragment {
                    generation: 1,
                    lines: vec![InlineFlowLine {
                        block_start: 0.0,
                        block_size: 20.0,
                        runs: vec![InlineFlowRun::Text {
                            entity: div,
                            text: "PageOne".to_string(),
                            inline_start: 0.0,
                        }],
                    }],
                },
                InlineFragment {
                    generation: 2,
                    lines: vec![InlineFlowLine {
                        block_start: 0.0,
                        block_size: 20.0,
                        runs: vec![InlineFlowRun::Text {
                            entity: div,
                            text: "PageTwo".to_string(),
                            inline_start: 0.0,
                        }],
                    }],
                },
            ],
        },
    );

    // Render the inline run with the paged context pinned to page 2.
    let font_db = elidex_text::FontDatabase::new();
    let mut dl = crate::display_list::DisplayList::default();
    let mut font_cache = FontCache::new();
    let mut ctx = PaintContext {
        dom: &dom,
        font_db: &font_db,
        font_cache: &mut font_cache,
        dl: &mut dl,
        caret_visible: false,
        scroll_offset: elidex_plugin::Vector::<f32>::ZERO,
        counter_state: elidex_style::counter::CounterState::new(),
        paged: true,
        expected_generation: Some(2),
        continuation_entities: None,
    };
    emit_inline_run(
        &mut ctx,
        div,
        &[text],
        0,
        &elidex_plugin::transform_math::Perspective::default(),
        false,
    );

    // Exactly the page-2 fragment paints: one Text item (page 1 is filtered out).
    let items = text_item_glyphs(&dl);
    assert_eq!(
        items.len(),
        1,
        "only the generation-2 fragment paints on page 2, not generation-1"
    );
}
