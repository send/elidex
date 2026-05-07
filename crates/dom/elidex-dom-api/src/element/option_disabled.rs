//! HTML §4.10.10.2 — `<option>` disabledness predicate.
//!
//! Hoisted from `elidex-form` per the M4-12 architectural drift
//! recovery slot `#11-tags-T1-v2-drift-hoist` (D-6) to consolidate
//! the duplicate that previously lived in
//! `elidex-dom-api/src/live_collection.rs::is_option_disabled_local`.
//!
//! `elidex-form` re-exports this fn; the form crate is the historical
//! caller surface, the canonical home is here because (a) `elidex-form`
//! depends on `elidex-dom-api`, not the reverse, and (b) the algorithm
//! is a pure DOM ancestor walk over content attributes.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

/// Returns `true` if `entity` is an `<option>` whose own `disabled`
/// attribute is set OR whose enclosing tree contains a disabled
/// `<optgroup>` ancestor. HTML §4.10.10.2 — an option is "disabled"
/// when either condition holds. In well-formed markup `<optgroup>`
/// elements don't nest (the parser flattens them), but the walker
/// climbs up to `MAX_ANCESTOR_DEPTH` ancestors and stops at the
/// enclosing `<select>`, so any disabled optgroup encountered before
/// that cutoff disables the option — mirrors browsers that accept
/// malformed nested-optgroup trees gracefully.
///
/// Returns `false` when `entity` is not actually an `<option>` (so
/// callers can pass arbitrary entities defensively without
/// mis-attributing a `disabled` attribute on, say, a `<button>` to
/// "option-disabled" semantics).
#[must_use]
pub fn is_option_disabled(dom: &EcsDom, entity: Entity) -> bool {
    let is_option = dom
        .world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("option"));
    if !is_option {
        return false;
    }
    if dom
        .world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.contains("disabled"))
    {
        return true;
    }
    let mut current = dom.get_parent(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let Some(ancestor) = current else {
            return false;
        };
        let is_disabled_optgroup = dom
            .world()
            .get::<&TagType>(ancestor)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("optgroup"))
            && dom
                .world()
                .get::<&Attributes>(ancestor)
                .is_ok_and(|a| a.contains("disabled"));
        if is_disabled_optgroup {
            return true;
        }
        // Stop at the enclosing `<select>` — disabled propagation
        // beyond the select is the form's `<fieldset>` concern.
        if dom
            .world()
            .get::<&TagType>(ancestor)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("select"))
        {
            return false;
        }
        current = dom.get_parent(ancestor);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn via_own_attribute() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("disabled", "");
        let opt = dom.create_element("option", attrs);
        assert!(is_option_disabled(&dom, opt));
    }

    #[test]
    fn via_optgroup_ancestor() {
        let mut dom = EcsDom::new();
        let mut grp_attrs = Attributes::default();
        grp_attrs.set("disabled", "");
        let grp = dom.create_element("optgroup", grp_attrs);
        let opt = dom.create_element("option", Attributes::default());
        assert!(dom.append_child(grp, opt));
        assert!(is_option_disabled(&dom, opt));
    }

    #[test]
    fn returns_false_when_neither_set() {
        let mut dom = EcsDom::new();
        let grp = dom.create_element("optgroup", Attributes::default());
        let opt = dom.create_element("option", Attributes::default());
        assert!(dom.append_child(grp, opt));
        assert!(!is_option_disabled(&dom, opt));
    }

    #[test]
    fn returns_false_for_non_option_tag() {
        // Defensive tag gate: a `<div disabled>` (or any non-option)
        // must not be reported as option-disabled even when the
        // attribute matches.
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("disabled", "");
        let div = dom.create_element("div", attrs);
        assert!(!is_option_disabled(&dom, div));
    }

    #[test]
    fn stops_at_enclosing_select() {
        // A `disabled` attribute on the enclosing `<select>` must NOT
        // propagate to options — that's a `<fieldset>`-style concern.
        let mut dom = EcsDom::new();
        let mut sel_attrs = Attributes::default();
        sel_attrs.set("disabled", "");
        let sel = dom.create_element("select", sel_attrs);
        let opt = dom.create_element("option", Attributes::default());
        assert!(dom.append_child(sel, opt));
        assert!(!is_option_disabled(&dom, opt));
    }

    #[test]
    fn nested_wrapper_with_disabled_optgroup_disables() {
        // `<select><optgroup disabled><div><option>...` — JS-driven
        // mutation can introduce arbitrary wrappers; the walker must
        // still climb past them and find the disabled optgroup.
        let mut dom = EcsDom::new();
        let sel = dom.create_element("select", Attributes::default());
        let mut grp_attrs = Attributes::default();
        grp_attrs.set("disabled", "");
        let grp = dom.create_element("optgroup", grp_attrs);
        let wrapper = dom.create_element("div", Attributes::default());
        let opt = dom.create_element("option", Attributes::default());
        assert!(dom.append_child(sel, grp));
        assert!(dom.append_child(grp, wrapper));
        assert!(dom.append_child(wrapper, opt));
        assert!(is_option_disabled(&dom, opt));
    }
}
