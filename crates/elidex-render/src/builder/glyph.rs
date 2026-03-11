//! Glyph placement helpers.

use crate::display_list::GlyphEntry;

/// Place shaped glyphs into a `Vec<GlyphEntry>`, advancing `cursor_x`.
///
/// Returns the placed glyphs. `cursor_x` is updated to reflect the total advance.
#[must_use]
pub(crate) fn place_glyphs(
    shaped_glyphs: &[elidex_text::ShapedGlyph],
    cursor_x: &mut f32,
    baseline_y: f32,
) -> Vec<GlyphEntry> {
    let mut glyphs = Vec::with_capacity(shaped_glyphs.len());
    for glyph in shaped_glyphs {
        let x = *cursor_x + glyph.x_offset;
        let y = baseline_y - glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        *cursor_x += glyph.x_advance;
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
    let mut glyphs = Vec::with_capacity(shaped_glyphs.len());
    for glyph in shaped_glyphs {
        let x = center_x + glyph.x_offset;
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
