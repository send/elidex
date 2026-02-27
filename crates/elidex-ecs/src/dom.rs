//! ECS DOM wrapper providing tree-manipulation API.
//!
//! # Tree invariants
//!
//! The DOM tree maintained by [`EcsDom`] guarantees:
//!
//! - **No cycles**: A node cannot be an ancestor of itself. All mutation
//!   methods (`append_child`, `insert_before`, `replace_child`) perform an
//!   ancestor walk to reject operations that would create cycles.
//! - **Consistent sibling links**: `prev_sibling` / `next_sibling` form a
//!   doubly-linked list among children of the same parent.
//! - **Parent ↔ child consistency**: A child's `parent` field always matches
//!   the parent whose `first_child` / `last_child` chain reaches it.
//! - **Destroyed entity safety**: Operations on entities that have been
//!   removed from the world via `destroy_entity` return `false` and never
//!   mutate the tree.

use crate::components::{Attributes, TagType, TextContent, TreeRelation};
use hecs::{Entity, World};

/// Maximum ancestor walk depth before assuming tree corruption.
///
/// Also used by `elidex-css` selector matching to cap descendant walks.
pub const MAX_ANCESTOR_DEPTH: usize = 10_000;

/// ECS-backed DOM storage.
///
/// Each DOM node is an `Entity` in the `hecs::World`, with component data
/// describing its type, attributes, tree relationships, and content.
///
/// See the module-level documentation for tree invariant guarantees.
pub struct EcsDom {
    world: World,
}

impl EcsDom {
    /// Create a new, empty DOM.
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    /// Provides read-only access to the underlying `hecs::World`.
    ///
    /// Use this for queries and component reads. Tree mutations **must** go
    /// through [`EcsDom`] methods to preserve invariants.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns `true` if the entity exists in this DOM world.
    #[must_use]
    pub fn contains(&self, entity: Entity) -> bool {
        self.world.contains(entity)
    }

    /// Provides mutable access to the underlying `hecs::World`.
    ///
    /// **Warning:** Tree mutations (parent/child/sibling links) **must** go
    /// through [`EcsDom`] methods to preserve invariants. Use this only for
    /// adding or modifying non-tree components (e.g., [`crate::InlineStyle`]).
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Create an element node with the given tag and attributes.
    pub fn create_element(&mut self, tag: impl Into<String>, attrs: Attributes) -> Entity {
        self.world
            .spawn((TagType(tag.into()), attrs, TreeRelation::default()))
    }

    /// Create a document root entity (no tag, only tree relations).
    ///
    /// The document root serves as the parent of the `<html>` element.
    pub fn create_document_root(&mut self) -> Entity {
        self.world.spawn((TreeRelation::default(),))
    }

    /// Create a text node.
    pub fn create_text(&mut self, text: impl Into<String>) -> Entity {
        self.world
            .spawn((TextContent(text.into()), TreeRelation::default()))
    }

    /// Append `child` as the last child of `parent`.
    ///
    /// If `child` already has a parent, it is first detached.
    /// Returns `false` if:
    /// - `parent == child` (self-append),
    /// - either entity has been destroyed,
    /// - `child` is an ancestor of `parent` (would create a cycle).
    #[must_use = "returns false if the operation failed"]
    pub fn append_child(&mut self, parent: Entity, child: Entity) -> bool {
        if parent == child {
            return false;
        }
        if !self.all_exist(&[parent, child]) {
            return false;
        }
        if self.is_ancestor(child, parent) {
            return false;
        }

        self.detach(child);

        let last_child = self.read_rel(parent, |rel| rel.last_child);
        self.link_node(parent, child, last_child, None);

        true
    }

    /// Remove `child` from `parent`.
    ///
    /// Returns `false` if either entity is destroyed or `child` is not a
    /// child of `parent`.
    #[must_use = "returns false if the operation failed"]
    pub fn remove_child(&mut self, parent: Entity, child: Entity) -> bool {
        if !self.all_exist(&[parent, child]) {
            return false;
        }
        if !self.is_child_of(child, parent) {
            return false;
        }
        self.detach(child);
        true
    }

