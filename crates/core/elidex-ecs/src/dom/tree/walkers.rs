//! Subtree and ancestor walkers for [`EcsDom`]: shadow-inclusive descendant
//! traversal, connectedness, document-order descent, tree-order comparison,
//! inclusive-ancestor checks, and tree-root resolution.

use super::super::{EcsDom, MAX_ANCESTOR_DEPTH};
use crate::components::{Attributes, NodeKind, ShadowRoot};
use hecs::Entity;

impl EcsDom {
    /// Pre-order shadow-including descendant walk (WHATWG DOM §4.2.2
    /// "shadow-including descendant"). Visits `root`, then every
    /// light-tree child via [`Self::children_iter`] (skips internal
    /// `ShadowRoot` entities), then — for elements that host a
    /// shadow root — recurses into the shadow root's own light-tree
    /// subtree via [`Self::get_shadow_root`].
    ///
    /// Iterative DFS with an explicit `Vec<(Entity, depth)>` work-list
    /// — matches the explicit-stack pattern used by `nodes_equal` /
    /// `clone_children_recursive` / `despawn_subtree` so a malicious
    /// deep DOM cannot blow Rust's call stack from inside synchronous
    /// mutation chokepoints (`set_attribute` / `append_child`) via the
    /// `CustomElementReactionConsumer::handle_insert` dispatch path.
    /// The per-frame depth value still caps at [`MAX_ANCESTOR_DEPTH`]
    /// to bound the work-list (corrupt parent chains can't generate
    /// infinite children chains downstream, but the cap defends the
    /// heap budget against pathological inputs).
    ///
    /// Used by Custom Elements `customElements.upgrade(root)` (HTML
    /// §4.13.4 step 8), by `CustomElementReactionConsumer` for the
    /// `Connected` / `Disconnected` per-subtree fan-out, and by
    /// `descendants_shadow_inclusive` in `elidex_dom_api` (which is
    /// now a thin re-export).
    pub fn for_each_shadow_inclusive_descendant<F>(&self, root: Entity, visit: &mut F)
    where
        F: FnMut(Entity),
    {
        let mut stack: Vec<(Entity, usize)> = vec![(root, 0)];
        while let Some((node, depth)) = stack.pop() {
            if depth > MAX_ANCESTOR_DEPTH {
                continue;
            }
            visit(node);
            // Push in REVERSE source order so `pop()` returns siblings
            // in source order; push shadow children FIRST (they sit
            // deeper in the stack and therefore visit AFTER the light
            // children) to preserve the original recursive visit
            // order: light-tree DFS then shadow-tree DFS.
            if let Some(sr) = self.get_shadow_root(node) {
                let shadow_children: Vec<Entity> = self.children_iter(sr).collect();
                for child in shadow_children.into_iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
            let children: Vec<Entity> = self.children_iter(node).collect();
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }

    /// Returns `true` if `entity`'s shadow-including root is a
    /// `Document` (WHATWG DOM §4.4 "connected"). A `Document` entity
    /// itself counts as connected — its shadow-including root IS a
    /// Document (self).
    ///
    /// Walks via [`Self::get_parent`], which (per the shadow plumbing
    /// in `shadow.rs`) returns `Some(host)` for a `ShadowRoot`, so the
    /// walk is shadow-including. Bounded by `MAX_ANCESTOR_DEPTH` to
    /// guard against corrupted parent chains. Returns `false` for
    /// orphaned subtrees.
    #[must_use]
    pub fn is_connected(&self, entity: Entity) -> bool {
        let mut current = Some(entity);
        let mut depth = 0;
        while let Some(e) = current {
            if let Ok(kind) = self.world.get::<&NodeKind>(e) {
                if matches!(*kind, NodeKind::Document) {
                    return true;
                }
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                return false;
            }
            current = self.get_parent(e);
        }
        false
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
}
