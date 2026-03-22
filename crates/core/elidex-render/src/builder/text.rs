//! Text measurement, alignment, and transformation.

use std::borrow::Cow;

use super::{families_as_refs, StyledTextSegment};
use elidex_plugin::{Direction, TextAlign, TextTransform};
use elidex_text::{measure_text, to_fontdb_style, FontDatabase, TextMeasureParams};

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

/// Result of text-align computation: initial offset and optional extra word spacing.
pub(crate) struct TextAlignResult {
    /// Horizontal offset from the line start.
    pub offset: f32,
    /// Extra word spacing for `text-align: justify` (CSS Text 3 §7.1).
    /// Added on top of the CSS `word-spacing` property.
    pub justify_extra_word_spacing: f32,
}

/// Compute the horizontal offset for `text-align` within a content box.
///
/// For `Left`, returns `0.0` immediately (no measurement needed).
/// For `Center`/`Right`, measures the total width of all collapsed segments
/// and returns the appropriate offset within `container_width`.
/// For `Justify`, distributes free space evenly among word gaps.
pub(crate) fn compute_text_align_offset(
    align: TextAlign,
    direction: Direction,
    container_width: f32,
    collapsed: &[(String, usize)],
    segments: &[StyledTextSegment],
    font_db: &FontDatabase,
) -> TextAlignResult {
    let resolved = resolve_text_align(align, direction);
    match resolved {
        TextAlign::Left | TextAlign::Start => TextAlignResult {
            offset: 0.0,
            justify_extra_word_spacing: 0.0,
        },
        TextAlign::Justify => {
            let total_width = measure_total_width(collapsed, segments, font_db);
            if !total_width.is_finite() {
                return TextAlignResult {
                    offset: 0.0,
                    justify_extra_word_spacing: 0.0,
                };
            }
            let free = (container_width - total_width).max(0.0);
            let word_gaps = count_word_gaps(collapsed, segments);
            #[allow(clippy::cast_precision_loss)] // word gap count is small
            let extra_ws = if word_gaps > 0 {
                free / word_gaps as f32
            } else {
                0.0
            };
            TextAlignResult {
                offset: 0.0,
                justify_extra_word_spacing: extra_ws,
            }
        }
        _ => {
            let total_width = measure_total_width(collapsed, segments, font_db);
            if !total_width.is_finite() {
                return TextAlignResult {
                    offset: 0.0,
                    justify_extra_word_spacing: 0.0,
                };
            }
            let free = (container_width - total_width).max(0.0);
            TextAlignResult {
                offset: align_offset(resolved, free),
                justify_extra_word_spacing: 0.0,
            }
        }
    }
}

/// Measure total width of all collapsed segments.
fn measure_total_width(
    collapsed: &[(String, usize)],
    segments: &[StyledTextSegment],
    font_db: &FontDatabase,
) -> f32 {
    collapsed
        .iter()
        .filter_map(|(text, idx)| {
            segments
                .get(*idx)
                .map(|seg| measure_segment_width(text, seg, font_db))
        })
        .sum()
}

/// Count word separator boundaries across all segments for justify distribution.
fn count_word_gaps(collapsed: &[(String, usize)], segments: &[StyledTextSegment]) -> usize {
    let mut count = 0;
    for (text, idx) in collapsed {
        if segments.get(*idx).is_none() {
            continue;
        }
        count += text
            .chars()
            .filter(|c| elidex_text::is_word_separator(*c))
            .count();
    }
    count
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

/// Measure a segment's text width after text-transform, including spacing.
///
/// Uses `elidex_text::measure_text` which includes per-cluster letter-spacing
/// and word-spacing via `compute_spacing_extra()`, matching `place_glyphs()`.
#[must_use]
pub(crate) fn measure_segment_width(
    text: &str,
    seg: &StyledTextSegment,
    font_db: &FontDatabase,
) -> f32 {
    let transformed = apply_text_transform(text, seg.text_transform);
    let families = families_as_refs(&seg.font_family);
    let params = TextMeasureParams {
        families: &families,
        font_size: seg.font_size,
        weight: seg.font_weight,
        style: to_fontdb_style(seg.font_style),
        letter_spacing: seg.letter_spacing,
        word_spacing: seg.word_spacing,
    };
    measure_text(font_db, &params, &transformed).map_or(0.0, |m| m.width)
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

/// Capitalize the first letter of each word per UAX #29 word boundaries.
///
/// Uses `unicode-segmentation` for word boundary detection (CSS Text Level 3
/// §2.4). This correctly handles punctuation-adjacent boundaries and
/// non-space separators (e.g. "hello-world" → "Hello-World").
#[must_use]
fn capitalize_words(text: &str) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    let mut result = String::with_capacity(text.len());
    for word in text.split_word_bounds() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            if first.is_alphabetic() {
                result.extend(first.to_uppercase());
                result.push_str(chars.as_str());
            } else {
                result.push_str(word);
            }
        }
    }
    result
}
