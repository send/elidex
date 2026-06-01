//! Inline content measurement (min/max content, segment widths).

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::ComputedStyle;
use elidex_text::{measure_text, FontDatabase, TextMeasureParams};

use super::{collect_inline_items, InlineItem};

/// Compute min-content inline size (maximum word width) for intrinsic sizing.
///
/// Min-content = the width of the longest unbreakable segment.
/// Text is split by whitespace; each word's width is measured individually.
/// Atomic inline-level boxes contribute zero (their intrinsic width
/// is not yet computed at this stage).
pub fn min_content_inline_size(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    font_db: &FontDatabase,
) -> f32 {
    let (items, _) = collect_inline_items(dom, children, parent_style, parent_entity);
    let mut max_word = 0.0_f32;
    for item in &items {
        if let InlineItem::Text(run) = item {
            let families = run.family_refs();
            let params = run.measure_params(&families);
            // Split by whitespace and measure each word individually.
            for word in run.text.split_whitespace() {
                if let Some(m) = measure_text(font_db, &params, word) {
                    max_word = max_word.max(m.width);
                }
            }
        }
    }
    max_word
}

/// Compute max-content inline size (no line breaking) for shrink-to-fit width.
///
/// Sums the measured width of all text runs without line breaking.
/// Atomic inline-level boxes contribute zero (their intrinsic width
/// is not yet computed at this stage).
pub fn max_content_inline_size(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    font_db: &FontDatabase,
) -> f32 {
    let (items, _) = collect_inline_items(dom, children, parent_style, parent_entity);
    let mut total = 0.0_f32;
    for item in &items {
        if let InlineItem::Text(run) = item {
            let families = run.family_refs();
            let params = run.measure_params(&families);
            if let Some(m) = measure_text(font_db, &params, &run.text) {
                total += m.width;
            }
        }
    }
    total
}

/// Measure a segment's full and trimmed widths.
///
/// Returns `(full_width, trimmed_width)` where `trimmed_width` excludes trailing
/// *collapsible* white space per CSS Text Level 3 §4.1.2 (trailing spaces/tabs
/// "hang" and don't trigger line overflow). Only collapsible white space
/// (ASCII space/tab, via [`super::is_collapsible_space`]) hangs — non-collapsible
/// characters such as a no-break space (U+00A0) count toward overflow, so Rust's
/// Unicode-aware `trim_end` (which would also strip NBSP) is not used here.
pub(super) fn measure_segment_widths(
    font_db: &FontDatabase,
    params: &TextMeasureParams<'_>,
    segment: &str,
) -> (f32, f32) {
    let seg_width = measure_text(font_db, params, segment).map_or(0.0, |m| m.width);
    let trimmed = segment.trim_end_matches(super::is_collapsible_space);
    let trimmed_width = if trimmed.len() == segment.len() {
        seg_width
    } else if trimmed.is_empty() {
        0.0
    } else {
        measure_text(font_db, params, trimmed).map_or(0.0, |m| m.width)
    };
    (seg_width, trimmed_width)
}
