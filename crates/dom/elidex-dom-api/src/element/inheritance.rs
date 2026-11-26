//! Element inheritance algorithms — ancestor-walk predicates that
//! resolve effective state from contributing ancestors.
//!
//! Hoisted from `elidex-js` VM host layer per the M4-12 architectural
//! drift recovery slot `#11-tags-T1-v2-drift-hoist` (D-5): vm/host/
//! must contain only engine-bound responsibilities (prototype install,
//! brand check, marshalling). Ancestor-walk algorithms over the DOM
//! tree are engine-independent and live here.

use elidex_ecs::{EcsDom, Entity, MAX_ANCESTOR_DEPTH};

/// Resolve `isContentEditable` for `entity` (HTML §6.8.1).
///
/// The walker checks the element and its ancestors for a `contenteditable`
/// attribute. It recognizes the empty string, `true` and `plaintext-only` as
/// editable, matching keywords ASCII case-insensitively. An explicit `false`
/// stops the walk. Reaching the root without an attribute returns `false`.
///
/// Defect (#11-contenteditable-invalid-keyword-inherits): the walker also
/// stops at an invalid keyword and returns `false`. HTML §6.8.1 gives invalid
/// and missing values the Inherit state, so both should continue the walk.
///
/// Ancestor walk is capped at [`MAX_ANCESTOR_DEPTH`] parent-edges to
/// defend against pathological trees.  The entity's own attribute is
/// checked outside the cap; an ancestor at parent-edge distance up to
/// and including `MAX_ANCESTOR_DEPTH` is reachable, anything beyond
/// inherits `false`.  Mirrors the sibling walker convention in
/// [`crate::element::is_option_disabled`].
#[must_use]
pub fn is_content_editable(dom: &EcsDom, entity: Entity) -> bool {
    if let Some(b) = match_contenteditable(dom, entity) {
        return b;
    }
    let mut cur = dom.get_parent(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let Some(e) = cur else {
            return false;
        };
        if let Some(b) = match_contenteditable(dom, e) {
            return b;
        }
        cur = dom.get_parent(e);
    }
    false
}

fn match_contenteditable(dom: &EcsDom, entity: Entity) -> Option<bool> {
    dom.with_attribute(entity, "contenteditable", |raw| {
        raw.map(|s| {
            s.eq_ignore_ascii_case("true")
                || s.eq_ignore_ascii_case("plaintext-only")
                || s.is_empty()
        })
    })
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
            let e = dom.create_element_ownerless_fixture("div", attrs);
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
        // HTML §6.8.1 gives the child the Inherit state, so it should inherit
        // the editable parent. This assertion records the current early return
        // (#11-contenteditable-invalid-keyword-inherits).
        let (dom, entities) = dom_with_chain(&[Some("true"), Some("bogus")]);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }

    #[test]
    fn caps_walk_at_max_ancestor_depth() {
        // Build a chain so the root carrying `contenteditable="true"`
        // sits `MAX_ANCESTOR_DEPTH + 1` parent-edges above the leaf —
        // one past the reachable boundary.  Self-check + cap iterations
        // (each visiting one ancestor) exhaust before reaching the root,
        // so the leaf inherits the spec default (`false`).  Locks the
        // intentional behaviour change introduced by this slot.
        let chain_len = MAX_ANCESTOR_DEPTH + 2;
        let mut states: Vec<Option<&str>> = Vec::with_capacity(chain_len);
        states.push(Some("true"));
        for _ in 1..chain_len {
            states.push(None);
        }
        let (dom, entities) = dom_with_chain(&states);
        let leaf = *entities.last().unwrap();
        assert!(!is_content_editable(&dom, leaf));
    }

    #[test]
    fn reaches_ancestor_exactly_at_max_ancestor_depth() {
        // Boundary companion: an ancestor at exactly `MAX_ANCESTOR_DEPTH`
        // parent-edges from the entity IS reachable (the cap bounds
        // ancestor parent-edges walked, not total entities visited).
        // Mirrors the sibling `is_option_disabled` convention.
        let chain_len = MAX_ANCESTOR_DEPTH + 1;
        let mut states: Vec<Option<&str>> = Vec::with_capacity(chain_len);
        states.push(Some("true"));
        for _ in 1..chain_len {
            states.push(None);
        }
        let (dom, entities) = dom_with_chain(&states);
        let leaf = *entities.last().unwrap();
        assert!(is_content_editable(&dom, leaf));
    }
}
