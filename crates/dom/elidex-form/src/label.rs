//! Label association for form controls.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::FormControlState;

/// Returns `true` if the entity is a "labelable element" per HTML
/// §4.10.4 — `<button>`, `<input>` (any non-hidden type),
/// `<meter>`, `<output>`, `<progress>`, `<select>`, `<textarea>`.
/// Tag-based check rather than `FormControlState`-based so a
/// JS-created element (`document.createElement('input')`) without
/// the side-table component still resolves correctly.
#[must_use]
pub fn is_labelable_element(dom: &EcsDom, entity: Entity) -> bool {
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    matches!(
        tag.0.as_str(),
        "button" | "input" | "meter" | "output" | "progress" | "select" | "textarea"
    )
}

/// Resolve the `for` attribute of a `<label>` to a target form
/// control entity.  Returns the entity with the matching `id` whose
/// tag is a labelable element (HTML §4.10.4), preferring entities
/// that already carry [`FormControlState`].
#[must_use]
pub fn resolve_label_for(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    let for_id: String = {
        let attrs = dom.world().get::<&Attributes>(label_entity).ok()?;
        let v = attrs.get("for")?;
        if v.is_empty() {
            return None;
        }
        v.to_owned()
    };

    // Walk every entity carrying an `id` attribute; accept the first
    // labelable match.  Falls back to `FormControlState` membership
    // when the labelable check rejects (defensive — older state-only
    // pathways stay observable).
    dom.world()
        .query::<(Entity, &Attributes)>()
        .iter()
        .find(|(entity, attrs)| {
            attrs.get("id") == Some(for_id.as_str())
                && (is_labelable_element(dom, *entity)
                    || dom.world().get::<&FormControlState>(*entity).is_ok())
        })
        .map(|(entity, _)| entity)
}

/// Find the first descendant labelable element of a label element.
///
/// Used when `<label>` wraps a control without a `for` attribute.
#[must_use]
pub fn find_label_target(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    // First check for explicit `for` attribute.
    if let Some(target) = resolve_label_for(dom, label_entity) {
        return Some(target);
    }

    // Otherwise, search descendants for the first labelable element.
    find_first_descendant_control(dom, label_entity, 0)
}

/// Recursively find the first descendant that is a labelable element
/// (HTML §4.10.4) or already carries a [`FormControlState`].
fn find_first_descendant_control(dom: &EcsDom, entity: Entity, depth: usize) -> Option<Entity> {
    if depth > MAX_ANCESTOR_DEPTH {
        return None;
    }
    let mut child = dom.get_first_child(entity)?;
    loop {
        if is_labelable_element(dom, child) || dom.world().get::<&FormControlState>(child).is_ok() {
            return Some(child);
        }
        // Recurse into subtree.
        if let Some(found) = find_first_descendant_control(dom, child, depth + 1) {
            return Some(found);
        }
        child = dom.get_next_sibling(child)?;
    }
}

/// Check if an entity is a `<label>` element.
#[must_use]
pub fn is_label(dom: &EcsDom, entity: Entity) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| t.0 == "label")
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::EcsDom;

    #[test]
    fn resolve_label_for_attribute() {
        let mut dom = EcsDom::new();

        // Create input with id="name"
        let mut input_attrs = Attributes::default();
        input_attrs.set("id", "name");
        let input = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );

        // Create label with for="name"
        let mut label_attrs = Attributes::default();
        label_attrs.set("for", "name");
        let label = dom.create_element("label", label_attrs);

        assert_eq!(resolve_label_for(&dom, label), Some(input));
    }

    #[test]
    fn find_label_target_wrapping() {
        let mut dom = EcsDom::new();

        let label = dom.create_element("label", Attributes::default());

        let input_attrs = Attributes::default();
        let input = dom.create_element("input", input_attrs.clone());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState::from_element("input", &input_attrs).unwrap(),
        );

        let _ = dom.append_child(label, input);

        assert_eq!(find_label_target(&dom, label), Some(input));
    }

    #[test]
    fn no_label_target() {
        let mut dom = EcsDom::new();
        let label = dom.create_element("label", Attributes::default());
        assert_eq!(find_label_target(&dom, label), None);
    }
}
