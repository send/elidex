//! Element-only DOM traversal helpers for selector matching.

use elidex_ecs::{EcsDom, Entity, TagType};

/// Return the first child of `parent` that is an element (has `TagType`).
pub(super) fn first_element_child(dom: &EcsDom, parent: Entity) -> Option<Entity> {
    let mut child = dom.get_first_child(parent);
    while let Some(c) = child {
        if dom.world().get::<&TagType>(c).is_ok() {
            return Some(c);
        }
        child = dom.get_next_sibling(c);
    }
    None
}

/// Return the last child of `parent` that is an element (has `TagType`).
pub(super) fn last_element_child(dom: &EcsDom, parent: Entity) -> Option<Entity> {
    let mut child = dom.get_last_child(parent);
    while let Some(c) = child {
        if dom.world().get::<&TagType>(c).is_ok() {
            return Some(c);
        }
        child = dom.get_prev_sibling(c);
    }
    None
}

/// Return the previous sibling that is an element (has `TagType`).
pub(super) fn prev_element_sibling(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let mut current = dom.get_prev_sibling(entity);
    while let Some(sib) = current {
        if dom.world().get::<&TagType>(sib).is_ok() {
            return Some(sib);
        }
        current = dom.get_prev_sibling(sib);
    }
    None
}

/// Check if the entity is the root element (`<html>`).
///
/// The root element is the `<html>` tag whose parent is the document root
/// (an entity without a `TagType` component).
pub(super) fn is_root_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| t.0 == "html")
        && dom
            .get_parent(entity)
            .is_some_and(|p| dom.world().get::<&TagType>(p).is_err())
}
