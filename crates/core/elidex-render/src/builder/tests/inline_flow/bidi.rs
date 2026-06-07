//! UAX #9 L2 paint-time visual reorder: layout persists each line's `Text` runs in
//! logical order; render reorders them to visual order at paint (master §4.2),
//! independently per line, sharing one accumulating cursor (hidden runs still
//! reserve their advance), with atomics excluded from the adapter.

use super::*;

/// Slice 4 / bidi: layout persists a line's `Text` runs in **logical** order; render
/// owns the UAX #9 L2 visual reorder at paint (master §4.2). Two adjacent RTL
/// (Hebrew) runs under an LTR paragraph form one level-1 run → render paints them in
/// REVERSED (visual) order. Distinct run lengths (`"אא"`=2, `"ב"`=1) make the paint
/// order observable via the glyph-count sequence: logical [2,1] → visual [1,2] (the
/// second logical run paints first, on the visual left). The reorder cursor starts at
/// the line's visual inline-start (`min(inline_start)` = 0 here), so the first painted
/// item begins at x≈0.
#[test]
#[allow(unused_must_use)]
fn converged_bidi_reorders_runs() {
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
    let text = dom.create_text("אאב");
    dom.append_child(div, text);

    // Logical order: run0 "אא" (inline_start 0), run1 "ב" (inline_start 20).
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "אא".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "ב".to_string(),
                    inline_start: 20.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    assert_eq!(
        text_item_glyph_counts(&dl),
        vec![1, 2],
        "RTL runs paint in reversed visual order: logical [\"אא\"(2),\"ב\"(1)] → visual [\"ב\"(1),\"אא\"(2)]"
    );
    // The reorder cursor starts at the line's visual inline-start = min(inline_start) = 0.
    let first_x = text_item_glyphs(&dl)[0][0].position.x;
    assert!(
        first_x.abs() < 2.0,
        "reordered line paints from min(inline_start)=0, got x={first_x}"
    );
}

/// LTR no-op: an all-LTR multi-run line is an identity permutation, so each run
/// paints at its OWN baked `inline_start` (cursor reset per run), NOT from an
/// accumulating origin — no regression for the common case.
#[test]
#[allow(unused_must_use)]
fn converged_ltr_identity_no_reorder() {
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
    let text = dom.create_text("AB");
    dom.append_child(div, text);

    // Two LTR runs at far-apart baked positions; identity order → painted there.
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "A".to_string(),
                    inline_start: 5.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "B".to_string(),
                    inline_start: 300.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    let items = text_item_glyphs(&dl);

    assert_eq!(items.len(), 2, "two LTR runs, logical order");
    assert!(
        (items[0][0].position.x - 5.0).abs() < 2.0,
        "run 0 at its baked inline_start 5.0, got {}",
        items[0][0].position.x
    );
    assert!(
        (items[1][0].position.x - 300.0).abs() < 2.0,
        "run 1 at its baked inline_start 300.0 (NOT accumulated from run 0), got {}",
        items[1][0].position.x
    );
}

/// Multi-line bidi (the payoff the legacy single-linear-pass could not produce):
/// each line's RTL runs reorder INDEPENDENTLY, at that line's own baseline. Line 0
/// = ["אא"(2),"ב"(1)] → visual [1,2]; line 1 = ["גגג"(3),"ד"(1)] → visual [1,3].
#[test]
#[allow(unused_must_use)]
fn converged_bidi_multi_line() {
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
    let text = dom.create_text("אאבגגגד");
    dom.append_child(div, text);

    let flow = InlineFlow::single(
        0,
        vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
                runs: vec![
                    InlineFlowRun::Text {
                        entity: div,
                        text: "אא".to_string(),
                        inline_start: 0.0,
                    },
                    InlineFlowRun::Text {
                        entity: div,
                        text: "ב".to_string(),
                        inline_start: 20.0,
                    },
                ],
            },
            InlineFlowLine {
                block_start: 20.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
                runs: vec![
                    InlineFlowRun::Text {
                        entity: div,
                        text: "גגג".to_string(),
                        inline_start: 0.0,
                    },
                    InlineFlowRun::Text {
                        entity: div,
                        text: "ד".to_string(),
                        inline_start: 30.0,
                    },
                ],
            },
        ],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    assert_eq!(
        text_item_glyph_counts(&dl),
        vec![1, 2, 1, 3],
        "each line reorders independently: line0 [2,1]→[1,2], line1 [3,1]→[1,3]"
    );
    let items = text_item_glyphs(&dl);
    let line0_baseline = items[0][0].position.y;
    let line1_baseline = items[2][0].position.y;
    assert!(
        (line1_baseline - line0_baseline - 20.0).abs() < 0.5,
        "line 1 reordered at its own baseline, one line below line 0; got {line0_baseline} / {line1_baseline}"
    );
}

