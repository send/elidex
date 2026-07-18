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

/// REGRESSION (Codex PR#473 R2) — the value-KIND gate must NOT fire for a
/// shorthand the coordinator does not serialize. Handler coverage is established
/// FIRST (dispatch returns `None`); the gate then only OVERRIDES a *covered*
/// handler's collapse. Without this ordering the gate emits a value for an
/// UNCOVERED shorthand and preempts the caller's fallback to a *direct* shorthand
/// declaration (a later whole-shorthand `var()` stored under the shorthand name),
/// which wins by order of appearance (css-cascade-4 §6.1) and is read back via the
/// caller's `.or_else` getter (outside the §6.6.1 shorthand-reconstruction path):
///   `background: initial; background: var(--bg)`
///   → getPropertyValue("background") == "var(--bg)" (Blink 148), NOT "initial".
///
/// `background` (and `flex`) is mapped but uncovered — every longhand set to the
/// same CSS-wide keyword, or an unresolved `var()`, must still yield `None`, not
/// `Some("initial")` / `Some("")`. (`column-rule`/`columns` became COVERED in PR1,
/// so they now serialize via the gate — see
/// `multicol_csswide_and_var_collapse_via_gate`.)
#[test]
fn uncovered_shorthand_with_nonphysical_longhands_is_none() {
    const BG: [&str; 8] = [
        "background-color",
        "background-image",
        "background-position",
        "background-size",
        "background-repeat",
        "background-origin",
        "background-clip",
        "background-attachment",
    ];

    // all-same CSS-wide keyword — a COVERED shorthand collapses to Some("initial")
    // via the gate; uncovered `background` must stay None (else it preempts the
    // caller's direct-declaration fallback).
    let mut all_initial = HashMap::new();
    for lh in BG {
        all_initial.insert(lh, "initial");
    }
    assert_eq!(serialize("background", &all_initial), None);

    // an unresolved var() component — Some("") for a covered shorthand; None here.
    let mut with_var = HashMap::new();
    for lh in BG {
        with_var.insert(lh, "var(--bg)");
    }
    assert_eq!(serialize("background", &with_var), None);
}

// --- CSSOM §6.7.2 step 1.2 value-KIND gate (property-agnostic coordinator) ---
//
// The gate classifies each component longhand's *serialized string* (CSSOM
// stores serialized values; the inline `el.style` path is string-backed) into
// Physical / CSS-wide keyword / unresolved `var()`, then applies §6.7.2 step 1.2
// BEFORE the per-family collapse. `revert`/`revert-layer` are classified for
// forward-compat but are intentionally UNTESTED here: the parser drops them
// (`parse_global_keyword` → None), so `margin: revert` is unreachable — deferred
// to `#11-css-wide-revert-keyword`.

/// Corner 1 — every component is the SAME CSS-wide keyword ⇒ the shorthand *is*
/// that keyword (css-cascade-4 §7.3). The gate is an **override of a covered
/// handler's collapse**, so it applies across the COVERED families — margin
/// (rectangular) and overflow (Box axis-pair) both collapse to the keyword.
/// UNCOVERED shorthands (`flex`, `background`) return `None` here and defer to the
/// caller's direct-declaration fallback — see
/// `uncovered_shorthand_with_nonphysical_longhands_is_none`. The now-COVERED
/// Multicol shorthands (`column-rule`/`columns`) collapse to the keyword too — see
/// `multicol_csswide_and_var_collapse_via_gate`.
#[test]
fn all_same_css_wide_keyword_collapses_to_keyword() {
    for kw in ["initial", "inherit", "unset"] {
        let mut margin = HashMap::new();
        for side in ["margin-top", "margin-right", "margin-bottom", "margin-left"] {
            margin.insert(side, kw);
        }
        assert_eq!(serialize("margin", &margin), Some(kw.to_string()));

        let mut overflow = HashMap::new();
        overflow.insert("overflow-x", kw);
        overflow.insert("overflow-y", kw);
        assert_eq!(serialize("overflow", &overflow), Some(kw.to_string()));
    }
}

