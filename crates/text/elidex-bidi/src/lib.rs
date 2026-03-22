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

/// Detect the directionality from the first strong character in `text`.
///
/// Implements the "first strong character" algorithm used by HTML `dir="auto"`
/// (WHATWG §15.3.6). Returns `Ltr` if no strong character is found.
#[must_use]
pub fn first_strong_direction(text: &str) -> ParagraphLevel {
    // BidiInfo::new with level=None auto-detects paragraph direction
    // from the first strong character (UAX #9 rule P2/P3).
    let bidi_info = BidiInfo::new(text, None);
    if let Some(para) = bidi_info.paragraphs.first() {
        if para.level.is_rtl() {
            ParagraphLevel::Rtl
        } else {
            ParagraphLevel::Ltr
        }
    } else {
        ParagraphLevel::Ltr
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

/// CSS `unicode-bidi` property values that affect `BiDi` analysis.
///
/// Mirrors the CSS Writing Modes Level 3 §2.2 values. Passed to
/// [`analyze_bidi`] to control embedding/isolation behavior.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BidiOverride {
    /// No special behavior — the natural `BiDi` algorithm runs.
    #[default]
    Normal,
    /// Creates an embedding (LRE/RLE + PDF).
    Embed,
    /// Overrides direction for all characters (LRO/RLO + PDF).
    BidiOverride,
    /// Creates a directional isolate (LRI/RLI + PDI).
    Isolate,
    /// Isolate + override (LRI/RLI + LRO/RLO + PDF + PDI).
    IsolateOverride,
    /// First-strong isolate (auto-detect paragraph direction).
    Plaintext,
}

/// Analyze bidirectional text and return a list of level runs.
///
/// Each run is a contiguous range of text at the same embedding level.
/// The `paragraph_level` parameter corresponds to the CSS `direction`
/// property of the containing block.
///
/// The `bidi_override` parameter applies the CSS `unicode-bidi` property
/// (CSS Writing Modes Level 3 §2.2), inserting the appropriate Unicode
/// control characters (UAX #9 X1-X8) before analysis:
/// - `Embed` → LRE/RLE + PDF
/// - `BidiOverride` → LRO/RLO + PDF
/// - `Isolate` → LRI/RLI + PDI
/// - `IsolateOverride` → LRI/RLI + LRO/RLO + PDF + PDI
/// - `Plaintext` → FSI + PDI (auto-detect direction)
///
/// Returned byte offsets refer to the original `text`, not the internal
/// control-character-augmented string.
#[must_use]
pub fn analyze_bidi(
    text: &str,
    paragraph_level: ParagraphLevel,
    bidi_override: BidiOverride,
) -> Vec<BidiRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let is_rtl = matches!(paragraph_level, ParagraphLevel::Rtl);
    let level = if matches!(bidi_override, BidiOverride::Plaintext) {
        // Plaintext: auto-detect paragraph direction from first strong character.
        None
    } else {
        Some(if is_rtl { Level::rtl() } else { Level::ltr() })
    };

    // Build augmented text with Unicode bidi control characters.
    let (prefix, suffix) = bidi_control_chars(bidi_override, is_rtl);
    let prefix_len = prefix.len();
    let suffix_len = suffix.len();

    if prefix_len == 0 && suffix_len == 0 {
        // Fast path: no control characters needed.
        return analyze_raw(text, level);
    }

    // Construct augmented text: prefix + original text + suffix.
    let mut augmented = String::with_capacity(prefix_len + text.len() + suffix_len);
    augmented.push_str(prefix);
    augmented.push_str(text);
    augmented.push_str(suffix);

    let raw_runs = analyze_raw(&augmented, level);

    // Adjust byte offsets to refer to the original text.
    raw_runs
        .into_iter()
        .filter_map(|r| {
            let start = r.start.saturating_sub(prefix_len);
            let end = r.end.saturating_sub(prefix_len).min(text.len());
            if start >= end {
                None
            } else {
                Some(BidiRun {
                    start,
                    end,
                    level: r.level,
                })
            }
        })
        .collect()
}

/// Legacy 2-argument form — equivalent to `analyze_bidi(text, level, Normal)`.
///
/// Preserved for callers that don't have a `unicode-bidi` CSS value.
#[must_use]
pub fn analyze_bidi_simple(text: &str, paragraph_level: ParagraphLevel) -> Vec<BidiRun> {
    analyze_bidi(text, paragraph_level, BidiOverride::Normal)
}