/// Vertical bidi: in a vertical writing mode the reorder runs along the block axis
/// (cursor_y). Two adjacent RTL runs reverse just like horizontal — the reorder
/// adapter is axis-shared.
#[test]
#[allow(unused_must_use)]
fn converged_bidi_vertical() {
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
    let text = dom.create_text("אאב");
    dom.append_child(div, text);

    // inline_start = physical y (pen top); block_start/size give the column center x.
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "אא".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "ב".to_string(),
                    inline_start: 40.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    assert_eq!(
        text_item_glyph_counts(&dl),
        vec![1, 2],
        "vertical RTL runs reorder along the block axis: logical [2,1] → visual [1,2]"
    );
    // The reorder cursor starts at the column's visual inline-start (min y = 0).
    let first_y = text_item_glyphs(&dl)[0][0].position.y;
    assert!(
        first_y < 20.0,
        "reordered vertical line paints from min(inline_start)=0 downward, got y={first_y}"
    );
}

/// Atomic + bidi (slice 4 Option (c)): a bidi line carrying a static inline-block now
/// PERSISTS (no gate) — the Text runs reorder, while the atomic is NOT in the reorder
/// adapter and paints via `walk()` at its layout-baked box. This is a net fix over
/// legacy (which flattened atomics to text on the bidi path).
///
/// This covers the atomic AFTER the text runs. The atomic *interspersed between*
/// reordered text runs (where the text cursor does not reserve the atomic's width →
/// text can overprint the box) is a facet of the deferred full-UBA bidi-fidelity
/// program, slot `#11-bidi-full-uba-fidelity` — not asserted here.
#[test]
#[allow(unused_must_use)]
fn converged_bidi_line_with_atomic() {
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
    let text = dom.create_text("אאב");
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
    dom.world_mut().insert_one(
        ib,
        elidex_plugin::LayoutBox {
            content: Rect::new(50.0, 0.0, 30.0, 20.0),
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
                    text: "אא".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "ב".to_string(),
                    inline_start: 20.0,
                },
                InlineFlowRun::AtomicBox {
                    entity: ib,
                    inline_start: 50.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    // Text runs reordered (atomic excluded from the adapter, so [2,1]→[1,2]).
    assert_eq!(
        text_item_glyph_counts(&dl),
        vec![1, 2],
        "atomic is not in the reorder adapter; only the two Text runs reverse"
    );
    // The atomic's box paints once via walk() at its layout-baked x=50.
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
        "the atomic box paints exactly once via walk()"
    );
    assert!(
        (red_boxes[0].origin.x - 50.0).abs() < 0.5,
        "atomic stays at its layout-baked x=50 (not reordered), got {}",
        red_boxes[0].origin.x
    );
}

/// Builds an RTL line `["א"(vis), middle, "ג"(vis)]` and returns the x of the
/// trailing run `"ג"` after bidi reorder. The reorder shares one accumulating
/// cursor, so `"ג"`'s position is the running advance past the earlier visual runs.
/// `middle_hidden` toggles `visibility:hidden` on the middle run. A hidden run must
/// still reserve its advance (CSS 2.1 §11.2), so the trailing run's x MUST be
/// identical visible-vs-hidden — that equality is the regression assertion.
#[allow(unused_must_use)]
fn trailing_run_x_with_middle(middle_hidden: bool) -> Option<f32> {
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
    let text = dom.create_text("אמג");
    dom.append_child(div, text);
    // The middle run is a separate entity so it can carry its own visibility.
    let mid = dom.create_element("span", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        mid,
        elidex_plugin::ComputedStyle {
            visibility: if middle_hidden {
                elidex_plugin::Visibility::Hidden
            } else {
                elidex_plugin::Visibility::Visible
            },
            font_family: test_font_family_strings(),
            ..Default::default()
        },
    );
    dom.append_child(div, mid);

    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "א".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: mid,
                    text: "מ".to_string(),
                    inline_start: 20.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "ג".to_string(),
                    inline_start: 40.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Visual order reverses [0,1,2]→[2,1,0]; "ג" (logical idx 2) paints FIRST and "א"
    // (logical idx 0) LAST. The LAST painted Text item is therefore "א"; its x is the
    // running advance past "ג" + the middle run — which must not depend on whether the
    // middle run is painted. Use the last item to capture the full accumulated advance.
    let items = text_item_glyphs(&dl);
    items.last().map(|g| g[0].position.x)
}

/// Regression (Codex P2, inline.rs hidden-run advance): a `visibility:hidden` run in
/// the middle of a reordered RTL line must still advance the shared cursor, so the
/// run painted after it lands at the same x whether the middle run is hidden or
/// visible. Before the fix the hidden run early-returned without advancing, shifting
/// the trailing run left by the hidden run's width.
#[test]
fn converged_bidi_hidden_run_reserves_advance() {
    let Some(visible_x) = trailing_run_x_with_middle(false) else {
        return;
    };
    let Some(hidden_x) = trailing_run_x_with_middle(true) else {
        return;
    };
    assert!(
        (visible_x - hidden_x).abs() < 0.5,
        "hidden middle run must reserve its advance: trailing-run x should match \
         visible ({visible_x}) vs hidden ({hidden_x})"
    );
}

/// Regression (Codex P2, inline.rs RTL-base fast path): under an RTL paragraph, a line
/// whose runs carry only NEUTRAL characters (here ASCII `!` / `..` — bidi class ON, no
/// strong R/AL/AN, so `text_has_rtl` is false) still reorders per UAX #9 L2 (neutrals
/// with no strong context inherit the odd paragraph level, N1/N2). The fast path must
/// NOT skip bidi for an RTL container. Logical `["!"(1),".."(2)]` → visual reverse →
/// paint-order glyph counts `[2,1]`. (ASCII neutrals chosen deliberately: an Arabic
/// mark like U+061F is AL and `text_has_rtl` would catch it, so the test would pass
/// even with the buggy fast path — it would not isolate the `direction == Rtl` gate.)
#[test]
#[allow(unused_must_use)]
fn converged_bidi_rtl_base_neutral_runs_reorder() {
    let (mut dom, div) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            direction: elidex_plugin::Direction::Rtl,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 20.0),
            ..Default::default()
        },
    );
    let text = dom.create_text("!..");
    dom.append_child(div, text);

    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![
                InlineFlowRun::Text {
                    entity: div,
                    text: "!".to_string(),
                    inline_start: 0.0,
                },
                InlineFlowRun::Text {
                    entity: div,
                    text: "..".to_string(),
                    inline_start: 20.0,
                },
            ],
        }],
    );
    dom.world_mut().insert_one(text, flow);

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    assert_eq!(
        text_item_glyph_counts(&dl),
        vec![2, 1],
        "RTL-base neutral-only runs reorder: logical [\"!\"(1),\"..\"(2)] → visual [\"..\"(2),\"!\"(1)]"
    );
}
