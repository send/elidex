//! Selector matching: right-to-left component matching against the DOM.

use elidex_ecs::{
    Attributes, EcsDom, ElementState, Entity, ShadowHost, ShadowRoot, SlottedMarker, TagType,
};

use super::traverse::{
    first_element_child, is_root_element, last_element_child, prev_element_sibling,
};
use super::types::{AttributeMatcher, SelectorComponent};

/// Recursive right-to-left selector matching.
pub(super) fn match_components(
    components: &[SelectorComponent],
    idx: usize,
    entity: Entity,
    dom: &EcsDom,
) -> bool {
    if idx >= components.len() {
        return true;
    }

    match &components[idx] {
        // Combinators -- navigate the tree, then continue matching.
        SelectorComponent::Descendant => {
            use elidex_ecs::MAX_ANCESTOR_DEPTH;
            let mut current = dom.get_parent(entity);
            let mut depth = 0;
            while let Some(ancestor) = current {
                depth += 1;
                if depth > MAX_ANCESTOR_DEPTH {
                    return false;
                }
                // Stop at shadow boundary — selectors don't cross into shadow trees.
                if dom.world().get::<&ShadowRoot>(ancestor).is_ok() {
                    return false;
                }
                if match_components(components, idx + 1, ancestor, dom) {
                    return true;
                }
                current = dom.get_parent(ancestor);
            }
            false
        }
        SelectorComponent::Child => {
            let Some(parent) = dom.get_parent(entity) else {
                return false;
            };
            // Stop at shadow boundary.
            if dom.world().get::<&ShadowRoot>(parent).is_ok() {
                return false;
            }
            match_components(components, idx + 1, parent, dom)
        }
        SelectorComponent::AdjacentSibling => prev_element_sibling(dom, entity)
            .is_some_and(|prev| match_components(components, idx + 1, prev, dom)),
        SelectorComponent::GeneralSibling => {
            let mut current = prev_element_sibling(dom, entity);
            while let Some(sib) = current {
                if match_components(components, idx + 1, sib, dom) {
                    return true;
                }
                current = prev_element_sibling(dom, sib);
            }
            false
        }
        SelectorComponent::Not(ref inner) => {
            let inner_matched = match_compound_forward(inner, entity, dom);
            !inner_matched && match_components(components, idx + 1, entity, dom)
        }
        // :host / :host(selector) — delegate to match_simple() (single source of truth).
        SelectorComponent::Host | SelectorComponent::HostFunction(_) => {
            match_simple(&components[idx], entity, dom)
                && match_components(components, idx + 1, entity, dom)
        }
        // ::slotted(selector) — matches slotted light DOM elements that match inner selector.
        SelectorComponent::Slotted(ref inner) => {
            is_slotted(entity, dom)
                && match_compound_forward(inner, entity, dom)
                && match_components(components, idx + 1, entity, dom)
        }
        // Simple selectors -- delegate to shared helper.
        other => {
            match_simple(other, entity, dom) && match_components(components, idx + 1, entity, dom)
        }
    }
}

/// Match a single simple selector (non-combinator) against an entity.
fn match_simple(component: &SelectorComponent, entity: Entity, dom: &EcsDom) -> bool {
    match component {
        SelectorComponent::Universal => true,
        SelectorComponent::Tag(tag) => dom
            .world()
            .get::<&TagType>(entity)
            .ok()
            .is_some_and(|t| t.0 == *tag),
        SelectorComponent::Class(class) => {
            dom.world()
                .get::<&Attributes>(entity)
                .ok()
                .is_some_and(|attrs| {
                    attrs
                        .get("class")
                        .is_some_and(|c| c.split_whitespace().any(|w| w == class.as_str()))
                })
        }
        SelectorComponent::Id(id) => dom
            .world()
            .get::<&Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.get("id") == Some(id.as_str())),
        SelectorComponent::PseudoClass(ref name) => match_pseudo_class(name, entity, dom),
        SelectorComponent::Attribute { name, matcher } => {
            match_attr(name, matcher.as_ref(), entity, dom)
        }
        SelectorComponent::Host => dom.world().get::<&ShadowHost>(entity).is_ok(),
        SelectorComponent::HostFunction(ref inner) => {
            dom.world().get::<&ShadowHost>(entity).is_ok()
                && match_compound_forward(inner, entity, dom)
        }
        // Combinators and :not() handled in match_components.
        _ => false,
    }
}