    /// Insert `new_child` before `ref_child` under `parent`.
    ///
    /// Returns `false` if any entity is destroyed, `ref_child` is not a child
    /// of `parent`, `new_child == parent`, `new_child == ref_child`, or
    /// `new_child` is an ancestor of `parent` (would create a cycle).
    #[must_use = "returns false if the operation failed"]
    pub fn insert_before(&mut self, parent: Entity, new_child: Entity, ref_child: Entity) -> bool {
        if new_child == parent || new_child == ref_child {
            return false;
        }
        if !self.all_exist(&[parent, new_child, ref_child]) {
            return false;
        }
        if self.is_ancestor(new_child, parent) {
            return false;
        }
        if !self.is_child_of(ref_child, parent) {
            return false;
        }

        // Detach new_child from its current position.
        self.detach(new_child);

        // Re-read ref_child's prev_sibling AFTER detach (it may have changed
        // if new_child was an adjacent sibling).
        let ref_prev = self.read_rel(ref_child, |rel| rel.prev_sibling);
        self.link_node(parent, new_child, ref_prev, Some(ref_child));

        true
    }

    /// Replace `old_child` with `new_child` under `parent`.
    ///
    /// `old_child` is detached from the tree. Returns `false` if any entity
    /// is destroyed, `old_child` is not a child of `parent`, or `new_child`
    /// is an ancestor of `parent` (would create a cycle).
    ///
    /// Validation is performed **before** detaching `new_child`, so the tree
    /// is never left in a corrupted state on failure.
    #[must_use = "returns false if the operation failed"]
    pub fn replace_child(&mut self, parent: Entity, new_child: Entity, old_child: Entity) -> bool {
        if new_child == parent || new_child == old_child {
            return false;
        }
        if !self.all_exist(&[parent, new_child, old_child]) {
            return false;
        }
        if self.is_ancestor(new_child, parent) {
            return false;
        }

        // Verify old_child is a child of parent BEFORE detaching new_child.
        if !self.is_child_of(old_child, parent) {
            return false;
        }

        // Detach new_child from its current position (validation passed).
        self.detach(new_child);

        // Re-read old_child's siblings AFTER detach (they may have changed
        // if new_child was an adjacent sibling).
        let (old_prev, old_next) =
            self.read_rel(old_child, |rel| (rel.prev_sibling, rel.next_sibling));

        // Place new_child in old_child's position.
        self.link_node(parent, new_child, old_prev, old_next);

        // Clear old_child's tree links.
        self.clear_rel(old_child);

        true
    }

    /// Destroy an entity and remove it from the world entirely.
    ///
    /// The entity is first detached from its parent. Children are NOT
    /// recursively destroyed; they become clean orphans (parent and sibling
    /// links are cleared so they do not reference the destroyed entity).
    /// Returns `false` if the entity does not exist.
    #[must_use = "returns false if the entity does not exist"]
    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        if !self.contains(entity) {
            return false;
        }
        self.detach(entity);

        // Orphan all children: clear their parent and sibling links so they
        // do not hold dangling references to the destroyed entity.
        let first_child = self.read_rel(entity, |rel| rel.first_child);
        let mut child = first_child;
        while let Some(c) = child {
            let next = self.read_rel(c, |rel| rel.next_sibling);
            self.clear_rel(c);
            child = next;
        }