/// Corner 2 — different CSS-wide keywords on different components cannot be
/// exactly represented (§6.7.2 step 1.2) ⇒ "".
#[test]
fn mixed_different_css_wide_keywords_are_empty() {
    let mut margin = HashMap::new();
    margin.insert("margin-top", "initial");
    margin.insert("margin-right", "inherit");
    margin.insert("margin-bottom", "initial");
    margin.insert("margin-left", "inherit");
    assert_eq!(serialize("margin", &margin), Some(String::new()));

    let mut gap = HashMap::new();
    gap.insert("row-gap", "initial");
    gap.insert("column-gap", "inherit");
    assert_eq!(serialize("gap", &gap), Some(String::new()));

    let mut overflow = HashMap::new();
    overflow.insert("overflow-x", "unset");
    overflow.insert("overflow-y", "inherit");
    assert_eq!(serialize("overflow", &overflow), Some(String::new()));
}

/// Corner 3 — any unsubstituted `var()` component makes the shorthand
/// non-representable at specified-value time (css-variables-1 §3/§2.2) ⇒ "".
/// Covers pure V with physical siblings, V mixed with a physical value, V mixed
/// with a CSS-wide keyword, and the `RawTokens("var(.., fallback)")` spelling.
#[test]
fn any_unresolved_var_is_empty() {
    let mut margin = HashMap::new();
    margin.insert("margin-top", "var(--x)");
    margin.insert("margin-right", "0px");
    margin.insert("margin-bottom", "0px");
    margin.insert("margin-left", "0px");
    assert_eq!(serialize("margin", &margin), Some(String::new()));

    let mut gap = HashMap::new();
    gap.insert("row-gap", "var(--g)");
    gap.insert("column-gap", "4px");
    assert_eq!(serialize("gap", &gap), Some(String::new()));

    let mut overflow = HashMap::new();
    overflow.insert("overflow-x", "var(--o)");
    overflow.insert("overflow-y", "hidden");
    assert_eq!(serialize("overflow", &overflow), Some(String::new()));

    // V + CSS-wide keyword — the var branch short-circuits before the keyword
    // count, so a var anywhere wins.
    let mut margin_vk = HashMap::new();
    margin_vk.insert("margin-top", "var(--x)");
    margin_vk.insert("margin-right", "initial");
    margin_vk.insert("margin-bottom", "inherit");
    margin_vk.insert("margin-left", "0px");
    assert_eq!(serialize("margin", &margin_vk), Some(String::new()));

    // A `var()` with a fallback still carries the `var(` substring (covered
    // Table axis-pair `border-spacing` — a different handler than Box).
    let mut border_spacing = HashMap::new();
    border_spacing.insert("border-spacing-h", "var(--w, 10px)");
    border_spacing.insert("border-spacing-v", "2px");
    assert_eq!(
        serialize("border-spacing", &border_spacing),
        Some(String::new())
    );
}