/// Get `ElementState` for a form element, or `None` if not a form element
/// or if the `ElementState` component has not been attached yet.
fn form_element_state(entity: Entity, dom: &EcsDom) -> Option<ElementState> {
    if !is_form_element(entity, dom) {
        return None;
    }
    dom.world().get::<&ElementState>(entity).ok().map(|s| *s)
}

/// Check if an entity can match `:required`/`:optional` (constraint validation candidates).
///
/// Per HTML §4.10.15.2.4: only `<input>`, `<select>`, and `<textarea>` are candidates.
/// `<button>` and `<fieldset>` are form elements but not candidates for these pseudo-classes.
fn is_requirable_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| matches!(t.0.as_str(), "input" | "select" | "textarea"))
}

/// Match a pseudo-class by name against an entity.
fn match_pseudo_class(name: &str, entity: Entity, dom: &EcsDom) -> bool {
    match name {
        "root" => is_root_element(entity, dom),
        "first-child" => dom
            .get_parent(entity)
            .is_some_and(|parent| first_element_child(dom, parent) == Some(entity)),
        "last-child" => dom
            .get_parent(entity)
            .is_some_and(|parent| last_element_child(dom, parent) == Some(entity)),
        "only-child" => dom.get_parent(entity).is_some_and(|parent| {
            first_element_child(dom, parent) == Some(entity)
                && last_element_child(dom, parent) == Some(entity)
        }),
        "empty" => dom.get_first_child(entity).is_none(),
        "hover" | "focus" | "active" | "link" | "visited" => {
            let state = dom
                .world()
                .get::<&ElementState>(entity)
                .ok()
                .map_or(ElementState::default(), |s| *s);
            state_flag_for_pseudo(name).is_some_and(|flag| state.contains(flag))
        }
        // Form-related pseudo-classes delegated to a separate function.
        "disabled" | "enabled" | "indeterminate" | "valid" | "invalid" | "checked" | "required"
        | "optional" | "read-only" | "read-write" => match_form_pseudo_class(name, entity, dom),
        _ => false,
    }
}

