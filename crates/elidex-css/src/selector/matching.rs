//! Selector matching: right-to-left component matching against the DOM.

use elidex_ecs::{Attributes, EcsDom, ElementState, Entity, TagType};

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
                if match_components(components, idx + 1, ancestor, dom) {
                    return true;
                }
                current = dom.get_parent(ancestor);
            }
            false
        }
        SelectorComponent::Child => dom
            .get_parent(entity)
            .is_some_and(|parent| match_components(components, idx + 1, parent, dom)),
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
        // Combinators and :not() handled in match_components.
        _ => false,
    }
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
            match name {
                "hover" => state.contains(ElementState::HOVER),
                "focus" => state.contains(ElementState::FOCUS),
                "active" => state.contains(ElementState::ACTIVE),
                "link" => state.contains(ElementState::LINK),
                "visited" => state.contains(ElementState::VISITED),
                _ => unreachable!(),
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
