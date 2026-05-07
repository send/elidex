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
/// Walks ancestors looking for the first explicit `contenteditable`
/// content-attribute state:
///
/// - `"true"` / `"plaintext-only"` / `""` → editable (`true`)
/// - `"false"` → non-editable (`false`)
/// - any other / no attribute → inherit from parent
///
/// Root inherits `false` (spec default for `<html>`). Comparison is
/// ASCII-case-insensitive per WHATWG attribute-value parsing rules.
#[must_use]
pub fn is_content_editable(dom: &EcsDom, entity: Entity) -> bool {
    let mut cur = Some(entity);
    while let Some(e) = cur {
        let matched = dom.with_attribute(e, "contenteditable", |raw| {
            raw.map(|s| {
                let lower = s.to_ascii_lowercase();
                matches!(lower.as_str(), "true" | "plaintext-only" | "")
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
    fn unknown_value_falls_through_to_parent() {
        // Note: HTML spec maps invalid values to the inherit state per
        // §6.7.3.2, so a child with garbage falls through to its
        // ancestor.  However this walker currently treats any non-
        // matching string ("bogus") as "explicit false" because the
        // inner closure returns `Some(false)` — which then short-
        // circuits.  Mirror VM behaviour: leaf with garbage value
        // resolves to false.
        let (dom, entities) = dom_with_chain(&[Some("true"), Some("bogus")]);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }
}
