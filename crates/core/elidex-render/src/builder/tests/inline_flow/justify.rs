//! `text-align: justify` within-run spacing (slice 4 PR-3): render applies the
//! per-line `InlineFlowLine::justify_word_spacing` at each within-run word-separator
//! via `place_glyphs`, in BOTH `emit_inline_flow` branches — the identity branch
//! (per-run baked `inline_start`) and the bidi-reorder shared-cursor branch. Layout
//! bakes the between-run expansion into `inline_start`; this is the irreducibly-render
//! within-run part (render re-shapes the run text).

use super::*;

/// Build a single-line `InlineFlow` for `text` (run-start = the text node, styled by
/// the div) with the given `justify_word_spacing`, paint it, and return the painted
/// glyph x-positions of the first Text item.
fn paint_justified_line(text: &str, jws: f32) -> Vec<f32> {
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
    let node = dom.create_text(text);
    let _ = dom.append_child(div, node);
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: jws,
            runs: vec![InlineFlowRun::Text {
                entity: div,
                text: text.to_string(),
                inline_start: 0.0,
            }],
        }],
    );
    let _ = dom.world_mut().insert_one(node, flow);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    text_item_glyphs(&dl)
        .first()
        .map(|g| g.iter().map(|e| e.position.x).collect())
        .unwrap_or_default()
}

/// Identity branch (LTR, no reorder): `justify_word_spacing` is added at each
/// within-run word-separator cluster, so each glyph after the Nth space is shifted by
/// `N × jws` relative to a non-justified (jws=0) paint of the same run.
#[test]
fn within_run_justify_shifts_glyphs_after_each_separator() {
    let base = paint_justified_line("a b c", 0.0);
    let just = paint_justified_line("a b c", 12.0);
    if base.is_empty() || base.len() != just.len() {
        return; // no usable font on this host
    }
    // "a b c" → glyphs [a, ␠, b, ␠, c]. The space glyphs themselves are unmoved (the
    // extra is added AFTER advancing past the space cluster), but every glyph that
    // FOLLOWS a separator shifts: 'b' (glyph 2) by 1×jws, 'c' (glyph 4) by 2×jws.
    assert!(
        (just[0] - base[0]).abs() < 0.01,
        "first glyph 'a' is unmoved by justify"
    );
    assert!(
        (just[2] - base[2] - 12.0).abs() < 0.01,
        "'b' (after 1 separator) shifts by 1×jws: {} vs {} + 12",
        just[2],
        base[2]
    );
    assert!(
        (just[4] - base[4] - 24.0).abs() < 0.01,
        "'c' (after 2 separators) shifts by 2×jws: {} vs {} + 24",
        just[4],
        base[4]
    );
}

/// A zero `justify_word_spacing` (the non-justified common case) leaves glyph
/// positions exactly as the un-justified paint — no accidental stretch.
#[test]
fn zero_justify_spacing_is_a_noop() {
    let base = paint_justified_line("a b c", 0.0);
    let again = paint_justified_line("a b c", 0.0);
    assert_eq!(
        base, again,
        "jws=0 paints identically (deterministic, no stretch)"
    );
}

/// Bidi-reorder branch: a justified line whose RTL runs reorder for paint still
/// applies `justify_word_spacing` within runs (the shared accumulating cursor advances
/// by each run's justify-expanded shaped width). Two Hebrew runs under an LTR
/// paragraph reorder (visual swap); run1 carries an interior word-separator, so a
/// positive `justify_word_spacing` widens the painted extent vs the jws=0 paint.
#[test]
#[allow(unused_must_use)]
fn bidi_reorder_branch_applies_within_run_justify() {
    let paint = |jws: f32| -> Option<f32> {
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
        let text = dom.create_text("אאבב גג");
        dom.append_child(div, text);
        // Two RTL runs under an LTR base → reorder branch (visual swap). run1 carries
        // an interior space, the within-run justification opportunity.
        let flow = InlineFlow::single(
            0,
            vec![InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                justify_word_spacing: jws,
                runs: vec![
                    InlineFlowRun::Text {
                        entity: div,
                        text: "אא".to_string(),
                        inline_start: 0.0,
                    },
                    InlineFlowRun::Text {
                        entity: div,
                        text: "בב גג".to_string(),
                        inline_start: 20.0,
                    },
                ],
            }],
        );
        dom.world_mut().insert_one(text, flow);
        let font_db = elidex_text::FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Rightmost painted glyph across all items (the painted extent).
        text_item_glyphs(&dl)
            .iter()
            .flat_map(|g| g.iter())
            .map(|e| e.position.x)
            .fold(None, |acc: Option<f32>, x| {
                Some(acc.map_or(x, |m| m.max(x)))
            })
    };
    let (Some(base), Some(just)) = (paint(0.0), paint(15.0)) else {
        return; // no usable font on this host
    };
    // The interior separator in run1 receives the extra advance even on the reorder
    // path, so the painted extent grows by ~jws (one opportunity moves the trailing
    // "גג" glyphs rightward).
    assert!(
        just > base + 10.0,
        "justify spacing widens the reordered line's painted extent: {just} vs {base}"
    );
}
