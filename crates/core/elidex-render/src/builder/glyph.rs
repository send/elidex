//! Glyph placement helpers.

use crate::display_list::GlyphEntry;

/// Place shaped glyphs into a `Vec<GlyphEntry>`, advancing `cursor_x`.
///
/// `letter_spacing` is added between clusters (not between glyphs within the
/// same cluster, and not after the last cluster) per CSS Text Level 3 §4.2.
/// `word_spacing` is added once per U+0020 cluster per CSS Text Level 3 §4.3.
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
    let mut last_ws_cluster: Option<u32> = None;
    for (i, glyph) in shaped_glyphs.iter().enumerate() {
        let x = *cursor_x + glyph.x_offset;
        let y = by - glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        *cursor_x += glyph.x_advance;

        // Word spacing: add once per U+0020 SPACE cluster (CSS Text L3 §4.3).
        // Multiple glyphs can share a cluster (ligatures/combining marks); only
        // apply word-spacing on the first glyph of each space cluster.
        if ws != 0.0 && last_ws_cluster != Some(glyph.cluster) {
            if let Some(ch) = text.get(glyph.cluster as usize..).and_then(|s| s.chars().next()) {
                if ch == ' ' {
                    *cursor_x += ws;
                    last_ws_cluster = Some(glyph.cluster);
                }
            }
        }

        // Letter spacing: between clusters, not between glyphs within the same
        // cluster (CSS Text L3 §4.2). Skip after the last glyph.
        if ls != 0.0 {
            if let Some(next) = shaped_glyphs.get(i + 1) {
                if next.cluster != glyph.cluster {
                    *cursor_x += ls;
                }
            }
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
