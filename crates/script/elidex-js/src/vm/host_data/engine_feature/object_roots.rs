//! VM `ObjectId` side-stores on `HostData` and their GC accessors.
//!
//! The listener store and the Node wrapper cache, `gc_root_object_ids`,
//! and the read-only accessors the GC trace fan-out uses (TreeWalker /
//! NodeIterator / Range / Selection tables).
//! Carved out of `host_data/mod.rs` as a cohesion seam (pure move).

use super::super::super::value::ObjectId;
use super::super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::{HostData, TreeWalkerState};
use elidex_ecs::Entity;
use elidex_script_session::ListenerId;
use std::collections::HashMap;

impl HostData {
    /// # Panics
    ///
    /// Panics if the `ListenerId` is already registered.  `ListenerId`
    /// values are expected to be unique per `addEventListener` call;
    /// a duplicate would silently orphan the prior `ObjectId` and
    /// drop it from `gc_root_object_ids` — a recipe for a
    /// use-after-free if any JS-side reference to the old listener
    /// still exists.  Enforced in release too.
    pub fn store_listener(&mut self, id: ListenerId, func: ObjectId) {
        let prev = self.listener_store.insert(id, func);
        assert!(prev.is_none(), "duplicate ListenerId {id:?}");
    }

    pub fn get_listener(&self, id: ListenerId) -> Option<ObjectId> {
        self.listener_store.get(&id).copied()
    }

    pub fn remove_listener(&mut self, id: ListenerId) -> Option<ObjectId> {
        self.listener_store.remove(&id)
    }

    pub fn get_cached_wrapper(&self, entity: Entity) -> Option<ObjectId> {
        self.wrapper_store
            .get(&WrapperKey::entity(entity, WrapperKind::Node))
            .copied()
    }

    /// # Panics
    ///
    /// Panics if the Entity already has a cached wrapper.  Wrapper cache
    /// identity (`el === el`) requires the caller to check
    /// `get_cached_wrapper()` first; silently overwriting would
    /// orphan the prior wrapper ObjectId, dropping it from
    /// `gc_root_object_ids` while live JS references may still
    /// reach it.  Enforced in release too.
    pub fn cache_wrapper(&mut self, entity: Entity, obj: ObjectId) {
        let prev = self
            .wrapper_store
            .insert(WrapperKey::entity(entity, WrapperKind::Node), obj);
        assert!(
            prev.is_none(),
            "wrapper already cached for Entity {entity:?}"
        );
    }

    /// Drop the cached wrapper for `entity`, returning the prior
    /// `ObjectId` if any.  Called when an entity is destroyed
    /// (DOM mutation removed it) so its wrapper becomes eligible
    /// for GC instead of leaking via the cache root.
    ///
    /// PR3 introduces the API; the DOM-mutation hook that calls
    /// it lives in PR4 alongside the rest of the tree-mutation
    /// surface (`removeChild`, `replaceWith`, etc.).  Until then
    /// wrappers for destroyed entities stay rooted — a known but
    /// bounded leak (capped by the number of distinct entities
    /// the page ever observes).
    pub fn remove_wrapper(&mut self, entity: Entity) -> Option<ObjectId> {
        self.wrapper_store
            .remove(&WrapperKey::entity(entity, WrapperKind::Node))
    }

    pub fn gc_root_object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        // Copilot R8: `TreeWalker` / `NodeIterator` filters are
        // NOT rooted here — doing so created a leak cycle when
        // the filter closure captured the wrapper (filter root →
        // closure trace → wrapper marked → state table entry
        // preserved → filter stays rooted → ...).  Filters are
        // instead reached via per-wrapper trace fan-out (see
        // `trace_work_list`'s `TreeWalker` / `NodeIterator`
        // arms), so an unreachable wrapper drops its filter
        // root naturally on the same sweep.
        //
        // Node wrappers (the former `wrapper_cache`) are NOT chained
        // here: they are strong-marked by the unified wrapper-store mark
        // loop in `gc/roots.rs` via the `MarkAgent::StrongRoot` arm
        // (`#11-wrapper-identity-seam`).
        //
        // The three `*_observer_bindings` are NOT chained here (S5-3c):
        // rooting them at construction for life was an over-root / leak (a
        // never-observed or `disconnect()`ed observer with no JS ref stayed
        // immortal until `Vm::unbind`). They are now rooted by the keepalive
        // seam's active-observation predicate (`gc/keepalive.rs`
        // `keepalive_survivors`) — kept iff the observer has ≥1 active
        // observation (or, MutationObserver-only, ≥1 pending undelivered
        // record) — and their binding-map rows are swept by the instance
        // mark bit in `gc/collect.rs`.
        self.listener_store
            .values()
            .copied()
            // D-17 `#11-custom-elements-vm`: every registered CE
            // constructor + cached whenDefined Promise must stay
            // GC-rooted for the registry's lifetime — otherwise an
            // upgrade after a major GC cycle would dereference a
            // freed `ObjectId`. Both maps are document-lifetime:
            // they SURVIVE a per-turn `Vm::unbind` (so these roots
            // persist across batches — which is exactly why the
            // constructor/promise `ObjectId`s stay valid for a
            // cross-batch upgrade / `whenDefined` resolution) and
            // release only at `Vm::teardown_document`
            // (`#11-per-batch-unbind-document-lifetime-state`).
            .chain(self.ce_constructors.values().copied())
            .chain(self.ce_when_defined_promises.values().copied())
    }

    /// GC trace fan-out accessor for `TreeWalker.filter_object_id`
    /// lookup.  Copilot R8 — filter is reached only when the
    /// wrapper itself is reachable via this state table.
    pub(crate) fn tree_walker_states_ref(&self) -> &HashMap<u64, TreeWalkerState> {
        &self.tree_walker_states
    }
    pub(crate) fn tree_walker_instances_ref(&self) -> &HashMap<u64, ObjectId> {
        &self.tree_walker_instances
    }
    pub(crate) fn node_iterator_states_shared_ref(
        &self,
    ) -> &std::sync::Arc<std::sync::Mutex<HashMap<u64, elidex_dom_api::NodeIteratorState>>> {
        &self.node_iterator_states_shared
    }
    pub(crate) fn node_iterator_instances_ref(&self) -> &HashMap<u64, ObjectId> {
        &self.node_iterator_instances
    }

    /// `[SameObject]` Range wrapper cache accessor for the GC trace
    /// fan-out (Selection → current Range wrapper).
    pub(crate) fn range_instances_ref(&self) -> &HashMap<u64, ObjectId> {
        &self.range_instances
    }

    /// Selection singleton wrapper ObjectId, if a `getSelection()`
    /// call has already materialised one.
    pub(crate) fn selection_instance_id(&self) -> Option<ObjectId> {
        self.selection_instance
    }

    /// Read access to the per-document Selection state for the GC
    /// trace fan-out — returns the currently-active `RangeId.bits()`
    /// when a range is set.
    pub(crate) fn selection_active_range_id_bits(&self) -> Option<u64> {
        self.selection_state
            .as_ref()
            .and_then(elidex_dom_api::SelectionState::current_range_id)
            .map(|rid| rid.0)
    }
}
