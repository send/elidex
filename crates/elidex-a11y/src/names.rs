//! Accessible name computation (simplified ACCNAME algorithm).
//!
//! Computes the accessible name for an element following a simplified
//! version of the WAI-ARIA Accessible Name and Description Computation
//! specification (ACCNAME).
//!
//! Priority: `aria-labelledby` (id reference) → `aria-label` → `alt` (img) → text content → `title`

use elidex_ecs::Entity;
use elidex_ecs::{Attributes, EcsDom, TagType, TextContent};

/// Maximum recursion depth for text content collection.
const MAX_TEXT_DEPTH: usize = 10_000;

/// Compute the accessible name for an element.
///
/// Returns `None` if no accessible name can be determined.
pub(crate) fn compute_accessible_name(dom: &EcsDom, entity: Entity) -> Option<String> {
    let world = dom.world();

    let attrs = world.get::<&Attributes>(entity).ok();
    let tag_component = world.get::<&TagType>(entity).ok();
    let tag = tag_component.as_ref().map_or("", |t| t.0.as_str());

    // 1. aria-labelledby — resolve referenced element's text content (ACCNAME §4.3.2 step 2B).
    if let Some(labelledby) = attrs.as_ref().and_then(|a| a.get("aria-labelledby")) {
        if let Some(n) = resolve_labelledby(dom, labelledby) {
            return Some(n);
        }
    }

    // 2. aria-label attribute (ACCNAME §4.3.2 step 2C).
    if let Some(label) = attrs.as_ref().and_then(|a| a.get("aria-label")) {
        let trimmed = label.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // 3. alt attribute (for img/area elements only, per ACCNAME spec).
    if matches!(tag, "img" | "area") {
        if let Some(alt) = attrs.as_ref().and_then(|a| a.get("alt")) {
            return Some(alt.to_string());
        }
    }

    // 4. Text content (concatenation of descendant text nodes).
    let text = collect_text_content(dom, entity);
    if !text.is_empty() {
        return Some(text);
    }

    // 5. title attribute (lowest priority).
    if let Some(title) = attrs.as_ref().and_then(|a| a.get("title")) {
        let trimmed = title.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Resolve an `aria-labelledby` value to text content.
///
/// The value is a space-separated list of element IDs. For each ID,
/// find the element and collect its text content.
pub(crate) fn resolve_labelledby(dom: &EcsDom, ids: &str) -> Option<String> {
    let mut parts = Vec::new();
    for id in ids.split_whitespace() {
        if let Some(entity) = find_element_by_id(dom, id) {
            let text = collect_text_content(dom, entity);
            if !text.is_empty() {
                parts.push(text);
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Find an element by its `id` attribute.
fn find_element_by_id(dom: &EcsDom, id: &str) -> Option<Entity> {
    dom.world()
        .query::<&Attributes>()
        .iter()
        .find(|(_, attrs)| attrs.get("id") == Some(id))
        .map(|(entity, _)| entity)
}

/// Collect text content from an element and all its descendants.
///
/// Per ACCNAME 1.2 §2F, child contributions are separated by spaces.
/// Whitespace runs are collapsed to a single space in the final result.
fn collect_text_content(dom: &EcsDom, entity: Entity) -> String {
    let mut result = String::new();
    collect_text_recursive(dom, entity, &mut result, 0);
    // Normalize: collapse whitespace runs and trim edges.
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_text_recursive(dom: &EcsDom, entity: Entity, out: &mut String, depth: usize) {
    if depth > MAX_TEXT_DEPTH {
        return;
    }

    // If this entity is a text node, append its content.
    if let Ok(text) = dom.world().get::<&TextContent>(entity) {
        out.push_str(&text.0);
        return;
    }

    // Recurse into children, separating contributions with spaces (ACCNAME 1.2 §2F).
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        let len_before = out.len();
        collect_text_recursive(dom, c, out, depth + 1);
        let contributed = out.len() > len_before;
        child = dom.get_next_sibling(c);
        // Insert separator space between non-empty child contributions.
        if contributed && child.is_some() && !out.ends_with(char::is_whitespace) {
            out.push(' ');
        }
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;

    /// Simple name computation tests: element with attributes + optional child text.
    #[test]
    fn name_computation_priority() {
        let cases = [
            // (description, tag, attrs, child_text, expected_name)
            (
                "aria-label over text content",
                "button",
                vec![("aria-label", "Click me"), ("title", "fallback")],
                Some("Button text"),
                Some("Click me"),
            ),
            (
                "alt attribute for img",
                "img",
                vec![("alt", "A photo")],
                None,
                Some("A photo"),
            ),
            (
                "text content fallback",
                "button",
                vec![],
                Some("Submit"),
                Some("Submit"),
            ),
            (
                "title fallback",
                "div",
                vec![("title", "Tooltip")],
                None,
                Some("Tooltip"),
            ),
            ("no name returns None", "div", vec![], None, None),
        ];

        for (desc, tag, attrs_list, child_text, expected) in cases {
            let mut dom = EcsDom::new();
            let root = dom.create_document_root();
            let mut attrs = Attributes::default();
            for &(k, v) in &attrs_list {
                attrs.set(k, v);
            }
            let el = dom.create_element(tag, attrs);
            dom.append_child(root, el);
            if let Some(text) = child_text {
                let text_node = dom.create_text(text);
                dom.append_child(el, text_node);
            }

            assert_eq!(
                compute_accessible_name(&dom, el),
                expected.map(str::to_string),
                "case: {desc}"
            );
        }
    }

    /// aria-labelledby takes priority over aria-label (ACCNAME §4.3.2).
    #[test]
    fn aria_labelledby_over_aria_label() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();

        let mut label_attrs = Attributes::default();
        label_attrs.set("id", "ref");
        let label_el = dom.create_element("span", label_attrs);
        dom.append_child(root, label_el);
        let label_text = dom.create_text("Referenced name");
        dom.append_child(label_el, label_text);

        let mut attrs = Attributes::default();
        attrs.set("aria-labelledby", "ref");
        attrs.set("aria-label", "Direct label");
        let el = dom.create_element("button", attrs);
        dom.append_child(root, el);

        assert_eq!(
            compute_accessible_name(&dom, el),
            Some("Referenced name".to_string())
        );
    }

    /// aria-labelledby resolves ID references to get label text.
    #[test]
    fn aria_labelledby_resolves_reference() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();

        let mut label_attrs = Attributes::default();
        label_attrs.set("id", "lbl");
        let label_el = dom.create_element("span", label_attrs);
        dom.append_child(root, label_el);
        let label_text = dom.create_text("Username");
        dom.append_child(label_el, label_text);

        let mut input_attrs = Attributes::default();
        input_attrs.set("aria-labelledby", "lbl");
        let input = dom.create_element("input", input_attrs);
        dom.append_child(root, input);

        assert_eq!(
            compute_accessible_name(&dom, input),
            Some("Username".to_string())
        );
    }

    /// Text content from multiple element children is separated by spaces (ACCNAME §2F).
    #[test]
    fn text_content_separates_element_children() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let btn = dom.create_element("button", Attributes::default());
        dom.append_child(root, btn);
        let span1 = dom.create_element("span", Attributes::default());
        dom.append_child(btn, span1);
        let t1 = dom.create_text("Hello");
        dom.append_child(span1, t1);
        let span2 = dom.create_element("span", Attributes::default());
        dom.append_child(btn, span2);
        let t2 = dom.create_text("World");
        dom.append_child(span2, t2);

        assert_eq!(
            compute_accessible_name(&dom, btn),
            Some("Hello World".to_string())
        );
    }
}
