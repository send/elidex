//! Render-side tests for consuming a layout-produced `InlineFlow` (the converged
//! inline-text path) vs falling back to the legacy single-line emit.

use super::*;
use crate::display_list::DisplayItem;
use elidex_ecs::{InlineFlow, InlineFlowLine, InlineFlowRun};

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
    let flow = InlineFlow {
        lines: vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun {
                    entity: div,
                    text: "Hello".to_string(),
                    inline_start: 5.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun {
                    entity: div,
                    text: "World".to_string(),
                    inline_start: 0.0,
                }],
            },
        ],
        layout_generation: 0,
    };
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
    let flow = InlineFlow {
        lines: vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            runs: vec![
                InlineFlowRun {
                    entity: root,
                    text: "AAA".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun {
                    entity: root,
                    text: "BBB".to_string(),
                    inline_start: 40.0,
                },
            ],
        }],
        layout_generation: 0,
    };
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
    let flow = InlineFlow {
        lines: vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun {
                    entity: div,
                    text: "AB".to_string(),
                    inline_start: 0.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                runs: vec![InlineFlowRun {
                    entity: div,
                    text: "AB".to_string(),
                    inline_start: 0.0,
                }],
            },
        ],
        layout_generation: 0,
    };
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
