//! Shorthand value reconstruction (CSSOM §6.6.1 `getPropertyValue` step
//! 1.2 — "serialize a CSS value" for a shorthand property).
//!
//! The CSSOM declaration block stores **longhands** (`setProperty` /
//! `parse_declaration_block` expand shorthands per §6.6.1 step 8). A
//! shorthand getter must reconstruct the shorthand string from those
//! longhands. This is the single canonical reconstruction used by BOTH
//! the inline `el.style.getPropertyValue` path (`elidex-dom-api`) and the
//! rule `cssRule.style.getPropertyValue` path — neither could read a
//! shorthand back before, since both store/parse expanded longhands.
//!
//! Coverage: the structurally-unambiguous shorthands — the rectangular
//! (1–4 value: `margin`, `padding`, `border-radius`) and axis-pair (1–2
//! value: `gap`, `overflow`, `border-spacing`) families — are serialized
//! exactly. The intricate omit-initial ordered families (`border`,
//! `flex`, `text-decoration`, `column-rule`, `list-style`, `columns`,
//! `flex-flow`) and the layered / grid-grammar families (`font`,
//! `background`, `grid`, `grid-template`, `grid-*`) return `None` →
//! `getPropertyValue` yields `""` (a CSSOM-valid "cannot serialize"
//! result). Their serialization is coverage-completeness on this seam,
//! tracked by slot `#11-style-shorthand-expand`.

use crate::declaration::shorthand_longhands;

/// Reconstruct the CSS shorthand value for `property` from its longhand
/// declarations, looked up via `get` (longhand name → `(serialized
/// value, important flag)`).
///
/// Returns `None` when `property` is not a shorthand, when any mapped
/// longhand is absent, when the mapped longhands do **not** all share the
/// same `!important` flag, or when the shorthand is not yet covered by
/// the reconstruction (the caller treats `None` as the empty string —
/// CSSOM-valid). The longhand values are the canonical serialized forms
/// already stored in the declaration block.
#[must_use]
pub fn serialize_shorthand_value(
    property: &str,
    get: impl Fn(&str) -> Option<(String, bool)>,
) -> Option<String> {
    let longhands = shorthand_longhands(property);
    if longhands.is_empty() {
        return None; // not a shorthand — caller reads the property directly
    }
    // All mapped longhands must be present (CSSOM §6.6.1 getPropertyValue
    // step 1.2.2.2: a missing longhand makes the shorthand
    // non-serializable → "").
    let decls: Vec<(String, bool)> = longhands
        .iter()
        .map(|lh| get(lh))
        .collect::<Option<Vec<_>>>()?;

    // CSSOM §6.6.1 getPropertyValue step 1.2.3/1.2.4: the shorthand
    // serializes only when the important flags of all longhand
    // declarations in the list are the *same* (all-important OR
    // all-normal); a mixed block yields "". Note this is uniformity, not
    // "all important" — `getPropertyPriority` checks all-important (it
    // reports the shared priority); the value getter checks all-equal.
    let first_important = decls[0].1;
    if !decls
        .iter()
        .all(|(_, important)| *important == first_important)
    {
        return None;
    }
    let values: Vec<String> = decls.into_iter().map(|(value, _)| value).collect();

    match property {
        // Rectangular 1–4 value families: top, right, bottom, left
        // (border-radius is top-left, top-right, bottom-right,
        // bottom-left, which collapses by the same rule for the common
        // non-elliptical case).
        "margin" | "padding" | "border-radius" => Some(serialize_rectangular(&values)),
        // Axis-pair 1–2 value families: first second (collapse when
        // equal). `border-spacing` h/v, `gap` row/column, `overflow`
        // x/y.
        "gap" | "overflow" | "border-spacing" => Some(serialize_axis_pair(&values)),
        // Intricate / layered / grid shorthands: not yet covered on this
        // seam (slot `#11-style-shorthand-expand`). `None` ⇒ `""`.
        _ => None,
    }
}

/// CSS rectangular shorthand serialization (`margin`/`padding`/
/// `border-radius`): collapse `[t, r, b, l]` to the shortest equivalent
/// 1–4 value form.
fn serialize_rectangular(v: &[String]) -> String {
    debug_assert_eq!(v.len(), 4, "rectangular shorthand has 4 longhands");
    let (top, right, bottom, left) = (&v[0], &v[1], &v[2], &v[3]);
    if top == right && right == bottom && bottom == left {
        top.clone()
    } else if top == bottom && right == left {
        format!("{top} {right}")
    } else if right == left {
        format!("{top} {right} {bottom}")
    } else {
        format!("{top} {right} {bottom} {left}")
    }
}