/// Corner 4 — a CSS-wide keyword on some components and a physical value on
/// others cannot be exactly represented (§6.7.2 step 1.2) ⇒ "" for every COVERED
/// family (the 6 landed families). The gate overrides the covered handler's
/// collapse spec-uniformly, diverging from Blink's `gap` outlier (Blink emits the
/// non-round-tripping `"initial 4px"`; see the `gap` note below) toward "".
/// (The now-COVERED Multicol `column-rule` / `columns` add their K+P `""` assertion
/// in `multicol_csswide_and_var_collapse_via_gate`.)
#[test]
fn css_wide_keyword_mixed_with_physical_is_empty() {
    // rectangular: initial + physical (structural)
    let mut margin = HashMap::new();
    margin.insert("margin-top", "initial");
    margin.insert("margin-right", "5px");
    margin.insert("margin-bottom", "initial");
    margin.insert("margin-left", "5px");
    assert_eq!(serialize("margin", &margin), Some(String::new()));

    // rectangular: inherit + physical
    let mut padding = HashMap::new();
    padding.insert("padding-top", "inherit");
    padding.insert("padding-right", "5px");
    padding.insert("padding-bottom", "5px");
    padding.insert("padding-left", "5px");
    assert_eq!(serialize("padding", &padding), Some(String::new()));

    // rectangular: border-radius
    let mut br = HashMap::new();
    br.insert("border-top-left-radius", "initial");
    br.insert("border-top-right-radius", "2px");
    br.insert("border-bottom-right-radius", "2px");
    br.insert("border-bottom-left-radius", "2px");
    assert_eq!(serialize("border-radius", &br), Some(String::new()));

    // Box axis-pair overflow
    let mut overflow = HashMap::new();
    overflow.insert("overflow-x", "inherit");
    overflow.insert("overflow-y", "hidden");
    assert_eq!(serialize("overflow", &overflow), Some(String::new()));

    // Table axis-pair border-spacing (different handler — proves the gate is in
    // the coordinator, ahead of every family's dispatch).
    let mut bs = HashMap::new();
    bs.insert("border-spacing-h", "initial");
    bs.insert("border-spacing-v", "2px");
    assert_eq!(serialize("border-spacing", &bs), Some(String::new()));

    // gap — INTENTIONAL Blink divergence. Blink returns "initial 4px" here, but
    // that output does NOT round-trip: `el.style.setProperty("gap","initial 4px")`
    // → `cssText === ""` (Blink rejects its own getter output as invalid input).
    // The sibling Box axis-pair `overflow` already returns "" for the identical
    // shape, so uniform "" is spec-faithful (§6.7.2 step 1.2) and internally
    // consistent — the property-agnostic gate cannot special-case `gap` without
    // destroying I2 (plan §2, §Notes).
    let mut gap = HashMap::new();
    gap.insert("row-gap", "initial");
    gap.insert("column-gap", "4px");
    assert_eq!(serialize("gap", &gap), Some(String::new()));
}

/// Regression — the exact mis-collapses measured on the landed families (plan
/// §Problem: elidex-today via throwaway probe) are now "", using the plan's
/// "stored" longhand strings directly.
#[test]
fn six_family_regression_mis_collapses_now_empty() {
    // was elidex "initial 5px" (Blink "")
    let mut m1 = HashMap::new();
    m1.insert("margin-top", "initial");
    m1.insert("margin-right", "5px");
    m1.insert("margin-bottom", "initial");
    m1.insert("margin-left", "5px");
    assert_eq!(serialize("margin", &m1), Some(String::new()));

    // was elidex "inherit 5px 5px" (Blink "")
    let mut m2 = HashMap::new();
    m2.insert("margin-top", "inherit");
    m2.insert("margin-right", "5px");
    m2.insert("margin-bottom", "5px");
    m2.insert("margin-left", "5px");
    assert_eq!(serialize("margin", &m2), Some(String::new()));

    // was elidex "var(--x) 0px 0px" (Blink "")
    let mut m3 = HashMap::new();
    m3.insert("margin-top", "var(--x)");
    m3.insert("margin-right", "0px");
    m3.insert("margin-bottom", "0px");
    m3.insert("margin-left", "0px");
    assert_eq!(serialize("margin", &m3), Some(String::new()));

    // was elidex "var(--g) 4px" (Blink "")
    let mut g = HashMap::new();
    g.insert("row-gap", "var(--g)");
    g.insert("column-gap", "4px");
    assert_eq!(serialize("gap", &g), Some(String::new()));

    // was elidex "initial inherit" (Blink "")
    let mut m4 = HashMap::new();
    m4.insert("margin-top", "initial");
    m4.insert("margin-right", "inherit");
    m4.insert("margin-bottom", "initial");
    m4.insert("margin-left", "inherit");
    assert_eq!(serialize("margin", &m4), Some(String::new()));

    // corner 1 unchanged — all-same-K still collapses to the keyword.
    let mut m5 = HashMap::new();
    m5.insert("margin-top", "initial");
    m5.insert("margin-right", "initial");
    m5.insert("margin-bottom", "initial");
    m5.insert("margin-left", "initial");
    assert_eq!(serialize("margin", &m5), Some("initial".to_string()));
}