/// Return the prefix/suffix Unicode control character strings for a given
/// `BidiOverride` value.
fn bidi_control_chars(bidi_override: BidiOverride, is_rtl: bool) -> (&'static str, &'static str) {
    match bidi_override {
        BidiOverride::Normal => ("", ""),
        BidiOverride::Embed => {
            if is_rtl {
                ("\u{202B}", "\u{202C}") // RLE + PDF
            } else {
                ("\u{202A}", "\u{202C}") // LRE + PDF
            }
        }
        BidiOverride::BidiOverride => {
            if is_rtl {
                ("\u{202E}", "\u{202C}") // RLO + PDF
            } else {
                ("\u{202D}", "\u{202C}") // LRO + PDF
            }
        }
        BidiOverride::Isolate => {
            if is_rtl {
                ("\u{2067}", "\u{2069}") // RLI + PDI
            } else {
                ("\u{2066}", "\u{2069}") // LRI + PDI
            }
        }
        BidiOverride::IsolateOverride => {
            // FSI + override direction + ... + PDF + PDI
            if is_rtl {
                ("\u{2067}\u{202E}", "\u{202C}\u{2069}") // RLI + RLO + ... + PDF + PDI
            } else {
                ("\u{2066}\u{202D}", "\u{202C}\u{2069}") // LRI + LRO + ... + PDF + PDI
            }
        }
        BidiOverride::Plaintext => {
            // First Strong Isolate: auto-detects direction.
            ("\u{2068}", "\u{2069}") // FSI + PDI
        }
    }
}

/// Run UAX #9 analysis on raw text with a given level hint.
fn analyze_raw(text: &str, level: Option<Level>) -> Vec<BidiRun> {
    let bidi_info = BidiInfo::new(text, level);
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

        // Final run.
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
        let runs = analyze_bidi("Hello world", ParagraphLevel::Ltr, BidiOverride::Normal);
        assert_eq!(runs.len(), 1);
        assert!(!runs[0].is_rtl());
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[0].end, 11);
    }

    #[test]
    fn pure_rtl_text() {
        let runs = analyze_bidi("مرحبا", ParagraphLevel::Rtl, BidiOverride::Normal);
        assert_eq!(runs.len(), 1);
        assert!(runs[0].is_rtl());
    }

    #[test]
    fn mixed_ltr_rtl() {
        // "Hello مرحبا World" — LTR text, then Arabic RTL, then LTR
        let text = "Hello مرحبا World";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr, BidiOverride::Normal);
        assert!(
            runs.len() >= 2,
            "expected at least 2 runs, got {}",
            runs.len()
        );
        assert!(!runs[0].is_rtl());
    }

    #[test]
    fn empty_text() {
        let runs = analyze_bidi("", ParagraphLevel::Ltr, BidiOverride::Normal);
        assert!(runs.is_empty());
    }

    #[test]
    fn reorder_preserves_ltr() {
        let runs = analyze_bidi("Hello world", ParagraphLevel::Ltr, BidiOverride::Normal);
        let reordered = reorder_line(&runs);
        assert_eq!(reordered.len(), 1);
        assert_eq!(reordered[0].start, 0);
    }

    #[test]
    fn reorder_reverses_rtl_in_ltr() {
        let text = "Hello مرحبا World";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr, BidiOverride::Normal);
        let reordered = reorder_line(&runs);
        assert!(!reordered.is_empty());
    }

    // --- unicode-bidi CSS property tests ---

    #[test]
    fn bidi_override_forces_direction() {
        // With BidiOverride + RTL, all characters should be at RTL level.
        let text = "Hello";
        let runs = analyze_bidi(text, ParagraphLevel::Rtl, BidiOverride::BidiOverride);
        assert!(!runs.is_empty());
        for run in &runs {
            assert!(run.is_rtl(), "bidi-override RTL should force RTL: {run:?}");
        }
    }

    #[test]
    fn isolate_preserves_offsets() {
        // Isolate should not change byte offsets — they should refer to original text.
        let text = "Hello world";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr, BidiOverride::Isolate);
        assert!(!runs.is_empty());
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs.last().unwrap().end, text.len());
    }

    #[test]
    fn plaintext_autodetects_direction() {
        // Arabic text with Plaintext should auto-detect as RTL.
        let text = "مرحبا";
        let runs = analyze_bidi(text, ParagraphLevel::Ltr, BidiOverride::Plaintext);
        assert!(!runs.is_empty());
        assert!(
            runs[0].is_rtl(),
            "plaintext should auto-detect Arabic as RTL"
        );
    }

    // --- first_strong_direction tests ---

    #[test]
    fn first_strong_ltr_text() {
        assert_eq!(first_strong_direction("Hello"), ParagraphLevel::Ltr);
    }

    #[test]
    fn first_strong_rtl_text() {
        assert_eq!(first_strong_direction("مرحبا"), ParagraphLevel::Rtl);
    }

    #[test]
    fn first_strong_empty_defaults_to_ltr() {
        assert_eq!(first_strong_direction(""), ParagraphLevel::Ltr);
    }

    #[test]
    fn first_strong_neutral_then_rtl() {
        // Digits are neutral; first strong is Arabic
        assert_eq!(first_strong_direction("123 مرحبا"), ParagraphLevel::Rtl);
    }

    #[test]
    fn normal_same_as_simple() {
        let text = "Hello مرحبا World";
        let r1 = analyze_bidi(text, ParagraphLevel::Ltr, BidiOverride::Normal);
        let r2 = analyze_bidi_simple(text, ParagraphLevel::Ltr);
        assert_eq!(r1, r2);
    }
}
