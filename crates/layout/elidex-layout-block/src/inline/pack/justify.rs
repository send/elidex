//! Inline alignment & `text-align: justify` math for [`LinePacker`](super::LinePacker):
//! `text-align` resolution (`resolve_align`/`align_offset`), justification opportunity
//! counting (`separators`/`justify_opportunity_counts`), the between-run expansion bake
//! (`bake_justify`), and the [`FlushReason`] that gates last-line / forced-break
//! justification. Split out of `pack.rs` to keep the line packer under the repo's
//! ~1000-line convention; all items are `pub(super)` for `flush_line` in the parent
//! module.

use elidex_ecs::InlineFlowRun;
use elidex_plugin::{Direction, TextAlign};
use elidex_text::is_word_separator;

/// Why a line was flushed — drives the `text-align: justify` last-line / forced-break
/// suppression (CSS Text 3 §6.3 `text-align-last: auto` → start on the block's last
/// line; §6.1 the last line before a forced break is start-aligned). The reason is
/// knowable at every `flush_line` call site (a soft-wrap/overflow flush, a forced
/// `<br>`/segment-break flush, or the final `finish` flush), so justification is gated
/// here rather than re-derived post-pack.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FlushReason {
    /// A soft-wrap or overflow break mid-paragraph — the only justifiable line kind.
    SoftWrap,
    /// A forced break (`<br>` or a preserved segment break) ended this line — §6.1
    /// start-aligns it (not justified).
    Forced,
    /// The block's final line (`finish`) — §6.3 `text-align-last: auto` start-aligns it.
    LastLine,
}

/// Resolve `text-align: start/end` to the corresponding edge for the given inline
/// base direction (`Left`/`Right` here name the inline-start/inline-end edge — for
/// vertical writing modes these are the block-flow-relative inline edges, matching
/// render's `compute_vertical_text_align_offset`).
pub(super) fn resolve_align(align: TextAlign, direction: Direction) -> TextAlign {
    match align {
        TextAlign::Start => match direction {
            Direction::Ltr => TextAlign::Left,
            Direction::Rtl => TextAlign::Right,
        },
        TextAlign::End => match direction {
            Direction::Ltr => TextAlign::Right,
            Direction::Rtl => TextAlign::Left,
        },
        other => other,
    }
}

/// Inline-start offset for a resolved alignment given the line's free space
/// (`container_inline_size − trimmed_line_width`). A *distributed* `justify` line
/// reaches here as `Justify` and falls to `0.0` (it fills the box from the start
/// edge; `bake_justify` does the distribution); a *suppressed* justify line is
/// resolved to `Start` by the caller first (§6.3), so it never reaches here as raw
/// `Justify` with non-zero intended offset.
pub(super) fn align_offset(resolved: TextAlign, free: f32) -> f32 {
    match resolved {
        TextAlign::Center => free / 2.0,
        TextAlign::Right | TextAlign::End => free,
        _ => 0.0,
    }
}

/// Count `text-align: justify` opportunities in `text` — every `is_word_separator`
/// cluster (CSS Text 3 §6.4 inter-word justification).
fn separators(text: &str) -> usize {
    text.chars().filter(|c| is_word_separator(*c)).count()
}

/// Per-run `text-align: justify` opportunity counts for a line's top-level group.
/// Every interior word-separator is an opportunity, but the **trailing collapsible
/// whitespace at the line's end hangs** (CSS Text 3 §4.1.2) and is excluded — so a
/// soft-wrapped line ending in a space distributes free space only between the visible
/// words, filling them to the line-box edge (counting it would assign `extra` to the
/// hung space and under-fill the visible content). A no-break space (U+00A0) never
/// hangs, so a trailing nbsp stays an opportunity. `AtomicBox` members contribute 0.
///
/// The trailing hang is trimmed only when BOTH (a) `group_is_line_last` — nothing (a
/// sub-flow run or positioned atomic) sits inline-after this group on the line — AND
/// (b) the group's own last member is a text run. If the line ends with an `AtomicBox`,
/// or a sub-flow follows the top-level group (`text ␠ <span rel>…</span>`), the
/// preceding text run's trailing space sits BETWEEN inline-level content — a genuine
/// opportunity, not a line-end hang — so it is kept (trimming it would under-count,
/// under-shift the following content via `cum_at`, and re-introduce overlap).
pub(super) fn justify_opportunity_counts(
    runs: &[InlineFlowRun],
    group_is_line_last: bool,
) -> Vec<usize> {
    let last_idx = runs.len().checked_sub(1);
    let trim_last = group_is_line_last && matches!(runs.last(), Some(InlineFlowRun::Text { .. }));
    runs.iter()
        .enumerate()
        .map(|(i, r)| match r {
            InlineFlowRun::Text { text, .. } if trim_last && Some(i) == last_idx => {
                separators(text.trim_end_matches(super::super::is_collapsible_space))
            }
            InlineFlowRun::Text { text, .. } => separators(text),
            InlineFlowRun::AtomicBox { .. } => 0,
        })
        .collect()
}

