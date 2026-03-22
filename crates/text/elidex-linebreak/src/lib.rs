//! UAX #14 line breaking for elidex.
//!
//! Delegates to the [`unicode_linebreak`] crate for full Unicode Line Break
//! Algorithm support, including CJK punctuation classes (OP/CL), word joiners,
//! and all break classes defined in UAX #14.

/// The kind of break opportunity at a given position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BreakOpportunity {
    /// A soft break opportunity (space, hyphen, CJK boundary, etc.).
    Allowed,
    /// A forced break (newline / paragraph separator).
    Mandatory,
}

/// Returns break opportunities within `text`.
///
/// Each entry is `(byte_offset, opportunity)` where `byte_offset` is the
/// position *between* characters at which a line break may occur.
///
/// Uses the full Unicode Line Break Algorithm (UAX #14) via the
/// `unicode-linebreak` crate, correctly handling CJK punctuation classes
/// (opening/closing brackets suppress breaks), word joiners, and all
/// Unicode break classes.
#[must_use]
pub fn find_break_opportunities(text: &str) -> Vec<(usize, BreakOpportunity)> {
    unicode_linebreak::linebreaks(text)
        .filter_map(|(offset, brk)| {
            // Filter out the end-of-text "mandatory" break that unicode-linebreak
            // always emits at text.len() — it's not a useful break point for layout.
            if offset >= text.len() {
                return None;
            }
            let opp = match brk {
                unicode_linebreak::BreakOpportunity::Mandatory => BreakOpportunity::Mandatory,
                unicode_linebreak::BreakOpportunity::Allowed => BreakOpportunity::Allowed,
            };
            Some((offset, opp))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use BreakOpportunity::{Allowed, Mandatory};

    #[test]
    fn break_opportunities_basic() {
        // Empty / no breaks
        assert_eq!(find_break_opportunities(""), &[]);
        assert_eq!(find_break_opportunities("abcdef"), &[]);

        // Space — break after
        let r = find_break_opportunities("hello world");
        assert!(r.contains(&(6, Allowed)), "expected break at 6: {r:?}");

        // Newline — mandatory break
        let r = find_break_opportunities("line1\nline2");
        assert!(
            r.contains(&(6, Mandatory)),
            "expected mandatory at 6: {r:?}"
        );
    }

    #[test]
    fn cjk_boundaries() {
        // CJK characters should have break opportunities between them.
        let r = find_break_opportunities("漢字列");
        assert!(r.len() >= 2, "expected ≥2 breaks for 3 CJK chars: {r:?}");
    }

    #[test]
    fn cjk_punctuation_no_break_after_opening() {
        // UAX #14 LB15: OP × — no break AFTER opening punctuation.
        // U+300C「 is OP class. In "テスト「文字」テスト":
        // テ(0) ス(3) ト(6) 「(9) 文(12) 字(15) 」(18) テ(21) ス(24) ト(27)
        // No break at byte 12 (between 「 and 文).
        let r = find_break_opportunities("テスト「文字」テスト");
        let breaks_at_12 = r.iter().any(|&(off, _)| off == 12);
        assert!(
            !breaks_at_12,
            "UAX #14 OP ×: should not break after 「 at offset 12: {r:?}"
        );
    }

    #[test]
    fn cjk_punctuation_no_break_before_closing() {
        // UAX #14 LB13: × CL — no break BEFORE closing punctuation.
        // U+300D」 is CL class. In "「文字」テスト":
        // 「(0) 文(3) 字(6) 」(9) テ(12) ス(15) ト(18)
        // No break at byte 9 (between 字 and 」).
        let r = find_break_opportunities("「文字」テスト");
        let breaks_at_9 = r.iter().any(|&(off, _)| off == 9);
        assert!(
            !breaks_at_9,
            "UAX #14 × CL: should not break before 」 at offset 9: {r:?}"
        );
    }

    #[test]
    fn hangul_breaks() {
        let r = find_break_opportunities("한글");
        assert!(!r.is_empty(), "expected breaks for Hangul: {r:?}");
    }

    #[test]
    fn hyphen_break() {
        let r = find_break_opportunities("well-known");
        assert!(
            r.iter().any(|&(off, opp)| off == 5 && opp == Allowed),
            "expected break after hyphen at 5: {r:?}"
        );
    }

    #[test]
    fn mixed_cjk_latin() {
        let r = find_break_opportunities("hello漢字world");
        // Should have breaks at CJK boundaries.
        assert!(
            r.len() >= 2,
            "expected ≥2 breaks for mixed CJK/Latin: {r:?}"
        );
    }
}
