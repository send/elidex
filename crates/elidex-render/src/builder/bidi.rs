//! `BiDi` visual reordering for inline text segments.

use elidex_plugin::Direction;
use elidex_text::{analyze_bidi, reorder_by_levels, ParagraphLevel};

/// Compute visual order of collapsed segments using the Unicode bidi algorithm.
///
/// Returns indices into `collapsed` in the order they should be rendered
/// left-to-right. For pure LTR text with LTR direction, returns identity
/// order. For mixed-direction text, segments are reordered per UAX #9 L2.
pub(crate) fn bidi_visual_order(collapsed: &[(String, usize)], direction: Direction) -> Vec<usize> {
    if collapsed.is_empty() {
        return Vec::new();
    }

    // Concatenate all segment text for bidi analysis.
    let full_text: String = collapsed.iter().map(|(t, _)| t.as_str()).collect();
    if full_text.is_empty() {
        return (0..collapsed.len()).collect();
    }

    let para_level = match direction {
        Direction::Ltr => ParagraphLevel::Ltr,
        Direction::Rtl => ParagraphLevel::Rtl,
    };
    let bidi_runs = analyze_bidi(&full_text, para_level);

    // Fast path: single LTR run = no reordering needed.
    if bidi_runs.len() <= 1 && !bidi_runs.iter().any(elidex_text::BidiRun::is_rtl) {
        return (0..collapsed.len()).collect();
    }

    // Map each collapsed segment to its byte offset in the concatenated text.
    let mut seg_starts = Vec::with_capacity(collapsed.len());
    let mut pos = 0;
    for (text, _) in collapsed {
        seg_starts.push(pos);
        pos += text.len();
    }

    // Assign each segment the bidi level of the run covering its start byte.
    // TODO(Phase 4): This assigns the level at the segment's start byte,
    // but a segment could span multiple bidi runs if it contains mixed-
    // direction characters. Correct handling requires splitting segments
    // at bidi run boundaries before level assignment.
    let seg_levels: Vec<u8> = seg_starts
        .iter()
        .map(|&start| {
            bidi_runs
                .iter()
                .find(|r| r.start <= start && r.end > start)
                .map_or(0, |r| r.level.number())
        })
        .collect();

    // UAX #9 L2: reverse contiguous runs at each level from max down to min odd.
    reorder_by_levels(&seg_levels)
}
