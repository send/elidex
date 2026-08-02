//! White space collapsing (CSS Text 3 §4.1.1 Phase I).

use elidex_plugin::WhiteSpace;

use super::InlineItem;

/// Apply CSS Text 3 §4.1.1 Phase I white-space collapsing/transformation to the
/// ordered text runs of an inline formatting context, parameterized by each
/// run's `white-space`.
///
/// Per CSS Text 3 §4.1.1 (`#white-space-phase-1`) + §4.1.3 (`#line-break-transform`),
/// for collapsible values (`normal`/`nowrap`/`pre-line`): tabs become spaces
/// (step 3); for `normal`/`nowrap` a segment break (`\n`) is collapsible and is
/// transformed to a space (§4.1.3, word-separator-language baseline), while for
/// `pre-line` it is preserved as a forced break with surrounding collapsible
/// spaces removed (step 1); a collapsible space immediately following another
/// collapsible space — even across inline (run) boundaries within the same IFC —
/// collapses to zero advance width (step 4), i.e. each run of collapsible spaces
/// becomes a single space. For preserve values (`pre`/`pre-wrap`) the text is left
/// intact (segment breaks stay as forced breaks).
///
/// Line-edge trimming (§4.1.2 Phase II) and the "white space that collapses away
/// generates no box" rule (CSS 2 §9.2.2.1 / §9.4.2 — the same composite
/// `LinePacker` cites) are applied at line-packing
/// time (see [`super::pack::LinePacker`]), not here.
pub(super) fn collapse_inline_whitespace(items: &mut [InlineItem]) {
    // Cross-run collapse state: true when the previously emitted character (in any
    // earlier run of this IFC) was a collapsible space, so a following collapsible
    // space collapses to zero advance width (§4.1.1 step 4). Initialized to `true`
    // so leading collapsible white space at the start of the inline formatting
    // context collapses away rather than becoming a leading space that shifts
    // content (CSS Text 3 §4.1.2; matches `elidex-render`'s `collapse_segments`).
    let mut prev_collapsible_space = true;
    // Index of the most recent text run, so a preserved segment break at the start
    // of a later run can remove a collapsible space left at the end of it (§4.1.1
    // step 1, across the run boundary).
    let mut prev_text_idx: Option<usize> = None;
    for i in 0..items.len() {
        // Move the run's text out (no clone) to collapse it, then write it back.
        let (text, white_space) = match &mut items[i] {
            InlineItem::Text(run) => (std::mem::take(&mut run.text), run.white_space),
            // Atomic inline boxes are rendered content: a collapsible space that
            // follows one is a fresh separator, not collapsed away.
            InlineItem::Atomic { .. } => {
                prev_collapsible_space = false;
                prev_text_idx = None;
                continue;
            }
            // Out-of-flow placeholders (absolutely positioned, CSS 2.1 §9.3.1/§9.6)
            // are removed from the normal flow and do not participate in the inline
            // text flow, so they neither emit nor reset collapse state.
            InlineItem::Placeholder(_) => continue,
        };
        let (collapsed, trim_prev_trailing_space) =
            collapse_run_text(&text, white_space, &mut prev_collapsible_space);
        if trim_prev_trailing_space {
            if let Some(j) = prev_text_idx {
                if let InlineItem::Text(prev) = &mut items[j] {
                    if prev.text.ends_with(' ') {
                        prev.text.pop();
                    }
                }
            }
        }
        let collapsed_is_empty = collapsed.is_empty();
        if let InlineItem::Text(run) = &mut items[i] {
            run.text = collapsed;
        }
        // Keep `prev_text_idx` pointing at the last run that actually emitted text. A
        // run that collapsed to empty holds no trailing space, so the cross-run
        // trim (§4.1.1 step 1) must target the earlier run that emitted the pending
        // space, not this empty one.
        if !collapsed_is_empty {
            prev_text_idx = Some(i);
        }
    }
}

/// Collapse a single run's text per its `white-space`, threading the cross-run
/// `prev_collapsible_space` state. See [`collapse_inline_whitespace`].
///
/// Returns the collapsed text and a flag requesting that the caller remove a
/// collapsible space left at the end of the *previous* run: true when this run
/// emits a preserved segment break before any content while a collapsible space
/// was pending from the previous run (§4.1.1 step 1, across the run boundary).
fn collapse_run_text(
    text: &str,
    white_space: WhiteSpace,
    prev_collapsible_space: &mut bool,
) -> (String, bool) {
    // HTML §13.2.3.5 preprocessing: normalize line endings before segment-break handling so a
    // bare CR or CRLF becomes the single canonical segment break (`\n`) for every
    // `white-space` value (otherwise a CR would be mishandled — e.g. preserved as a
    // forced break under pre-line). Matches `elidex-render`'s `normalize_line_endings`.
    // The common case has no CR, so only allocate when one is actually present.
    let text: std::borrow::Cow<str> = if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(text)
    };
    match white_space {
        // Preserve values: apart from the line-ending normalization applied above,
        // the text is preserved as-is (no space/tab collapsing, segment breaks kept
        // as forced breaks). A non-empty preserved run is rendered content, so it
        // resets the collapse state (a following collapsible run's leading space is a
        // fresh separator, not collapsed into the run).
        WhiteSpace::Pre | WhiteSpace::PreWrap => {
            if !text.is_empty() {
                *prev_collapsible_space = false;
            }
            (text.into_owned(), false)
        }
        WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine => {
            let preserve_break = white_space == WhiteSpace::PreLine;
            // Whether a collapsible space was pending from the previous run on entry
            // (needed to remove it across the run boundary before a leading break).
            let entry_prev_space = *prev_collapsible_space;
            let mut out = String::with_capacity(text.len());
            let mut trim_prev_trailing_space = false;
            for c in text.chars() {
                if c == '\n' && preserve_break {
                    // §4.1.1 step 1 / §4.1.3: collapsible spaces around a preserved
                    // segment break are removed.
                    if out.ends_with(' ') {
                        // The space is in this run's own output — drop it directly.
                        out.pop();
                    } else if out.is_empty() && entry_prev_space {
                        // The space immediately preceding this break was emitted at
                        // the end of the previous run; ask the caller to remove it.
                        trim_prev_trailing_space = true;
                    }
                    out.push('\n');
                    *prev_collapsible_space = true;
                } else if is_collapsible_space(c) || c == '\n' {
                    // A collapsible space/tab, or (for normal/nowrap) a segment break
                    // transformed to a space (§4.1.3). Collapse runs to a single
                    // space (step 4); a space following another collapsible space has
                    // zero advance width and is dropped from the string.
                    if !*prev_collapsible_space {
                        out.push(' ');
                        *prev_collapsible_space = true;
                    }
                } else {
                    out.push(c);
                    *prev_collapsible_space = false;
                }
            }
            (out, trim_prev_trailing_space)
        }
    }
}

/// CSS Text 3 collapsible space characters: space and tab. CR/CRLF are normalized
/// to the segment break `\n` upstream in [`collapse_run_text`], so they are not
/// treated here; the segment break itself is handled separately because its
/// transformation depends on `white-space` (§4.1.3).
pub(in crate::inline) fn is_collapsible_space(c: char) -> bool {
    matches!(c, ' ' | '\t')
}
