//! Label association for form controls.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::FormControlState;

/// Resolve the `for` attribute of a `<label>` to a target form control entity.
///
/// Returns the entity with the matching `id` that has a `FormControlState`.
#[must_use]
pub fn resolve_label_for(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    let attrs = dom.world().get::<&Attributes>(label_entity).ok()?;
    let for_id = attrs.get("for")?;
    if for_id.is_empty() {
        return None;
    }

    // Search all entities for matching id with FormControlState.
    dom.world()
        .query::<(Entity, &Attributes, &FormControlState)>()
        .iter()
        .find(|(_, attrs, _)| attrs.get("id") == Some(for_id))
        .map(|(entity, _, _)| entity)
}

/// Find the first descendant form control of a label element.
///
/// Used when `<label>` wraps a control without a `for` attribute.
#[must_use]
pub fn find_label_target(dom: &EcsDom, label_entity: Entity) -> Option<Entity> {
    // First check for explicit `for` attribute.
    if let Some(target) = resolve_label_for(dom, label_entity) {
        return Some(target);
    }

    // Otherwise, search descendants for the first form control.
    find_first_descendant_control(dom, label_entity, 0)
}

/// Recursively find the first descendant with a `FormControlState`.
fn find_first_descendant_control(dom: &EcsDom, entity: Entity, depth: usize) -> Option<Entity> {
    if depth > MAX_ANCESTOR_DEPTH {
        return None;
    }
    let mut child = dom.get_first_child(entity)?;
    loop {
        if dom.world().get::<&FormControlState>(child).is_ok() {
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
