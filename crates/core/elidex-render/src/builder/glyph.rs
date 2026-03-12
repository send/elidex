//! Glyph placement helpers.

use crate::display_list::GlyphEntry;

/// Place shaped glyphs into a `Vec<GlyphEntry>`, advancing `cursor_x`.
///
/// `letter_spacing` is added between each glyph (not after the last).
/// `word_spacing` is added for glyphs at word separator (U+0020) cluster positions,
/// per CSS Text Level 3 §4.3.
///
/// Returns the placed glyphs. `cursor_x` is updated to reflect the total advance.
#[must_use]
pub(crate) fn place_glyphs(
    shaped_glyphs: &[elidex_text::ShapedGlyph],
    cursor_x: &mut f32,
    baseline_y: f32,
    letter_spacing: f32,
    word_spacing: f32,
    text: &str,
) -> Vec<GlyphEntry> {
    // Sanitize inputs: NaN/infinity would corrupt cursor position and glyph coordinates.
    let ls = if letter_spacing.is_finite() { letter_spacing } else { 0.0 };
    let ws = if word_spacing.is_finite() { word_spacing } else { 0.0 };
    let by = if baseline_y.is_finite() { baseline_y } else { 0.0 };

    let mut glyphs = Vec::with_capacity(shaped_glyphs.len());
    let last_idx = shaped_glyphs.len().saturating_sub(1);
    for (i, glyph) in shaped_glyphs.iter().enumerate() {
        let x = *cursor_x + glyph.x_offset;
        let y = by - glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        *cursor_x += glyph.x_advance;

        // Word spacing: add extra space at U+0020 SPACE clusters (CSS Text L3 §4.3).
        if ws != 0.0 {
            if let Some(ch) = text.get(glyph.cluster as usize..).and_then(|s| s.chars().next()) {
                if ch == ' ' {
                    *cursor_x += ws;
                }
            }
        }

        // Letter spacing: between glyphs (not after last).
        if ls != 0.0 && i < last_idx {
            *cursor_x += ls;
        }
    }
    glyphs
}

/// Place shaped glyphs vertically (top-to-bottom), advancing `cursor_y`.
///
/// Each glyph is positioned at `(center_x + x_offset, cursor_y + y_offset)` and
/// the cursor advances by `y_advance`. Used for vertical writing modes.
#[must_use]
pub(crate) fn place_glyphs_vertical(
    shaped_glyphs: &[elidex_text::ShapedGlyph],
    center_x: f32,
    cursor_y: &mut f32,
) -> Vec<GlyphEntry> {
    // Sanitize: NaN/infinity would corrupt glyph coordinates.
    let cx = if center_x.is_finite() { center_x } else { 0.0 };
    let mut glyphs = Vec::with_capacity(shaped_glyphs.len());
    for glyph in shaped_glyphs {
        let x = cx + glyph.x_offset;
        let y = *cursor_y + glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        *cursor_y += glyph.y_advance;
    }
    glyphs
}

/// Convert a `Vec<String>` of font family names to a `Vec<&str>` for font queries.
#[must_use]
pub(crate) fn families_as_refs(families: &[String]) -> Vec<&str> {
    families.iter().map(String::as_str).collect()
}
