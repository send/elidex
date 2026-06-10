//! Subtree teardown and re-home for [`EcsDom`].
//!
//! - `destroy_entity` — single-node "hard delete": detach, fire one
//!   `MutationEvent::Remove`, orphan the children, despawn from the world.
//! - `despawn_subtree` — raw structural teardown of a whole subtree
//!   (shadow-including, event-free), reclaiming every entity.
//! - `adopt_subtree` — WHATWG DOM §4.5 node-document update over a subtree.
//!
//! These live together because they share the `detach` / `fire_after_remove`
//! plumbing and the uncapped shadow-inclusive walk, and belong with the
//! tree-mutation family.

use std::collections::HashSet;

use super::super::EcsDom;
use crate::components::ShadowRoot;
use hecs::Entity;

impl EcsDom {
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
    /// # Mutation dispatch contract
    ///
    /// Fires [`MutationEvent::Remove`](super::super::MutationEvent::Remove)
    /// exactly once if `entity` has a parent AND `entity` is not itself a
    /// shadow root, with the pre-removal index in the parent's child
    /// list. Descendant entities orphaned by the destroy do NOT receive
    /// individual `Remove` events — this is a "hard delete" with no
    /// per-descendant notification.
    ///
    /// Consumers (e.g. `LiveRangeRegistry`) MUST tolerate dangling boundary
    /// container references and lazily collapse such Ranges on next access
    /// (e.g. by checking [`Self::contains`] before use). Shadow roots are
    /// suppressed explicitly per the light-tree-only contract: although
    /// `attach_shadow` sets `TreeRelation.parent = Some(host)`, shadow
    /// roots are not exposed as DOM children, and shadow-tree boundaries
    /// are the consumer's responsibility per WHATWG §5.5.
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

        // Capture parent + pre-removal index + pre-detach connectedness for the
        // Remove hook before detach (same `was_connected` gating rationale as
        // `remove_child`). These feed ONLY `fire_after_remove`, so when no
        // dispatcher is installed skip them — `is_connected` is O(ancestor
        // depth), and recomputing it per node would make a deep
        // `despawn_subtree` O(n²) (a maliciously deep fragment's rollback).
        let parent = self.get_parent(entity);
        let fire = if self.dispatcher.is_some() {
            self.index_in_parent(entity)
                .map(|idx| (idx, self.is_connected(entity)))
        } else {
            None
        };

        self.detach(entity);

        // Fire BEFORE orphaning children + despawn so:
        //  - the shadow-root suppression check inside `fire_after_remove`
        //    (which inspects `entity`'s `ShadowRoot` component) still
        //    sees the live entity, AND
        //  - descendant walks rooted at children's `parent` chain still
        //    reach `entity` — `LiveRangeRegistry::finalize_pending`
        //    uses [`EcsDom::is_ancestor_or_self`] which walks UPWARD
        //    from a Range boundary container through `get_parent`; that
        //    walk only finds `entity` while children still hold their
        //    parent link.
        //
        // Without this ordering, a Range boundary on a still-live
        // descendant of `entity` would silently miss the
        // `(parent, removed_index)` collapse required by WHATWG §5.5
        // remove step 4 (Copilot PR186 R2 #3).
        if let (Some(p), Some((idx, was_connected))) = (parent, fire) {
            self.fire_after_remove(entity, p, idx, was_connected);
        }

        // Orphan all children: clear their parent and sibling links so
        // they do not hold dangling references to the destroyed entity.
        // Runs AFTER the hook fire so the descendant walk above sees
        // intact parent chains.
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