/// Match form-related pseudo-classes against an entity.
fn match_form_pseudo_class(name: &str, entity: Entity, dom: &EcsDom) -> bool {
    match name {
        // :disabled matches form elements that can be "actually disabled" (HTML §4.10.18.5).
        // <fieldset> can be disabled but is NOT in the set of elements matching :disabled/:enabled.
        "disabled" => {
            is_disableable_element(entity, dom)
                && form_element_state(entity, dom)
                    .is_some_and(|s| s.contains(ElementState::DISABLED))
        }
        // :indeterminate only matches <input type=checkbox|radio> and <progress>
        // per HTML §4.10.18.3.
        "indeterminate" => {
            is_indeterminate_candidate(entity, dom)
                && form_element_state(entity, dom)
                    .is_some_and(|s| s.contains(ElementState::INDETERMINATE))
        }
        // :valid/:invalid also match <output> (HTML §4.10.18.7) — always valid.
        "valid" | "invalid" => {
            if let Some(s) = form_element_state(entity, dom) {
                state_flag_for_pseudo(name).is_some_and(|f| s.contains(f))
            } else {
                // <output> is always valid (no validation constraints).
                name == "valid"
                    && dom
                        .world()
                        .get::<&TagType>(entity)
                        .is_ok_and(|t| t.0 == "output")
            }
        }
        // :checked matches <input type=checkbox|radio> AND <option selected>.
        "checked" => {
            if is_checkable_element(entity, dom) {
                form_element_state(entity, dom).is_some_and(|s| s.contains(ElementState::CHECKED))
            } else {
                dom.world()
                    .get::<&TagType>(entity)
                    .is_ok_and(|t| t.0 == "option")
                    && dom
                        .world()
                        .get::<&Attributes>(entity)
                        .ok()
                        .is_some_and(|a| a.contains("selected"))
            }
        }
        // :required only matches input/select/textarea (HTML §4.10.15.2.4).
        "required" => {
            is_requirable_element(entity, dom)
                && form_element_state(entity, dom)
                    .is_some_and(|s| s.contains(ElementState::REQUIRED))
        }
        "enabled" => {
            // HTML spec: :enabled matches "actually disableable" form elements
            // that are not disabled (excludes <fieldset>).
            // Note: <a>/<area>/<link> with href are NOT :enabled per WHATWG spec;
            // they are reachable via :any-link/:link instead.
            is_disableable_element(entity, dom)
                && form_element_state(entity, dom)
                    .is_some_and(|s| !s.contains(ElementState::DISABLED))
        }
        // :optional only matches input/select/textarea (same constraint as :required).
        "optional" => {
            is_requirable_element(entity, dom)
                && form_element_state(entity, dom)
                    .is_some_and(|s| !s.contains(ElementState::REQUIRED))
        }
        "read-only" => {
            if let Some(s) = form_element_state(entity, dom) {
                s.contains(ElementState::READ_ONLY) || s.contains(ElementState::DISABLED)
            } else {
                dom.world().get::<&TagType>(entity).is_ok() && !dom.is_contenteditable(entity)
            }
        }
        "read-write" => {
            if let Some(s) = form_element_state(entity, dom) {
                !s.contains(ElementState::READ_ONLY) && !s.contains(ElementState::DISABLED)
            } else {
                dom.is_contenteditable(entity)
            }
        }
        _ => false,
    }
}

/// Match an attribute selector against an entity.
///
/// Attribute names are compared case-sensitively. Both the selector name
/// (lowercased during parse) and the DOM attribute name (lowercased by
/// html5ever) are stored in lowercase, so this is effectively
/// case-insensitive for HTML documents.
fn match_attr(
    name: &str,
    matcher: Option<&AttributeMatcher>,
    entity: Entity,
    dom: &EcsDom,
) -> bool {
    dom.world()
        .get::<&Attributes>(entity)
        .ok()
        .is_some_and(|attrs| match matcher {
            None => attrs.get(name).is_some(),
            Some(m) => attrs.get(name).is_some_and(|v| match_attribute(m, v)),
        })
}

/// Match a compound selector in forward (parse) order.
///
/// Used for `:not()` inner selectors, which contain only simple selectors
/// (no combinators) stored in parse order.
fn match_compound_forward(components: &[SelectorComponent], entity: Entity, dom: &EcsDom) -> bool {
    components.iter().all(|c| match_simple(c, entity, dom))
}

/// Map a dynamic pseudo-class name to its `ElementState` flag bit.
fn state_flag_for_pseudo(name: &str) -> Option<u16> {
    match name {
        "hover" => Some(ElementState::HOVER),
        "focus" => Some(ElementState::FOCUS),
        "active" => Some(ElementState::ACTIVE),
        "link" => Some(ElementState::LINK),
        "visited" => Some(ElementState::VISITED),
        "disabled" => Some(ElementState::DISABLED),
        "checked" => Some(ElementState::CHECKED),
        "required" => Some(ElementState::REQUIRED),
        "indeterminate" => Some(ElementState::INDETERMINATE),
        "valid" => Some(ElementState::VALID),
        "invalid" => Some(ElementState::INVALID),
        _ => None,
    }
}

/// Check if an entity is a form element that can be disabled/enabled.
fn is_form_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world().get::<&TagType>(entity).is_ok_and(|t| {
        matches!(
            t.0.as_str(),
            "input"
                | "button"
                | "textarea"
                | "select"
                | "fieldset"
                | "progress"
                | "meter"
                | "output"
        )
    })
}