        let _ = self.world.despawn(entity);
        true
    }

    /// Place `node` into `parent`'s child list between `prev` and `next`.
    ///
    /// Updates all affected sibling pointers and parent's first/last child.
    fn link_node(
        &mut self,
        parent: Entity,
        node: Entity,
        prev: Option<Entity>,
        next: Option<Entity>,
    ) {
        self.update_rel(node, |rel| {
            rel.parent = Some(parent);
            rel.prev_sibling = prev;
            rel.next_sibling = next;
        });

        if let Some(prev_entity) = prev {
            self.update_rel(prev_entity, |rel| rel.next_sibling = Some(node));
        } else {
            self.update_rel(parent, |rel| rel.first_child = Some(node));
        }

        if let Some(next_entity) = next {
            self.update_rel(next_entity, |rel| rel.prev_sibling = Some(node));
        } else {
            self.update_rel(parent, |rel| rel.last_child = Some(node));
        }
    }

    // ---- Internal helpers ----

    /// Returns `true` if all given entities exist in this DOM world.
    fn all_exist(&self, entities: &[Entity]) -> bool {
        entities.iter().all(|e| self.world.contains(*e))
    }

    /// Check if `ancestor` is an ancestor of `descendant` by walking up the tree.
    ///
    /// Uses a depth counter to prevent infinite loops on corrupted trees.
    fn is_ancestor(&self, ancestor: Entity, descendant: Entity) -> bool {
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
    fn is_child_of(&self, child: Entity, parent: Entity) -> bool {
        self.world
            .get::<&TreeRelation>(child)
            .ok()
            .is_some_and(|rel| rel.parent == Some(parent))
    }

    /// Read a field from an entity's `TreeRelation` component.
    fn read_rel<R>(&self, entity: Entity, f: impl FnOnce(&TreeRelation) -> R) -> R
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
    fn update_rel(&mut self, entity: Entity, f: impl FnOnce(&mut TreeRelation)) {
        if let Ok(mut rel) = self.world.get::<&mut TreeRelation>(entity) {
            f(&mut rel);
        }
    }

    /// Clear parent and sibling links on an entity, preserving its own
    /// `first_child` / `last_child` (children stay with the node).
    fn clear_rel(&mut self, entity: Entity) {
        self.update_rel(entity, |rel| {
            rel.parent = None;
            rel.prev_sibling = None;
            rel.next_sibling = None;
        });
    }

    /// Returns the parent of `entity`, or `None` if it has no parent or does not exist.
    #[must_use]
    pub fn get_parent(&self, entity: Entity) -> Option<Entity> {
        self.read_rel(entity, |rel| rel.parent)
    }

    /// Returns the first child of `entity`, or `None` if it has no children or does not exist.
    #[must_use]
    pub fn get_first_child(&self, entity: Entity) -> Option<Entity> {
        self.read_rel(entity, |rel| rel.first_child)
    }

    /// Returns the last child of `entity`, or `None` if it has no children or does not exist.
    #[must_use]
    pub fn get_last_child(&self, entity: Entity) -> Option<Entity> {
        self.read_rel(entity, |rel| rel.last_child)
    }

    /// Returns the next sibling of `entity`, or `None` if it is the last sibling or does not exist.
    #[must_use]
    pub fn get_next_sibling(&self, entity: Entity) -> Option<Entity> {
        self.read_rel(entity, |rel| rel.next_sibling)
    }

    /// Returns the previous sibling of `entity`, or `None` if it is the first sibling or does not exist.
    #[must_use]
    pub fn get_prev_sibling(&self, entity: Entity) -> Option<Entity> {
        self.read_rel(entity, |rel| rel.prev_sibling)
    }

    /// Collect all direct children of `parent` in order.
    ///
    /// Uses a depth counter (capped at `MAX_ANCESTOR_DEPTH`) to prevent
    /// infinite loops on corrupted sibling chains.
    #[must_use]
    pub fn children(&self, parent: Entity) -> Vec<Entity> {
        let mut result = Vec::new();
        let mut current = self.read_rel(parent, |rel| rel.first_child);
        let mut count = 0;
        while let Some(entity) = current {
            count += 1;
            if count > MAX_ANCESTOR_DEPTH {
                break;
            }
            result.push(entity);
            current = self.read_rel(entity, |rel| rel.next_sibling);
        }
        result
    }

    /// Returns a zero-allocation iterator over direct children of `parent`.
    ///
    /// Yields entities in sibling order. Stops after `MAX_ANCESTOR_DEPTH`
    /// iterations to guard against corrupted sibling chains.
    #[must_use]
    pub fn children_iter(&self, parent: Entity) -> ChildrenIter<'_> {
        let next = self.read_rel(parent, |rel| rel.first_child);
        ChildrenIter {
            dom: self,
            next,
            remaining: MAX_ANCESTOR_DEPTH,
        }
    }

    /// Find all element entities with the given tag name.
    ///
    /// Comparison is **case-sensitive**. Callers should pass lowercase tag names
    /// to match the parser's normalized output.
    ///
    /// **Complexity:** O(n) full scan over all entities with a `TagType`
    /// component. Consider caching results or adding a tag→entity index if
    /// this becomes a hot path (e.g., CSS selector matching).
    #[must_use]
    pub fn query_by_tag(&self, tag: &str) -> Vec<Entity> {
        self.world
            .query::<&TagType>()
            .iter()
            .filter(|(_, t)| t.0 == tag)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Returns all entities that have no parent, sorted by entity ID.
    ///
    /// Useful for finding layout roots or document roots for tree walks.
    #[must_use]
    pub fn root_entities(&self) -> Vec<Entity> {
        let mut roots: Vec<Entity> = self
            .world
            .query::<()>()
            .iter()
            .map(|(entity, ())| entity)
            .filter(|&entity| self.get_parent(entity).is_none())
            .collect();
        roots.sort_by_key(|e| e.to_bits());
        roots
    }

    /// Detach an entity from its parent and siblings.
    fn detach(&mut self, entity: Entity) {
        let (parent, prev, next) = self.read_rel(entity, |rel| {
            (rel.parent, rel.prev_sibling, rel.next_sibling)
        });
        if parent.is_none() {
            return;
        }

        if let Some(prev_entity) = prev {
            self.update_rel(prev_entity, |rel| rel.next_sibling = next);
        }

        if let Some(next_entity) = next {
            self.update_rel(next_entity, |rel| rel.prev_sibling = prev);
        }

        if let Some(parent_entity) = parent {
            self.update_rel(parent_entity, |rel| {
                if rel.first_child == Some(entity) {
                    rel.first_child = next;
                }
                if rel.last_child == Some(entity) {
                    rel.last_child = prev;
                }
            });
        }

        self.clear_rel(entity);
    }
}