/// CSS axis-pair shorthand serialization (`gap`/`overflow`/
/// `border-spacing`): one value when both axes are equal, else two.
fn serialize_axis_pair(v: &[String]) -> String {
    debug_assert_eq!(v.len(), 2, "axis-pair shorthand has 2 longhands");
    let (first, second) = (&v[0], &v[1]);
    if first == second {
        first.clone()
    } else {
        format!("{first} {second}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// All longhands present with normal (non-important) priority.
    fn lookup<'a>(map: &'a HashMap<&str, &str>) -> impl Fn(&str) -> Option<(String, bool)> + 'a {
        move |name: &str| map.get(name).map(|s| ((*s).to_string(), false))
    }

    #[test]
    fn rectangular_collapse_forms() {
        let mut m = HashMap::new();
        m.insert("margin-top", "10px");
        m.insert("margin-right", "10px");
        m.insert("margin-bottom", "10px");
        m.insert("margin-left", "10px");
        assert_eq!(
            serialize_shorthand_value("margin", lookup(&m)),
            Some("10px".to_string())
        );

        m.insert("margin-bottom", "20px");
        // t=10 r=10 b=20 l=10 → r==l, so "t r b" = "10px 10px 20px"
        assert_eq!(
            serialize_shorthand_value("margin", lookup(&m)),
            Some("10px 10px 20px".to_string())
        );

        m.insert("margin-left", "30px");
        // t=10 r=10 b=20 l=30 → all-four form
        assert_eq!(
            serialize_shorthand_value("margin", lookup(&m)),
            Some("10px 10px 20px 30px".to_string())
        );

        m.insert("margin-bottom", "10px");
        m.insert("margin-left", "10px");
        m.insert("margin-right", "20px");
        // t=10 r=20 b=10 l=10 → t==b && r==l? r=20 l=10 no; r==l? no → four
        assert_eq!(
            serialize_shorthand_value("margin", lookup(&m)),
            Some("10px 20px 10px 10px".to_string())
        );
    }

    #[test]
    fn axis_pair_collapse() {
        let mut m = HashMap::new();
        m.insert("row-gap", "4px");
        m.insert("column-gap", "4px");
        assert_eq!(
            serialize_shorthand_value("gap", lookup(&m)),
            Some("4px".to_string())
        );
        m.insert("column-gap", "8px");
        assert_eq!(
            serialize_shorthand_value("gap", lookup(&m)),
            Some("4px 8px".to_string())
        );
    }

    #[test]
    fn missing_longhand_is_none() {
        let mut m = HashMap::new();
        m.insert("margin-top", "10px");
        // only one of four present
        assert_eq!(serialize_shorthand_value("margin", lookup(&m)), None);
    }

    #[test]
    fn non_shorthand_is_none() {
        let m = HashMap::new();
        assert_eq!(serialize_shorthand_value("color", lookup(&m)), None);
    }

    #[test]
    fn uncovered_shorthand_is_none_even_when_complete() {
        // `flex` is mapped and all longhands present, but the
        // omit-initial ordered families are not yet covered on this seam
        // → None (CSSOM-valid "cannot serialize"), tracked by
        // `#11-style-shorthand-expand`.
        let owned: Vec<(String, &str)> = shorthand_longhands("flex")
            .into_iter()
            .map(|lh| (lh, "0"))
            .collect();
        let get = |name: &str| {
            owned
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| ((*v).to_string(), false))
        };
        assert_eq!(serialize_shorthand_value("flex", get), None);
    }

    #[test]
    fn mixed_importance_is_none() {
        // CSSOM §6.6.1 getPropertyValue step 1.2.3/1.2.4: a shorthand
        // whose longhands carry *different* important flags is not
        // serializable → "". Here `margin-top` is `!important` while the
        // other three are normal.
        let get = |name: &str| {
            let important = name == "margin-top";
            match name {
                "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
                    Some(("10px".to_string(), important))
                }
                _ => None,
            }
        };
        assert_eq!(serialize_shorthand_value("margin", get), None);
    }

    #[test]
    fn uniform_importance_serializes() {
        // All longhands `!important` (uniform) → serializes normally; the
        // step-1.2.3 check is uniformity, not absence-of-importance.
        let get = |name: &str| match name {
            "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
                Some(("10px".to_string(), true))
            }
            _ => None,
        };
        assert_eq!(
            serialize_shorthand_value("margin", get),
            Some("10px".to_string())
        );
    }
}