/// Check if an entity is a checkable form element (`<input type=checkbox|radio>`).
///
/// Per CSS Selectors L4, `:checked` only matches these input types (plus `<option>`
/// which is handled separately).
fn is_checkable_element(entity: Entity, dom: &EcsDom) -> bool {
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    if tag.0 != "input" {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .ok()
        .map(|a| a.get("type").unwrap_or("text").to_ascii_lowercase())
        .is_some_and(|t| matches!(t.as_str(), "checkbox" | "radio"))
}

/// Check if an entity can match `:indeterminate` (HTML §4.10.18.3).
///
/// Only `<input type=checkbox>`, `<input type=radio>`, and `<progress>` can be indeterminate.
fn is_indeterminate_candidate(entity: Entity, dom: &EcsDom) -> bool {
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    match tag.0.as_str() {
        "progress" => true,
        "input" => is_checkable_element(entity, dom),
        _ => false,
    }
}

/// Check if an entity is an "actually disableable" element (HTML §4.10.18.5).
///
/// Per spec, `:enabled`/`:disabled` only match `<button>`, `<input>`, `<select>`,
/// and `<textarea>`. `<fieldset>` has its own disabled concept but does NOT
/// participate in the `:enabled`/`:disabled` pseudo-classes.
fn is_disableable_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| matches!(t.0.as_str(), "input" | "button" | "textarea" | "select"))
}

/// Check if an entity is a slotted element (assigned to a slot).
///
/// O(1) lookup via `SlottedMarker` component, which is attached to
/// assigned nodes by `distribute_slots()`.
fn is_slotted(entity: Entity, dom: &EcsDom) -> bool {
    dom.world().get::<&SlottedMarker>(entity).is_ok()
}

