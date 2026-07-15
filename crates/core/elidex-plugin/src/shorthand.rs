//! Shared CSS shorthand serialization helpers.
//!
//! The collapse rules that [`CssPropertyHandler::serialize_shorthand`]
//! implementations reuse (CSSOM §6.7.2 "serialize a CSS value"). They live in this
//! base crate rather than `elidex-css` because handler crates are a *tier*, not a
//! chain: `elidex-css-flex` depends only on `elidex-plugin`, so helpers homed in
//! `elidex-css` would be unreachable from it.
//!
//! Two shapes, both fed the coordinator's already-validated longhands (all present,
//! uniform `!important` — CSSOM §6.6.1):
//! - the *structural* collapsers ([`serialize_rectangular`] / [`serialize_axis_pair`])
//!   take `(longhand-name, serialized-value)` pairs and collapse on component
//!   equality (no initials needed);
//! - the *omit-initial* collapser ([`serialize_omit_initial`]) instead takes
//!   `(serialized-value, serialized-initial)` pairs — it drops each component equal
//!   to its initial, so the handler pre-pairs each value with `initial_value(name)`
//!   (its own SoT) before the call.
//!
//! [`CssPropertyHandler::serialize_shorthand`]: crate::CssPropertyHandler::serialize_shorthand

/// Collapse a **rectangular** (1–4 value) shorthand — `margin` / `padding` /
/// `border-radius` — from its `[top, right, bottom, left]` longhands to the
/// shortest equivalent form.
///
/// (`border-radius` is top-left, top-right, bottom-right, bottom-left, which
/// collapses by the same rule for the common non-elliptical case.)
///
/// Returns `None` when the longhand count is not 4 — defensive only; the
/// coordinator supplies the canonical list.
#[must_use]
pub fn serialize_rectangular(longhands: &[(&str, &str)]) -> Option<String> {
    let [(_, top), (_, right), (_, bottom), (_, left)] = longhands else {
        return None;
    };
    Some(if top == right && right == bottom && bottom == left {
        (*top).to_string()
    } else if top == bottom && right == left {
        format!("{top} {right}")
    } else if right == left {
        format!("{top} {right} {bottom}")
    } else {
        format!("{top} {right} {bottom} {left}")
    })
}

/// Collapse an **axis-pair** (1–2 value) shorthand — `gap` (row/column),
/// `overflow` (x/y), `border-spacing` (h/v): one value when both axes are equal,
/// else two.
///
/// Returns `None` when the longhand count is not 2 — defensive only.
#[must_use]
pub fn serialize_axis_pair(longhands: &[(&str, &str)]) -> Option<String> {
    let [(_, first), (_, second)] = longhands else {
        return None;
    };
    Some(if first == second {
        (*first).to_string()
    } else {
        format!("{first} {second}")
    })
}

/// Collapse an omit-initial `||` shorthand from ordered
/// `(serialized-value, serialized-initial)` component pairs (CSSOM §6.7.2).
/// Third shared shorthand-collapse helper (with `serialize_rectangular` /
/// `serialize_axis_pair`) for the omit-initial families under slot
/// `#11-style-shorthand-expand`.
/// Omit each component equal to its initial; join survivors with " " in the
/// given (canonical) order. When ALL are initial, keep the FIRST component —
/// omitting all would yield "" (invalid / "less backwards-compatible",
/// §6.7.2 step 2). Verified vs Blink: `column-rule: medium none currentcolor`
/// ⇒ first = width; `columns: auto auto` ⇒ first = width.
#[must_use]
pub fn serialize_omit_initial(components: &[(&str, &str)]) -> Option<String> {
    if components.is_empty() {
        return None; // defensive; the coordinator always supplies the full set
    }
    let kept: Vec<&str> = components
        .iter()
        .filter(|(value, initial)| value != initial)
        .map(|(value, _)| *value)
        .collect();
    Some(if kept.is_empty() {
        components[0].0.to_string()
    } else {
        kept.join(" ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangular_collapses_to_shortest_form() {
        let all = [("t", "1px"), ("r", "1px"), ("b", "1px"), ("l", "1px")];
        assert_eq!(serialize_rectangular(&all).unwrap(), "1px");

        let vh = [("t", "1px"), ("r", "2px"), ("b", "1px"), ("l", "2px")];
        assert_eq!(serialize_rectangular(&vh).unwrap(), "1px 2px");

        let three = [("t", "1px"), ("r", "2px"), ("b", "3px"), ("l", "2px")];
        assert_eq!(serialize_rectangular(&three).unwrap(), "1px 2px 3px");

        let four = [("t", "1px"), ("r", "2px"), ("b", "3px"), ("l", "4px")];
        assert_eq!(serialize_rectangular(&four).unwrap(), "1px 2px 3px 4px");
    }

    #[test]
    fn axis_pair_collapses_when_equal() {
        let same = [("row", "10px"), ("col", "10px")];
        assert_eq!(serialize_axis_pair(&same).unwrap(), "10px");

        let diff = [("row", "10px"), ("col", "20px")];
        assert_eq!(serialize_axis_pair(&diff).unwrap(), "10px 20px");
    }

    #[test]
    fn wrong_arity_is_none() {
        assert!(serialize_rectangular(&[("t", "1px")]).is_none());
        assert!(serialize_axis_pair(&[("a", "1px"), ("b", "2px"), ("c", "3px")]).is_none());
    }

    #[test]
    fn omit_initial_all_initial_keeps_first() {
        // Every component equals its initial ⇒ omit-all would yield "" (invalid);
        // keep the FIRST canonical component instead (`column-rule` width).
        let all_initial = [
            ("3px", "3px"),
            ("none", "none"),
            ("currentcolor", "currentcolor"),
        ];
        assert_eq!(serialize_omit_initial(&all_initial).unwrap(), "3px");
    }

    #[test]
    fn omit_initial_middle_gap_preserves_order() {
        // width + color non-initial, style = initial (middle gap): drop the
        // middle, keep survivors in the given (canonical) order — no re-sort.
        let gap = [
            ("5px", "3px"),
            ("none", "none"),
            ("#0000ff", "currentcolor"),
        ];
        assert_eq!(serialize_omit_initial(&gap).unwrap(), "5px #0000ff");
    }

    #[test]
    fn omit_initial_single_non_initial() {
        // Only the style differs from its initial ⇒ that lone survivor.
        let one = [
            ("3px", "3px"),
            ("solid", "none"),
            ("currentcolor", "currentcolor"),
        ];
        assert_eq!(serialize_omit_initial(&one).unwrap(), "solid");
    }

    #[test]
    fn omit_initial_all_non_initial_full_join() {
        // Nothing is initial ⇒ full join in the given order.
        let all = [
            ("5px", "3px"),
            ("solid", "none"),
            ("#ff0000", "currentcolor"),
        ];
        assert_eq!(serialize_omit_initial(&all).unwrap(), "5px solid #ff0000");
    }

    #[test]
    fn omit_initial_two_component_all_initial() {
        // `columns: auto auto` ⇒ both initial ⇒ keep first ⇒ single `auto`.
        let columns = [("auto", "auto"), ("auto", "auto")];
        assert_eq!(serialize_omit_initial(&columns).unwrap(), "auto");
    }

    #[test]
    fn omit_initial_empty_is_none() {
        assert!(serialize_omit_initial(&[]).is_none());
    }
}