/// Zero-allocation iterator over direct children of a DOM node.
///
/// Created by [`EcsDom::children_iter()`].
pub struct ChildrenIter<'a> {
    dom: &'a EcsDom,
    next: Option<Entity>,
    remaining: usize,
}

impl Iterator for ChildrenIter<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let entity = self.next?;
        if self.remaining == 0 {
            self.next = None;
            return None;
        }
        self.remaining -= 1;
        self.next = self.dom.read_rel(entity, |rel| rel.next_sibling);
        Some(entity)
    }
}

impl Default for EcsDom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use crate::components::{Attributes, TextContent};

    fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn create_element() {
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let tags = dom.query_by_tag("div");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], div);
    }

    #[test]
    fn create_text_node() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("Hello, world!");
        let content = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(content.0, "Hello, world!");
    }

    #[test]
    fn append_and_children() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child1 = elem(&mut dom, "span");
        let child2 = elem(&mut dom, "p");

        dom.append_child(parent, child1);
        dom.append_child(parent, child2);

        let children = dom.children(parent);
        assert_eq!(children, vec![child1, child2]);
    }

    #[test]
    fn parent_relation() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent, child);
        assert_eq!(dom.get_parent(child), Some(parent));
    }

    #[test]
    fn remove_child_from_middle() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c);

        dom.remove_child(parent, b);

        let children = dom.children(parent);
        assert_eq!(children, vec![a, c]);
        assert_eq!(dom.get_parent(b), None);
    }

    #[test]
    fn remove_first_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.remove_child(parent, a);

        assert_eq!(dom.children(parent), vec![b]);
    }

    #[test]
    fn remove_last_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.remove_child(parent, b);

        assert_eq!(dom.children(parent), vec![a]);
    }

    #[test]
    fn remove_only_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent, child);
        dom.remove_child(parent, child);

        assert!(dom.children(parent).is_empty());
    }

    #[test]
    fn query_by_tag_multiple() {
        let mut dom = EcsDom::new();
        let _div1 = elem(&mut dom, "div");
        let _span = elem(&mut dom, "span");
        let _div2 = elem(&mut dom, "div");

        assert_eq!(dom.query_by_tag("div").len(), 2);
        assert_eq!(dom.query_by_tag("span").len(), 1);
        assert!(dom.query_by_tag("p").is_empty());
    }

    #[test]
    fn text_node_as_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let text = dom.create_text("Hello");

        dom.append_child(parent, text);

        assert_eq!(dom.children(parent), vec![text]);
        let content = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(content.0, "Hello");
    }

    #[test]
    fn reparenting_detaches_from_old_parent() {
        let mut dom = EcsDom::new();
        let parent_a = elem(&mut dom, "div");
        let parent_b = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent_a, child);
        assert_eq!(dom.children(parent_a), vec![child]);

        dom.append_child(parent_b, child);
        assert!(dom.children(parent_a).is_empty());
        assert_eq!(dom.children(parent_b), vec![child]);
    }

    #[test]
    fn self_append_rejected() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        assert!(!dom.append_child(e, e));
        assert!(dom.children(e).is_empty());
        assert_eq!(dom.get_parent(e), None);
    }

    #[test]
    fn remove_non_child_returns_false() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let unrelated = elem(&mut dom, "span");
        assert!(!dom.remove_child(parent, unrelated));
    }

    #[test]
    fn double_append_same_parent() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent, child);
        dom.append_child(parent, child);

        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn insert_before_first() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");

        dom.append_child(parent, b);
        assert!(dom.insert_before(parent, a, b));

        assert_eq!(dom.children(parent), vec![a, b]);
    }

    #[test]
    fn insert_before_middle() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, c);
        assert!(dom.insert_before(parent, b, c));

        assert_eq!(dom.children(parent), vec![a, b, c]);
    }

    #[test]
    fn insert_before_invalid_ref() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let unrelated = elem(&mut dom, "span");

        assert!(!dom.insert_before(parent, a, unrelated));
    }

    #[test]
    fn replace_child_basic() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "p");

        dom.append_child(parent, a);
        dom.append_child(parent, b);

        assert!(dom.replace_child(parent, c, b));
        assert_eq!(dom.children(parent), vec![a, c]);
        assert_eq!(dom.get_parent(b), None);
    }

    #[test]
    fn replace_only_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let old = elem(&mut dom, "span");
        let new = elem(&mut dom, "p");

        dom.append_child(parent, old);
        assert!(dom.replace_child(parent, new, old));

        assert_eq!(dom.children(parent), vec![new]);
    }

    #[test]
    fn replace_child_invalid() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let unrelated = elem(&mut dom, "span");

        assert!(!dom.replace_child(parent, a, unrelated));
    }

    #[test]
    fn destroy_entity_removes_from_world() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent, child);
        dom.destroy_entity(child);

        assert!(dom.children(parent).is_empty());
        assert!(!dom.contains(child));
    }

    #[test]
    fn destroy_detached_entity() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        dom.destroy_entity(e);
        assert!(dom.query_by_tag("div").is_empty());
    }

    #[test]
    fn circular_append_rejected() {
        let mut dom = EcsDom::new();
        let a = elem(&mut dom, "div");
        let b = elem(&mut dom, "span");

        dom.append_child(a, b);
        assert!(!dom.append_child(b, a));
        assert_eq!(dom.children(a), vec![b]);
        assert!(dom.children(b).is_empty());
    }

    #[test]
    fn circular_deep_rejected() {
        let mut dom = EcsDom::new();
        let a = elem(&mut dom, "div");
        let b = elem(&mut dom, "div");
        let c = elem(&mut dom, "div");

        dom.append_child(a, b);
        dom.append_child(b, c);
        assert!(!dom.append_child(c, a));
        assert_eq!(dom.children(b), vec![c]);
    }

    #[test]
    fn circular_insert_before_rejected() {
        let mut dom = EcsDom::new();
        let a = elem(&mut dom, "div");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "p");

        dom.append_child(a, b);
        dom.append_child(a, c);
        assert!(!dom.insert_before(b, a, c));
    }

    #[test]
    fn circular_replace_child_rejected() {
        let mut dom = EcsDom::new();
        let a = elem(&mut dom, "div");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "p");

        dom.append_child(a, b);
        dom.append_child(b, c);
        assert!(!dom.replace_child(b, a, c));
        assert_eq!(dom.children(b), vec![c]);
    }

    #[test]
    fn append_destroyed_parent_returns_false() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.destroy_entity(parent);
        assert!(!dom.append_child(parent, child));
    }

    #[test]
    fn append_destroyed_child_returns_false() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.destroy_entity(child);
        assert!(!dom.append_child(parent, child));
    }

    #[test]
    fn remove_destroyed_child_returns_false() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        dom.append_child(parent, child);
        dom.destroy_entity(child);
        assert!(!dom.remove_child(parent, child));
    }

    #[test]
    fn replace_child_validates_before_detach() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let existing = elem(&mut dom, "span");
        let new_child = elem(&mut dom, "p");
        let unrelated = elem(&mut dom, "em");

        dom.append_child(parent, existing);
        dom.append_child(parent, new_child);

        assert!(!dom.replace_child(parent, new_child, unrelated));
        assert_eq!(dom.children(parent), vec![existing, new_child]);
    }

    #[test]
    fn destroy_entity_returns_false_for_already_destroyed() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        assert!(dom.destroy_entity(e));
        assert!(!dom.destroy_entity(e));
    }

    #[test]
    fn sibling_links_consistent() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c);

        assert_eq!(dom.get_next_sibling(a), Some(b));
        assert_eq!(dom.get_prev_sibling(a), None);
        assert_eq!(dom.get_prev_sibling(b), Some(a));
        assert_eq!(dom.get_next_sibling(b), Some(c));
        assert_eq!(dom.get_prev_sibling(c), Some(b));
        assert_eq!(dom.get_next_sibling(c), None);
    }

    #[test]
    fn deep_tree() {
        let mut dom = EcsDom::new();
        let mut parent = elem(&mut dom, "div");
        let root = parent;

        for _ in 0..50 {
            let child = elem(&mut dom, "div");
            dom.append_child(parent, child);
            parent = child;
        }

        assert!(dom.children(parent).is_empty());
        assert_eq!(dom.children(root).len(), 1);
    }

    #[test]
    fn helper_methods() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c);

        assert_eq!(dom.get_parent(a), Some(parent));
        assert_eq!(dom.get_parent(parent), None);
        assert_eq!(dom.get_first_child(parent), Some(a));
        assert_eq!(dom.get_last_child(parent), Some(c));
        assert_eq!(dom.get_first_child(a), None);
        assert_eq!(dom.get_last_child(a), None);
        assert_eq!(dom.get_next_sibling(a), Some(b));
        assert_eq!(dom.get_next_sibling(b), Some(c));
        assert_eq!(dom.get_next_sibling(c), None);
        assert_eq!(dom.get_prev_sibling(a), None);
        assert_eq!(dom.get_prev_sibling(b), Some(a));
        assert_eq!(dom.get_prev_sibling(c), Some(b));
    }

    #[test]
    fn contains_method() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        assert!(dom.contains(e));
        dom.destroy_entity(e);
        assert!(!dom.contains(e));
    }

    #[test]
    fn attributes_accessors() {
        let mut attrs = Attributes::default();
        assert!(!attrs.contains("class"));
        assert_eq!(attrs.get("class"), None);

        attrs.set("class", "foo");
        assert!(attrs.contains("class"));
        assert_eq!(attrs.get("class"), Some("foo"));

        let old = attrs.set("class", "bar");
        assert_eq!(old, Some("foo".to_string()));
        assert_eq!(attrs.get("class"), Some("bar"));

        let removed = attrs.remove("class");
        assert_eq!(removed, Some("bar".to_string()));
        assert!(!attrs.contains("class"));
    }

    #[test]
    fn many_siblings() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let mut entities = Vec::new();

        for _ in 0..100 {
            let child = elem(&mut dom, "span");
            dom.append_child(parent, child);
            entities.push(child);
        }

        let children = dom.children(parent);
        assert_eq!(children.len(), 100);
        assert_eq!(children, entities);

        dom.remove_child(parent, entities[50]);
        let children = dom.children(parent);
        assert_eq!(children.len(), 99);
        assert!(!children.contains(&entities[50]));
    }

    #[test]
    fn insert_before_adjacent_prev_sibling() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c);

        assert!(dom.insert_before(parent, b, c));
        assert_eq!(dom.children(parent), vec![a, b, c]);
    }

    #[test]
    fn replace_child_adjacent_sibling() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);

        assert!(dom.replace_child(parent, b, a));
        assert_eq!(dom.children(parent), vec![b]);
        assert_eq!(dom.get_parent(a), None);
    }

    #[test]
    fn destroy_entity_orphans_children() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let c = elem(&mut dom, "span");

        dom.append_child(parent, a);
        dom.append_child(parent, b);
        dom.append_child(parent, c);

        dom.destroy_entity(parent);

        assert_eq!(dom.get_parent(a), None);
        assert_eq!(dom.get_parent(b), None);
        assert_eq!(dom.get_parent(c), None);
        assert_eq!(dom.get_next_sibling(a), None);
        assert_eq!(dom.get_prev_sibling(b), None);
        assert_eq!(dom.get_next_sibling(b), None);
        assert_eq!(dom.get_prev_sibling(c), None);
    }
}