/// Bake the *between-run* `text-align: justify` expansion into the top-level group's
/// `runs` (CSS Text 3 §6.4): each run's `inline_start` advances by the accumulated
/// expansion of all preceding word-separators (`extra` per opportunity in `per_run`,
/// summed in inline/placement order). `per_run` has the trailing hang already excluded;
/// `extra = free / Σ per_run` is computed by the caller (which feeds the SAME `extra` to
/// the cross-flow `cum_at` shift, so sub-flow groups + positioned atomics ride the same
/// expansion and never overlap the justified text). Render's `place_glyphs` adds `extra`
/// *within* each run at every `is_word_separator` cluster; the two are disjoint (no
/// double-count). An `AtomicBox` contributes 0 opportunities but its `inline_start` still
/// rides the accumulated `cum`.
pub(super) fn bake_justify(runs: &mut [InlineFlowRun], extra: f32, per_run: &[usize]) {
    let mut cum = 0.0_f32;
    for (r, &seps) in runs.iter_mut().zip(per_run) {
        *r.inline_start_mut() += cum;
        #[allow(clippy::cast_precision_loss)]
        let seps = seps as f32;
        cum += extra * seps;
    }
}

#[cfg(test)]
mod justify_opportunity_tests {
    use super::*;
    use elidex_ecs::{EcsDom, Entity};

    // A throwaway entity to stamp on synthetic runs (its value is irrelevant —
    // `justify_opportunity_counts` matches on the variant + reads `text` only).
    fn entity() -> Entity {
        EcsDom::new().create_text("x")
    }

    fn text(s: &str) -> InlineFlowRun {
        InlineFlowRun::Text {
            entity: entity(),
            text: s.to_string(),
            inline_start: 0.0,
        }
    }

    fn atomic() -> InlineFlowRun {
        InlineFlowRun::AtomicBox {
            entity: entity(),
            inline_start: 0.0,
        }
    }

    #[test]
    fn trailing_text_hang_is_excluded() {
        // Last member is a text run ending in a collapsible space → that space hangs
        // (§4.1.2) and is NOT an opportunity; the interior "aa␠bb" gap is.
        assert_eq!(
            justify_opportunity_counts(&[text("aa "), text("bb ")], true),
            vec![1, 0]
        );
    }

    #[test]
    fn gap_before_trailing_atomic_is_kept() {
        // Last member is an AtomicBox → the preceding text run's trailing space sits
        // BETWEEN inline-level content (a real opportunity), so it is NOT trimmed.
        // (The bug being guarded: trimming the last *text* run unconditionally would
        // yield `[0, 0]` here and mis-classify the line as unexpandable.)
        assert_eq!(
            justify_opportunity_counts(&[text("aa "), atomic()], true),
            vec![1, 0]
        );
    }

    #[test]
    fn trailing_space_kept_when_subflow_follows() {
        // group_is_line_last == false (a sub-flow / atomic sits inline-after this group)
        // → the top-level group's last text run's trailing space is interior, NOT a hang,
        // so it stays an opportunity (else the following sub-flow is under-shifted by
        // cum_at and the justified text overlaps it).
        assert_eq!(
            justify_opportunity_counts(&[text("aa "), text("bb ")], false),
            vec![1, 1]
        );
    }

    #[test]
    fn trailing_nbsp_is_not_trimmed() {
        // A no-break space (U+00A0) never hangs, so a trailing nbsp stays an opportunity.
        assert_eq!(
            justify_opportunity_counts(&[text("aa\u{00A0}")], true),
            vec![1]
        );
    }

    #[test]
    fn interior_atomic_keeps_both_text_gaps() {
        // text "a ", atomic, text "b " (last) → "a " gap kept, "b " trailing hang trimmed.
        assert_eq!(
            justify_opportunity_counts(&[text("a "), atomic(), text("b ")], true),
            vec![1, 0, 0]
        );
    }
}
