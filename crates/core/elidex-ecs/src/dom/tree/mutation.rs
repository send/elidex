//! Relinking tree mutations for [`EcsDom`] plus their `MutationEvent` fire
//! sites: `append_child` / `remove_child` / `insert_before` / `replace_child`,
//! the low-level `link_node` / `detach` primitives, and the centralized
//! insert/remove dispatch helpers.

use super::super::mutation_event::MutationEvent;
use super::super::EcsDom;
use crate::ElementState;
use hecs::Entity;

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

        // Capture pre-mutation connectedness of `child` for
        // `MutationEvent::Insert.was_connected` (HTML §4.13.6 Custom
        // Element `connectedCallback` transition gate). MUST be read
        // BEFORE `detach_with_hook` runs, which itself fires an
        // implicit Remove whose `was_connected` reflects the same
        // pre-mutation state.
        let child_was_connected = self.is_connected(child);
        self.detach_with_hook(child, parent);

        let last_child = self.read_rel(parent, |rel| rel.last_child);
        self.link_node(parent, child, last_child, None);
        self.rev_version(parent);

        if let Some(new_index) = self.index_in_parent(child) {
            self.fire_after_insert(child, parent, new_index, child_was_connected);
        }

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
        let removed_index = self.index_in_parent(child);
        // Pre-detach connectedness for `MutationEvent::Remove.
        // was_connected` (HTML §4.13.6 Custom Element
        // `disconnectedCallback` transition gate).
        let was_connected = self.is_connected(child);
        self.detach(child);
        self.rev_version(parent);
        if let Some(idx) = removed_index {
            self.fire_after_remove(child, parent, idx, was_connected);
        }
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

        // Pre-mutation connectedness for `was_connected` — same
        // gating rationale as `append_child`.
        let new_child_was_connected = self.is_connected(new_child);
        // Detach new_child from its current position (fires after_remove
        // on the implicit old-parent removal, per WHATWG insert algorithm).
        self.detach_with_hook(new_child, parent);

        // Re-read ref_child's prev_sibling AFTER detach (it may have changed
        // if new_child was an adjacent sibling).
        let ref_prev = self.read_rel(ref_child, |rel| rel.prev_sibling);
        self.link_node(parent, new_child, ref_prev, Some(ref_child));
        self.rev_version(parent);

        if let Some(new_index) = self.index_in_parent(new_child) {
            self.fire_after_insert(new_child, parent, new_index, new_child_was_connected);
        }

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

        // Pre-mutation connectedness for both subjects. `old_child` is
        // connected via `parent` (which is unaffected by the
        // forthcoming `detach_with_hook(new_child, parent)` — per the
        // `is_ancestor(new_child, parent)` rejection above, `new_child`
        // cannot be on `old_child`'s ancestor chain).
        let new_child_was_connected = self.is_connected(new_child);
        let old_child_was_connected = self.is_connected(old_child);
        // Detach new_child from its current position (fires after_remove on
        // the implicit old-parent removal, per WHATWG replace algorithm step
        // "If node has a parent, then remove node").
        self.detach_with_hook(new_child, parent);

        // Capture old_child's index AFTER detach so that — when new_child was
        // an earlier sibling of old_child in the same parent — the index
        // reflects old_child's actual position at the moment of removal
        // (WHATWG DOM §5.5 "remove a node" step 4).
        let old_index = self.index_in_parent(old_child);

        // Re-read old_child's siblings AFTER detach (they may have changed
        // if new_child was an adjacent sibling).
        let (old_prev, old_next) =
            self.read_rel(old_child, |rel| (rel.prev_sibling, rel.next_sibling));

        // Copilot R20: fully detach `old_child` from parent's sibling chain
        // BEFORE firing the after_remove hook + linking the replacement.
        // The mutation-hook post-detach invariant promises that, at fire
        // time, `parent.children[removed_index]` is the first follower of
        // the removed node — earlier impl linked `new_child` into that
        // slot first, so `LiveRangeRegistry` / `MutationBridge` consumers
        // (NodeIterator pre-removing-steps especially) would pick the
        // replacement as the follower instead of `old_child`'s real next
        // sibling.
        self.detach(old_child);

        if let Some(idx) = old_index {
            self.fire_after_remove(old_child, parent, idx, old_child_was_connected);
        }

        // Now link `new_child` into the slot `old_child` used to occupy.
        self.link_node(parent, new_child, old_prev, old_next);
        self.rev_version(parent);

        if let Some(idx) = old_index {
            self.fire_after_insert(new_child, parent, idx, new_child_was_connected);
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

    /// Centralized `MutationEvent::Insert` fire site for tree mutations.
    ///
    /// Per the light-tree-only contract, the event is suppressed when
    /// **either** `node` or `parent` is itself a shadow root —
    /// shadow roots are internal to the engine and shadow-tree
    /// mutations under the root must not surface to light-tree
    /// consumers (e.g. Range live-tracking). Deeper mutations within
    /// the shadow tree (where `parent` is a normal element inside the
    /// shadow tree) are NOT filtered here — those consumers should
    /// filter by tree root if they want light-tree-only events.
    ///
    /// `was_connected` is the connectedness of `node` BEFORE this
    /// mutation began (captured by callers via [`Self::is_connected`]
    /// prior to any implicit detach). Required by Custom Elements
    /// `connectedCallback` gating (HTML §4.13.6).
    fn fire_after_insert(
        &mut self,
        node: Entity,
        parent: Entity,
        index: usize,
        was_connected: bool,
    ) {
        if self.is_shadow_root(node) || self.is_shadow_root(parent) {
            return;
        }
        let event = MutationEvent::Insert {
            node,
            parent,
            index,
            was_connected,
        };
        self.dispatch_event(&event);
    }

    /// Centralized `MutationEvent::Remove` fire site for tree mutations.
    /// Suppression rules match [`Self::fire_after_insert`].
    ///
    /// `was_connected` is the connectedness of `node` BEFORE detach
    /// (captured by callers via [`Self::is_connected`] prior to
    /// `self.detach(node)`). Required by Custom Elements
    /// `disconnectedCallback` gating (HTML §4.13.6).
    pub(super) fn fire_after_remove(
        &mut self,
        node: Entity,
        parent: Entity,
        removed_index: usize,
        was_connected: bool,
    ) {
        // WHATWG HTML §2.1.4 "removing steps for the HTML Standard" step 2:
        // when the document's focused area leaves the tree, reset it to the
        // viewport — clear `ElementState::FOCUS`. The spec is explicit it
        // "does not perform the unfocusing steps, focusing steps, or focus
        // update steps, and thus no blur or change events are fired" (focusout
        // / focusin too, as those only fire via the steps that don't run) — so
        // this is a silent component mutation (the event-dispatching focusing
        // steps stay engine-bound in the shell). Run it BEFORE the light-tree
        // event suppression below
        // and via the SHADOW-INCLUSIVE `is_connected` (not the light-tree
        // `descendants` snapshot), so focus held inside a removed host's shadow
        // tree — or a light child of a removed shadow root — is still reset.
        // Only the removal of a previously-connected node can disconnect the
        // (connected-by-construction) holder, so gate on `was_connected`.
        if was_connected {
            self.clear_focus_if_disconnected();
        }
        if self.is_shadow_root(node) || self.is_shadow_root(parent) {
            return;
        }
        // PR186 R2 #3: snapshot the light-tree inclusive-descendant
        // set BEFORE the caller orphans children / despawns the
        // subtree (the `destroy_entity` path clears descendant parent
        // links after this fire). The dispatcher consumer uses this
        // snapshot to collapse Range boundaries on any inclusive
        // descendant per WHATWG DOM §4.2.3 remove algorithm —
        // `is_ancestor_or_self` walking up from the boundary container
        // would miss orphaned descendants whose parent link is about
        // to be cleared.
        let descendants = self.collect_inclusive_descendants(node);
        let event = MutationEvent::Remove {
            node,
            parent,
            removed_index,
            descendants: &descendants,
            was_connected,
        };
        self.dispatch_event(&event);
    }

    /// Clear [`ElementState::FOCUS`] if its (single) holder is no longer
    /// connected (WHATWG HTML §2.1.4 removing steps step 2 — focused-area reset
    /// on removal, *silent*: no events). Single-focus ⇒ at most one holder; the
    /// `is_connected` check is **shadow-inclusive** (it walks the shadow-
    /// including ancestor chain), so this uniformly covers light-tree removal,
    /// a removed shadow host whose shadow tree held focus, and a removed shadow
    /// root's light children — none of which the light-tree event path reaches.
    /// Despawn-based removal needs no hook (hecs drops the component with the
    /// entity); this covers detach-without-despawn — including
    /// [`EcsDom::destroy_entity`]'s no-dispatcher path, which orphans (does not
    /// despawn) descendants and skips [`Self::fire_after_remove`], so it calls
    /// this directly.
    pub(super) fn clear_focus_if_disconnected(&mut self) {
        let holder = self
            .world
            .query::<(Entity, &ElementState)>()
            .iter()
            .find(|(_, state)| state.contains(ElementState::FOCUS))
            .map(|(e, _)| e);
        let Some(holder) = holder else { return };
        if self.is_connected(holder) {
            return;
        }
        let cleared = match self.world.get::<&ElementState>(holder) {
            Ok(state) => {
                let mut next = *state;
                next.remove(ElementState::FOCUS);
                Some(next)
            }
            Err(_) => None,
        };
        if let Some(next) = cleared {
            let _ = self.world.insert_one(holder, next);
        }
    }

    /// Light-tree inclusive-descendant walker — collects `node` plus
    /// every descendant reachable via [`Self::children_iter`] (which
    /// suppresses shadow roots). Used by [`Self::fire_after_remove`]
    /// to snapshot the subtree pre-orphan so `MutationDispatcher`
    /// consumers can apply boundary adjustments without depending on
    /// intact parent links.
    fn collect_inclusive_descendants(&self, node: Entity) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            out.push(n);
            for child in self.children_iter(n) {
                stack.push(child);
            }
        }
        out
    }

    /// Detach `child` from its current parent and fire
    /// [`MutationEvent::Remove`](super::super::MutationEvent::Remove) on the
    /// installed [`MutationDispatcher`](super::super::MutationDispatcher), if
    /// any.
    ///
    /// Used by `append_child` / `insert_before` / `replace_child` to
    /// notify consumers of the **implicit** removal that happens when a
    /// node is moved between parents (WHATWG DOM "insert" algorithm step
    /// "If node has a parent, then remove node"). Returns `false` when
    /// `child` has no parent (nothing to detach).
    ///
    /// `new_parent` is the parent the caller is about to re-link `child`
    /// into. When `old_parent == new_parent` the caller's post-link
    /// `rev_version(new_parent)` already covers the bump, so the
    /// version-tracking call here is skipped to avoid a redundant
    /// ancestor-walk per same-parent move.
    ///
    /// Per the light-tree-only contract of `MutationDispatcher`, the
    /// `Remove` event is suppressed when `child` is itself a shadow
    /// root — shadow roots have a `TreeRelation.parent` set to the host
    /// (via the internal `attach_shadow → append_child` plumbing), but
    /// are not exposed as DOM children.
    fn detach_with_hook(&mut self, child: Entity, new_parent: Entity) -> bool {
        let Some(old_parent) = self.get_parent(child) else {
            return false;
        };
        let old_index = self.index_in_parent(child);
        // Pre-detach connectedness for `was_connected` — same gating
        // rationale as the public `remove_child` path.
        let was_connected = self.is_connected(child);
        self.detach(child);
        // Only bump the old parent's version when it differs from the
        // new parent. Same-parent moves rely on the caller's
        // post-link `rev_version(new_parent)`.
        if old_parent != new_parent {
            self.rev_version(old_parent);
        }
        if let Some(idx) = old_index {
            self.fire_after_remove(child, old_parent, idx, was_connected);
        }
        true
    }
}
