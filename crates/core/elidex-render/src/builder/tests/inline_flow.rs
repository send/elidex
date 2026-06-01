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
