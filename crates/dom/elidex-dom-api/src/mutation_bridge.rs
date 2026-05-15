//! Multi-consumer `MutationHook` adapter wrapping
//! [`LiveRangeBridge`] + per-iterator pre-removing-steps state for
//! `NodeIterator` (WHATWG DOM §6.1).
//!
//! # Why a separate module?
//!
//! Per the layering mandate (CLAUDE.md "VM host/ は engine-bound
//! 責務のみ" + `m4-12-pr-d8-pr-a-infra-landing.md` Round 1 IMP-3),
//! `range/live.rs` stays **range-only** so it does not become a
//! catch-all multiplexer.  This module hosts the multi-consumer
//! wrapper that the singleton `EcsDom.mutation_hook` field actually
//! receives at `Vm::bind` time.
//!
//! # Design (plan-v4 §A-NI-1 final)
//!
//! [`MutationBridge`] holds:
//! - an **inner** [`LiveRangeBridge`] that handles all 7
//!   [`MutationHook`] methods for Range boundary adjustment, and
//! - an [`Arc<Mutex<HashMap<u64, NodeIteratorState>>>`] shared with
//!   `HostData` so VM-side `NodeIterator` registrations can be
//!   walked by [`MutationBridge::after_remove_with_descendants`]
//!   for the WHATWG §6.1 pre-removing-steps adjustment.
//!
//! The 7 trait methods delegate to the inner bridge; only
//! `after_remove_with_descendants` additionally walks the inner
//! `node_iterators` map and applies
//! [`crate::traversal::adjust_node_iterator_for_removal`].
//!
//! # Lock ordering (plan-v4 §A-NI-1 Round 2 IMP-3)
//!
//! Each consumer's lock is acquired in a **disjoint** scope — no
//! nested locking.  The Range-side guard drops before the
//! NodeIterator-side guard is acquired.  Enforced syntactically by
//! the explicit-block pattern inside
//! [`MutationBridge::after_remove_with_descendants`].

#![deny(clippy::significant_drop_tightening)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use elidex_ecs::{EcsDom, Entity, MutationHook};

use crate::range::{LiveRangeBridge, LiveRangeRegistry};

/// Per-iterator state for `NodeIterator` (WHATWG DOM §6.1).  Held
/// in `HostData::node_iterator_states_shared`'s
/// `Arc<Mutex<HashMap<u64, NodeIteratorState>>>`, shared with
/// [`MutationBridge`] so the hook-fire path can apply WHATWG §6.1
/// "pre-removing steps" synchronously.
///
/// `filter_object_id` is an **opaque** `Option<u64>` carrying the
/// VM-side `ObjectId` bits.  This crate is engine-indep and must
/// NOT depend on `vm/object_kind.rs::ObjectId`; the VM-side filter
/// dispatch (`vm/host/node_filter_dispatch.rs`) converts back via
/// `ObjectId::from_bits(filter)` at access time (matches the
/// `ObjectKind::MutationObserver { observer_id: u64 }` opaque-bits
/// pattern).
#[derive(Debug, Clone)]
pub struct NodeIteratorState {
    /// `root` per spec §6.1 — never mutates after construction.
    pub root: Entity,
    /// `whatToShow` bitmask per spec §6.3.
    pub what_to_show: u32,
    /// VM-side filter callback `ObjectId` bits, or `None` for
    /// "no filter" (every node ACCEPTed).
    pub filter_object_id: Option<u64>,
    /// `referenceNode` per spec §6.1 — adjusted by pre-removing
    /// steps when its tree position is invalidated.
    pub reference: Entity,
    /// `pointerBeforeReferenceNode` per spec §6.1.
    pub pointer_before: bool,
    /// Active-flag for filter re-entrancy detection (spec §6.3
    /// step 2 — throw `InvalidStateError` if a filter callback
    /// re-enters the iterator).
    pub active: bool,
}

/// Multi-consumer `MutationHook` adapter installed via
/// `EcsDom::set_mutation_hook` at `Vm::bind`.  Dispatches each of
/// the 7 hook methods to the appropriate per-consumer logic
/// ([`LiveRangeBridge`] for Range, in-line walk for NodeIterator).
///
/// # Multiplexer migration path
///
/// When `#11-mutation-hook-multiplexer` lands (real N-consumer
/// hub), `MutationBridge` is replaced by a `Multiplexer` accepting
/// typed observer registrations.  The 2-consumer Bridge → drop-in
/// replaceable: VM-side calls `MutationBridge::new_pair` → switch
/// to `Multiplexer::register({...range, ...node_iter})` with no
/// trait-level changes.
pub struct MutationBridge {
    /// Range-side consumer.  Receives ALL 7 hook callbacks; the
    /// `after_remove_with_descendants` impl handles
    /// boundary collapse / decrement per WHATWG §5.5.
    inner: LiveRangeBridge,
    /// NodeIterator-side consumer state, shared with
    /// `HostData::node_iterator_states_shared`.  Acquired ONLY
    /// inside `after_remove_with_descendants` (the only hook
    /// with NodeIterator-side §6.1 semantics).
    node_iterators: Arc<Mutex<HashMap<u64, NodeIteratorState>>>,
}

