//! Tree mutation and navigation methods for [`EcsDom`].
//!
//! Split into cohesive submodules:
//! - [`mutation`] — relinking mutations (`append_child` / `insert_before` /
//!   `replace_child` / `remove_child`) and their `MutationEvent` fire sites.
//! - [`teardown`] — subtree destruction / despawn / re-home (`destroy_entity` /
//!   `despawn_subtree` / `adopt_subtree`).
//! - [`navigation`] — parent / child / sibling accessors, element-filtered and
//!   shadow-exposed navigation, tag helpers, and the children iterators.
//! - [`walkers`] — shadow-inclusive descendant / ancestor walks, connectedness,
//!   tree-order comparison, and tree-root resolution.
//!
//! This module itself hosts the low-level `TreeRelation` plumbing shared by all
//! four submodules — and by sibling `dom` modules such as `tree_clone` (hence
//! `pub(super)`, which from here resolves to the `dom` module).

mod mutation;
mod navigation;
mod teardown;
mod walkers;

use super::{EcsDom, MAX_ANCESTOR_DEPTH};
use crate::components::TreeRelation;
use hecs::Entity;

impl EcsDom {
    // ---- Internal helpers ----

    /// Returns `true` if all given entities exist in this DOM world.
    pub(super) fn all_exist(&self, entities: &[Entity]) -> bool {
        entities.iter().all(|e| self.world.contains(*e))
    }

    /// Check if `ancestor` is an ancestor of `descendant` by walking up the tree.
    ///
    /// Uses a depth counter to prevent infinite loops on corrupted trees.
    pub(super) fn is_ancestor(&self, ancestor: Entity, descendant: Entity) -> bool {
        let mut current = Some(descendant);
        let mut depth = 0;
        while let Some(entity) = current {
            if entity == ancestor {
                return true;
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
            current = self.get_parent(entity);
        }
        false
    }

    /// Returns `true` if `child`'s parent is `parent`.
    pub(super) fn is_child_of(&self, child: Entity, parent: Entity) -> bool {
        self.world
            .get::<&TreeRelation>(child)
            .ok()
            .is_some_and(|rel| rel.parent == Some(parent))
    }

    /// Read a field from an entity's `TreeRelation` component.
    pub(super) fn read_rel<R>(&self, entity: Entity, f: impl FnOnce(&TreeRelation) -> R) -> R
    where
        R: Default,
    {
        self.world
            .get::<&TreeRelation>(entity)
            .ok()
            .map(|rel| f(&rel))
            .unwrap_or_default()
    }

    /// Mutate an entity's `TreeRelation` component in-place.
    pub(super) fn update_rel(&mut self, entity: Entity, f: impl FnOnce(&mut TreeRelation)) {
        if let Ok(mut rel) = self.world.get::<&mut TreeRelation>(entity) {
            f(&mut rel);
        }
    }

    /// Clear parent and sibling links on an entity, preserving its own
    /// `first_child` / `last_child` (children stay with the node).
    pub(super) fn clear_rel(&mut self, entity: Entity) {
        self.update_rel(entity, |rel| {
            rel.parent = None;
            rel.prev_sibling = None;
            rel.next_sibling = None;
        });
    }
}
