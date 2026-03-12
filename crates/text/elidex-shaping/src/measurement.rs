//! High-level text measurement combining shaping and font metrics.
//!
//! Provides the primary API consumed by `elidex-layout` to determine
//! text dimensions for line and block layout.

use crate::database::FontDatabase;
use crate::shaping::shape_text;
use crate::unicode::is_word_separator;

/// Measurement result for a text string at a given font size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    /// Total text width in pixels.
    pub width: f32,
    /// Font ascent in pixels (positive, above baseline).
    pub ascent: f32,
    /// Font descent in pixels (negative, below baseline).
    pub descent: f32,
    /// Line height in pixels: `ascent - descent + line_gap`.
    pub line_height: f32,
}

/// Parameters for text measurement: font selection + spacing.
#[derive(Clone, Debug)]
pub struct TextMeasureParams<'a> {
    /// Font family names to try in order.
    pub families: &'a [&'a str],
    /// Font size in pixels.
    pub font_size: f32,
    /// CSS font-weight (100–900).
    pub weight: u16,
    /// Font style (Normal, Italic, Oblique).
    pub style: fontdb::Style,
    /// CSS `letter-spacing` in pixels (`0.0` for `normal`).
    pub letter_spacing: f32,
    /// CSS `word-spacing` in pixels (`0.0` for `normal`).
    pub word_spacing: f32,
}

/// Measures `text` using the first matching font family.
///
/// This is the primary entry point for `elidex-layout`. It resolves the font,
/// shapes the text, and combines glyph advances with font metrics to produce
/// a complete measurement. Letter-spacing and word-spacing are applied using
/// the same per-cluster logic as the renderer's `place_glyphs()`.
///
/// Returns `None` if no matching font is found.
#[must_use]
pub fn measure_text(
    db: &FontDatabase,
    params: &TextMeasureParams<'_>,
    text: &str,
) -> Option<TextMetrics> {
    let font_id = db.query(params.families, params.weight, params.style)?;
    let metrics = db.font_metrics(font_id, params.font_size)?;
    let shaped = shape_text(db, font_id, params.font_size, text)?;

    let spacing_extra = compute_spacing_extra(
        &shaped.glyphs,
        text,
        params.letter_spacing,
        params.word_spacing,
    );

    Some(TextMetrics {
        width: shaped.total_advance + spacing_extra,
        ascent: metrics.ascent,
        descent: metrics.descent,
        line_height: metrics.ascent - metrics.descent + metrics.line_gap,
    })
}

/// Compute the extra width contributed by letter-spacing and word-spacing.
///
/// Uses the same per-cluster logic as `place_glyphs()` in `elidex-render`:
/// - Letter-spacing is added between clusters (not after the last).
/// - Word-spacing is added once per Unicode Zs (space separator) cluster.
fn compute_spacing_extra(
    glyphs: &[crate::shaping::ShapedGlyph],
    text: &str,
    letter_spacing: f32,
    word_spacing: f32,
) -> f32 {
    let ls = if letter_spacing.is_finite() {
        letter_spacing
    } else {
        0.0
    };
    let ws = if word_spacing.is_finite() {
        word_spacing
    } else {
        0.0
    };
    if ls == 0.0 && ws == 0.0 {
        return 0.0;
    }

    let mut extra = 0.0_f32;
    let mut last_ws_cluster: Option<u32> = None;

    for (i, glyph) in glyphs.iter().enumerate() {
        // Word spacing: once per Zs cluster.
        if ws != 0.0 && last_ws_cluster != Some(glyph.cluster) {
            let idx = glyph.cluster as usize;
            if idx < text.len() && text.is_char_boundary(idx) {
                if let Some(ch) = text[idx..].chars().next() {
                    if is_word_separator(ch) {
                        extra += ws;
                        last_ws_cluster = Some(glyph.cluster);
                    }
                }
            }
        }

        // Letter spacing: between clusters, not after last.
        if ls != 0.0 {
            if let Some(next) = glyphs.get(i + 1) {
                if next.cluster != glyph.cluster {
                    extra += ls;
                }
            }
        }
    }

    extra
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::TEST_FONT_FAMILIES as TEST_FAMILIES;

    fn default_params<'a>(families: &'a [&'a str]) -> TextMeasureParams<'a> {
        TextMeasureParams {
            families,
            font_size: 16.0,
            weight: 400,
            style: fontdb::Style::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }

    #[test]
    fn measure_text_positive_width() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(&db, &default_params(TEST_FAMILIES), "Hello, world!") else {
            return;
        };
        assert!(m.width > 0.0);
        assert!(m.ascent > 0.0);
        assert!(m.line_height > 0.0);
    }

    #[test]
    fn line_height_formula() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(&db, &default_params(TEST_FAMILIES), "Test") else {
            return;
        };
        // line_height should be >= ascent - descent (line_gap >= 0 for most fonts)
        let expected = m.ascent - m.descent;
        assert!(m.line_height >= expected - f32::EPSILON);
    }

    #[test]
    fn nonexistent_font_returns_none() {
        let db = FontDatabase::new();
        let result = measure_text(
            &db,
            &default_params(&["__nonexistent_font_12345__"]),
            "test",
        );
        assert!(result.is_none());
    }

    #[test]
    fn empty_string_zero_width() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(&db, &default_params(TEST_FAMILIES), "") else {
            return;
        };
        assert!(m.width.abs() < f32::EPSILON);
    }

    #[test]
    fn letter_spacing_increases_width() {
        let db = FontDatabase::new();
        let Some(base) = measure_text(&db, &default_params(TEST_FAMILIES), "abc") else {
            return;
        };
        let params = TextMeasureParams {
            letter_spacing: 5.0,
            ..default_params(TEST_FAMILIES)
        };
        let Some(spaced) = measure_text(&db, &params, "abc") else {
            return;
        };
        // "abc" has 3 chars → 2 gaps × 5px = 10px extra
        assert!(
            spaced.width > base.width,
            "letter-spacing should increase width"
        );
        let diff = spaced.width - base.width;
        assert!(
            (diff - 10.0).abs() < 0.5,
            "expected ~10px extra, got {diff}"
        );
    }

    #[test]
    fn word_spacing_increases_width() {
        let db = FontDatabase::new();
        let Some(base) = measure_text(&db, &default_params(TEST_FAMILIES), "a b c") else {
            return;
        };
        let params = TextMeasureParams {
            word_spacing: 10.0,
            ..default_params(TEST_FAMILIES)
        };
        let Some(spaced) = measure_text(&db, &params, "a b c") else {
            return;
        };
        // "a b c" has 2 spaces → 2 × 10px = 20px extra
        assert!(
            spaced.width > base.width,
            "word-spacing should increase width"
        );
        let diff = spaced.width - base.width;
        assert!(
            (diff - 20.0).abs() < 0.5,
            "expected ~20px extra, got {diff}"
        );
    }
}