impl MutationBridge {
    /// Factory that pairs a fresh [`LiveRangeRegistry`] with a
    /// [`MutationBridge`] holding both `Range` consumer state
    /// (via the inner [`LiveRangeBridge`]) and NodeIterator
    /// consumer state (via `node_iterators_shared`).
    ///
    /// The VM-side caller (`Vm::bind`) creates the shared
    /// `Arc<Mutex<HashMap>>` for `node_iterator_states_shared`
    /// first (it needs its own clone for `HostData`), then passes
    /// the shared handle here.
    ///
    /// # Install pattern
    ///
    /// ```ignore
    /// let iter_shared = Arc::new(Mutex::new(HashMap::new()));
    /// host_data.node_iterator_states_shared = iter_shared.clone();
    /// let (registry, bridge) = MutationBridge::new_pair(iter_shared);
    /// host_data.live_range_registry = registry;
    /// let displaced = dom.set_mutation_hook(Box::new(bridge));
    /// debug_assert!(displaced.is_none(),
    ///     "Vm::bind: EcsDom already had a MutationHook installed");
    /// ```
    #[must_use]
    pub fn new_pair(
        node_iterators_shared: Arc<Mutex<HashMap<u64, NodeIteratorState>>>,
    ) -> (LiveRangeRegistry, Self) {
        let (registry, inner) = LiveRangeRegistry::new_pair();
        let bridge = Self {
            inner,
            node_iterators: node_iterators_shared,
        };
        (registry, bridge)
    }
}

impl MutationHook for MutationBridge {
    fn after_remove(&mut self, node: Entity, parent: Entity, removed_index: usize) {
        self.inner.after_remove(node, parent, removed_index);
        // NodeIterator §6.1 pre-removing-steps run via
        // `after_remove_with_descendants` exclusively (engine
        // dispatches `after_remove_with_descendants` for every
        // remove, so this method is reached only via the default
        // trait impl from a non-overriding consumer — irrelevant
        // for our use).
    }

    fn after_remove_with_descendants(
        &mut self,
        node: Entity,
        parent: Entity,
        removed_index: usize,
        descendants: &[Entity],
        dom: &EcsDom,
    ) {
        // Range-side apply (disjoint scope per plan-v4 §A-NI-1
        // Round 2 IMP-3 lock ordering).
        {
            self.inner
                .after_remove_with_descendants(node, parent, removed_index, descendants, dom);
        } // inner mutex (if any) drops here

        // NodeIterator-side apply (separate lock acquisition).
        {
            let mut iterators = self
                .node_iterators
                .lock()
                .expect("NodeIterator state mutex poisoned");
            for state in iterators.values_mut() {
                crate::traversal::adjust_node_iterator_for_removal(
                    state,
                    node,
                    parent,
                    removed_index,
                    descendants,
                    dom,
                );
            }
        } // iterators guard drops here
    }

    fn after_insert(&mut self, node: Entity, parent: Entity, index: usize) {
        self.inner.after_insert(node, parent, index);
    }

    fn after_text_change(&mut self, node: Entity, new_utf16_len: usize) {
        self.inner.after_text_change(node, new_utf16_len);
    }

    fn after_replace_data(
        &mut self,
        node: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        new_data_len_utf16: usize,
    ) {
        self.inner
            .after_replace_data(node, offset_utf16, count_utf16, new_data_len_utf16);
    }

    fn after_split_text(
        &mut self,
        node: Entity,
        new_node: Entity,
        offset_utf16: usize,
        parent: Option<Entity>,
        node_index: Option<usize>,
    ) {
        self.inner
            .after_split_text(node, new_node, offset_utf16, parent, node_index);
    }

    fn after_normalize_merge(
        &mut self,
        merged_child: Entity,
        prev: Entity,
        prev_old_len_utf16: usize,
        parent: Option<Entity>,
        child_index: Option<usize>,
    ) {
        self.inner.after_normalize_merge(
            merged_child,
            prev,
            prev_old_len_utf16,
            parent,
            child_index,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_bridge_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MutationBridge>();
        assert_send_sync::<NodeIteratorState>();
    }

    #[test]
    fn new_pair_constructs_with_empty_iterators() {
        let iter_shared = Arc::new(Mutex::new(HashMap::new()));
        let (_registry, _bridge) = MutationBridge::new_pair(iter_shared.clone());
        assert!(iter_shared.lock().unwrap().is_empty());
    }
}
