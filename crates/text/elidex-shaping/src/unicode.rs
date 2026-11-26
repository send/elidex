//! Unicode utility functions for text processing.

use std::borrow::Cow;

use elidex_plugin::TextTransform;
use unicode_segmentation::UnicodeSegmentation;

/// Apply CSS `text-transform` to a string (CSS Text Level 3 §2.1).
///
/// Per §2.1.2 Order of Operations this runs *after* §4.1.1 Phase I (white-space
/// collapsing) and *before* §4.1.2 Phase II (trimming/positioning), so callers
/// must collapse first, then transform, then measure/pack. Casing uses the
/// Unicode-default (locale-agnostic) mappings; locale-specific tailoring is not
/// yet implemented (no ICU). `full-width`/`full-size-kana` are not in the
/// [`TextTransform`] enum and so are not handled here.
#[must_use]
pub fn apply_text_transform(text: &str, transform: TextTransform) -> Cow<'_, str> {
    transform_text(text, transform, None)
}

/// Transform the entire run, reporting each input scalar and its output scalar
/// count for clients preserving logical source boundaries through expansion.
pub fn transform_with_source(
    text: &str,
    transform: TextTransform,
    mut source: impl FnMut(usize, usize),
) -> Cow<'_, str> {
    transform_text(text, transform, Some(&mut source))
}

fn transform_text<'a>(
    text: &'a str,
    transform: TextTransform,
    mut source: Option<&mut dyn FnMut(usize, usize)>,
) -> Cow<'a, str> {
    if let Some(report) = source.as_mut() {
        if transform != TextTransform::Capitalize {
            for (byte, c) in text.char_indices() {
                let count = match transform {
                    TextTransform::Lowercase => c.to_lowercase().count(),
                    TextTransform::Uppercase => c.to_uppercase().count(),
                    _ => 1,
                };
                report(byte, count);
            }
        }
    }
    match transform {
        TextTransform::None => Cow::Borrowed(text),
        TextTransform::Uppercase => Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => Cow::Owned(capitalize_words(text, source)),
    }
}

/// Capitalize the first typographic letter unit of each word (CSS Text Level 3
/// §2.1.1 `capitalize`).
///
/// Word boundaries are determined with `unicode-segmentation` (UAX #29, which
/// §2.1.1 suggests but does not require). This correctly handles
/// punctuation-adjacent boundaries and non-space separators (e.g.
/// "hello-world" → "Hello-World"). Inline-box boundaries are not visible here
/// (each run is transformed independently), matching the existing per-segment
/// behavior; §2.1.1's "inline box boundaries must not introduce a word
/// boundary" across runs is a pre-existing gap.
fn capitalize_words(text: &str, mut source: Option<&mut dyn FnMut(usize, usize)>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut offset = 0;
    for word in text.split_word_bounds() {
        if let Some(report) = source.as_mut() {
            for (byte, c) in word.char_indices() {
                report(
                    offset + byte,
                    if byte == 0 && c.is_alphabetic() {
                        c.to_uppercase().count()
                    } else {
                        1
                    },
                );
            }
        }
        offset += word.len();
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            if first.is_alphabetic() {
                result.extend(first.to_uppercase());
                result.push_str(chars.as_str());
            } else {
                result.push_str(word);
            }
        }
    }
    result
}

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

#[cfg(test)]
mod tests {
    use super::apply_text_transform;
    use elidex_plugin::TextTransform;

    #[test]
    fn apply_text_transform_cases() {
        let cases = [
            ("hello", TextTransform::Uppercase, "HELLO"),
            ("HELLO", TextTransform::Lowercase, "hello"),
            ("hello world", TextTransform::Capitalize, "Hello World"),
            // UAX #29: punctuation-adjacent word boundaries.
            ("hello-world", TextTransform::Capitalize, "Hello-World"),
            ("it's a test", TextTransform::Capitalize, "It's A Test"),
            ("Hello", TextTransform::None, "Hello"),
        ];
        for (input, transform, expected) in cases {
            assert_eq!(
                apply_text_transform(input, transform),
                expected,
                "input={input:?}, transform={transform:?}"
            );
        }
    }
}
