//! Simplified UAX #14 line breaking for elidex.
//!
//! Supports break opportunities at:
//! - ASCII space (0x20) — break after
//! - Hyphen (`-`) — break after
//! - CJK/Hangul boundaries — break before and after CJK/Hangul characters
//! - Newline (`\n`) — mandatory break
//!
//! Full ICU4X / unicode-linebreak support is deferred to Phase 4.

/// The kind of break opportunity at a given position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BreakOpportunity {
    /// A soft break opportunity (space, hyphen, CJK boundary).
    Allowed,
    /// A forced break (newline character).
    Mandatory,
}

/// Returns break opportunities within `text`.
///
/// Each entry is `(byte_offset, opportunity)` where `byte_offset` is the
/// position *between* characters at which a line break may occur.
#[must_use]
pub fn find_break_opportunities(text: &str) -> Vec<(usize, BreakOpportunity)> {
    let mut result = Vec::new();
    let mut iter = text.char_indices().peekable();

    while let Some((byte_offset, ch)) = iter.next() {
        // Mandatory break at newline — the break is *after* the newline character.
        // Uses `<=` (not `<`) so a trailing newline forces an empty last line,
        // matching CSS white-space handling where `\n` always terminates a line.
        if ch == '\n' {
            let break_pos = byte_offset + ch.len_utf8();
            if break_pos <= text.len() {
                result.push((break_pos, BreakOpportunity::Mandatory));
            }
            continue;
        }

        // Allowed break after ASCII space or hyphen.
        // Uses `<` (not `<=`): a trailing space/hyphen at end-of-string is not
        // a useful break point since there is no following content to wrap.
        if ch == ' ' || ch == '-' {
            let break_pos = byte_offset + 1;
            if break_pos < text.len() {
                result.push((break_pos, BreakOpportunity::Allowed));
            }
            continue;
        }

        // CJK boundaries: break before and after CJK characters.
        // This covers CJK–CJK, CJK–non-CJK, and non-CJK–CJK boundaries.
        // TODO(Phase 4): Per UAX #14, opening punctuation (OP class, e.g.
        // U+3008「〈」) should suppress breaks before them, and closing
        // punctuation (CL class, e.g. U+3009「〉」) should suppress breaks
        // after them. Replace with `unicode-linebreak` crate for full support.
        if is_cjk_codepoint(ch) {
            if let Some(&(next_offset, _)) = iter.peek() {
                result.push((next_offset, BreakOpportunity::Allowed));
            }
        } else if let Some(&(next_offset, next_ch)) = iter.peek() {
            if is_cjk_codepoint(next_ch) {
                result.push((next_offset, BreakOpportunity::Allowed));
            }
        }
    }

    result
}

/// Returns `true` if `c` is in a CJK or Hangul block.
///
/// Covers the main CJK blocks used in Chinese, Japanese, and Korean text.
fn is_cjk_codepoint(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
        // CJK Unified Ideographs Extension A
        || (0x3400..=0x4DBF).contains(&cp)
        // CJK Unified Ideographs Extension B
        || (0x20000..=0x2A6DF).contains(&cp)
        // CJK Compatibility Ideographs
        || (0xF900..=0xFAFF).contains(&cp)
        // Katakana
        || (0x30A0..=0x30FF).contains(&cp)
        // Hiragana
        || (0x3040..=0x309F).contains(&cp)
        // CJK Symbols and Punctuation
        || (0x3000..=0x303F).contains(&cp)
        // Fullwidth Forms
        || (0xFF00..=0xFFEF).contains(&cp)
        // Hangul Syllables
        || (0xAC00..=0xD7AF).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    use BreakOpportunity::{Allowed, Mandatory};

    #[test]
    fn break_opportunities() {
        let cases: &[(&str, &[(usize, BreakOpportunity)])] = &[
            // Empty / no breaks
            ("", &[]),
            ("abcdef", &[]),
            // Space
            ("hello world", &[(6, Allowed)]),
            ("a  b", &[(2, Allowed), (3, Allowed)]),
            // Hyphen
            ("well-known", &[(5, Allowed)]),
            // Newline (mandatory)
            ("line1\nline2", &[(6, Mandatory)]),
            // CJK boundaries
            ("漢字列", &[(3, Allowed), (6, Allowed)]),
            ("hello 漢字", &[(6, Allowed), (9, Allowed)]),
            ("hello漢字", &[(5, Allowed), (8, Allowed)]),
            ("漢字hello", &[(3, Allowed), (6, Allowed)]),
            // Hangul
            ("한글", &[(3, Allowed)]),
            ("hello한글", &[(5, Allowed), (8, Allowed)]),
        ];
        for (input, expected) in cases {
            let result = find_break_opportunities(input);
            assert_eq!(result, *expected, "input: {input:?}");
        }
    }
}
