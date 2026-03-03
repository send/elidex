//! Whitespace normalization and collapsing.

use elidex_plugin::WhiteSpace;

use super::StyledTextSegment;

/// Normalize line endings per CSS Text §4.1 Phase I.
///
/// Converts `\r\n` sequences to `\n` first, then any remaining bare `\r` to `\n`.
pub(crate) fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Collapse whitespace across segments according to `white-space` mode.
///
/// | Mode    | collapse spaces | collapse newlines | wrap  |
/// |---------|:---:|:---:|:---:|
/// | Normal  | Yes | Yes | Yes |
/// | Pre     | No  | No  | No  |
/// | NoWrap  | Yes | Yes | No  |
/// | PreWrap | No  | No  | Yes |
/// | PreLine | Yes | No  | Yes |
pub(crate) fn collapse_segments(
    segments: &[StyledTextSegment],
    white_space: WhiteSpace,
) -> Vec<(String, usize)> {
    let collapse_spaces = matches!(
        white_space,
        WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine
    );
    let collapse_newlines = matches!(white_space, WhiteSpace::Normal | WhiteSpace::NoWrap);

    // Pre / PreWrap: preserve text, but still normalize \r\n → \n (CSS Text §4.1).
    if !collapse_spaces && !collapse_newlines {
        return segments
            .iter()
            .enumerate()
            .filter(|(_, seg)| !seg.text.is_empty())
            .map(|(idx, seg)| {
                let text = normalize_line_endings(&seg.text);
                (text, idx)
            })
            .collect();
    }

    let mut result: Vec<(String, usize)> = Vec::new();
    let mut prev_was_space = true; // Leading whitespace is trimmed.
    for (idx, seg) in segments.iter().enumerate() {
        // CSS Text §4.1 Phase I: normalize \r\n → \n, bare \r → \n.
        let normalized = normalize_line_endings(&seg.text);
        let mut seg_text = String::new();
        for ch in normalized.chars() {
            let is_newline = ch == '\n';
            let is_space = ch == ' ' || ch == '\t';

            if is_newline {
                if collapse_newlines {
                    // Treat newlines as spaces (Normal / NoWrap).
                    if collapse_spaces && !prev_was_space {
                        seg_text.push(' ');
                        prev_was_space = true;
                    }
                } else {
                    // PreLine: preserve newlines; strip spaces/tabs immediately
                    // before the forced break (CSS Text §4).
                    let trimmed = seg_text.trim_end_matches([' ', '\t']);
                    seg_text.truncate(trimmed.len());
                    seg_text.push('\n');
                    prev_was_space = true; // Reset space state after newline.
                }
            } else if is_space {
                if collapse_spaces {
                    if !prev_was_space {
                        seg_text.push(' ');
                        prev_was_space = true;
                    }
                } else {
                    seg_text.push(ch);
                    prev_was_space = false;
                }
            } else {
                seg_text.push(ch);
                prev_was_space = false;
            }
        }
        if !seg_text.is_empty() {
            result.push((seg_text, idx));
        }
    }
    // Trim trailing/leading whitespace from the result.
    // For PreLine: only trim spaces/tabs, preserve newlines.
    if collapse_newlines {
        if let Some(last) = result.last_mut() {
            last.0 = last.0.trim_end().to_string();
        }
        if let Some(first) = result.first_mut() {
            first.0 = first.0.trim_start().to_string();
        }
    } else {
        // PreLine: trim only spaces/tabs, not newlines.
        if let Some(last) = result.last_mut() {
            last.0 = last.0.trim_end_matches([' ', '\t']).to_string();
        }
        if let Some(first) = result.first_mut() {
            first.0 = first.0.trim_start_matches([' ', '\t']).to_string();
        }
    }
    result.retain(|(text, _)| !text.is_empty());
    result
}
