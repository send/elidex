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

use crate::components::{
    Attributes, ShadowHost, ShadowRoot, ShadowRootMode, SlotAssignment, TagType, TextContent,
    TreeRelation,
};
use hecs::{Entity, World};

/// Tags allowed as shadow hosts per WHATWG DOM §4.2.14.
/// Custom elements (valid custom element names) are also valid shadow hosts.
const VALID_SHADOW_HOST_TAGS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "body",
    "div",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "main",
    "nav",
    "p",
    "section",
    "span",
];

/// Reserved custom element names per HTML §4.13.2.
/// These contain a hyphen but are NOT valid custom element names.
const RESERVED_CUSTOM_ELEMENT_NAMES: &[&str] = &[
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "missing-glyph",
];

/// Check if a tag name is a valid custom element name per HTML §4.13.2.
///
/// A valid custom element name must:
/// - Start with a lowercase ASCII letter
/// - Contain a hyphen
/// - Not be a reserved name
/// - Contain no uppercase ASCII letters
/// - Contain only `PCENChar` characters (simplified: ASCII lowercase, digits,
///   `-`, `_`, `.`, and non-ASCII)
fn is_valid_custom_element_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.contains('-')
        && !RESERVED_CUSTOM_ELEMENT_NAMES.contains(&name)
        && name.chars().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || c == '-'
                || c == '_'
                || c == '.'
                || !c.is_ascii()
        })
}

