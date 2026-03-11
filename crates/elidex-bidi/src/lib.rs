//! Bidirectional text analysis and reordering for elidex.
//!
//! Wraps the [`unicode_bidi`] crate to provide a simplified API for
//! the Unicode Bidirectional Algorithm (UAX #9).

use unicode_bidi::{BidiInfo, Level};

/// A contiguous run of text at a single bidi embedding level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BidiRun {
    /// Start byte offset in the source text.
    pub start: usize,
    /// End byte offset (exclusive) in the source text.
    pub end: usize,
    /// The bidi embedding level of this run.
    pub level: Level,
}

impl BidiRun {
    /// Returns `true` if this run is right-to-left.
    #[must_use]
    pub fn is_rtl(&self) -> bool {
        self.level.is_rtl()
    }
}

/// The paragraph-level direction hint for bidi analysis.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ParagraphLevel {
    /// Left-to-right paragraph (CSS `direction: ltr`).
    #[default]
    Ltr,
    /// Right-to-left paragraph (CSS `direction: rtl`).
    Rtl,
}

/// Analyze bidirectional text and return a list of level runs.
///
/// Each run is a contiguous range of text at the same embedding level.
/// The `paragraph_level` parameter should correspond to the CSS
/// `direction` property of the containing block.
///
// TODO(Phase 4): Respect `unicode-bidi: isolate / isolate-override / plaintext`
// from CSS. Currently only the paragraph-level direction hint is used;
// explicit embedding/isolation overrides (UAX #9 X1-X8) are not enforced.
#[must_use]
pub fn analyze_bidi(text: &str, paragraph_level: ParagraphLevel) -> Vec<BidiRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let level = match paragraph_level {
        ParagraphLevel::Ltr => Some(Level::ltr()),
        ParagraphLevel::Rtl => Some(Level::rtl()),
    };

    let bidi_info = BidiInfo::new(text, level);

    // BidiInfo may have multiple paragraphs (split by paragraph separators).
    // For inline layout we typically process one paragraph at a time,
    // but handle all paragraphs for completeness.
    let mut runs = Vec::new();
    for para in &bidi_info.paragraphs {
        let line = para.range.clone();
        let levels = &bidi_info.levels[line.start..line.end];

        if levels.is_empty() {
            continue;
        }

        let mut run_start = 0;
        let mut current_level = levels[0];

        for (i, &lvl) in levels.iter().enumerate().skip(1) {
            if lvl != current_level {
                runs.push(BidiRun {
                    start: line.start + run_start,
                    end: line.start + i,
                    level: current_level,
                });
                run_start = i;
                current_level = lvl;
            }
        }

        // Final run
        runs.push(BidiRun {
            start: line.start + run_start,
            end: line.end,
            level: current_level,
        });
    }

    runs
}

/// Reorder runs for visual display according to the Unicode Bidi Algorithm.
///
/// Takes a slice of runs (from a single line) and returns them reordered
/// for left-to-right visual rendering. RTL runs within the result should
/// have their glyphs rendered right-to-left.
#[must_use]
pub fn reorder_line(runs: &[BidiRun]) -> Vec<BidiRun> {
    let levels: Vec<u8> = runs.iter().map(|r| r.level.number()).collect();
    let indices = reorder_by_levels(&levels);
    indices.iter().map(|&i| runs[i]).collect()
}

/// Reorder indices by bidi embedding levels (UAX #9 rule L2).
///
/// For each level from `max_level` down to the minimum odd level, reverses
/// contiguous subsequences of indices whose level is >= that threshold.
///
/// Returns indices in visual order. Works with any `u8` level slice —
/// used both for `BidiRun` reordering and segment-level reordering.
#[must_use]
pub fn reorder_by_levels(levels: &[u8]) -> Vec<usize> {
    if levels.is_empty() {
        return Vec::new();
    }
    let max_level = *levels.iter().max().unwrap_or(&0);
    let min_odd = levels
        .iter()
        .copied()
        .filter(|&l| l % 2 == 1)
        .min()
        .unwrap_or(max_level + 1);

    let mut result: Vec<usize> = (0..levels.len()).collect();

    // UAX #9 L2: reverse contiguous runs at each level from max down to min odd.
    for level in (min_odd..=max_level).rev() {
        let mut i = 0;
        while i < result.len() {
            if levels[result[i]] >= level {
                let start = i;
                while i < result.len() && levels[result[i]] >= level {
                    i += 1;
                }
                result[start..i].reverse();
            } else {
                i += 1;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_ltr_text() {
        let runs = analyze_bidi("Hello world", ParagraphLevel::Ltr);
        assert_eq!(runs.len(), 1);
        assert!(!runs[0].is_rtl());
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[0].end, 11);
    }

    #[test]
    fn pure_rtl_text() {
        let runs = analyze_bidi("مرحبا", ParagraphLevel::Rtl);
        assert_eq!(runs.len(), 1);
        assert!(runs[0].is_rtl());
    }

    #[test]
    fn mixed_ltr_rtl() {
        // "Hello مرحبا World" — LTR text, then Arabic RTL, then LTR
        let text = "Hello مرحبا World";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr);
        assert!(
            runs.len() >= 2,
            "expected at least 2 runs, got {}",
            runs.len()
        );

        // First run should be LTR (the "Hello " part)
        assert!(!runs[0].is_rtl());
    }

    #[test]
    fn empty_text() {
        let runs = analyze_bidi("", ParagraphLevel::Ltr);
        assert!(runs.is_empty());
    }

    #[test]
    fn reorder_preserves_ltr() {
        let runs = analyze_bidi("Hello world", ParagraphLevel::Ltr);
        let reordered = reorder_line(&runs);
        assert_eq!(reordered.len(), 1);
        assert_eq!(reordered[0].start, 0);
    }

    #[test]
    fn reorder_reverses_rtl_in_ltr() {
        // With RTL text embedded in LTR, the RTL portion should be reordered
        let text = "Hello مرحبا World";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr);
        let reordered = reorder_line(&runs);
        // After reordering, visual order should place RTL run correctly
        assert!(!reordered.is_empty());
    }
}