/// Check if an attribute value matches the given matcher.
fn match_attribute(matcher: &AttributeMatcher, value: &str) -> bool {
    match matcher {
        AttributeMatcher::Exact(expected) => value == expected.as_str(),
        AttributeMatcher::Includes(word) => value.split_whitespace().any(|w| w == word.as_str()),
        AttributeMatcher::DashMatch(prefix) => {
            value == prefix.as_str()
                || (value.starts_with(prefix.as_str())
                    && value.as_bytes().get(prefix.len()) == Some(&b'-'))
        }
        AttributeMatcher::Prefix(p) => value.starts_with(p.as_str()),
        AttributeMatcher::Suffix(s) => value.ends_with(s.as_str()),
        AttributeMatcher::Substring(sub) => value.contains(sub.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    #[test]
    fn disabled_does_not_match_non_form_element() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        // Manually set DISABLED flag on a <div> — should NOT match :disabled.
        let mut es = ElementState::default();
        es.insert(ElementState::DISABLED);
        let _ = dom.world_mut().insert_one(div, es);

        assert!(!match_pseudo_class("disabled", div, &dom));
    }

    #[test]
    fn checked_does_not_match_non_form_element() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::CHECKED);
        let _ = dom.world_mut().insert_one(div, es);

        assert!(!match_pseudo_class("checked", div, &dom));
    }

    #[test]
    fn disabled_matches_form_element() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::DISABLED);
        let _ = dom.world_mut().insert_one(input, es);

        assert!(match_pseudo_class("disabled", input, &dom));
    }

    #[test]
    fn enabled_matches_non_disabled_form_element() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.world_mut().insert_one(input, ElementState::default());

        assert!(match_pseudo_class("enabled", input, &dom));
    }

    #[test]
    fn enabled_does_not_match_non_form_element() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(div, ElementState::default());

        assert!(!match_pseudo_class("enabled", div, &dom));
    }

    #[test]
    fn required_matches_form_element_with_flag() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::REQUIRED);
        let _ = dom.world_mut().insert_one(input, es);
        assert!(match_pseudo_class("required", input, &dom));
    }

    #[test]
    fn optional_matches_form_element_without_required() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.world_mut().insert_one(input, ElementState::default());
        assert!(match_pseudo_class("optional", input, &dom));
    }

    #[test]
    fn valid_invalid_pseudo_classes() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::VALID);
        let _ = dom.world_mut().insert_one(input, es);
        assert!(match_pseudo_class("valid", input, &dom));
        assert!(!match_pseudo_class("invalid", input, &dom));
    }

    #[test]
    fn read_only_read_write_pseudo_classes() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::READ_ONLY);
        let _ = dom.world_mut().insert_one(input, es);
        assert!(match_pseudo_class("read-only", input, &dom));
        assert!(!match_pseudo_class("read-write", input, &dom));
    }

    #[test]
    fn disabled_matches_read_only_not_read_write() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::DISABLED);
        let _ = dom.world_mut().insert_one(input, es);
        // Disabled elements are :read-only and not :read-write per HTML spec.
        assert!(match_pseudo_class("read-only", input, &dom));
        assert!(!match_pseudo_class("read-write", input, &dom));
    }

    #[test]
    fn indeterminate_matches_checkbox() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("type", "checkbox");
        let input = dom.create_element("input", attrs);
        let mut es = ElementState::default();
        es.insert(ElementState::INDETERMINATE);
        let _ = dom.world_mut().insert_one(input, es);
        assert!(match_pseudo_class("indeterminate", input, &dom));
    }

    #[test]
    fn fieldset_does_not_match_enabled_disabled() {
        // Per HTML §4.10.18.5: <fieldset> is NOT "actually disableable" —
        // it does not match :enabled or :disabled pseudo-classes.
        let mut dom = EcsDom::new();
        let fs = dom.create_element("fieldset", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::DISABLED);
        let _ = dom.world_mut().insert_one(fs, es);
        assert!(!match_pseudo_class("disabled", fs, &dom));
        assert!(!match_pseudo_class("enabled", fs, &dom));
    }

    #[test]
    fn required_does_not_match_non_form() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::REQUIRED);
        let _ = dom.world_mut().insert_one(div, es);
        assert!(!match_pseudo_class("required", div, &dom));
    }

    #[test]
    fn required_does_not_match_fieldset() {
        let mut dom = EcsDom::new();
        let fs = dom.create_element("fieldset", Attributes::default());
        let mut es = ElementState::default();
        es.insert(ElementState::REQUIRED);
        let _ = dom.world_mut().insert_one(fs, es);
        assert!(!match_pseudo_class("required", fs, &dom));
    }

    #[test]
    fn checked_matches_option_selected() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("selected", "");
        let option = dom.create_element("option", attrs);
        assert!(match_pseudo_class("checked", option, &dom));
    }

    #[test]
    fn checked_does_not_match_option_without_selected() {
        let mut dom = EcsDom::new();
        let option = dom.create_element("option", Attributes::default());
        assert!(!match_pseudo_class("checked", option, &dom));
    }

    #[test]
    fn read_only_matches_non_form_without_contenteditable() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        assert!(match_pseudo_class("read-only", div, &dom));
    }

    #[test]
    fn read_write_matches_contenteditable() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("contenteditable", "true");
        let div = dom.create_element("div", attrs);
        assert!(match_pseudo_class("read-write", div, &dom));
        assert!(!match_pseudo_class("read-only", div, &dom));
    }

    #[test]
    fn enabled_does_not_match_anchor_with_href() {
        // Per WHATWG HTML spec, :enabled does NOT match <a>/<area>/<link>.
        // These elements are reachable via :any-link/:link instead.
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("href", "https://example.com");
        let a = dom.create_element("a", attrs);
        assert!(!match_pseudo_class("enabled", a, &dom));
    }

    #[test]
    fn enabled_matches_button_without_form() {
        // <button> is :enabled even without a parent <form>.
        let mut dom = EcsDom::new();
        let btn = dom.create_element("button", Attributes::default());
        let _ = dom.world_mut().insert_one(btn, ElementState::default());
        assert!(match_pseudo_class("enabled", btn, &dom));
        assert!(!match_pseudo_class("disabled", btn, &dom));
    }

    #[test]
    fn read_only_not_contenteditable_empty_value() {
        // contenteditable="" is equivalent to "true".
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("contenteditable", "");
        let div = dom.create_element("div", attrs);
        assert!(match_pseudo_class("read-write", div, &dom));
    }
}