/// Check if a tag name is valid as a shadow host (custom element or WHATWG whitelist).
fn is_valid_shadow_host(tag: &str) -> bool {
    is_valid_custom_element_name(tag) || VALID_SHADOW_HOST_TAGS.contains(&tag)
}

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
    ///
    /// Shadow DOM cleanup: if the entity is a shadow root, the host's
    /// `ShadowHost` component is removed. If the entity is a shadow host,
    /// the shadow root's `ShadowRoot` component is removed. This prevents
    /// stale cross-references after destruction.
    ///
    /// Returns `false` if the entity does not exist.
    #[must_use = "returns false if the entity does not exist"]
    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        if !self.contains(entity) {
            return false;
        }

        // Clean up shadow DOM cross-references before despawn.
        // Extract references first to avoid borrow conflicts.
        let shadow_host_of = self.world.get::<&ShadowRoot>(entity).ok().map(|sr| sr.host);
        let shadow_root_of = self
            .world
            .get::<&ShadowHost>(entity)
            .ok()
            .map(|sh| sh.shadow_root);
        // If destroying a shadow root, remove ShadowHost from the host.
        if let Some(host) = shadow_host_of {
            let _ = self.world.remove_one::<ShadowHost>(host);
        }
        // If destroying a shadow host, remove ShadowRoot from the shadow root.
        if let Some(sr) = shadow_root_of {
            let _ = self.world.remove_one::<ShadowRoot>(sr);
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

    /// Check if the entity's tag matches `tag`.
    ///
    /// Returns `false` for text nodes and entities without a `TagType` component.
    #[must_use]
    pub fn has_tag(&self, entity: Entity, tag: &str) -> bool {
        self.world
            .get::<&TagType>(entity)
            .ok()
            .is_some_and(|t| t.0 == tag)
    }

    /// Check if `ancestor` is an ancestor of `descendant` (or is `descendant` itself).
    ///
    /// Uses a depth counter to prevent infinite loops on corrupted trees.
    #[must_use]
    pub fn is_ancestor_or_self(&self, ancestor: Entity, descendant: Entity) -> bool {
        self.is_ancestor(ancestor, descendant)
    }

    /// Find the tree root of an entity.
    ///
    /// For nodes inside a shadow tree, the root is the `ShadowRoot` entity
    /// (not the document root). For normal DOM nodes, it's the topmost ancestor.
    /// If `entity` itself is a `ShadowRoot`, returns `entity`.
    #[must_use]
    pub fn find_tree_root(&self, entity: Entity) -> Entity {
        // L7: If entity itself is a ShadowRoot, it IS the tree root.
        if self.world.get::<&ShadowRoot>(entity).is_ok() {
            return entity;
        }
        let mut current = entity;
        let mut depth = 0;
        while let Some(parent) = self.get_parent(current) {
            if self.world.get::<&ShadowRoot>(parent).is_ok() {
                return parent;
            }
            current = parent;
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
        }
        current
    }

    /// Collect all direct children of `parent` in order.
    ///
    /// Shadow root entities are excluded from the result — use
    /// [`get_shadow_root()`](Self::get_shadow_root) to access them.
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
            // M1: ShadowRoot entities are internal — not exposed as children.
            if self.world.get::<&ShadowRoot>(entity).is_err() {
                result.push(entity);
            }
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

    /// Attach a shadow root to the given host element.
    ///
    /// Creates a new shadow root entity as a child of `host` and marks
    /// `host` with a `ShadowHost` component. Returns the shadow root entity.
    ///
    /// Returns `Err(())` if:
    /// - The host element's tag is not in the valid shadow host list (WHATWG DOM §4.2.14)
    /// - The host already has a shadow root attached
    /// - The entity does not exist or has no `TagType`
    #[must_use = "returns Err if the operation failed"]
    #[allow(clippy::result_unit_err)] // WHATWG convention: attach_shadow fails with no useful error detail.
    pub fn attach_shadow(&mut self, host: Entity, mode: ShadowRootMode) -> Result<Entity, ()> {
        // Validate host exists and has a valid tag per WHATWG DOM §4.2.14.
        let tag = self.world.get::<&TagType>(host).map_err(|_| ())?.0.clone();
        if !is_valid_shadow_host(&tag) {
            return Err(());
        }

        // Reject if already a shadow host.
        if self.world.get::<&ShadowHost>(host).is_ok() {
            return Err(());
        }

        // Create shadow root entity.
        let shadow_root_entity = self
            .world
            .spawn((ShadowRoot { mode, host }, TreeRelation::default()));

        // Attach shadow root as child of host.
        if !self.append_child(host, shadow_root_entity) {
            let _ = self.world.despawn(shadow_root_entity);
            return Err(());
        }

        // Mark host.
        let _ = self.world.insert_one(
            host,
            ShadowHost {
                shadow_root: shadow_root_entity,
            },
        );

        Ok(shadow_root_entity)
    }

    /// Returns the shadow root entity for the given host, if any.
    ///
    /// Returns `None` if the shadow root entity has been destroyed (stale reference).
    #[must_use]
    pub fn get_shadow_root(&self, host: Entity) -> Option<Entity> {
        self.world
            .get::<&ShadowHost>(host)
            .ok()
            .map(|sh| sh.shadow_root)
            .filter(|&sr| self.world.contains(sr))
    }

    /// Returns the composed children for layout/render traversal.
    ///
    /// - Shadow host → shadow root's children (skip shadow root entity itself)
    /// - `<slot>` with `SlotAssignment` → assigned nodes (or fallback: slot's own children)
    /// - Otherwise → normal `children()`
    #[must_use]
    pub fn composed_children(&self, entity: Entity) -> Vec<Entity> {
        // If entity is a shadow host, return shadow tree content.
        // Verify shadow root still exists (stale reference safety).
        if let Ok(sh) = self.world.get::<&ShadowHost>(entity) {
            if self.world.contains(sh.shadow_root) {
                return self.children(sh.shadow_root);
            }
            // Stale shadow root — fall through to normal children.
        }

        // If entity is a <slot> with SlotAssignment, return assigned nodes.
        if let Ok(slot) = self.world.get::<&SlotAssignment>(entity) {
            if !slot.assigned_nodes.is_empty() {
                return slot.assigned_nodes.clone();
            }
            // Fallback: slot's own children (default content).
            return self.children(entity);
        }

        self.children(entity)
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
        loop {
            let entity = self.next?;
            if self.remaining == 0 {
                self.next = None;
                return None;
            }
            self.remaining -= 1;
            self.next = self.dom.read_rel(entity, |rel| rel.next_sibling);
            // M1: Skip ShadowRoot entities — not exposed as children.
            if self.dom.world.get::<&ShadowRoot>(entity).is_err() {
                return Some(entity);
            }
        }
    }
}

impl Default for EcsDom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
