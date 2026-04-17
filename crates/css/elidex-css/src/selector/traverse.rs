//! CSS-specific selector traversal helpers.
//!
//! Element-only child / sibling navigation lives on [`EcsDom`]
//! (`first_element_child`, `last_element_child`, `prev_element_sibling`,
//! `next_element_sibling`) — this module keeps only the CSS-specific
//! predicates that the generic DOM layer would not know about.

use elidex_ecs::{EcsDom, Entity, TagType};

/// Return the previous sibling that is an element (has `TagType`).
///
/// Mirrors [`EcsDom::prev_element_sibling`]; kept as a module-local
/// helper so CSS-specific shadow-host fallback semantics (planned when
/// `:host` selectors need it) can evolve without disturbing the generic
/// DOM navigation surface.
pub(super) fn prev_element_sibling(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    dom.prev_element_sibling(entity)
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
