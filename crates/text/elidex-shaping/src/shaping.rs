//! Text shaping via [`rustybuzz`] (HarfBuzz-compatible OpenType shaping).

use crate::database::{pixel_scale, FontDatabase};

/// A single shaped glyph with positioning information.
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
    shape_with_options(db, font_id, font_size, text, false)
}

/// Shapes `text` for vertical (top-to-bottom) layout.
///
/// Enables the OpenType `vert` feature for vertical glyph substitution.
/// `total_advance` represents the total vertical advance (sum of `y_advance`).
///
/// Returns `None` if the font data cannot be accessed or the face cannot be parsed.
///
// TODO(Phase 4): Apply CSS `text-orientation` (mixed/upright/sideways).
// Currently all glyphs are shaped with TTB direction + `vert` feature,
// but `text-orientation: mixed` requires per-character classification
// (UTR #50 vo=R/Tr/Tu) to decide rotation vs. upright rendering.
#[must_use]
pub fn shape_text_vertical(
    db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
    text: &str,
) -> Option<ShapedText> {
    shape_with_options(db, font_id, font_size, text, true)
}

/// Internal shaping implementation shared by horizontal and vertical paths.
///
/// When `vertical` is true, sets the buffer direction to top-to-bottom,
/// enables the `vert` OpenType feature, and accumulates `y_advance` for
/// `total_advance`. Otherwise uses left-to-right with `x_advance`.
fn shape_with_options(
    db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
    text: &str,
    vertical: bool,
) -> Option<ShapedText> {
    db.with_face_data(font_id, |data, face_index| {
        let mut face = rustybuzz::Face::from_slice(data, face_index)?;
        // Clamp font_size to valid range: non-negative, finite, within u16 ppem range.
        let clamped_size = font_size.max(0.0).min(f32::from(u16::MAX));
        let scale = pixel_scale(&face, clamped_size)?;

        // Set ppem for hinting.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ppem = (clamped_size.round() as u16).max(1);
        face.set_pixels_per_em(Some((ppem, ppem)));

        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);

        let features;
        let feature_slice: &[rustybuzz::Feature] = if vertical {
            buffer.set_direction(rustybuzz::Direction::TopToBottom);
            features =
                rustybuzz::Feature::new(rustybuzz::ttf_parser::Tag::from_bytes(b"vert"), 1, ..);
            std::slice::from_ref(&features)
        } else {
            &[]
        };

        let output = rustybuzz::shape(&face, feature_slice, buffer);

        let infos = output.glyph_infos();
        let positions = output.glyph_positions();

        let mut glyphs = Vec::with_capacity(infos.len());
        let mut total_advance = 0.0;

        #[allow(clippy::cast_precision_loss)]
        for (info, pos) in infos.iter().zip(positions.iter()) {
            let x_advance = (pos.x_advance as f32) * scale;
            let x_offset = (pos.x_offset as f32) * scale;

            // rustybuzz uses font coordinate space where y-axis points upward.
            // In TTB mode, downward advance produces negative y_advance/y_offset.
            // Negate to convert to screen coordinates (y-axis points downward).
            let y_advance = if vertical {
                -((pos.y_advance as f32) * scale)
            } else {
                (pos.y_advance as f32) * scale
            };
            let y_offset = if vertical {
                -((pos.y_offset as f32) * scale)
            } else {
                (pos.y_offset as f32) * scale
            };

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

            total_advance += if vertical { y_advance } else { x_advance };
        }

        Some(ShapedText {
            glyphs,
            total_advance,
        })
    })?
}

/// A run of shaped glyphs all using the same font.
#[derive(Clone, Debug)]
pub struct ShapedRun {
    /// Shaped glyphs in this run.
    pub glyphs: Vec<ShapedGlyph>,
    /// Font ID used for these glyphs.
    pub font_id: fontdb::ID,
}

/// Result of shaping with per-glyph font fallback.
///
/// Contains one or more [`ShapedRun`]s, each using a potentially different font.
/// When the primary font covers all glyphs, this contains a single run.
#[derive(Clone, Debug)]
pub struct ShapedTextWithFonts {
    /// Runs of glyphs, each with their own font.
    pub runs: Vec<ShapedRun>,
    /// Total advance width in pixels (sum of all runs).
    pub total_advance: f32,
}

