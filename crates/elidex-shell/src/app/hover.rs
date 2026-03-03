//! Hover chain collection and element state tracking.

use elidex_ecs::{ElementState as DomElementState, Entity, MAX_ANCESTOR_DEPTH};

/// Collect the hover chain: the entity itself and all its ancestors.
///
/// Depth-limited to [`MAX_ANCESTOR_DEPTH`] to guard against tree corruption.
pub(super) fn collect_hover_chain(dom: &elidex_ecs::EcsDom, entity: Entity) -> Vec<Entity> {
    let mut chain = Vec::new();
    let mut current = Some(entity);
    let mut depth = 0;
    while let Some(e) = current {
        if depth > MAX_ANCESTOR_DEPTH {
            break;
        }
        chain.push(e);
        current = dom.get_parent(e);
        depth += 1;
    }
    chain
}

/// Update the `ElementState` component on an entity, creating one if absent.
pub(super) fn update_element_state(
    dom: &mut elidex_ecs::EcsDom,
    entity: Entity,
    f: impl FnOnce(&mut DomElementState),
) {
    let mut state = dom
        .world()
        .get::<&DomElementState>(entity)
        .ok()
        .map_or(DomElementState::default(), |s| *s);
    f(&mut state);
    let _ = dom.world_mut().insert_one(entity, state);
}