/// I3 behavior-preservation — an all-physical shorthand falls through the gate
/// (`value_kind_gate` → None) to the family collapse, byte-identical to #468.
#[test]
fn all_physical_falls_through_to_collapse() {
    // t=10 r=20 b=10 l=20 → left==right and top==bottom ⇒ two-value form.
    let mut margin = HashMap::new();
    margin.insert("margin-top", "10px");
    margin.insert("margin-right", "20px");
    margin.insert("margin-bottom", "10px");
    margin.insert("margin-left", "20px");
    assert_eq!(serialize("margin", &margin), Some("10px 20px".to_string()));

    let mut gap = HashMap::new();
    gap.insert("row-gap", "4px");
    gap.insert("column-gap", "8px");
    assert_eq!(serialize("gap", &gap), Some("4px 8px".to_string()));
}

/// Corner 5 (coordinator view) — a WHOLE-shorthand `var()` is stored under the
/// shorthand name, not longhand-expanded, so the §6.6.1 all-present check fails
/// and `serialize_shorthand_value` returns `None` BEFORE the gate. The caller's
/// `.or_else` fallback (elidex-dom-api, out of this crate) then reads the
/// shorthand's own stored `var(--x)`. The gate never runs, so it cannot regress
/// corner 5.
#[test]
fn whole_shorthand_var_not_reached_by_gate() {
    let mut m = HashMap::new();
    m.insert("margin", "var(--x)"); // no longhands present
    assert_eq!(serialize("margin", &m), None);
}

// --- end-to-end reachability: author CSS → parser → gate (populated registry) ---

/// Serialize a shorthand from GENUINELY parsed author CSS. MUST pass a populated
/// registry — multicol/box longhands are registry-backed and a `None` registry
/// silently drops them (plan §Parse-discrepancy investigation: the guard for any
/// elidex-internal multicol/flex/grid probe).
fn serialize_parsed(property: &str, css: &str) -> Option<String> {
    let decls = elidex_css::parse_declaration_block_with_registry(
        css,
        Some(default_css_property_registry()),
    );
    serialize_shorthand_value(default_css_property_registry(), property, move |lh| {
        decls
            .iter()
            .rev() // last-declaration-wins (CSSOM cascade within a block)
            .find(|d| d.property == lh)
            .map(|d| (d.value.to_css_string(), d.important))
    })
}

/// The gate is reachable from ordinary author CSS: the parser expands a
/// shorthand CSS-wide keyword into per-longhand keyword declarations
/// (`declaration.rs` `expand_global_keyword`) and stores a longhand `var()` as a
/// var-carrying value — both serialize to the strings the gate classifies.
#[test]
fn author_css_reaches_gate_end_to_end() {
    // shorthand css-wide keyword → per-longhand `initial` → all-same-K → "initial"
    assert_eq!(
        serialize_parsed("margin", "margin: initial"),
        Some("initial".to_string())
    );

    // a longhand var() with physical siblings → any-V → ""
    assert_eq!(
        serialize_parsed(
            "margin",
            "margin-top: var(--x); margin-right: 0px; margin-bottom: 0px; margin-left: 0px",
        ),
        Some(String::new())
    );

    // Covered Multicol `columns` (PR1): a var-carrying longhand → any-V → "".
    // Both longhands parse ONLY with a populated registry (the parse-discrepancy
    // guard — the no-registry `parse_declaration_block` drops registry-backed
    // multicol longhands).
    assert_eq!(
        serialize_parsed("columns", "column-width: var(--w); column-count: auto"),
        Some(String::new())
    );
}

// --- Multicol omit-initial family (MulticolHandler: column-rule / columns) ---
//
// PR1 makes `column-rule`/`columns` COVERED (`MulticolHandler::serialize_shorthand`
// via `serialize_omit_initial`). Under the coverage-first coordinator they now
// serialize like every covered family: the value-KIND gate overrides css-wide/var
// component KINDs (fixing the two R1 Codex findings), and an all-physical block
// falls through to the handler's concrete omit-initial collapse.