/// Round down `index` to the nearest UTF-8 char boundary in `text`.
fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Round up `index` to the nearest UTF-8 char boundary in `text`.
fn ceil_char_boundary(text: &str, mut index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// Shape text with per-glyph font fallback.
///
/// 1. Shape using the primary font
/// 2. Scan for `.notdef` glyphs (`glyph_id` == 0)
/// 3. If none found, return a single run (fast path)
/// 4. For `.notdef` ranges, try fallback families in order
/// 5. Return multi-run result with each run's font ID
///
/// Returns `None` if no font in the family list is available.
#[must_use]
#[allow(clippy::too_many_lines)]
// Fallback loop is a single linear pass; extracting sub-functions would scatter the state.
pub fn shape_text_with_fallback(
    db: &FontDatabase,
    families: &[&str],
    weight: u16,
    style: fontdb::Style,
    font_size: f32,
    text: &str,
) -> Option<ShapedTextWithFonts> {
    if text.is_empty() {
        // Return empty result for empty text.
        let font_id = db.query(families, weight, style)?;
        return Some(ShapedTextWithFonts {
            runs: vec![ShapedRun {
                glyphs: Vec::new(),
                font_id,
            }],
            total_advance: 0.0,
        });
    }

    let primary_id = db.query(families, weight, style)?;
    let shaped = shape_text(db, primary_id, font_size, text)?;

    // Fast path: no .notdef glyphs → single run.
    if !shaped.glyphs.iter().any(|g| g.glyph_id == 0) {
        return Some(ShapedTextWithFonts {
            total_advance: shaped.total_advance,
            runs: vec![ShapedRun {
                glyphs: shaped.glyphs,
                font_id: primary_id,
            }],
        });
    }

    // Slow path: find .notdef ranges and try fallback fonts.
    let mut runs: Vec<ShapedRun> = Vec::new();
    let mut total_advance = 0.0;
    let mut i = 0;
    let glyphs = &shaped.glyphs;

    while i < glyphs.len() {
        if glyphs[i].glyph_id != 0 {
            // Good glyph range from primary font.
            let start = i;
            while i < glyphs.len() && glyphs[i].glyph_id != 0 {
                i += 1;
            }
            let run_glyphs: Vec<ShapedGlyph> = glyphs[start..i].to_vec();
            let advance: f32 = run_glyphs.iter().map(|g| g.x_advance).sum();
            total_advance += advance;
            runs.push(ShapedRun {
                glyphs: run_glyphs,
                font_id: primary_id,
            });
        } else {
            // .notdef range — extract text substring via cluster mapping.
            let cluster_start = glyphs[i].cluster as usize;
            let notdef_start = i;
            while i < glyphs.len() && glyphs[i].glyph_id == 0 {
                i += 1;
            }
            // End cluster: use next good glyph's cluster, or end of text.
            let cluster_end = if i < glyphs.len() {
                glyphs[i].cluster as usize
            } else {
                text.len()
            };

            // Ensure valid slice boundaries (clusters may be non-monotonic for RTL/complex scripts).
            let sub_start = cluster_start.min(text.len());
            let sub_end = cluster_end.min(text.len()).max(sub_start);
            // Verify we land on UTF-8 char boundaries.
            let sub_start = floor_char_boundary(text, sub_start);
            let sub_end = ceil_char_boundary(text, sub_end);
            let sub_text = &text[sub_start..sub_end];

            // Skip empty substrings (collapsed cluster boundaries).
            if sub_text.is_empty() {
                let run_glyphs: Vec<ShapedGlyph> = glyphs[notdef_start..i].to_vec();
                let advance: f32 = run_glyphs.iter().map(|g| g.x_advance).sum();
                total_advance += advance;
                runs.push(ShapedRun {
                    glyphs: run_glyphs,
                    font_id: primary_id,
                });
                continue;
            }

            // Try fallback families (skip the primary).
            let mut found = false;
            for &family in families.iter().skip(1) {
                let Some(fallback_id) = db.query(&[family], weight, style) else {
                    continue;
                };
                if fallback_id == primary_id {
                    continue;
                }
                let Some(fb_shaped) = shape_text(db, fallback_id, font_size, sub_text) else {
                    continue;
                };
                // Check if this fallback covers the range (no .notdef).
                if fb_shaped.glyphs.iter().any(|g| g.glyph_id == 0) {
                    continue;
                }
                total_advance += fb_shaped.total_advance;
                runs.push(ShapedRun {
                    glyphs: fb_shaped.glyphs,
                    font_id: fallback_id,
                });
                found = true;
                break;
            }
            if !found {
                // No fallback covered this range; keep the .notdef glyphs from primary.
                let run_glyphs: Vec<ShapedGlyph> = glyphs[notdef_start..i].to_vec();
                let advance: f32 = run_glyphs.iter().map(|g| g.x_advance).sum();
                total_advance += advance;
                runs.push(ShapedRun {
                    glyphs: run_glyphs,
                    font_id: primary_id,
                });
            }
        }
    }

    Some(ShapedTextWithFonts {
        runs,
        total_advance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: try to find a system font for testing.
    fn test_font(db: &FontDatabase) -> Option<fontdb::ID> {
        db.query(crate::TEST_FONT_FAMILIES, 400, fontdb::Style::Normal)
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
        // Total advance (vertical) should be positive (screen-coordinate downward).
        // Note: total_advance for vertical shaping is the sum of y_advances.
        assert!(
            shaped.total_advance > f32::EPSILON,
            "vertical total_advance should be positive, got {}",
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

    // --- M4-1: shape_text_with_fallback ---

    #[test]
    fn fallback_ascii_single_run() {
        let db = FontDatabase::new();
        let result = shape_text_with_fallback(
            &db,
            crate::TEST_FONT_FAMILIES,
            400,
            fontdb::Style::Normal,
            16.0,
            "Hello",
        );
        let Some(result) = result else { return };
        // ASCII text should be covered by the primary font → single run.
        assert_eq!(result.runs.len(), 1);
        assert_eq!(result.runs[0].glyphs.len(), 5);
        assert!(result.total_advance > 0.0);
    }

    #[test]
    fn fallback_empty_string() {
        let db = FontDatabase::new();
        let result = shape_text_with_fallback(
            &db,
            crate::TEST_FONT_FAMILIES,
            400,
            fontdb::Style::Normal,
            16.0,
            "",
        );
        let Some(result) = result else { return };
        assert_eq!(result.runs.len(), 1);
        assert!(result.runs[0].glyphs.is_empty());
        assert!(result.total_advance.abs() < f32::EPSILON);
    }

    #[test]
    fn fallback_no_notdef_fast_path() {
        let db = FontDatabase::new();
        // Normal ASCII text: should take the fast path (no .notdef).
        let result = shape_text_with_fallback(
            &db,
            crate::TEST_FONT_FAMILIES,
            400,
            fontdb::Style::Normal,
            16.0,
            "The quick brown fox",
        );
        let Some(result) = result else { return };
        assert_eq!(result.runs.len(), 1);
        assert!(result.total_advance > 0.0);
        // No .notdef glyphs in the result.
        assert!(!result.runs[0].glyphs.iter().any(|g| g.glyph_id == 0));
    }

    #[test]
    fn fallback_nonexistent_font_returns_none() {
        let db = FontDatabase::new();
        let result = shape_text_with_fallback(
            &db,
            &["__nonexistent_font_12345__"],
            400,
            fontdb::Style::Normal,
            16.0,
            "test",
        );
        assert!(result.is_none());
    }

    #[test]
    fn fallback_preserves_total_advance() {
        let db = FontDatabase::new();
        // Compare single-font shaping with fallback shaping for ASCII text.
        let Some(id) = db.query(crate::TEST_FONT_FAMILIES, 400, fontdb::Style::Normal) else {
            return;
        };
        let direct = shape_text(&db, id, 16.0, "Hello").unwrap();
        let fallback = shape_text_with_fallback(
            &db,
            crate::TEST_FONT_FAMILIES,
            400,
            fontdb::Style::Normal,
            16.0,
            "Hello",
        )
        .unwrap();
        // For ASCII text covered by primary font, advances should match.
        assert!(
            (direct.total_advance - fallback.total_advance).abs() < f32::EPSILON,
            "direct={}, fallback={}",
            direct.total_advance,
            fallback.total_advance
        );
    }

    #[test]
    fn fallback_with_italic_style() {
        let db = FontDatabase::new();
        let result = shape_text_with_fallback(
            &db,
            crate::TEST_FONT_FAMILIES,
            400,
            fontdb::Style::Italic,
            16.0,
            "Hello",
        );
        let Some(result) = result else { return };
        assert_eq!(result.runs.len(), 1);
        assert!(result.total_advance > 0.0);
    }
}
