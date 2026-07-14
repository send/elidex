//! Shared CSS shorthand serialization helpers.
//!
//! The structural collapse rules that [`CssPropertyHandler::serialize_shorthand`]
//! implementations reuse (CSSOM §6.7.2 "serialize a CSS value"). They live in this
//! base crate rather than `elidex-css` because handler crates are a *tier*, not a
//! chain: `elidex-css-flex` depends only on `elidex-plugin`, so helpers homed in
//! `elidex-css` would be unreachable from it.
//!
//! Each helper takes the ordered `(longhand-name, serialized-value)` pairs the
//! coordinator has already validated (all present, uniform `!important` — CSSOM
//! §6.6.1) and returns the collapsed shorthand string.
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
}
