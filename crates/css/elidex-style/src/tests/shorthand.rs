//! `serialize_shorthand_value` — the CSSOM §6.6.1 shorthand-reconstruction
//! coordinator: property-agnostic validation here, per-family collapse dispatched
//! to the owning `CssPropertyHandler::serialize_shorthand`.
//!
//! Behavior-preserving port of the coverage the old `elidex-css` seam carried.

use crate::{default_css_property_registry, serialize_shorthand_value};
use std::collections::HashMap;

/// All longhands present with normal (non-important) priority.
fn lookup<'a>(map: &'a HashMap<&str, &str>) -> impl Fn(&str) -> Option<(String, bool)> + 'a {
    move |name: &str| map.get(name).map(|s| ((*s).to_string(), false))
}

fn serialize(property: &str, map: &HashMap<&str, &str>) -> Option<String> {
    serialize_shorthand_value(default_css_property_registry(), property, lookup(map))
}

// --- rectangular family (BoxHandler: margin / padding / border-radius) ---

#[test]
fn rectangular_collapse_forms() {
    let mut m = HashMap::new();
    m.insert("margin-top", "10px");
    m.insert("margin-right", "10px");
    m.insert("margin-bottom", "10px");
    m.insert("margin-left", "10px");
    assert_eq!(serialize("margin", &m), Some("10px".to_string()));

    // t=10 r=10 b=20 l=10 → r==l ⇒ three-value form.
    m.insert("margin-bottom", "20px");
    assert_eq!(serialize("margin", &m), Some("10px 10px 20px".to_string()));

    // t=10 r=10 b=20 l=30 → all-four form.
    m.insert("margin-left", "30px");
    assert_eq!(
        serialize("margin", &m),
        Some("10px 10px 20px 30px".to_string())
    );

    // t=10 r=20 b=10 l=10 → neither pair matches ⇒ all-four form.
    m.insert("margin-bottom", "10px");
    m.insert("margin-left", "10px");
    m.insert("margin-right", "20px");
    assert_eq!(
        serialize("margin", &m),
        Some("10px 20px 10px 10px".to_string())
    );
}

#[test]
fn rectangular_padding_and_border_radius() {
    let mut m = HashMap::new();
    m.insert("padding-top", "5px");
    m.insert("padding-right", "5px");
    m.insert("padding-bottom", "5px");
    m.insert("padding-left", "5px");
    assert_eq!(serialize("padding", &m), Some("5px".to_string()));

    let mut r = HashMap::new();
    r.insert("border-top-left-radius", "2px");
    r.insert("border-top-right-radius", "2px");
    r.insert("border-bottom-right-radius", "4px");
    r.insert("border-bottom-left-radius", "2px");
    // tl=2 tr=2 br=4 bl=2 → tr==bl ⇒ three-value form.
    assert_eq!(
        serialize("border-radius", &r),
        Some("2px 2px 4px".to_string())
    );
}

// --- axis-pair family (BoxHandler: gap / overflow; TableHandler: border-spacing) ---

#[test]
fn axis_pair_collapse() {
    let mut m = HashMap::new();
    m.insert("row-gap", "4px");
    m.insert("column-gap", "4px");
    assert_eq!(serialize("gap", &m), Some("4px".to_string()));

    m.insert("column-gap", "8px");
    assert_eq!(serialize("gap", &m), Some("4px 8px".to_string()));
}

#[test]
fn axis_pair_overflow_and_border_spacing() {
    let mut o = HashMap::new();
    o.insert("overflow-x", "hidden");
    o.insert("overflow-y", "hidden");
    assert_eq!(serialize("overflow", &o), Some("hidden".to_string()));
    o.insert("overflow-y", "scroll");
    assert_eq!(serialize("overflow", &o), Some("hidden scroll".to_string()));

    // border-spacing is owned by a DIFFERENT handler (TableHandler) — proves the
    // registry dispatch reaches each family's owner, not one central table.
    let mut b = HashMap::new();
    b.insert("border-spacing-h", "2px");
    b.insert("border-spacing-v", "2px");
    assert_eq!(serialize("border-spacing", &b), Some("2px".to_string()));
    b.insert("border-spacing-v", "3px");
    assert_eq!(serialize("border-spacing", &b), Some("2px 3px".to_string()));
}

// --- CSSOM §6.6.1 property-agnostic validation (owned by the coordinator) ---

#[test]
fn missing_longhand_is_none() {
    let mut m = HashMap::new();
    m.insert("margin-top", "10px"); // only 1 of 4 present
    assert_eq!(serialize("margin", &m), None);
}

#[test]
fn non_shorthand_is_none() {
    let m = HashMap::new();
    assert_eq!(serialize("color", &m), None);
}

#[test]
fn mixed_important_is_none() {
    // §6.6.1: the shorthand serializes only when the longhands' important flags
    // are uniform; a mixed block yields "" (None here).
    let get = |name: &str| match name {
        "row-gap" => Some(("4px".to_string(), true)),
        "column-gap" => Some(("4px".to_string(), false)),
        _ => None,
    };
    assert_eq!(
        serialize_shorthand_value(default_css_property_registry(), "gap", get),
        None
    );

    // Uniformly important → serializes.
    let all_important = |name: &str| match name {
        "row-gap" | "column-gap" => Some(("4px".to_string(), true)),
        _ => None,
    };
    assert_eq!(
        serialize_shorthand_value(default_css_property_registry(), "gap", all_important),
        Some("4px".to_string())
    );
}

#[test]
fn uncovered_shorthand_is_none_even_when_complete() {
    // `flex` is a mapped shorthand and all longhands are present, but the
    // omit-initial ordered families are not yet covered by their handler's
    // `serialize_shorthand` (follow-up PRs under `#11-style-shorthand-expand`) —
    // the handler returns None, which the caller maps to "" (a CSSOM-valid
    // "cannot serialize" result).
    let mut m = HashMap::new();
    m.insert("flex-grow", "1");
    m.insert("flex-shrink", "1");
    m.insert("flex-basis", "0%");
    assert_eq!(serialize("flex", &m), None);
}
