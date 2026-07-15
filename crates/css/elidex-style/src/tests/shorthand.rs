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

// --- omit-initial family (MulticolHandler: column-rule / columns) ---
//
// GENUINE parse→serialize round-trips: the author declaration is expanded by
// elidex's real parser, each longhand serialized via `to_css_string`, and the
// coordinator reconstructs the shorthand. This exercises the R2 keyword/color
// serialization divergence *for real* (thick→5px, blue→#0000ff, …) — a literal
// string map would only test the collapse, never elidex's own serialization.

/// Parse `author` through elidex, then reconstruct `shorthand` via the coordinator.
fn roundtrip(author: &str, shorthand: &str) -> Option<String> {
    let stored: HashMap<String, String> = elidex_css::parse_declaration_block(author)
        .into_iter()
        .map(|d| (d.property, d.value.to_css_string()))
        .collect();
    serialize_shorthand_value(default_css_property_registry(), shorthand, |name| {
        stored.get(name).map(|s| (s.clone(), false))
    })
}

#[test]
fn column_rule_roundtrip_corners() {
    // Corner 1: only style non-initial.
    assert_eq!(
        roundtrip("column-rule: solid", "column-rule"),
        Some("solid".to_string())
    );
    // Corner 2: width+color kept, style=initial in the MIDDLE (thick→5px, blue→#0000ff — R2).
    assert_eq!(
        roundtrip("column-rule: thick blue", "column-rule"),
        Some("5px #0000ff".to_string())
    );
    // Corner 2b: leading width omitted, style+color survive (green→#008000 — R2).
    assert_eq!(
        roundtrip("column-rule: dashed green", "column-rule"),
        Some("dashed #008000".to_string())
    );
    // Corner 3: ALL initial ⇒ keep first (width). `3px` vs Blink's `medium` (R2).
    assert_eq!(
        roundtrip("column-rule: medium none currentcolor", "column-rule"),
        Some("3px".to_string())
    );
    // Corner 3b: all non-initial (thick→5px, red→#ff0000 — R2).
    assert_eq!(
        roundtrip("column-rule: thick solid red", "column-rule"),
        Some("5px solid #ff0000".to_string())
    );
}

#[test]
fn columns_roundtrip_corners() {
    // Corner 4: all initial (both auto) ⇒ single `auto`, both author forms.
    assert_eq!(
        roundtrip("columns: auto", "columns"),
        Some("auto".to_string())
    );
    assert_eq!(
        roundtrip("columns: auto auto", "columns"),
        Some("auto".to_string())
    );
    // Corner 5: width only (count=initial auto dropped).
    assert_eq!(
        roundtrip("columns: 200px", "columns"),
        Some("200px".to_string())
    );
    // Corner 5b: count only (width=initial auto dropped); Number(3.0)→"3" (R3).
    assert_eq!(roundtrip("columns: 3", "columns"), Some("3".to_string()));
    assert_eq!(
        roundtrip("columns: 3 auto", "columns"),
        Some("3".to_string())
    );
    // Corner 5c: both kept, canonical order width→count.
    assert_eq!(
        roundtrip("columns: 200px 3", "columns"),
        Some("200px 3".to_string())
    );
}

#[test]
fn column_rule_mixed_important_is_none() {
    // §6.6.1: mixed priority on the longhands ⇒ the coordinator rejects the
    // block before dispatch (the handler is never reached → None → "").
    let mixed = |name: &str| match name {
        "column-rule-width" => Some(("5px".to_string(), true)),
        "column-rule-style" => Some(("none".to_string(), false)),
        "column-rule-color" => Some(("#0000ff".to_string(), false)),
        _ => None,
    };
    assert_eq!(
        serialize_shorthand_value(default_css_property_registry(), "column-rule", mixed),
        None
    );

    // Uniformly important → serializes (keep style, drop the initial width+color).
    let all_important = |name: &str| match name {
        "column-rule-width" => Some(("3px".to_string(), true)),
        "column-rule-style" => Some(("solid".to_string(), true)),
        "column-rule-color" => Some(("currentcolor".to_string(), true)),
        _ => None,
    };
    assert_eq!(
        serialize_shorthand_value(
            default_css_property_registry(),
            "column-rule",
            all_important
        ),
        Some("solid".to_string())
    );
}
