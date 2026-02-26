//! Text shaping via [`rustybuzz`] (HarfBuzz-compatible OpenType shaping).

use crate::database::FontDatabase;

/// A single shaped glyph with positioning information.
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    /// Glyph ID in the font.
    pub glyph_id: u16,
    /// Horizontal advance in pixels.
    pub x_advance: f32,
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
pub fn shape_text(
    db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
    text: &str,
) -> Option<ShapedText> {
    db.inner().with_face_data(font_id, |data, face_index| {
        let mut face = rustybuzz::Face::from_slice(data, face_index)?;
        let upem = face.units_per_em();
        let upem_u16 = u16::try_from(upem).ok()?;
        let scale = font_size / f32::from(upem_u16);

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

        for (info, pos) in infos.iter().zip(positions.iter()) {
            #[allow(clippy::cast_precision_loss)]
            let x_advance = (pos.x_advance as f32) * scale;
            #[allow(clippy::cast_precision_loss)]
            let x_offset = (pos.x_offset as f32) * scale;
            #[allow(clippy::cast_precision_loss)]
            let y_offset = (pos.y_offset as f32) * scale;

            #[allow(clippy::cast_possible_truncation)]
            let glyph_id = info.glyph_id as u16;

            glyphs.push(ShapedGlyph {
                glyph_id,
                x_advance,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: try to find a system font for testing.
    fn test_font(db: &FontDatabase) -> Option<fontdb::ID> {
        db.query(&[
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ])
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
}
