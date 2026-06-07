//! Paged consume: a flow carrying one fragment per page paints only the fragment
//! whose `generation` matches the page being walked (`expected_generation`).

use super::*;

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
