//! Live `Range` tracking (WHATWG DOM §5.5 "Live range updates").
//!
//! Wires the [`elidex_ecs::MutationDispatcher`] events fired from
//! `EcsDom` tree / text-data primitives into the active set of JS-visible
//! `Range` objects so their boundaries follow tree mutations per spec.
//!
//! # Architecture (PR-A plan v3 §A2–§A6, post-R2/R4 simplification)
//!
//! Two halves split across the HostData / EcsDom boundary:
//!
//! - [`LiveRangeRegistry`] (HostData-side consumer) owns the Range
//!   hash and the RangeId counter. JS-visible Range accessors route
//!   through it. [`LiveRangeRegistry::finalize_pending`] runs a
//!   dangling-collapse fallback pass on every read-path access for
//!   the orphan-then-destroy corner case where no event fires.
//! - [`LiveRangeBridge`] is a typed consumer invoked through its
//!   `handle` method by `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher` (which is the
//!   actual [`elidex_ecs::MutationDispatcher`] impl).  The bridge
//!   matches on each [`elidex_ecs::MutationEvent`] variant and
//!   forwards into the shared `ranges`
//!   `Arc<Mutex<HashMap<RangeId, Range>>>`, applying boundary
//!   adjustments SYNCHRONOUSLY. The engine pre-snapshots inclusive
//!   descendants before any `destroy_entity` orphaning (PR186 R2 #3)
//!   so no deferred / queue-drain phase is needed.
//!
//! Pair construction goes through [`LiveRangeRegistry::new_pair`] —
//! the registry and the bridge share a single `Arc<Mutex<>>` handle
//! over the Range map.
//!
//! ## Lock invariant
//!
//! `ranges` is the only mutex shared between the registry and the
//! bridge. Each dispatch (and each registry method) acquires the
//! mutex once for the duration of its read or adjustment. There is
//! no second lock to deadlock against.
//!
//! ## Shallow light-tree contract (lesson #229, prereq #185)
//!
//! `EcsDom` already filters out shadow-tree-internal mutations at fire
//! sites (mutation whose `node` OR `parent` is a `ShadowRoot` is
//! suppressed). `LiveRangeRegistry` trusts that filter and does NOT
//! walk to the tree root on every event — doing so would defeat the
//! cost model spelled out in the
//! [`elidex_ecs::MutationDispatcher`] trait doc. Range boundaries
//! placed on a node *inside* a shadow tree but reached via a normal
//! element parent (not the ShadowRoot itself) are within the shallow
//! filter's grey zone; the
//! [`#11-shadow-tree-live-range-tracking`](
//!   ../../../../../../../../memory/MEMORY.md
//! ) defer slot tracks proper coverage. PR-A behaviour: Range setStart
//! on a shadow-tree node succeeds (Chrome-compat), but mutations
//! whose parent is a regular element nested inside the shadow tree
//! still fire normally — consumers needing strict semantics filter
//! by tree root themselves.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use elidex_ecs::{EcsDom, Entity, MutationEvent};

use super::{
    adjust_ranges_for_insertion, adjust_ranges_for_normalize_merge,
    adjust_ranges_for_removal_snapshot, adjust_ranges_for_replace_data,
    adjust_ranges_for_split_text, adjust_ranges_for_text_change, Range,
};

// ---------------------------------------------------------------------------
// RangeId
// ---------------------------------------------------------------------------

/// Stable identifier for a registered [`Range`] within a
/// [`LiveRangeRegistry`].
///
/// Monotonically allocated by [`LiveRangeRegistry::register`]; never
/// reused even after [`LiveRangeRegistry::unregister`] removes the
/// Range. This prevents use-after-unregister bugs if a JS reference to
/// the wrapper survives the unregister call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeId(pub u64);

// ---------------------------------------------------------------------------
// LiveRangeBridge
// ---------------------------------------------------------------------------

/// `EcsDom` dispatcher-side adapter. Forwards [`MutationEvent`]
/// dispatches into the shared `Arc<Mutex<HashMap<RangeId, Range>>>`
/// owned jointly with the HostData-side [`LiveRangeRegistry`].
///
/// Constructed only via [`LiveRangeRegistry::new_pair`]; the pair-shape
/// keeps the shared `ranges` handle bound at construction time so no
/// callsite can mismatch-pair them.
///
/// All dispatches apply boundary adjustments SYNCHRONOUSLY (PR186 R2 #3):
/// the engine pre-snapshots inclusive descendants before any
/// `destroy_entity` orphaning, so the consumer no longer needs to
/// defer the descendant walk to a separate read-path drain phase. The
/// earlier queue (`PendingMutation`) has been removed accordingly.
///
/// `Send + Sync` because the only field is `Arc<Mutex<HashMap<...>>>`
/// over `Send` data — `Range` contains only `Entity` and `usize`.
pub struct LiveRangeBridge {
    ranges: Arc<Mutex<HashMap<RangeId, Range>>>,
}

