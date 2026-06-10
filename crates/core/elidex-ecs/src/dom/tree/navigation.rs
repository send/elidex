//! Tree navigation accessors for [`EcsDom`]: parent / child / sibling getters,
//! element-filtered and shadow-exposed navigation, tag helpers, and the
//! children iterators.

use std::collections::HashSet;

use super::super::{ChildrenIter, ChildrenIterRev, EcsDom, MAX_ANCESTOR_DEPTH};
use crate::components::{ShadowRoot, TagType};
use hecs::Entity;

impl EcsDom {
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

    /// Returns the **exposed** (light-tree) index of `entity` in its
    /// parent's child list, or `None` if `entity` has no parent OR the
    /// sibling walk hits the corruption guard [`MAX_ANCESTOR_DEPTH`].
    ///
    /// Walks the `prev_sibling` chain to count predecessors (O(siblings)).
    /// Shadow-root siblings are **skipped** so the count matches the
    /// indices yielded by [`Self::children_iter`] / [`Self::children`],
    /// which excludes [`ShadowRoot`] entities from the exposed child
    /// list. Used by [`MutationEvent`](super::super::MutationEvent) fire sites
    /// to capture the pre-mutation index without paying the O(n²) cost
    /// of `children_iter(parent).count()`.
    ///
    /// `None` on depth-cap is intentional: dispatcher consumers (e.g.
    /// Range live-tracking) depend on the returned index for
    /// correctness, so signalling corruption is safer than handing them
    /// a truncated count.
    #[must_use]
    pub fn index_in_parent(&self, entity: Entity) -> Option<usize> {
        self.get_parent(entity)?;
        let mut index = 0usize;
        let mut current = self.get_prev_sibling(entity);
        let mut depth = 0;
        while let Some(prev) = current {
            // Skip shadow-root siblings — they are not exposed as DOM
            // children, so light-tree consumers see the host's
            // remaining children with the lower index.
            if self.world.get::<&ShadowRoot>(prev).is_err() {
                index += 1;
            }
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                return None;
            }
            current = self.get_prev_sibling(prev);
        }
        Some(index)
    }

    /// Returns `true` if `entity` carries a [`ShadowRoot`] component.
    /// Shadow roots are internal to the engine and not exposed as
    /// light-tree children; [`MutationEvent`](super::super::MutationEvent)
    /// fire sites suppress events whose subject is a shadow root.
    /// Public so tree-scoped algorithms (e.g. the HTML fragment parser's
    /// form-pointer ancestor walk) can stop at a shadow boundary rather than
    /// follow [`Self::get_parent`]'s shadow-inclusive `ShadowRoot → host` hop.
    pub fn is_shadow_root(&self, entity: Entity) -> bool {
        self.world.get::<&ShadowRoot>(entity).is_ok()
    }

    /// Return the first child of `parent` that is an element.  Text,
    /// comment, and shadow-root children are skipped.  Uses
    /// [`Self::is_element`] so the brand check matches the canonical
    /// NodeKind-first predicate shared with `child_element_count` and
    /// the ParentNode reader mixin.
    #[must_use]
    pub fn first_element_child(&self, parent: Entity) -> Option<Entity> {
        let mut child = self.get_first_child(parent);
        while let Some(c) = child {
            if self.is_element(c) {
                return Some(c);
            }
            child = self.get_next_sibling(c);
        }
        None
    }

    /// Return the last child of `parent` that is an element.
    #[must_use]
    pub fn last_element_child(&self, parent: Entity) -> Option<Entity> {
        let mut child = self.get_last_child(parent);
        while let Some(c) = child {
            if self.is_element(c) {
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
            if self.is_element(sib) {
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
            if self.is_element(sib) {
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

    /// Returns the tag name of an entity.
    ///
    /// Returns `None` for non-element entities (text / comment /
    /// document / window — anything without a `TagType` component)
    /// AND for any `World::get::<&TagType>` failure (entity
    /// destroyed, hecs borrow conflict).  Callers cannot
    /// distinguish these from a genuinely tagless entity.
    ///
    /// Allocates a fresh `String` for the present-value arm;
    /// prefer [`Self::with_tag_name`] for borrow-only consumers
    /// (equality comparisons, case-insensitive matching,
    /// intern-on-Some) — that path keeps the value as
    /// `Option<&str>` and skips the per-call `String` allocation.
    #[must_use]
    pub fn get_tag_name(&self, entity: Entity) -> Option<String> {
        self.with_tag_name(entity, |t| t.map(String::from))
    }

    /// Borrow the tag name of `entity` and project through `f`.
    ///
    /// `f` is called with `Some(tag)` for elements carrying a
    /// `TagType` component, and `None` for every other case —
    /// non-element nodes (text / comment / document / window) AND
    /// any `World::get::<&TagType>` failure (entity destroyed,
    /// borrow conflict).  Callers cannot distinguish these cases
    /// from `None`.  Zero-allocation sibling of
    /// [`Self::get_tag_name`].
    ///
    /// The closure parameter is `for<'b> FnOnce(Option<&'b str>) -> R`
    /// so the borrowed `&str` cannot escape `f`'s scope: `hecs::World`
    /// supports interior-mutable borrows via `&World`, so leaking the
    /// `&str` past the internal `Ref<'_, TagType>` guard could allow a
    /// later `&mut TagType` borrow to alias it.
    pub fn with_tag_name<R>(
        &self,
        entity: Entity,
        f: impl for<'b> FnOnce(Option<&'b str>) -> R,
    ) -> R {
        match self.world.get::<&TagType>(entity) {
            Ok(tag) => f(Some(&tag.0)),
            Err(_) => f(None),
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

    /// All light-tree children of `parent`, in order, with **no**
    /// `MAX_ANCESTOR_DEPTH` cap on the sibling count — unlike [`Self::children`]
    /// / [`Self::children_iter`], which truncate a very wide child list (their
    /// cap guards a corrupted/cyclic sibling chain). Use this only where
    /// dropping children is a correctness bug, not a safe approximation: e.g.
    /// complete-subtree teardown ([`Self::despawn_subtree`]) and the WHATWG HTML
    /// §13.4 step 20 fragment-root return, where a dropped child would be
    /// orphaned as a live, unreachable remnant. A `visited` set still bounds a
    /// malformed/cyclic sibling chain (the walk stops at the first repeat),
    /// preserving the caps' termination guarantee without their silent
    /// truncation. Internal `ShadowRoot` entities are skipped, as in `children`.
    pub fn child_list_uncapped(&self, parent: Entity) -> Vec<Entity> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        let mut current = self.read_rel(parent, |rel| rel.first_child);
        while let Some(entity) = current {
            if !seen.insert(entity) {
                break;
            }
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

    /// Returns a zero-allocation **reverse** iterator over direct
    /// children of `parent` (last child first, via `prev_sibling`).
    ///
    /// Like [`Self::children_iter`], internal [`ShadowRoot`] entities
    /// are skipped and iteration caps at [`MAX_ANCESTOR_DEPTH`].
    #[must_use]
    pub fn children_iter_rev(&self, parent: Entity) -> ChildrenIterRev<'_> {
        let next = self.read_rel(parent, |rel| rel.last_child);
        ChildrenIterRev {
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
}
