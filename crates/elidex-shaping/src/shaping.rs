//! Text shaping via [`rustybuzz`] (HarfBuzz-compatible OpenType shaping).

use crate::database::{pixel_scale, FontDatabase};

/// A single shaped glyph with positioning information.
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    /// Glyph ID in the font.
    pub glyph_id: u16,
    /// Horizontal advance in pixels.
    pub x_advance: f32,
    /// Vertical advance in pixels (used for vertical text).
    pub y_advance: f32,
    /// Horizontal offset in pixels.
    pub x_offset: f32,
    /// Vertical offset in pixels.
    pub y_offset: f32,
    /// Cluster index mapping back to the input string.
    pub cluster: u32,
}

/// Result of shaping a text string.
#[derive(Clone, Debug)]
pub struct ShapedText {
    /// Individual shaped glyphs.
    pub glyphs: Vec<ShapedGlyph>,
    /// Total advance width of the entire string in pixels.
    pub total_advance: f32,
}

/// Shapes `text` using the specified font and returns glyph positions.
///
/// Uses `fontdb::Database::with_face_data` to access the font binary, then
/// creates a `rustybuzz::Face` and runs OpenType shaping.
///
/// Returns `None` if the font data cannot be accessed or the face cannot be
/// parsed.
#[must_use]
pub fn shape_text(
    db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
    text: &str,
) -> Option<ShapedText> {
    db.with_face_data(font_id, |data, face_index| {
        let mut face = rustybuzz::Face::from_slice(data, face_index)?;
        let scale = pixel_scale(&face, font_size)?;

        // Set ppem for hinting. font_size is always non-negative and small.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ppem = font_size.round() as u16;
        face.set_pixels_per_em(Some((ppem, ppem)));

        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);

        let output = rustybuzz::shape(&face, &[], buffer);

        let infos = output.glyph_infos();
        let positions = output.glyph_positions();

        let mut glyphs = Vec::with_capacity(infos.len());
        let mut total_advance = 0.0;

        #[allow(clippy::cast_precision_loss)]
        for (info, pos) in infos.iter().zip(positions.iter()) {
            let x_advance = (pos.x_advance as f32) * scale;
            let y_advance = (pos.y_advance as f32) * scale;
            let x_offset = (pos.x_offset as f32) * scale;
            let y_offset = (pos.y_offset as f32) * scale;

            // rustybuzz guarantees glyph_id <= u16::MAX.
            debug_assert!(u16::try_from(info.glyph_id).is_ok());
            #[allow(clippy::cast_possible_truncation)]
            let glyph_id = info.glyph_id as u16;

            glyphs.push(ShapedGlyph {
                glyph_id,
                x_advance,
                y_advance,
                x_offset,
                y_offset,
                cluster: info.cluster,
            });

            total_advance += x_advance;
        }

        Some(ShapedText {
            glyphs,
            total_advance,
        })
    })?
}

/// Shapes `text` for vertical (top-to-bottom) layout.
///
/// Enables the OpenType `vert` feature for vertical glyph substitution.
/// `total_advance` represents the total vertical advance (sum of `y_advance`).
///
/// Returns `None` if the font data cannot be accessed or the face cannot be parsed.
#[must_use]
pub fn shape_text_vertical(
    db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
    text: &str,
) -> Option<ShapedText> {
    db.with_face_data(font_id, |data, face_index| {
        let mut face = rustybuzz::Face::from_slice(data, face_index)?;
        let scale = pixel_scale(&face, font_size)?;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ppem = font_size.round() as u16;
        face.set_pixels_per_em(Some((ppem, ppem)));

        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(rustybuzz::Direction::TopToBottom);

        let vert_feature =
            rustybuzz::Feature::new(rustybuzz::ttf_parser::Tag::from_bytes(b"vert"), 1, ..);
        let output = rustybuzz::shape(&face, &[vert_feature], buffer);

        let infos = output.glyph_infos();
        let positions = output.glyph_positions();

        let mut glyphs = Vec::with_capacity(infos.len());
        let mut total_advance = 0.0;

        #[allow(clippy::cast_precision_loss)]
        for (info, pos) in infos.iter().zip(positions.iter()) {
            let x_advance = (pos.x_advance as f32) * scale;
            let y_advance = (pos.y_advance as f32) * scale;
            let x_offset = (pos.x_offset as f32) * scale;
            let y_offset = (pos.y_offset as f32) * scale;

            debug_assert!(u16::try_from(info.glyph_id).is_ok());
            #[allow(clippy::cast_possible_truncation)]
            let glyph_id = info.glyph_id as u16;

            glyphs.push(ShapedGlyph {
                glyph_id,
                x_advance,
                y_advance,
                x_offset,
                y_offset,
                cluster: info.cluster,
            });

            total_advance += y_advance;
        }

        Some(ShapedText {
            glyphs,
            total_advance,
        })
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: try to find a system font for testing.
    fn test_font(db: &FontDatabase) -> Option<fontdb::ID> {
        db.query(crate::TEST_FONT_FAMILIES, 400)
    }

    #[test]
    fn empty_string_zero_advance() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let result = shape_text(&db, id, 16.0, "");
        // Empty string still produces a ShapedText (rustybuzz returns empty output)
        if let Some(shaped) = result {
            assert!(shaped.glyphs.is_empty());
            assert!((shaped.total_advance - 0.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn ascii_text_produces_glyphs() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let shaped = shape_text(&db, id, 16.0, "Hello").unwrap();
        assert_eq!(shaped.glyphs.len(), 5);
        assert!(shaped.total_advance > 0.0);
    }

    #[test]
    fn total_advance_positive() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let shaped = shape_text(&db, id, 16.0, "The quick brown fox").unwrap();
        assert!(shaped.total_advance > 0.0);
        // Longer text should have larger advance
        let shaped_short = shape_text(&db, id, 16.0, "Hi").unwrap();
        assert!(shaped.total_advance > shaped_short.total_advance);
    }

    #[test]
    fn valid_font_returns_some() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        assert!(shape_text(&db, id, 16.0, "test").is_some());
    }

    // --- M3.5-4: Vertical shaping ---

    #[test]
    fn y_advance_zero_for_horizontal() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let shaped = shape_text(&db, id, 16.0, "Hello").unwrap();
        // Horizontal shaping: y_advance should be 0 for all glyphs.
        for g in &shaped.glyphs {
            assert!(
                g.y_advance.abs() < f32::EPSILON,
                "horizontal glyph y_advance should be 0, got {}",
                g.y_advance
            );
        }
    }

    #[test]
    fn vertical_shaping_returns_some() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        // shape_text_vertical should produce valid output for ASCII text.
        let result = shape_text_vertical(&db, id, 16.0, "Hello");
        assert!(result.is_some(), "vertical shaping should not fail");
        let shaped = result.unwrap();
        assert_eq!(shaped.glyphs.len(), 5);
    }

    #[test]
    fn vertical_shaping_has_nonzero_total_advance() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let shaped = shape_text_vertical(&db, id, 16.0, "Hello").unwrap();
        // Total advance (vertical) should be non-zero.
        // Note: total_advance for vertical shaping is the sum of y_advances.
        assert!(
            shaped.total_advance.abs() > f32::EPSILON,
            "vertical total_advance should be non-zero, got {}",
            shaped.total_advance
        );
    }

    #[test]
    fn vertical_empty_string() {
        let db = FontDatabase::new();
        let Some(id) = test_font(&db) else {
            return;
        };
        let result = shape_text_vertical(&db, id, 16.0, "");
        if let Some(shaped) = result {
            assert!(shaped.glyphs.is_empty());
        }
    }
}