impl LiveRangeBridge {
    /// Single-method dispatch entry point invoked by
    /// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.  Pattern-matches the
    /// [`MutationEvent`] variant and forwards to the per-variant
    /// helper below.  Variants not affecting Range live-tracking
    /// (Insert handled by `after_insert`, AttributeChange ignored)
    /// fall through the `_` arm.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Remove {
                node,
                parent,
                removed_index,
                descendants,
                ..
            } => self.after_remove_with_descendants(node, parent, removed_index, descendants, dom),
            MutationEvent::Insert {
                node,
                parent,
                index,
                ..
            } => self.after_insert(node, parent, index),
            MutationEvent::TextChange {
                node,
                new_utf16_len,
            } => {
                self.after_text_change(node, new_utf16_len);
            }
            MutationEvent::ReplaceData {
                node,
                offset_utf16,
                count_utf16,
                new_data_len_utf16,
            } => self.after_replace_data(node, offset_utf16, count_utf16, new_data_len_utf16),
            MutationEvent::SplitText {
                node,
                new_node,
                offset_utf16,
                parent,
                node_index,
            } => self.after_split_text(node, new_node, offset_utf16, parent, node_index),
            MutationEvent::NormalizeMerge {
                merged_child,
                prev,
                prev_old_len_utf16,
                parent,
                merged_child_index,
            } => self.after_normalize_merge(
                merged_child,
                prev,
                prev_old_len_utf16,
                parent,
                merged_child_index,
            ),
            MutationEvent::AttributeChange { .. } | MutationEvent::CloneCreated { .. } => {
                // Range live-tracking does not depend on attribute
                // mutations — Range boundaries are tree-positional.  A clone
                // creates detached nodes no existing boundary refers to.
            }
        }
    }

    fn after_remove_with_descendants(
        &mut self,
        _node: Entity,
        parent: Entity,
        removed_index: usize,
        descendants: &[Entity],
        _dom: &EcsDom,
    ) {
        // PR186 R2 #3 fix: the engine pre-snapshots descendants
        // before any `destroy_entity` orphaning runs, so we apply the
        // boundary collapse SYNCHRONOUSLY using the snapshot — no
        // need to queue + walk dom later (which would miss orphaned
        // descendants whose parent links have already been cleared).
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values_mut() {
            adjust_ranges_for_removal_snapshot(
                std::slice::from_mut(range),
                descendants,
                parent,
                removed_index,
            );
        }
    }

    fn after_insert(&mut self, _node: Entity, parent: Entity, index: usize) {
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values_mut() {
            adjust_ranges_for_insertion(std::slice::from_mut(range), parent, index);
        }
    }

    fn after_text_change(&mut self, node: Entity, new_utf16_len: usize) {
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values_mut() {
            adjust_ranges_for_text_change(std::slice::from_mut(range), node, new_utf16_len);
        }
    }

    fn after_replace_data(
        &mut self,
        node: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        new_data_len_utf16: usize,
    ) {
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values_mut() {
            adjust_ranges_for_replace_data(
                std::slice::from_mut(range),
                node,
                offset_utf16,
                count_utf16,
                new_data_len_utf16,
            );
        }
    }

    fn after_split_text(
        &mut self,
        node: Entity,
        new_node: Entity,
        offset_utf16: usize,
        parent: Option<Entity>,
        node_index: Option<usize>,
    ) {
        // LiveRangeBridge runs AFTER the `insert_before` / `append_child` hook
        // (`after_insert`) has already fired with the new node's
        // post-insert index (`node_index + 1`). `after_insert` shifted
        // every parent-side boundary with `off > node_index + 1` by
        // +1. The split-text rule (spec §4.10 step 7.2) wants `off >
        // node_index` to shift — the only remaining slot is `off ==
        // node_index + 1`, which we top up here.
        //
        // Node-side migration: `off > offset` → `(new_node, off -
        // offset)`, strict-greater per spec §4.10 step 7.2 / 7.3 —
        // boundaries at exactly `offset` stay on the original node at
        // its new end (PR186 R2 #2). The helper applies that strict
        // bound for the node-side rules; we pass null parent args to
        // skip the helper's parent-side step, because the delta
        // top-up above + the `after_insert` hook already cover
        // §4.10 step 7's parent-side rule (`off > node_idx → +1`)
        // exhaustively.
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        if let (Some(parent), Some(node_idx)) = (parent, node_index) {
            let equal_slot = node_idx + 1;
            for range in guard.values_mut() {
                if range.start_container == parent && range.start_offset == equal_slot {
                    range.start_offset += 1;
                }
                if range.end_container == parent && range.end_offset == equal_slot {
                    range.end_offset += 1;
                }
            }
        }
        for range in guard.values_mut() {
            adjust_ranges_for_split_text(
                std::slice::from_mut(range),
                node,
                new_node,
                offset_utf16,
                None,
                None,
            );
        }
    }

    fn after_normalize_merge(
        &mut self,
        merged_child: Entity,
        prev: Entity,
        prev_old_len_utf16: usize,
        parent: Option<Entity>,
        merged_child_index: Option<usize>,
    ) {
        // LiveRangeBridge fires BEFORE the `Normalize` handler's `remove_child`
        // (which itself fires `after_remove` shifting parent-side
        // boundaries with `off > merged_child_index` by `-= 1`). Spec
        // §4.5 step 6.4 wants the parent-side boundary at exactly
        // `off == merged_child_index` to migrate to `(prev,
        // prev_old_len)` — the merge splice point. The subsequent
        // `after_remove` handles `off > merged_child_index` itself; we
        // apply ONLY the equality migration here to avoid double-
        // decrement.
        //
        // The merged_child-side migration (rule 6.4.a:
        // `(merged_child, off)` → `(prev, prev_old_len + off)`)
        // applies regardless of parent; we call the helper with null
        // parent args so it does not re-walk the parent-side rules
        // that this LiveRangeBridge inlines.
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        if let (Some(parent), Some(idx)) = (parent, merged_child_index) {
            for range in guard.values_mut() {
                if range.start_container == parent && range.start_offset == idx {
                    range.start_container = prev;
                    range.start_offset = prev_old_len_utf16;
                }
                if range.end_container == parent && range.end_offset == idx {
                    range.end_container = prev;
                    range.end_offset = prev_old_len_utf16;
                }
            }
        }
        for range in guard.values_mut() {
            adjust_ranges_for_normalize_merge(
                std::slice::from_mut(range),
                merged_child,
                prev,
                prev_old_len_utf16,
                None,
                None,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// LiveRangeRegistry
// ---------------------------------------------------------------------------

/// HostData-side consumer of [`MutationEvent`] dispatches for Range
/// live-tracking.
///
/// Owns the Range hash + the RangeId monotonic counter, sharing the
/// `ranges` `Arc<Mutex<>>` with the EcsDom-side [`LiveRangeBridge`].
/// Dispatch callbacks apply boundary adjustments synchronously
/// (PR186 R2 #3 fix: descendants snapshot eliminates the prior queue);
/// the remaining read-path concern is the orphan-then-destroy case
/// where no event fires for an entity with no parent —
/// [`Self::finalize_pending`] runs a dangling-collapse pass to catch
/// those.
pub struct LiveRangeRegistry {
    ranges: Arc<Mutex<HashMap<RangeId, Range>>>,
    next_id: u64,
}

impl LiveRangeRegistry {
    /// Construct a paired registry + bridge sharing the `ranges`
    /// `Arc<Mutex<>>` handle. The bridge is intended for
    /// [`EcsDom::set_mutation_dispatcher`] at `Vm::bind` time.
    #[must_use]
    pub fn new_pair() -> (Self, LiveRangeBridge) {
        let ranges = Arc::new(Mutex::new(HashMap::new()));
        let registry = Self {
            ranges: Arc::clone(&ranges),
            next_id: 0,
        };
        let bridge = LiveRangeBridge { ranges };
        (registry, bridge)
    }

    /// Register `range` and return its monotonic [`RangeId`].
    ///
    /// IDs are never recycled — see [`RangeId`] doc for rationale.
    pub fn register(&mut self, range: Range) -> RangeId {
        let id = RangeId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("RangeId overflow — page allocated > 2^64 Ranges");
        self.ranges
            .lock()
            .expect("ranges mutex poisoned")
            .insert(id, range);
        id
    }

    /// Current `next_id` value — the ID that will be assigned by the
    /// NEXT [`Self::register`] call.  Used by `Vm::bind` to preserve
    /// the monotonic counter across rebind cycles (Copilot R6):
    /// reset-to-zero would collide with retained `Range` wrappers
    /// from the previous bind.
    #[must_use]
    pub fn next_id_marker(&self) -> u64 {
        self.next_id
    }

    /// Reset `next_id` to `marker` — used by `Vm::bind` after
    /// replacing the registry with a fresh pair.  Caller MUST have
    /// captured `marker` from the previous registry's
    /// [`Self::next_id_marker`].  Panics if `marker` is LESS than
    /// the current `next_id` (would recycle).
    pub fn restore_next_id_marker(&mut self, marker: u64) {
        assert!(
            marker >= self.next_id,
            "restore_next_id_marker: would recycle ID {} (current {})",
            marker,
            self.next_id,
        );
        self.next_id = marker;
    }

    /// Drop the registration for `id` from the live set. Returns the
    /// stored [`Range`] if it was registered, `None` otherwise.
    ///
    /// Called from GC sweep when a Range JS wrapper becomes
    /// unreachable. Does NOT recycle the ID slot.
    pub fn unregister(&mut self, id: RangeId) -> Option<Range> {
        self.ranges
            .lock()
            .expect("ranges mutex poisoned")
            .remove(&id)
    }

    /// Clear all registered Ranges. Called from `Vm::unbind` to
    /// release Entity references before the next-bound `EcsDom`
    /// invalidates them.
    pub fn clear(&mut self) {
        self.ranges.lock().expect("ranges mutex poisoned").clear();
    }

    /// Apply the dangling-collapse fallback (boundary container no
    /// longer exists in `dom` → collapse to `owner_document`).
    ///
    /// Runs UNCONDITIONALLY on every read access because
    /// [`elidex_ecs::EcsDom::destroy_entity`] of an entity with **no
    /// parent** fires no `after_remove` hook (prereq #185 fire-gate
    /// `(parent, removed_index).is_some()`). The corner case:
    /// `destroy_entity(parent)` orphans `child`, then
    /// `destroy_entity(child)` despawns it without a hook fire. A
    /// boundary on `child` needs dangling-collapse on next access.
    /// Cost is O(R) per call where R = live range count (typically
    /// < 10).
    pub fn finalize_pending(&mut self, dom: &EcsDom) {
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values_mut() {
            if !dom.contains(range.start_container) {
                if dom.contains(range.owner_document) {
                    range.start_container = range.owner_document;
                }
                range.start_offset = 0;
            }
            if !dom.contains(range.end_container) {
                if dom.contains(range.owner_document) {
                    range.end_container = range.owner_document;
                }
                range.end_offset = 0;
            }
        }
    }

    /// Run `f` with a shared borrow of the Range identified by `id`,
    /// after draining pending events. Returns `None` if `id` is not
    /// registered (or has been unregistered).
    pub fn with_range<F, R>(&mut self, id: RangeId, dom: &EcsDom, f: F) -> Option<R>
    where
        F: FnOnce(&Range, &EcsDom) -> R,
    {
        self.finalize_pending(dom);
        let guard = self.ranges.lock().expect("ranges mutex poisoned");
        guard.get(&id).map(|range| f(range, dom))
    }

    /// Run `f` with a mutable borrow of the Range identified by `id`,
    /// after draining pending events. Returns `None` if `id` is not
    /// registered.
    pub fn with_range_mut<F, R>(&mut self, id: RangeId, dom: &EcsDom, f: F) -> Option<R>
    where
        F: FnOnce(&mut Range, &EcsDom) -> R,
    {
        self.finalize_pending(dom);
        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        guard.get_mut(&id).map(|range| f(range, dom))
    }

    /// Visit every live Range's container Entities. Used by GC to root
    /// the entity references held by Range boundaries +
    /// `owner_document`.
    pub fn for_each_entity<F: FnMut(Entity)>(&self, mut f: F) {
        let guard = self.ranges.lock().expect("ranges mutex poisoned");
        for range in guard.values() {
            f(range.start_container);
            f(range.end_container);
            f(range.owner_document);
        }
    }

    /// Count of currently-registered Ranges. Used by tests and the
    /// `Vm::unbind` post-clear assertion.
    pub fn len(&self) -> usize {
        self.ranges.lock().expect("ranges mutex poisoned").len()
    }

    /// `true` when no Range is registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