    /// Despawn `root` and its entire subtree (shadow-including), returning
    /// `false` if `root` does not exist.
    ///
    /// Where [`Self::destroy_entity`] removes a single node and *orphans*
    /// its descendants (clearing their parent/sibling links but leaving them
    /// live in the world), this tears the whole subtree out of existence.
    /// It is the teardown counterpart used when a detached subtree must leave
    /// no live remnant — e.g. the strict HTML fragment parser's synthetic
    /// `<html>` root on the parse-error rollback path, where leaving the
    /// partially-built subtree as orphaned live entities would pollute the
    /// caller's `EcsDom` and break the parser's "dom is pristine on failure"
    /// isolation contract.
    ///
    /// Iterative explicit-stack walk — matches the deep-DOM
    /// stack-overflow-safe family (`nodes_equal` / `clone_children_recursive`)
    /// so a pathologically deep subtree cannot blow Rust's call stack. The full
    /// descendant set is snapshotted from the intact tree first, then each node
    /// is destroyed deepest-first; the snapshot makes the destroy order
    /// independent of the link mutations [`Self::destroy_entity`] performs as it
    /// goes. The enumeration is **uncapped in both depth and breadth**
    /// ([`Self::child_list_uncapped`], not the `MAX_ANCESTOR_DEPTH`-capped
    /// `children` / `for_each_shadow_inclusive_descendant`) — a complete
    /// teardown must reach every node or it leaks; a `visited` set replaces the
    /// caps as the corruption/cycle termination guard.
    ///
    /// **Raw structural teardown — fires no mutation events.** Dispatch is
    /// suppressed for the whole walk, so this is *not* a connected-subtree
    /// removal: it does not run the WHATWG DOM "remove" steps and emits no
    /// `MutationEvent`s. That keeps it a layering-clean primitive — the
    /// remove algorithm (which owns shadow-host `disconnectedCallback`
    /// ordering: a host's shadow tree must be visited before the host's
    /// `ShadowHost` back-reference is cleared) lives in the DOM layer, not
    /// here. Were this primitive to fire events, the deepest-first walk would
    /// tear a host's shadow root out ahead of the host and the consumer would
    /// miss the shadow tree's `disconnectedCallback`s. Suppressing dispatch
    /// makes that unreachable *by construction*, so callers needing removal
    /// semantics on a connected subtree must go through the DOM remove path,
    /// not this reclaim-the-entities primitive.
    pub fn despawn_subtree(&mut self, root: Entity) -> bool {
        if !self.contains(root) {
            return false;
        }
        // Snapshot the full shadow-inclusive descendant set (uncapped in depth
        // and breadth — teardown must reach every node or it leaks).
        let mut nodes: Vec<Entity> = Vec::new();
        self.for_each_uncapped_shadow_inclusive(root, &mut |e| nodes.push(e));
        // The only live-tree effect of tearing the subtree out is on the root's
        // *external* parent (if any) — its child list loses `root`, so live
        // collections rooted at/above it must invalidate. Capture it now (its
        // version is bumped once, after the walk); every other version bump is
        // internal to the doomed subtree and is suppressed below.
        let root_parent = self.get_parent(root);
        // Event-free structural teardown: take the dispatcher out for the whole
        // walk so no node's `destroy_entity` fires a (mis-ordered, partial)
        // `MutationEvent::Remove`. Restored before returning.
        let saved_dispatcher = self.take_mutation_dispatcher();
        // Suppress per-node version propagation: `destroy_entity` ends with
        // `rev_version(parent)`, which walks all ancestors (O(depth)); per node
        // that is O(n²) for a maliciously deep subtree — the rollback path this
        // primitive is built for. Every such bump targets a doomed node anyway.
        self.version_propagation_suppressed = true;
        // Deepest-first: children precede their parents, so each
        // `destroy_entity` runs before its parent orphans it (cheaper, and the
        // collected set is already frozen against the link mutations).
        for &entity in nodes.iter().rev() {
            let _ = self.destroy_entity(entity);
        }
        self.version_propagation_suppressed = false;
        // The single surviving version effect: the root's external parent.
        if let Some(parent) = root_parent {
            self.rev_version(parent);
        }
        if let Some(dispatcher) = saved_dispatcher {
            self.set_mutation_dispatcher(dispatcher);
        }
        true
    }

    /// Visit `root` and every shadow-inclusive descendant exactly once, with no
    /// `MAX_ANCESTOR_DEPTH` cap in either dimension — the complete-subtree walk
    /// shared by [`Self::despawn_subtree`] (teardown) and [`Self::adopt_subtree`]
    /// (re-home). Uses the uncapped [`Self::child_list_uncapped`] for breadth
    /// and an explicit work-list for depth; a `visited` set replaces the caps as
    /// the corruption/cycle termination guard (each entity enumerated once,
    /// including a host's shadow-root entity, which the light-child walk does not
    /// otherwise reach).
    fn for_each_uncapped_shadow_inclusive<F: FnMut(Entity)>(&self, root: Entity, visit: &mut F) {
        let mut visited = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            visit(node);
            // The shadow-root entity is attached out-of-band (not a light
            // sibling); push it so it is visited and its own light children are
            // walked when popped (nested shadow hosts recurse the same way).
            if let Some(sr) = self.get_shadow_root(node) {
                stack.push(sr);
            }
            for child in self.child_list_uncapped(node) {
                stack.push(child);
            }
        }
    }

    /// WHATWG DOM §4.5 "adopt" node-document update: set the `AssociatedDocument`
    /// of `root` and every shadow-inclusive descendant to `document`. Uncapped in
    /// depth and breadth, so it re-homes a whole subtree however deep/wide. Used
    /// by the HTML §13.4 fragment parser to give returned nodes the context's
    /// node document before the throwaway parse document is despawned — otherwise
    /// their `ownerDocument` would dangle / resolve to `None`.
    pub fn adopt_subtree(&mut self, root: Entity, document: Entity) {
        let mut nodes: Vec<Entity> = Vec::new();
        self.for_each_uncapped_shadow_inclusive(root, &mut |e| nodes.push(e));
        for node in nodes {
            let _ = self.set_associated_document(node, document);
        }
    }
}