/// The 2 R1 Codex findings on PR1 are fixed by the coverage-first value-KIND gate:
/// a covered Multicol shorthand whose components are a uniform CSS-wide keyword
/// collapses to that keyword (NOT `"initial initial initial"` — PR1's handler
/// collapse alone), and any unresolved `var()` (or CSS-wide+physical) component
/// yields `""`.
#[test]
fn multicol_csswide_and_var_collapse_via_gate() {
    // R1 finding #1 — all-same CSS-wide keyword ⇒ the keyword (was
    // "initial initial initial" before the gate overrode the omit-initial collapse).
    let mut cr = HashMap::new();
    for lh in [
        "column-rule-width",
        "column-rule-style",
        "column-rule-color",
    ] {
        cr.insert(lh, "initial");
    }
    assert_eq!(serialize("column-rule", &cr), Some("initial".to_string()));

    let mut cols = HashMap::new();
    cols.insert("column-width", "inherit");
    cols.insert("column-count", "inherit");
    assert_eq!(serialize("columns", &cols), Some("inherit".to_string()));

    // R1 finding #2 — an unresolved `var()` component ⇒ "" (any-V short-circuit).
    let mut cr_var = HashMap::new();
    cr_var.insert("column-rule-width", "var(--w)");
    cr_var.insert("column-rule-style", "solid");
    cr_var.insert("column-rule-color", "red");
    assert_eq!(serialize("column-rule", &cr_var), Some(String::new()));

    // K+P (CSS-wide keyword + physical) ⇒ "". The Blink-faithful `initial`-omit
    // (`"solid red"`) is the deferred `#11-shorthand-omit-initial-csswide-omission`
    // under-approximation, not a bug.
    let mut cr_kp = HashMap::new();
    cr_kp.insert("column-rule-width", "initial");
    cr_kp.insert("column-rule-style", "solid");
    cr_kp.insert("column-rule-color", "red");
    assert_eq!(serialize("column-rule", &cr_kp), Some(String::new()));
}

/// PR1 concrete omit-initial (the common case): a component whose *concrete* value
/// equals its initial is omitted. GENUINE parse→serialize via `serialize_parsed`
/// (registry-backed multicol longhands — the no-registry `parse_declaration_block`
/// drops them, plan §Parse-discrepancy) — exercises elidex's own keyword/color
/// serialization (thick→5px, blue→#0000ff, …), which a literal string map cannot.
#[test]
fn column_rule_omit_initial_corners() {
    // only style non-initial
    assert_eq!(
        serialize_parsed("column-rule", "column-rule: solid"),
        Some("solid".to_string())
    );
    // width+color kept, style=initial in the MIDDLE (thick→5px, blue→#0000ff)
    assert_eq!(
        serialize_parsed("column-rule", "column-rule: thick blue"),
        Some("5px #0000ff".to_string())
    );
    // leading width omitted, style+color survive (green→#008000)
    assert_eq!(
        serialize_parsed("column-rule", "column-rule: dashed green"),
        Some("dashed #008000".to_string())
    );
    // ALL initial ⇒ keep first (width); `3px` vs Blink's `medium`
    assert_eq!(
        serialize_parsed("column-rule", "column-rule: medium none currentcolor"),
        Some("3px".to_string())
    );
    // all non-initial (thick→5px, red→#ff0000)
    assert_eq!(
        serialize_parsed("column-rule", "column-rule: thick solid red"),
        Some("5px solid #ff0000".to_string())
    );
}

#[test]
fn columns_omit_initial_corners() {
    // both auto (initial) ⇒ single `auto`, both author forms
    assert_eq!(
        serialize_parsed("columns", "columns: auto"),
        Some("auto".to_string())
    );
    assert_eq!(
        serialize_parsed("columns", "columns: auto auto"),
        Some("auto".to_string())
    );
    // width only (count=initial auto dropped)
    assert_eq!(
        serialize_parsed("columns", "columns: 200px"),
        Some("200px".to_string())
    );
    // count only (width=initial auto dropped); Number(3.0)→"3"
    assert_eq!(
        serialize_parsed("columns", "columns: 3"),
        Some("3".to_string())
    );
    assert_eq!(
        serialize_parsed("columns", "columns: 3 auto"),
        Some("3".to_string())
    );
    // both kept, canonical order width→count
    assert_eq!(
        serialize_parsed("columns", "columns: 200px 3"),
        Some("200px 3".to_string())
    );
}

/// §6.6.1 uniform-`!important` on a COVERED Multicol shorthand — mixed priority
/// rejects before dispatch (None); uniform serializes (keep style, drop the
/// initial width+color).
#[test]
fn column_rule_mixed_important_is_none() {
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
