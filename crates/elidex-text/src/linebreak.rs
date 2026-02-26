//! Simplified UAX #14 line breaking for Phase 1.
//!
//! Supports break opportunities at:
//! - ASCII space (0x20) — break after
//! - Hyphen (`-`) — break after
//! - CJK character boundaries — break between CJK characters
//! - Newline (`\n`) — mandatory break
//!
//! Full ICU4X / unicode-linebreak support is deferred to Phase 2.

/// The kind of break opportunity at a given position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
pub fn find_break_opportunities(text: &str) -> Vec<(usize, BreakOpportunity)> {
    let mut result = Vec::new();
    let mut iter = text.char_indices().peekable();

    while let Some((byte_offset, ch)) = iter.next() {
        // Mandatory break at newline — the break is *after* the newline character.
        if ch == '\n' {
            let break_pos = byte_offset + ch.len_utf8();
            if break_pos <= text.len() {
                result.push((break_pos, BreakOpportunity::Mandatory));
            }
            continue;
        }

        // Allowed break after ASCII space.
        if ch == ' ' {
            let break_pos = byte_offset + 1;
            if break_pos < text.len() {
                result.push((break_pos, BreakOpportunity::Allowed));
            }
            continue;
        }

        // Allowed break after hyphen.
        if ch == '-' {
            let break_pos = byte_offset + 1;
            if break_pos < text.len() {
                result.push((break_pos, BreakOpportunity::Allowed));
            }
            continue;
        }

        // CJK boundary: break between two adjacent CJK characters.
        if is_cjk_codepoint(ch) {
            if let Some(&(next_offset, next_ch)) = iter.peek() {
                if is_cjk_codepoint(next_ch) {
                    result.push((next_offset, BreakOpportunity::Allowed));
                }
            }
        }
    }

    result
}

/// Returns `true` if `c` is in a CJK Unified Ideographs block.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        let result = find_break_opportunities("");
        assert!(result.is_empty());
    }

    #[test]
    fn space_break() {
        let result = find_break_opportunities("hello world");
        assert_eq!(result, vec![(6, BreakOpportunity::Allowed)]);
    }

    #[test]
    fn hyphen_break() {
        let result = find_break_opportunities("well-known");
        assert_eq!(result, vec![(5, BreakOpportunity::Allowed)]);
    }

    #[test]
    fn newline_mandatory() {
        let result = find_break_opportunities("line1\nline2");
        assert_eq!(result, vec![(6, BreakOpportunity::Mandatory)]);
    }

    #[test]
    fn cjk_boundaries() {
        // Three CJK chars: 漢字列 → breaks between each pair
        let text = "漢字列";
        let result = find_break_opportunities(text);
        // 漢 = 3 bytes, 字 = 3 bytes, 列 = 3 bytes
        // Break between 漢 and 字 at offset 3, between 字 and 列 at offset 6
        assert_eq!(
            result,
            vec![
                (3, BreakOpportunity::Allowed),
                (6, BreakOpportunity::Allowed),
            ]
        );
    }

    #[test]
    fn ascii_only_no_cjk() {
        let result = find_break_opportunities("abcdef");
        assert!(result.is_empty());
    }

    #[test]
    fn mixed_ascii_cjk_space() {
        let result = find_break_opportunities("hello 漢字");
        // Space at offset 5 → break at 6, then CJK pair 漢字 → break at 9
        assert_eq!(
            result,
            vec![
                (6, BreakOpportunity::Allowed),
                (9, BreakOpportunity::Allowed),
            ]
        );
    }

    #[test]
    fn consecutive_spaces() {
        let result = find_break_opportunities("a  b");
        // Space at offset 1 → break at 2, space at offset 2 → break at 3
        assert_eq!(
            result,
            vec![
                (2, BreakOpportunity::Allowed),
                (3, BreakOpportunity::Allowed),
            ]
        );
    }
}
