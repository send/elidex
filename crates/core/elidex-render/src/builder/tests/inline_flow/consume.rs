//! Basic `InlineFlow` consume: per-line text, atomic boxes, legacy fallback, the
//! no-re-transform invariant, and interspersed-abspos no-double-paint.

use super::*;

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
                justify_word_spacing: 0.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "Hello".to_string(),
                    inline_start: 5.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
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
            justify_word_spacing: 0.0,
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

/// The converged path must paint the persisted run text verbatim, NOT re-apply
/// the entity's `text-transform` (layout already transformed before measuring).
/// As a probe we hold the persisted run text constant at `"abc"` for two builds
/// whose div `text-transform` differs (`None` vs `Uppercase`) — deliberately
/// *unrealistic* for `Uppercase` (real layout would persist "ABC"), so that any
/// style-driven re-transform on the paint path is observable: the glyph ids must
/// be identical, since a re-transform would shape "ABC" (different ids). §2.1.
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
        // Probe: hold the persisted run text constant at "abc" regardless of the
        // div's `text-transform` (real layout would persist "ABC" for Uppercase) so
        // a style-driven re-transform on the paint path would change the glyph ids.
        let flow = InlineFlow::single(
            0,
            vec![InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
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
            justify_word_spacing: 0.0,
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
