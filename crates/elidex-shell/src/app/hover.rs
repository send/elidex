//! Hover chain collection and element state tracking.

use std::collections::HashSet;

use elidex_ecs::{ElementState as DomElementState, Entity, MAX_ANCESTOR_DEPTH};

/// Collect the hover chain: the entity itself and all its ancestors.
///
/// Depth-limited to [`MAX_ANCESTOR_DEPTH`] to guard against tree corruption.
pub(crate) fn collect_hover_chain(dom: &elidex_ecs::EcsDom, entity: Entity) -> Vec<Entity> {
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

/// Apply a hover chain diff: remove `HOVER` from entities that left the chain,
/// add `HOVER` to entities that entered the chain.
///
/// Returns the new chain for storage.
pub(crate) fn apply_hover_diff(
    dom: &mut elidex_ecs::EcsDom,
    old_chain: &[Entity],
    new_chain: &[Entity],
) {
    let new_set: HashSet<Entity> = new_chain.iter().copied().collect();
    let old_set: HashSet<Entity> = old_chain.iter().copied().collect();

    for &e in old_chain {
        if !new_set.contains(&e) {
            update_element_state(dom, e, |s| {
                s.remove(DomElementState::HOVER);
            });
        }
    }
    for &e in new_chain {
        if !old_set.contains(&e) {
            update_element_state(dom, e, |s| {
                s.insert(DomElementState::HOVER);
            });
        }
    }
}

/// Update the `ElementState` component on an entity, creating one if absent.
pub(crate) fn update_element_state(
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
