//! High-level text measurement combining shaping and font metrics.
//!
//! Provides the primary API consumed by `elidex-layout` to determine
//! text dimensions for line and block layout.

use crate::database::FontDatabase;
use crate::shaping::shape_text;

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

/// Measures `text` using the first matching font family at `font_size`.
///
/// This is the primary entry point for `elidex-layout`. It resolves the font,
/// shapes the text, and combines glyph advances with font metrics to produce
/// a complete measurement.
///
/// `weight` is the CSS font-weight value (100-900). Pass `400` for normal.
/// `style` is the font style (Normal, Italic, Oblique).
///
/// Returns `None` if no matching font is found.
#[must_use]
pub fn measure_text(
    db: &FontDatabase,
    families: &[&str],
    font_size: f32,
    weight: u16,
    style: fontdb::Style,
    text: &str,
) -> Option<TextMetrics> {
    let font_id = db.query(families, weight, style)?;
    let metrics = db.font_metrics(font_id, font_size)?;
    let shaped = shape_text(db, font_id, font_size, text)?;

    Some(TextMetrics {
        width: shaped.total_advance,
        ascent: metrics.ascent,
        descent: metrics.descent,
        line_height: metrics.ascent - metrics.descent + metrics.line_gap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::TEST_FONT_FAMILIES as TEST_FAMILIES;

    #[test]
    fn measure_text_positive_width() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(
            &db,
            TEST_FAMILIES,
            16.0,
            400,
            fontdb::Style::Normal,
            "Hello, world!",
        ) else {
            return;
        };
        assert!(m.width > 0.0);
        assert!(m.ascent > 0.0);
        assert!(m.line_height > 0.0);
    }

    #[test]
    fn line_height_formula() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(&db, TEST_FAMILIES, 16.0, 400, fontdb::Style::Normal, "Test")
        else {
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
            &["__nonexistent_font_12345__"],
            16.0,
            400,
            fontdb::Style::Normal,
            "test",
        );
        assert!(result.is_none());
    }

    #[test]
    fn empty_string_zero_width() {
        let db = FontDatabase::new();
        let Some(m) = measure_text(&db, TEST_FAMILIES, 16.0, 400, fontdb::Style::Normal, "") else {
            return;
        };
        assert!(m.width.abs() < f32::EPSILON);
    }
}
