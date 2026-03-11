//! Margin resolution and collapsing logic.

use elidex_plugin::{ComputedStyle, Dimension, Direction};

use crate::{resolve_dimension_value, sanitize};

/// Resolve a `Dimension` margin value to pixels.
///
/// `Auto` returns 0.0 here; horizontal auto centering is handled separately.
/// Non-finite results are replaced with 0.0. Margins may be negative (unlike
/// padding/border), so `sanitize()` is used instead of clamping to non-negative.
pub fn resolve_margin(dim: Dimension, containing_width: f32) -> f32 {
    sanitize(resolve_dimension_value(dim, containing_width, 0.0))
}

/// Apply horizontal `margin: auto` centering (CSS 2.1 §10.3.3).
///
/// `used_horizontal` = `content_width` + padding + border (already sanitized).
/// When the box is overconstrained:
/// - LTR: `margin-right` is recalculated to satisfy the constraint.
/// - RTL: `margin-left` is recalculated to satisfy the constraint.
pub(super) fn apply_margin_auto_centering(
    style: &ComputedStyle,
    containing_width: f32,
    used_horizontal: f32,
    direction: Direction,
) -> (f32, f32) {
    let remaining = containing_width - used_horizontal;
    let left_auto = matches!(style.margin_left, Dimension::Auto);
    let right_auto = matches!(style.margin_right, Dimension::Auto);

    match (left_auto, right_auto) {
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
            let mr = resolve_margin(style.margin_right, containing_width);
            (remaining - mr, mr)
        }
        (false, true) => {
            let ml = resolve_margin(style.margin_left, containing_width);
            (ml, remaining - ml)
        }
        (false, false) => {
            // CSS 2.1 §10.3.3: no margins auto, over-constrained.
            // LTR: margin-right recalculated. RTL: margin-left recalculated.
            match direction {
                Direction::Ltr => {
                    let ml = resolve_margin(style.margin_left, containing_width);
                    (ml, containing_width - used_horizontal - ml)
                }
                Direction::Rtl => {
                    let mr = resolve_margin(style.margin_right, containing_width);
                    (containing_width - used_horizontal - mr, mr)
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
