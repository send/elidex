//! Margin resolution and collapsing logic.

use elidex_plugin::{Dimension, Direction};

use crate::{resolve_dimension_value, sanitize};

/// Resolve a `Dimension` margin value to pixels.
///
/// `Auto` returns 0.0 here; horizontal auto centering is handled separately.
/// Non-finite results are replaced with 0.0. Margins may be negative (unlike
/// padding/border), so `sanitize()` is used instead of clamping to non-negative.
pub fn resolve_margin(dim: Dimension, containing_width: f32) -> f32 {
    sanitize(resolve_dimension_value(dim, containing_width, 0.0))
}

/// Apply inline-axis `margin: auto` centering (CSS 2.1 §10.3.3).
///
/// Generalized for writing modes: accepts inline-start and inline-end margin
/// dimensions directly, along with the inline-axis containing size and used size.
///
/// `used_inline` = `content_inline` + inline padding + inline border (already sanitized).
/// When the box is overconstrained:
/// - Normal direction: inline-end margin is recalculated.
/// - Reversed direction: inline-start margin is recalculated.
///
/// Returns `(margin_inline_start, margin_inline_end)`.
pub(super) fn apply_margin_auto_centering(
    margin_start_dim: Dimension,
    margin_end_dim: Dimension,
    containing_inline: f32,
    used_inline: f32,
    direction: Direction,
) -> (f32, f32) {
    let remaining = containing_inline - used_inline;
    let start_auto = matches!(margin_start_dim, Dimension::Auto);
    let end_auto = matches!(margin_end_dim, Dimension::Auto);

    match (start_auto, end_auto) {
        (true, true) => {
            if remaining >= 0.0 {
                (remaining / 2.0, remaining / 2.0)
            } else {
                // Both auto, overconstrained: start-side margin = 0,
                // end-side margin absorbs overflow.
                match direction {
                    Direction::Ltr => (0.0, remaining),
                    Direction::Rtl => (remaining, 0.0),
                }
            }
        }
        (true, false) => {
            let me = resolve_margin(margin_end_dim, containing_inline);
            (remaining - me, me)
        }
        (false, true) => {
            let ms = resolve_margin(margin_start_dim, containing_inline);
            (ms, remaining - ms)
        }
        (false, false) => {
            // CSS 2.1 §10.3.3: no margins auto, over-constrained.
            // Normal: end margin recalculated. Reversed: start margin recalculated.
            match direction {
                Direction::Ltr => {
                    let ms = resolve_margin(margin_start_dim, containing_inline);
                    (ms, containing_inline - used_inline - ms)
                }
                Direction::Rtl => {
                    let me = resolve_margin(margin_end_dim, containing_inline);
                    (containing_inline - used_inline - me, me)
                }
            }
        }
    }
}

/// Collapse two adjacent margins per CSS 2.1 §8.3.1.
///
/// - Both positive: the larger wins.
/// - Both negative: the more negative (smaller) wins.
/// - Mixed: they are summed.
pub(super) fn collapse_margins(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}
