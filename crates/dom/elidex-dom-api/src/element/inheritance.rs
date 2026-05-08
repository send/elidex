//! Element inheritance algorithms — ancestor-walk predicates that
//! resolve effective state from contributing ancestors.
//!
//! Hoisted from `elidex-js` VM host layer per the M4-12 architectural
//! drift recovery slot `#11-tags-T1-v2-drift-hoist` (D-5): vm/host/
//! must contain only engine-bound responsibilities (prototype install,
//! brand check, marshalling). Ancestor-walk algorithms over the DOM
//! tree are engine-independent and live here.

use elidex_ecs::{EcsDom, Entity};

/// Resolve the effective `isContentEditable` state for `entity`
/// (HTML §6.7.3 — `contentEditable` IDL attribute).
///
/// Walks ancestors looking for the first **present** `contenteditable`
/// content-attribute on the chain:
///
/// - `"true"` / `"plaintext-only"` / `""` → editable (`true`)
/// - any other present value (incl. `"false"` and invalid garbage) →
///   non-editable (`false`); short-circuits the ancestor walk
/// - **absent attribute on this node** → continue walking the parent
///   chain
///
/// Root with no `contenteditable` along the chain inherits `false`
/// (spec default for `<html>`).
///
/// **Known spec divergence (HTML §6.7.3.2)**: the spec maps invalid
/// values to the *inherit* state, but this walker preserves the
/// historical VM behaviour of treating any present non-matching
/// value as explicit `false`. Pinned by the regression test
/// `unknown_value_short_circuits_as_false_diverging_from_spec`.
/// Comparison is ASCII-case-insensitive per WHATWG attribute-value
/// parsing rules.
#[must_use]
pub fn is_content_editable(dom: &EcsDom, entity: Entity) -> bool {
    // Unbounded ancestor walk preserves pre-PR VM-host behaviour
    // exactly (the user mandate for slot `#11-tags-T1-v2-drift-hoist`
    // is "pure hoist / no behavior change").  Adding a
    // `MAX_ANCESTOR_DEPTH` cap to mirror sibling walkers would change
    // `isContentEditable` results for trees deeper than the cap; the
    // safety hardening is deferred to a separate slot
    // `#11-content-editable-depth-cap` where it can be introduced as
    // an intentional behaviour change with a regression test that
    // covers the depth cutoff.
    let mut cur = Some(entity);
    while let Some(e) = cur {
        let matched = dom.with_attribute(e, "contenteditable", |raw| {
            raw.map(|s| {
                s.eq_ignore_ascii_case("true")
                    || s.eq_ignore_ascii_case("plaintext-only")
                    || s.is_empty()
            })
        });
        if let Some(b) = matched {
            return b;
        }
        cur = dom.get_parent(e);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn dom_with_chain(states: &[Option<&str>]) -> (EcsDom, Vec<Entity>) {
        // Build a parent → ... → leaf chain, with each entry assigning
        // the contenteditable attribute (or none).  Returns the chain
        // entities root-first.
        let mut dom = EcsDom::new();
        let mut entities = Vec::new();
        let mut parent: Option<Entity> = None;
        for state in states {
            let mut attrs = Attributes::default();
            if let Some(value) = state {
                attrs.set("contenteditable", *value);
            }
            let e = dom.create_element("div", attrs);
            if let Some(p) = parent {
                assert!(dom.append_child(p, e));
            }
            parent = Some(e);
            entities.push(e);
        }
        (dom, entities)
    }

    #[test]
    fn no_attribute_anywhere_returns_false() {
        let (dom, entities) = dom_with_chain(&[None, None, None]);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }

    #[test]
    fn explicit_true_on_self_returns_true() {
        let (dom, entities) = dom_with_chain(&[None, Some("true")]);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }

    #[test]
    fn explicit_false_on_self_returns_false_even_with_true_ancestor() {
        let (dom, entities) = dom_with_chain(&[Some("true"), Some("false")]);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }

    #[test]
    fn ancestor_true_inherits_when_self_unset() {
        let (dom, entities) = dom_with_chain(&[Some("true"), None, None]);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }

    #[test]
    fn plaintext_only_treated_as_editable() {
        let (dom, entities) = dom_with_chain(&[Some("plaintext-only")]);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }

    #[test]
    fn empty_string_treated_as_editable() {
        // HTML enumerated attribute: empty string maps to the canonical
        // (non-default) state.
        let (dom, entities) = dom_with_chain(&[Some("")]);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }

    #[test]
    fn case_insensitive_match() {
        let (dom, entities) = dom_with_chain(&[Some("TRUE")]);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }

    #[test]
    fn unknown_value_short_circuits_as_false_diverging_from_spec() {
        // KNOWN SPEC DIVERGENCE: HTML §6.7.3.2 maps invalid values to
        // the inherit state (i.e. a child with garbage SHOULD fall
        // through to its ancestor's resolution).  This walker
        // currently treats any non-matching string ("bogus") as
        // "explicit false" because the inner closure returns
        // `Some(false)` and short-circuits the ancestor walk —
        // mirroring the historical VM behaviour preserved by the
        // hoist.  Spec compliance is deferred; this test pins the
        // current behaviour so a future fix is observable.
        let (dom, entities) = dom_with_chain(&[Some("true"), Some("bogus")]);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }
}
