//! Glyph placement helpers.

use crate::display_list::GlyphEntry;

/// Returns `true` if `ch` is a Unicode word separator (General Category Zs).
///
/// Per CSS Text Level 3 §4.3, word-spacing applies to all characters with
/// Unicode General Category Zs (space separators), not just U+0020 SPACE.
fn is_word_separator(ch: char) -> bool {
    matches!(
        ch,
        '\u{0020}'  // SPACE
        | '\u{00A0}'  // NO-BREAK SPACE
        | '\u{1680}'  // OGHAM SPACE MARK
        | '\u{2000}'  // EN QUAD
        | '\u{2001}'  // EM QUAD
        | '\u{2002}'  // EN SPACE
        | '\u{2003}'  // EM SPACE
        | '\u{2004}'  // THREE-PER-EM SPACE
        | '\u{2005}'  // FOUR-PER-EM SPACE
        | '\u{2006}'  // SIX-PER-EM SPACE
        | '\u{2007}'  // FIGURE SPACE
        | '\u{2008}'  // PUNCTUATION SPACE
        | '\u{2009}'  // THIN SPACE
        | '\u{200A}'  // HAIR SPACE
        | '\u{202F}'  // NARROW NO-BREAK SPACE
        | '\u{205F}'  // MEDIUM MATHEMATICAL SPACE
        | '\u{3000}' // IDEOGRAPHIC SPACE
    )
}

/// Place shaped glyphs into a `Vec<GlyphEntry>`, advancing `cursor_x`.
///
/// `letter_spacing` is added between clusters (not between glyphs within the
/// same cluster, and not after the last cluster) per CSS Text Level 3 §4.2.
/// `word_spacing` is added once per Unicode Zs (space separator) cluster per CSS Text Level 3 §4.3.
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
    let by = if baseline_y.is_finite() {
        baseline_y
    } else {
        0.0
    };

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

        // Word spacing: add once per Unicode Zs separator cluster (CSS Text L3 §4.3).
        // Multiple glyphs can share a cluster (ligatures/combining marks); only
        // apply word-spacing on the first glyph of each space cluster.
        if ws != 0.0 && last_ws_cluster != Some(glyph.cluster) {
            let idx = glyph.cluster as usize;
            if idx < text.len() && text.is_char_boundary(idx) {
                if let Some(ch) = text[idx..].chars().next() {
                    if is_word_separator(ch) {
                        *cursor_x += ws;
                        last_ws_cluster = Some(glyph.cluster);
                    }
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
