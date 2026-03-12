//! Text measurement, alignment, and transformation.

use std::borrow::Cow;

use super::{families_as_refs, StyledTextSegment};
use elidex_plugin::{Direction, TextAlign, TextTransform};
use elidex_text::{shape_text, to_fontdb_style, FontDatabase};

/// Resolve `text-align: start/end` to physical `left/right` based on direction.
///
/// `Start` → `Left` (LTR) or `Right` (RTL).
/// `End` → `Right` (LTR) or `Left` (RTL).
/// Other values pass through unchanged.
pub(crate) fn resolve_text_align(align: TextAlign, direction: Direction) -> TextAlign {
    match align {
        TextAlign::Start => match direction {
            Direction::Ltr => TextAlign::Left,
            Direction::Rtl => TextAlign::Right,
        },
        TextAlign::End => match direction {
            Direction::Ltr => TextAlign::Right,
            Direction::Rtl => TextAlign::Left,
        },
        other => other,
    }
}

/// Compute the alignment offset for a given resolved text-align and free space.
///
/// `Left` → 0, `Center` → half, `Right` → full free space.
fn align_offset(resolved: TextAlign, free_space: f32) -> f32 {
    match resolved {
        TextAlign::Left | TextAlign::Start => 0.0,
        TextAlign::Center => free_space / 2.0,
        _ => free_space,
    }
}

/// Compute the horizontal offset for `text-align` within a content box.
///
/// For `Left`, returns `0.0` immediately (no measurement needed).
/// For `Center`/`Right`, measures the total width of all collapsed segments
/// and returns the appropriate offset within `container_width`.
pub(crate) fn compute_text_align_offset(
    align: TextAlign,
    direction: Direction,
    container_width: f32,
    collapsed: &[(String, usize)],
    segments: &[StyledTextSegment],
    font_db: &FontDatabase,
) -> f32 {
    let resolved = resolve_text_align(align, direction);
    match resolved {
        TextAlign::Left | TextAlign::Start => 0.0,
        _ => {
            let total_width: f32 = collapsed
                .iter()
                .filter_map(|(text, idx)| {
                    segments
                        .get(*idx)
                        .map(|seg| measure_segment_width(text, seg, font_db))
                })
                .sum();
            let free = (container_width - total_width).max(0.0);
            align_offset(resolved, free)
        }
    }
}

/// Transform text and query the matching font for a segment.
///
/// Shared setup for [`measure_segment_width`] and `measure_segment_height`.
/// Returns the transformed text and font ID, or `None` if no font matches.
#[must_use]
pub(crate) fn query_segment_font<'a>(
    text: &'a str,
    seg: &StyledTextSegment,
    font_db: &FontDatabase,
) -> Option<(Cow<'a, str>, elidex_text::FontId)> {
    let transformed = apply_text_transform(text, seg.text_transform);
    let families = families_as_refs(&seg.font_family);
    let style = to_fontdb_style(seg.font_style);
    let font_id = font_db.query(&families, seg.font_weight, style)?;
    Some((transformed, font_id))
}

/// Measure a segment's text width after text-transform.
#[must_use]
pub(crate) fn measure_segment_width(
    text: &str,
    seg: &StyledTextSegment,
    font_db: &FontDatabase,
) -> f32 {
    let Some((transformed, font_id)) = query_segment_font(text, seg, font_db) else {
        return 0.0;
    };
    let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
        return 0.0;
    };
    shaped.glyphs.iter().map(|g| g.x_advance).sum()
}

/// Apply CSS `text-transform` to a string before shaping.
#[must_use]
pub(crate) fn apply_text_transform(text: &str, transform: TextTransform) -> Cow<'_, str> {
    match transform {
        TextTransform::None => Cow::Borrowed(text),
        TextTransform::Uppercase => Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => Cow::Owned(capitalize_words(text)),
    }
}

/// Capitalize the first letter of each word (whitespace-delimited).
///
/// TODO(Phase 4+): Word boundary detection uses `is_whitespace()` as a
/// simplification. Full CSS Text Level 3 compliance requires UAX #29 word
/// segmentation (e.g. via the `unicode-segmentation` crate), which would
/// correctly handle punctuation-adjacent boundaries and non-space separators.
#[must_use]
fn capitalize_words(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_whitespace = true;
    for ch in text.chars() {
        if prev_was_whitespace && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            prev_was_whitespace = false;
        } else {
            result.push(ch);
            prev_was_whitespace = ch.is_whitespace();
        }
    }
    result
}
