//! Vertical-writing-mode consume: the flow's per-line projection maps `block_start`
//! to physical x (columns) and `inline_start` to physical y (downward glyph advance).

use super::*;

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
                justify_word_spacing: 0.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "AB".to_string(),
                    inline_start: 0.0,
                }],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
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
