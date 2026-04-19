//! Tree mutation and navigation methods for [`EcsDom`].

use crate::components::{
    Attributes, CommentData, DocTypeData, NodeKind, ShadowRoot, TagType, TextContent, TreeRelation,
};
use hecs::Entity;

use super::{EcsDom, MAX_ANCESTOR_DEPTH};

impl EcsDom {
    // ---- Tree mutation ----

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
        self.rev_version(parent);

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
        self.rev_version(parent);
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
        self.rev_version(parent);

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
        self.rev_version(parent);

        true
    }

    /// Destroy an entity and remove it from the world entirely.
    ///
    /// The entity is first detached from its parent. Children are NOT
    /// recursively destroyed; they become clean orphans (parent and sibling
    /// links are cleared so they do not hold dangling references to the destroyed entity).
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
            .get::<&crate::components::ShadowHost>(entity)
            .ok()
            .map(|sh| sh.shadow_root);
        // If destroying a shadow root, remove ShadowHost from the host.
        if let Some(host) = shadow_host_of {
            let _ = self.world.remove_one::<crate::components::ShadowHost>(host);
        }
        // If destroying a shadow host, remove ShadowRoot from the shadow root.
        if let Some(sr) = shadow_root_of {
            let _ = self.world.remove_one::<ShadowRoot>(sr);
        }

        // Capture parent for version bump before detach.
        let parent = self.get_parent(entity);

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

        // Bump version on parent after successful removal.
        if let Some(p) = parent {
            self.rev_version(p);
        }

        true
    }

    /// Place `node` into `parent`'s child list between `prev` and `next`.
    ///
    /// Updates all affected sibling pointers and parent's first/last child.
    pub(super) fn link_node(
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

    /// Detach an entity from its parent and siblings.
    pub(super) fn detach(&mut self, entity: Entity) {
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

    // ---- Tree navigation ----

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

    /// Return the first child of `parent` that is an element (has a
    /// [`TagType`] component).  Text, comment, and shadow-root
    /// children are skipped.
    #[must_use]
    pub fn first_element_child(&self, parent: Entity) -> Option<Entity> {
        let mut child = self.get_first_child(parent);
        while let Some(c) = child {
            if self.world.get::<&TagType>(c).is_ok() {
                return Some(c);
            }
            child = self.get_next_sibling(c);
        }
        None
    }

    /// Return the last child of `parent` that is an element (has a
    /// [`TagType`] component).
    #[must_use]
    pub fn last_element_child(&self, parent: Entity) -> Option<Entity> {
        let mut child = self.get_last_child(parent);
        while let Some(c) = child {
            if self.world.get::<&TagType>(c).is_ok() {
                return Some(c);
            }
            child = self.get_prev_sibling(c);
        }
        None
    }

    /// Return the next sibling of `entity` that is an element.
    #[must_use]
    pub fn next_element_sibling(&self, entity: Entity) -> Option<Entity> {
        let mut current = self.get_next_sibling(entity);
        while let Some(sib) = current {
            if self.world.get::<&TagType>(sib).is_ok() {
                return Some(sib);
            }
            current = self.get_next_sibling(sib);
        }
        None
    }

    /// Return the previous sibling of `entity` that is an element.
    #[must_use]
    pub fn prev_element_sibling(&self, entity: Entity) -> Option<Entity> {
        let mut current = self.get_prev_sibling(entity);
        while let Some(sib) = current {
            if self.world.get::<&TagType>(sib).is_ok() {
                return Some(sib);
            }
            current = self.get_prev_sibling(sib);
        }
        None
    }

    /// Return the next sibling of `entity` that is **exposed** as a
    /// DOM child — i.e. does not carry a [`ShadowRoot`] component.
    /// Mirrors the filtering `children_iter` applies, for walks that
    /// start from a specific sibling rather than the parent's first
    /// child.
    #[must_use]
    pub fn next_exposed_sibling(&self, entity: Entity) -> Option<Entity> {
        let mut current = self.get_next_sibling(entity);
        while let Some(sib) = current {
            if self.world.get::<&ShadowRoot>(sib).is_err() {
                return Some(sib);
            }
            current = self.get_next_sibling(sib);
        }
        None
    }

    /// Symmetric partner of [`Self::next_exposed_sibling`] — walks
    /// the prev-sibling chain, skipping shadow-root entities.
    #[must_use]
    pub fn prev_exposed_sibling(&self, entity: Entity) -> Option<Entity> {
        let mut current = self.get_prev_sibling(entity);
        while let Some(sib) = current {
            if self.world.get::<&ShadowRoot>(sib).is_err() {
                return Some(sib);
            }
            current = self.get_prev_sibling(sib);
        }
        None
    }

    /// Return the first direct child of `parent` whose tag name
    /// matches `tag` (ASCII case-insensitive).  Non-element children
    /// are skipped.
    #[must_use]
    pub fn first_child_with_tag(&self, parent: Entity, tag: &str) -> Option<Entity> {
        for child in self.children_iter(parent) {
            if let Ok(tag_comp) = self.world.get::<&TagType>(child) {
                if tag_comp.0.eq_ignore_ascii_case(tag) {
                    return Some(child);
                }
            }
        }
        None
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

    /// Returns the tag name of an entity, or `None` for text nodes.
    #[must_use]
    pub fn get_tag_name(&self, entity: Entity) -> Option<String> {
        self.world.get::<&TagType>(entity).ok().map(|t| t.0.clone())
    }

    /// Compare two entities by tree order (pre-order depth-first traversal).
    ///
    /// Walks ancestor chains to find the lowest common ancestor, then compares
    /// sibling order. Handles the case where one node is an ancestor of the other.
    /// Falls back to entity bits comparison if the nodes are in different trees.
    #[must_use]
    pub fn tree_order_cmp(&self, a: Entity, b: Entity) -> std::cmp::Ordering {
        if a == b {
            return std::cmp::Ordering::Equal;
        }
        // Build ancestor chains (child -> root order).
        let chain_a = self.ancestor_chain(a);
        let chain_b = self.ancestor_chain(b);
        // Find the lowest common ancestor by walking from the root end.
        // chain[last] is the root.
        let mut ia = chain_a.len();
        let mut ib = chain_b.len();
        // If roots differ, nodes are in different trees -- fall back to entity bits.
        if chain_a.last() != chain_b.last() {
            return a.to_bits().cmp(&b.to_bits());
        }
        // Walk from root toward leaves, finding where paths diverge.
        loop {
            if ia == 0 {
                // `a` is an ancestor of `b` -> `a` comes first.
                return std::cmp::Ordering::Less;
            }
            if ib == 0 {
                // `b` is an ancestor of `a` -> `b` comes first.
                return std::cmp::Ordering::Greater;
            }
            ia -= 1;
            ib -= 1;
            if chain_a[ia] != chain_b[ib] {
                // These are siblings under the same parent -- compare sibling order.
                return self.sibling_order(chain_a[ia], chain_b[ib]);
            }
        }
    }

    /// Build the ancestor chain from entity to root (entity first, root last).
    fn ancestor_chain(&self, entity: Entity) -> Vec<Entity> {
        let mut chain = vec![entity];
        let mut current = entity;
        let mut depth = 0;
        while let Some(parent) = self.get_parent(current) {
            chain.push(parent);
            current = parent;
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
        }
        chain
    }

    /// Compare sibling order: walk from `first_child` of their parent.
    /// Returns `Less` if `a` appears before `b`, `Greater` otherwise.
    fn sibling_order(&self, a: Entity, b: Entity) -> std::cmp::Ordering {
        // Walk from `a` forward; if we find `b`, then a < b.
        let mut cursor = self.get_next_sibling(a);
        let mut steps = 0;
        while let Some(sib) = cursor {
            if sib == b {
                return std::cmp::Ordering::Less;
            }
            cursor = self.get_next_sibling(sib);
            steps += 1;
            if steps > 100_000 {
                break;
            }
        }
        // `b` was not found after `a`, so `b` must come before `a`.
        std::cmp::Ordering::Greater
    }

    /// Check if `ancestor` is an ancestor of `descendant` (or is `descendant` itself).
    ///
    /// Uses a depth counter to prevent infinite loops on corrupted trees.
    #[must_use]
    pub fn is_ancestor_or_self(&self, ancestor: Entity, descendant: Entity) -> bool {
        self.is_ancestor(ancestor, descendant)
    }

    /// Inclusive light-tree ancestor check: like
    /// [`Self::is_ancestor_or_self`] but stops at the shadow
    /// boundary.  A [`ShadowRoot`] component terminates the walk —
    /// hosts therefore do **not** light-tree-contain their shadow
    /// descendants, matching WHATWG "descendant" semantics (§4.2.1).
    /// Queries where the ancestor **is** the shadow root itself
    /// still succeed — `entity == ancestor` is checked before the
    /// boundary test on each iteration.
    ///
    /// Used by `Node.prototype.contains` (WHATWG §4.4.2) so
    /// `host.contains(nodeInShadowTree)` returns `false` in step
    /// with `childNodes`, `firstChild`, and the rest of the
    /// light-tree navigation surface.
    #[must_use]
    pub fn is_light_tree_ancestor_or_self(&self, ancestor: Entity, descendant: Entity) -> bool {
        let mut current = Some(descendant);
        let mut depth = 0;
        while let Some(entity) = current {
            if entity == ancestor {
                return true;
            }
            // Shadow root is not a light-tree child of its host, so
            // the walk stops here even if `ancestor` lies above.
            if self.world.get::<&ShadowRoot>(entity).is_ok() {
                return false;
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
            current = self.get_parent(entity);
        }
        false
    }

    /// Like `is_ancestor_or_self`, but when the walk reaches a `ShadowRoot`,
    /// it jumps to the host element and continues upward (host-including
    /// inclusive ancestor per WHATWG DOM 4.2.1).
    #[must_use]
    pub fn is_host_including_ancestor_or_self(&self, ancestor: Entity, descendant: Entity) -> bool {
        if ancestor == descendant {
            return true;
        }
        let mut current = descendant;
        let mut depth = 0;
        loop {
            if let Some(parent) = self.get_parent(current) {
                if parent == ancestor {
                    return true;
                }
                current = parent;
            } else if let Ok(sr) = self.world.get::<&ShadowRoot>(current) {
                let host = sr.host;
                drop(sr);
                if host == ancestor {
                    return true;
                }
                current = host;
            } else {
                return false;
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                return false;
            }
        }
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

    /// Like `find_tree_root`, but when reaching a `ShadowRoot`, jumps to
    /// the host and continues upward (composed tree root).
    #[must_use]
    pub fn find_tree_root_composed(&self, entity: Entity) -> Entity {
        let mut current = entity;
        let mut depth = 0;
        loop {
            if let Some(parent) = self.get_parent(current) {
                current = parent;
            } else if let Ok(sr) = self.world.get::<&ShadowRoot>(current) {
                current = sr.host;
            } else {
                return current;
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                return current;
            }
        }
    }

    /// Collect all direct children of `parent` in order.
    ///
    /// Shadow root entities are excluded from the result -- use
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
            // M1: ShadowRoot entities are internal -- not exposed as children.
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
    pub fn children_iter(&self, parent: Entity) -> super::ChildrenIter<'_> {
        let next = self.read_rel(parent, |rel| rel.first_child);
        super::ChildrenIter {
            dom: self,
            next,
            remaining: MAX_ANCESTOR_DEPTH,
        }
    }

    /// Returns a zero-allocation **reverse** iterator over direct
    /// children of `parent` (last child first, via `prev_sibling`).
    ///
    /// Like [`Self::children_iter`], internal [`ShadowRoot`] entities
    /// are skipped and iteration caps at [`MAX_ANCESTOR_DEPTH`].
    #[must_use]
    pub fn children_iter_rev(&self, parent: Entity) -> super::ChildrenIterRev<'_> {
        let next = self.read_rel(parent, |rel| rel.last_child);
        super::ChildrenIterRev {
            dom: self,
            next,
            remaining: MAX_ANCESTOR_DEPTH,
        }
    }

    /// Pre-order DFS over descendants of `root` (excluding `root`
    /// itself).  Traversal uses [`Self::children_iter_rev`], so it
    /// respects shadow boundaries — shadow-root subtrees are not
    /// entered.  `visitor` receives each entity in document order
    /// and returns `true` to continue or `false` to stop early.
    pub fn traverse_descendants(&self, root: Entity, mut visitor: impl FnMut(Entity) -> bool) {
        let mut stack: Vec<Entity> = self.children_iter_rev(root).collect();
        while let Some(entity) = stack.pop() {
            if !visitor(entity) {
                return;
            }
            stack.extend(self.children_iter_rev(entity));
        }
    }

    /// Find the first descendant of `root` whose `id` attribute equals
    /// `id`.  Searches in document order (pre-order DFS) and returns on
    /// first match — WHATWG DOM §4.2.4.
    #[must_use]
    pub fn find_by_id(&self, root: Entity, id: &str) -> Option<Entity> {
        let mut result = None;
        self.traverse_descendants(root, |entity| {
            if let Ok(attrs) = self.world().get::<&Attributes>(entity) {
                if attrs.get("id") == Some(id) {
                    result = Some(entity);
                    return false;
                }
            }
            true
        });
        result
    }

    /// Find all element entities with the given tag name.
    ///
    /// Comparison is **case-sensitive**. Callers should pass lowercase tag names
    /// to match the parser's normalized output.
    ///
    /// **Complexity:** O(n) full scan over all entities with a `TagType`
    /// component. Consider caching results or adding a tag->entity index if
    /// this becomes a hot path (e.g., CSS selector matching).
    #[must_use]
    pub fn query_by_tag(&self, tag: &str) -> Vec<Entity> {
        self.world
            .query::<(Entity, &TagType)>()
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
            .query::<Entity>()
            .iter()
            .filter(|&entity| self.get_parent(entity).is_none())
            .collect();
        roots.sort_by_key(|e| e.to_bits());
        roots
    }

    // ---- Node cloning (WHATWG DOM §4.5) ----

    /// Copy every attribute from `src` to `dst`, creating an
    /// [`Attributes`] component on `dst` if one does not exist.
    /// No-op if `src` has no attributes or if either entity has been
    /// destroyed.
    pub fn clone_attributes(&mut self, src: Entity, dst: Entity) {
        if !self.all_exist(&[src, dst]) {
            return;
        }
        let snapshot: Option<Attributes> = self
            .world
            .get::<&Attributes>(src)
            .ok()
            .map(|attrs| (*attrs).clone());
        let Some(attrs) = snapshot else { return };
        if self.world.get::<&Attributes>(dst).is_ok() {
            if let Ok(mut existing) = self.world.get::<&mut Attributes>(dst) {
                *existing = attrs;
            }
        } else {
            let _ = self.world.insert_one(dst, attrs);
        }
    }

    /// Copy character data from `src` to `dst`.  Only acts when both
    /// entities share the same character-data [`NodeKind`] —
    /// `Text` → `Text` copies [`TextContent`], `Comment` → `Comment`
    /// copies [`CommentData`].  Mismatched kinds (and non-character-data
    /// entities) are no-ops so the destination never ends up with
    /// both a `TextContent` and a `CommentData` component.
    ///
    /// Currently unused by [`Self::clone_subtree`] — which handles
    /// character data internally when allocating the cloned root —
    /// but kept as a public helper for ad-hoc cross-entity payload
    /// copies.
    pub fn clone_character_data(&mut self, src: Entity, dst: Entity) {
        if !self.all_exist(&[src, dst]) {
            return;
        }
        let src_kind = self.node_kind(src);
        let dst_kind = self.node_kind(dst);
        match (src_kind, dst_kind) {
            (Some(NodeKind::Text), Some(NodeKind::Text)) => {
                let Some(text) = self
                    .world
                    .get::<&TextContent>(src)
                    .ok()
                    .map(|t| t.0.clone())
                else {
                    return;
                };
                if self.world.get::<&TextContent>(dst).is_ok() {
                    if let Ok(mut existing) = self.world.get::<&mut TextContent>(dst) {
                        existing.0 = text;
                    }
                } else {
                    let _ = self.world.insert_one(dst, TextContent(text));
                }
            }
            (Some(NodeKind::Comment), Some(NodeKind::Comment)) => {
                let Some(data) = self
                    .world
                    .get::<&CommentData>(src)
                    .ok()
                    .map(|c| c.0.clone())
                else {
                    return;
                };
                if self.world.get::<&CommentData>(dst).is_ok() {
                    if let Ok(mut existing) = self.world.get::<&mut CommentData>(dst) {
                        existing.0 = data;
                    }
                } else {
                    let _ = self.world.insert_one(dst, CommentData(data));
                }
            }
            _ => {}
        }
    }

    /// Recursively clone `src` and its descendants, returning the
    /// new root entity (WHATWG DOM §4.5 "clone a node").
    ///
    /// Invariants:
    /// - The returned root has default tree links
    ///   (`parent: None`, `prev_sibling: None`, `next_sibling: None`)
    ///   — i.e. the clone is a fresh orphan.
    /// - Descendant tree links mirror the source structure between
    ///   the newly-allocated entities (parent and sibling pointers
    ///   point inside the clone, never into the source subtree).
    /// - `ShadowRoot` / `ShadowHost` components are **not** copied —
    ///   closed shadow trees clone only the host's light tree.
    /// - `EventListeners` (and any other script-session-owned state
    ///   that lives outside the ECS) is **not** copied — those live
    ///   in `SessionCore`, which is outside this crate, and WHATWG
    ///   §4.5 specifies listeners are not cloned.
    ///
    /// Returns `None` when `src` does not exist, so a missing source
    /// can never alias the original via the returned handle.
    #[must_use = "returns None when src does not exist"]
    pub fn clone_subtree(&mut self, src: Entity) -> Option<Entity> {
        if !self.contains(src) {
            return None;
        }
        let root = self.clone_node_shallow(src);
        // Walk original children and append clones in order.  Recursing
        // here is simpler than maintaining a parallel stack of
        // (src, dst) pairs — depth is bounded by
        // `MAX_ANCESTOR_DEPTH` transitively because DOM trees are
        // bounded.
        self.clone_children_recursive(src, root);
        Some(root)
    }

    /// Allocate a new entity carrying the same [`NodeKind`] and core
    /// component data as `src` (attributes for Elements, text/comment
    /// data for CharacterData, doctype fields for DocumentType).
    ///
    /// Tree relations are left at defaults — the caller threads the
    /// clone into place.  Does **not** copy `ShadowRoot` or
    /// `EventListeners`.
    fn clone_node_shallow(&mut self, src: Entity) -> Entity {
        // Determine node kind and snapshot the payload under a read
        // borrow, then spawn under a mutable borrow.  Spawning a bare
        // tuple first, then inserting components one at a time, keeps
        // the hecs API happy (each component needs its own `insert_one`
        // call).
        let kind = self.node_kind(src).unwrap_or(NodeKind::Element);
        // `.map(|t| (*t).clone())` — `*` dereferences the hecs `Ref`
        // so `clone()` clones the *data* rather than the reference
        // handle (which would keep `self.world` borrowed and block
        // the subsequent `spawn`).
        let tag = self.world.get::<&TagType>(src).ok().map(|t| (*t).clone());
        let text = self
            .world
            .get::<&TextContent>(src)
            .ok()
            .map(|t| (*t).clone());
        let comment = self
            .world
            .get::<&CommentData>(src)
            .ok()
            .map(|c| (*c).clone());
        let doc_type = self
            .world
            .get::<&DocTypeData>(src)
            .ok()
            .map(|d| (*d).clone());
        let attrs = self
            .world
            .get::<&Attributes>(src)
            .ok()
            .map(|a| (*a).clone());

        let dst = self.world.spawn((TreeRelation::default(), kind));
        if let Some(tag) = tag {
            let _ = self.world.insert_one(dst, tag);
        }
        if let Some(text) = text {
            let _ = self.world.insert_one(dst, text);
        }
        if let Some(comment) = comment {
            let _ = self.world.insert_one(dst, comment);
        }
        if let Some(doc_type) = doc_type {
            let _ = self.world.insert_one(dst, doc_type);
        }
        if let Some(attrs) = attrs {
            let _ = self.world.insert_one(dst, attrs);
        }
        dst
    }

    /// Recursively clone each child of `src` and append to `dst`.
    /// [`ShadowRoot`] children are skipped (matches `children_iter`
    /// semantics — the shadow root is not part of the light tree).
    fn clone_children_recursive(&mut self, src: Entity, dst: Entity) {
        // Snapshot children first so we can freely spawn new entities
        // during iteration without invalidating the sibling chain we
        // are walking.  `children` (not `children_iter`) allocates a
        // Vec, which is the right trade-off here — tree mutations
        // during clone would break `children_iter`'s `next_sibling`
        // reads.
        let kids: Vec<Entity> = self.children(src);
        for child_src in kids {
            let child_dst = self.clone_node_shallow(child_src);
            // `append_child` bumps the version on `dst`; for a fresh
            // clone tree nobody is watching that version, but the
            // invariant (parent is an ancestor of new child) is still
            // satisfied.
            let _ = self.append_child(dst, child_dst);
            self.clone_children_recursive(child_src, child_dst);
        }
    }

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
