//! Unicode utility functions for text processing.

/// Returns `true` if `ch` is a Unicode word separator (General Category Zs).
///
/// Per CSS Text Level 3 §4.3, word-spacing applies to all characters with
/// Unicode General Category Zs (space separators), not just U+0020 SPACE.
pub fn is_word_separator(ch: char) -> bool {
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
