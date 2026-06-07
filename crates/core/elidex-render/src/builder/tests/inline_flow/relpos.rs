//! `position:relative` inline sub-flow: the top-level flow leaves an in-flow gap
//! where the relpos span sits (CSS 2 §9.4.3); a sub-flow keyed on the span's first
//! child paints the span's text in that gap, exactly once.

use super::*;

/// Slice 3p-b: a `position:relative` inline `<span>` converges as a per-positioned-
/// subtree SUB-FLOW. Render (unchanged) consumes it end-to-end: Layer 5 paints the
/// top-level flow (`a`,`c`) keyed on the run-start `a`, leaving the in-flow GAP where
/// the span sits (CSS 2 §9.4.3); Layer 6 `walk(span)` consumes the sub-flow keyed on
/// the span's first child (`b`). `b` is painted EXACTLY ONCE (the legacy path would
/// either drop the gap or — for a nested relpos — double-paint).
#[test]
#[allow(unused_must_use)]
#[allow(clippy::too_many_lines)]
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
                justify_word_spacing: 0.0,
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
                justify_word_spacing: 0.0,
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
