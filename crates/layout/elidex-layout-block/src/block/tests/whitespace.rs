//! Collapsible whitespace between block-level boxes must generate no box
//! (CSS 2 §9.2.1.1 anonymous block boxes / §9.2.2.1 anonymous inline boxes).

use super::*;
use elidex_text::{measure_text, FontStyle, TextMeasureParams};

const TEST_FAMILIES: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];

/// Build `<div>` containing two 40px block children with a whitespace-only text
/// node between them. The parent carries a test font family so the anonymous
/// inline box for the whitespace has a usable font — otherwise the inline layout
/// short-circuits to zero height and the test would pass vacuously without
/// exercising the box-suppression path. Returns `None` if no test font is found.
fn blocks_with_whitespace_between(ws: &str) -> Option<(EcsDom, Entity, FontDatabase)> {
    let font_db = FontDatabase::new();
    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: 16.0,
        weight: 400,
        style: FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    // Guard: ensure a real font is available so the whitespace run is measured.
    measure_text(&font_db, &params, "x")?;

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        },
    );

    let child1 = make_block_child(&mut dom, parent, 40.0);
    let _ = child1;
    let ws_text = dom.create_text(ws);
    dom.append_child(parent, ws_text);
    let child2 = make_block_child(&mut dom, parent, 40.0);
    let _ = child2;

    Some((dom, parent, font_db))
}

#[test]
fn whitespace_text_between_blocks_adds_no_block_extent_spaces() {
    let Some((mut dom, parent, font_db)) = blocks_with_whitespace_between("   ") else {
        return;
    };
    let lb = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);
    // Two 40px blocks; the collapsible whitespace between them collapses away and
    // generates no box, so the parent is exactly 80px tall.
    assert!(
        (lb.content.size.height - 80.0).abs() < f32::EPSILON,
        "spaces-only whitespace between blocks must add no block extent (80), got {}",
        lb.content.size.height,
    );
}

#[test]
fn whitespace_text_between_blocks_adds_no_block_extent_newline() {
    let Some((mut dom, parent, font_db)) = blocks_with_whitespace_between("\n  ") else {
        return;
    };
    let lb = layout_block(&mut dom, parent, 800.0, Point::ZERO, &font_db);
    // The newline is collapsible under `white-space: normal` (§4.1.3) and must not
    // be treated as a forced break producing a spurious line box.
    assert!(
        (lb.content.size.height - 80.0).abs() < f32::EPSILON,
        "newline-containing whitespace between blocks must add no block extent (80), got {}",
        lb.content.size.height,
    );
}
