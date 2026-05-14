//! Live `Range` tracking (WHATWG DOM §5.5 "Live range updates").
//!
//! Wires the [`elidex_ecs::MutationHook`] callbacks fired from
//! `EcsDom` tree / text-data primitives into the active set of JS-visible
//! `Range` objects so their boundaries follow tree mutations per spec.
//!
//! # Architecture (PR-A plan v3 §A2–§A6)
//!
//! Two halves split across the HostData / EcsDom boundary:
//!
//! - [`LiveRangeRegistry`] (HostData-side consumer) owns the Range hash
//!   and the RangeId counter. JS-visible Range accessors route through
//!   it. Calls
//!   [`LiveRangeRegistry::finalize_pending`] on every read-path access
//!   to drain queued events that need `&EcsDom`.
//! - [`Bridge`] (`EcsDom.mutation_hook` consumer) is a small struct
//!   that implements [`elidex_ecs::MutationHook`] by forwarding into
//!   the same `Arc<Mutex<>>`-shared state. It queues the only event
//!   that needs `&EcsDom` (`after_remove` — descendant walk) and
//!   applies the other five synchronously.
//!
//! Pair construction goes through [`LiveRangeRegistry::new_pair`] —
//! the registry and the bridge share two `Arc<Mutex<>>` handles (the
//! Range map and the pending-remove queue).
//!
//! ## Lock ordering invariant
//!
//! The two `Arc<Mutex<>>` handles (`ranges`, `pending_remove`) are
//! independent — Bridge hooks acquire exactly one, registry methods
//! drain the queue then acquire the map (in that order). No callsite
//! holds both simultaneously, so deadlock is impossible.
//!
//! ## Shallow light-tree contract (lesson #229, prereq #185)
//!
//! `EcsDom` already filters out shadow-tree-internal mutations at fire
//! sites (mutation whose `node` OR `parent` is a `ShadowRoot` is
//! suppressed). `LiveRangeRegistry` trusts that filter and does NOT
//! walk to the tree root on every event — doing so would defeat the
//! cost model spelled out in the
//! [`elidex_ecs::MutationHook`] trait doc. Range boundaries
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

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use elidex_ecs::{EcsDom, Entity, MutationHook};

use super::{
    adjust_ranges_for_insertion, adjust_ranges_for_normalize_merge, adjust_ranges_for_removal,
    adjust_ranges_for_replace_data, adjust_ranges_for_split_text, adjust_ranges_for_text_change,
    Range,
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
// PendingMutation
// ---------------------------------------------------------------------------

/// Events queued from [`MutationHook`] callbacks that need `&EcsDom`
/// access to finalise (descendant walks). Drained by the HostData-side
/// consumer ([`LiveRangeRegistry::finalize_pending`]) on every Range
/// read-path access.
///
/// Only `after_remove` queues — the five other hooks apply
/// synchronously inside the [`MutationHook`] callback (no `&EcsDom`
/// needed for [`adjust_ranges_for_insertion`] / `_text_change` /
/// `_replace_data` / `_split_text` / `_normalize_merge`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum PendingMutation {
    Remove {
        node: Entity,
        parent: Entity,
        removed_index: usize,
    },
}

// ---------------------------------------------------------------------------
// Bridge
// ---------------------------------------------------------------------------

/// `EcsDom.mutation_hook`-side adapter. Forwards [`MutationHook`]
/// callbacks into the shared `Arc<Mutex<>>` state owned jointly with
/// the HostData-side [`LiveRangeRegistry`].
///
/// Constructed only via [`LiveRangeRegistry::new_pair`]; the pair-shape
/// keeps the two halves' shared handles bound at construction time so
/// no callsite can mismatched-pair them.
///
/// `Send + Sync` because all fields are `Arc<Mutex<_: Send>>` — the
/// inner `HashMap<RangeId, Range>` and `VecDeque<PendingMutation>` are
/// both `Send`; `Range` contains only `Entity` and `usize`.
pub struct Bridge {
    ranges: Arc<Mutex<HashMap<RangeId, Range>>>,
    pending_remove: Arc<Mutex<VecDeque<PendingMutation>>>,
}

impl MutationHook for Bridge {
    fn after_remove(&mut self, node: Entity, parent: Entity, removed_index: usize) {
        self.pending_remove
            .lock()
            .expect("pending_remove mutex poisoned")
            .push_back(PendingMutation::Remove {
                node,
                parent,
                removed_index,
            });
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
        // Bridge runs AFTER the `insert_before` / `append_child` hook
        // (`after_insert`) has already fired with the new node's
        // post-insert index (`node_index + 1`). `after_insert` shifted
        // every parent-side boundary with `off > node_index + 1` by
        // +1. The split-text rule (spec §4.10 step 7.2) wants `off >
        // node_index` to shift — the only remaining slot is `off ==
        // node_index + 1`, which we top up here.
        //
        // Node-side migration: `off >= offset` → `(new_node, off -
        // offset)`, covering both spec rule 7.3 (strict greater) and
        // rule 7.4 (equality maps to offset 0). The helper applies
        // the inclusive lower-bound semantics; we still null the
        // parent args because we ALREADY applied the delta-only
        // increment above and don't want the helper to double-shift
        // boundaries the `after_insert` hook already handled.
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
        // Bridge fires BEFORE the `Normalize` handler's `remove_child`
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
        // that this Bridge inlines.
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

/// HostData-side consumer of [`MutationHook`] events for Range
/// live-tracking.
///
/// Owns the Range hash, the RangeId monotonic counter, and the
/// HostData half of the shared `Arc<Mutex<>>` state. Read paths
/// ([`Self::with_range`] / [`Self::with_range_mut`]) call
/// [`Self::finalize_pending`] first so descendant-walk events queued
/// by [`Bridge::after_remove`] are applied before the JS-visible
/// boundary state is read.
pub struct LiveRangeRegistry {
    ranges: Arc<Mutex<HashMap<RangeId, Range>>>,
    pending_remove: Arc<Mutex<VecDeque<PendingMutation>>>,
    next_id: u64,
}

impl LiveRangeRegistry {
    /// Construct a paired registry + bridge sharing the two
    /// `Arc<Mutex<>>` handles. The bridge is intended for
    /// [`EcsDom::set_mutation_hook`] at `Vm::bind` time.
    #[must_use]
    pub fn new_pair() -> (Self, Bridge) {
        let ranges = Arc::new(Mutex::new(HashMap::new()));
        let pending_remove = Arc::new(Mutex::new(VecDeque::new()));
        let registry = Self {
            ranges: Arc::clone(&ranges),
            pending_remove: Arc::clone(&pending_remove),
            next_id: 0,
        };
        let bridge = Bridge {
            ranges,
            pending_remove,
        };
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

    /// Clear all registered Ranges + drop pending events. Called from
    /// `Vm::unbind` to release Entity references before the
    /// next-bound `EcsDom` invalidates them.
    pub fn clear(&mut self) {
        self.ranges.lock().expect("ranges mutex poisoned").clear();
        self.pending_remove
            .lock()
            .expect("pending_remove mutex poisoned")
            .clear();
    }

    /// Drain queued `after_remove` events using `dom` for descendant
    /// walks (§5.5 step 4 inclusive-descendant collapse), then apply
    /// the dangling-collapse fallback (boundary container no longer
    /// exists in `dom` → collapse to `owner_document`).
    ///
    /// The dangling-collapse pass runs UNCONDITIONALLY — not gated on
    /// queue non-emptiness — because [`elidex_ecs::EcsDom::destroy_entity`]
    /// of an entity with **no parent** fires no `after_remove` hook
    /// (the prereq #185 contract: fire only when `(parent,
    /// removed_index)` is `Some`). The corner case is rare but
    /// reachable: `destroy_entity(parent)` orphans `child`, then
    /// `destroy_entity(child)` despawns it without a hook fire. A
    /// boundary on `child` then needs dangling-collapse on next
    /// access. Cost is O(R) per call where R = live range count
    /// (typically < 10), so the always-on model is cheap enough to
    /// skip a separate orphan-destroy hook for PR-A.
    pub fn finalize_pending(&mut self, dom: &EcsDom) {
        let drained: Vec<PendingMutation> = {
            let mut pending = self
                .pending_remove
                .lock()
                .expect("pending_remove mutex poisoned");
            pending.drain(..).collect()
        };

        let mut guard = self.ranges.lock().expect("ranges mutex poisoned");
        for event in drained {
            match event {
                PendingMutation::Remove {
                    node,
                    parent,
                    removed_index,
                } => {
                    for range in guard.values_mut() {
                        adjust_ranges_for_removal(
                            std::slice::from_mut(range),
                            node,
                            parent,
                            removed_index,
                            dom,
                        );
                    }
                }
            }
        }

        // Dangling-collapse: see method-level doc for rationale.
        // owner_document fallback target itself may be destroyed
        // (e.g. test fixtures); in that case keep the previous
        // container identity but zero the offset so the boundary is
        // at least dimensionally well-formed (`!dom.contains(...)` is
        // surfaced as undefined / null by VM-side accessors per
        // WebIDL).
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
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn register_assigns_monotonic_ids() {
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let n = elem(&mut dom, "div");
        let r1 = reg.register(Range::new(n));
        let r2 = reg.register(Range::new(n));
        let r3 = reg.register(Range::new(n));
        assert_eq!(r1.0, 0);
        assert_eq!(r2.0, 1);
        assert_eq!(r3.0, 2);
    }

    #[test]
    fn unregister_does_not_recycle_id() {
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let n = elem(&mut dom, "div");
        let r1 = reg.register(Range::new(n));
        let _r2 = reg.register(Range::new(n));
        assert!(reg.unregister(r1).is_some());
        let r3 = reg.register(Range::new(n));
        assert_eq!(r3.0, 2, "RangeId 0 must NOT be recycled");
        assert!(reg.unregister(r1).is_none());
    }

    #[test]
    fn bridge_after_insert_increments_parent_offset() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let c0 = elem(&mut dom, "a");
        let _ = dom.append_child(parent, c0);

        let mut r = Range::new(parent);
        r.set_start(parent, 1);
        r.set_end(parent, 2);
        let id = reg.register(r);

        // Fire after_insert(parent, index=0): boundaries at offset > 0 → +1.
        bridge.after_insert(c0, parent, 0);

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_offset, 2);
            assert_eq!(range.end_offset, 3);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_text_change_clamps_boundary() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello");

        let mut r = Range::new(t);
        r.set_start(t, 2);
        r.set_end(t, 5);
        let id = reg.register(r);

        bridge.after_text_change(t, 3);

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_offset, 2);
            assert_eq!(range.end_offset, 3);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_replace_data_collapse_inside_splice() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello");

        let mut r = Range::new(t);
        r.set_start(t, 2);
        r.set_end(t, 3);
        let id = reg.register(r);

        // Splice (offset=1, count=3, new_data=3): boundaries at 2 / 3
        // both inside [1, 4] → collapse to 1.
        bridge.after_replace_data(t, 1, 3, 3);

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_offset, 1);
            assert_eq!(range.end_offset, 1);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_split_text_migrates_to_new_node() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let t = dom.create_text("hello world");
        let new_t = dom.create_text("");
        let _ = dom.append_child(parent, t);
        let _ = dom.append_child(parent, new_t);

        let mut r = Range::new(t);
        r.set_start(t, 3);
        r.set_end(t, 8);
        let id = reg.register(r);

        // Split at offset 5: end_offset 8 > 5 → migrate to (new_t, 3).
        bridge.after_split_text(t, new_t, 5, Some(parent), Some(0));

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, t);
            assert_eq!(range.start_offset, 3);
            assert_eq!(range.end_container, new_t);
            assert_eq!(range.end_offset, 3);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_split_text_parent_boundary_at_equal_slot_shifts() {
        // Spec §4.10 step 7.2 requires the parent boundary at exactly
        // `node_idx + 1` to shift. The `after_insert` hook fired by
        // the prior `insert_before` only shifts boundaries with
        // `off > node_idx + 1`, so the Bridge tops up the equality
        // case. Boundary at `parent + node_idx` (= the slot BEFORE the
        // original node) must stay unchanged because the node is not
        // re-inserted there.
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let t = dom.create_text("hello world");
        let trailing = elem(&mut dom, "span");
        let _ = dom.append_child(parent, t);
        let _ = dom.append_child(parent, trailing);

        // Boundary in the gap between `t` (idx 0) and `trailing` (idx 1):
        // offset 1.
        let mut r_gap = Range::new(parent);
        r_gap.set_start(parent, 1);
        r_gap.set_end(parent, 1);
        let id_gap = reg.register(r_gap);

        // Boundary before `t` (idx 0): offset 0 — must stay.
        let mut r_before = Range::new(parent);
        r_before.set_start(parent, 0);
        r_before.set_end(parent, 0);
        let id_before = reg.register(r_before);

        bridge.after_split_text(t, dom.create_text(""), 5, Some(parent), Some(0));

        reg.with_range(id_gap, &dom, |range, _| {
            assert_eq!(range.start_offset, 2, "equal-slot boundary shifted +1");
            assert_eq!(range.end_offset, 2);
        })
        .expect("gap range present");
        reg.with_range(id_before, &dom, |range, _| {
            assert_eq!(range.start_offset, 0, "before-node boundary unchanged");
            assert_eq!(range.end_offset, 0);
        })
        .expect("before range present");
    }

    #[test]
    fn bridge_after_split_text_node_side_equality_migrates_to_new_node_zero() {
        // Spec §4.10 step 7.4: boundary on `node` at exactly `offset`
        // migrates to `(new_node, 0)` (the `>= offset` lower-bound
        // covers this — see `after_split_text` trait doc).
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let t = dom.create_text("hello world");
        let new_t = dom.create_text("");
        let _ = dom.append_child(parent, t);
        let _ = dom.append_child(parent, new_t);

        let mut r = Range::new(t);
        r.set_start(t, 5);
        r.set_end(t, 5);
        let id = reg.register(r);

        bridge.after_split_text(t, new_t, 5, Some(parent), Some(0));

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, new_t);
            assert_eq!(range.start_offset, 0);
            assert_eq!(range.end_container, new_t);
            assert_eq!(range.end_offset, 0);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_normalize_merge_migrates_to_prev() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let prev = dom.create_text("helloworld");
        let merged = dom.create_text("");

        let mut r = Range::new(merged);
        r.set_start(merged, 2);
        r.set_end(merged, 4);
        let id = reg.register(r);

        // prev had len 5 ("hello") before absorbing "world".
        bridge.after_normalize_merge(merged, prev, 5, None, None);

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, prev);
            assert_eq!(range.start_offset, 7);
            assert_eq!(range.end_container, prev);
            assert_eq!(range.end_offset, 9);
        })
        .expect("range present");
    }

    #[test]
    fn bridge_after_normalize_merge_parent_boundary_at_idx_migrates_to_splice_point() {
        // Spec §4.5 step 6.4: boundary on `parent` at exactly
        // `merged_child_index` migrates to `(prev, prev_old_len)` — the
        // merge splice point. The Bridge applies ONLY the equality
        // case; boundaries at `off > merged_child_index` are handled by
        // the subsequent `after_remove` hook fired from the Normalize
        // handler's `remove_child(parent, merged_child)`.
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "p");
        let prev = dom.create_text("helloworld"); // post-merge state
        let merged = dom.create_text(""); // post-merge empty, pre-detach
        let trailing = elem(&mut dom, "span");
        let _ = dom.append_child(parent, prev);
        let _ = dom.append_child(parent, merged);
        let _ = dom.append_child(parent, trailing);

        // Boundary in the gap between `prev` and `merged_child` (idx 1):
        // must migrate to (prev, 5) post-merge.
        let mut r_eq = Range::new(parent);
        r_eq.set_start(parent, 1);
        r_eq.set_end(parent, 1);
        let id_eq = reg.register(r_eq);

        // Boundary BEFORE `prev` (idx 0): must stay unchanged.
        let mut r_before = Range::new(parent);
        r_before.set_start(parent, 0);
        r_before.set_end(parent, 0);
        let id_before = reg.register(r_before);

        bridge.after_normalize_merge(merged, prev, 5, Some(parent), Some(1));

        reg.with_range(id_eq, &dom, |range, _| {
            assert_eq!(range.start_container, prev);
            assert_eq!(range.start_offset, 5);
            assert_eq!(range.end_container, prev);
            assert_eq!(range.end_offset, 5);
        })
        .expect("eq range present");
        reg.with_range(id_before, &dom, |range, _| {
            assert_eq!(range.start_container, parent);
            assert_eq!(range.start_offset, 0);
        })
        .expect("before range present");
    }

    #[test]
    fn bridge_after_remove_queues_until_finalize() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "section");
        let grandchild = dom.create_text("inner");
        let _ = dom.append_child(parent, child);
        let _ = dom.append_child(child, grandchild);
        // `child` is still in the tree (we did NOT actually remove it
        // — we only fire the hook for testing). is_ancestor_or_self
        // (child, grandchild) is true, so the boundary collapses to
        // (parent, 0) per §5.5 step 4.

        // Boundary inside the subtree to be removed.
        let mut r = Range::new(grandchild);
        r.set_start(grandchild, 2);
        r.set_end(grandchild, 3);
        // owner_document = parent so dangling-collapse has somewhere safe to go.
        r.owner_document = parent;
        let id = reg.register(r);

        // Mid-mutation: queue after_remove. The boundaries should NOT
        // yet be adjusted (queue is the lazy buffer).
        bridge.after_remove(child, parent, 0);

        // Reading WITHOUT draining... actually `with_range` always
        // drains. We verify the drain by reading and observing the
        // collapsed state.
        reg.with_range(id, &dom, |range, _| {
            // child is still in the tree (we did NOT actually remove
            // it — we only fired the hook for testing).
            // is_ancestor_or_self(child, grandchild) is true, so the
            // boundary collapses to (parent, 0) per §5.5 step 4.
            assert_eq!(range.start_container, parent);
            assert_eq!(range.start_offset, 0);
            assert_eq!(range.end_container, parent);
            assert_eq!(range.end_offset, 0);
        })
        .expect("range present");
    }

    #[test]
    fn finalize_pending_idempotent_on_empty_queue() {
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let n = elem(&mut dom, "div");
        let r = Range::new(n);
        let id = reg.register(r);

        // Drain when empty — must be no-op.
        reg.finalize_pending(&dom);
        reg.finalize_pending(&dom);

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, n);
        })
        .expect("range present");
    }

    #[test]
    fn dangling_collapse_redirects_to_owner_document() {
        // The orphan-then-destroy path that the dangling-collapse
        // fallback covers: `destroy_entity(parent)` orphans `inner`
        // (no after_remove for inner per the lazy-collapse contract),
        // then `destroy_entity(inner)` despawns it but — because
        // inner is now an orphan with no parent — `EcsDom` fires no
        // after_remove (prereq #185 fires only when `(parent, idx)`
        // is `Some`). No queue event exists; only the unconditional
        // dangling-collapse pass catches the dead boundary.
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let doc = elem(&mut dom, "html");
        let target = elem(&mut dom, "section");
        let inner = dom.create_text("hello");
        let _ = dom.append_child(doc, target);
        let _ = dom.append_child(target, inner);

        let mut r = Range::new_with_owner(inner, doc);
        r.set_start(inner, 2);
        r.set_end(inner, 3);
        let id = reg.register(r);

        assert!(dom.destroy_entity(target));
        // inner is orphaned but still in dom.
        assert!(dom.contains(inner));
        assert!(dom.get_parent(inner).is_none());

        // Now destroy inner directly — no after_remove fires because
        // it has no parent.
        assert!(dom.destroy_entity(inner));
        assert!(!dom.contains(inner));

        reg.with_range(id, &dom, |range, _| {
            // Dangling-collapse moved both boundaries to the
            // owner_document (`doc`) at offset 0.
            assert_eq!(range.start_container, doc);
            assert_eq!(range.start_offset, 0);
            assert_eq!(range.end_container, doc);
            assert_eq!(range.end_offset, 0);
        })
        .expect("range present");
    }

    #[test]
    fn dangling_collapse_keeps_offset_if_owner_doc_also_destroyed() {
        // Edge: if owner_document itself is destroyed, dangling-collapse
        // keeps the previous container identity but zeroes the offset
        // — boundary is at least dimensionally well-formed.
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let doc = elem(&mut dom, "html");
        let inner = dom.create_text("hello");

        let mut r = Range::new_with_owner(inner, doc);
        r.set_start(inner, 2);
        r.set_end(inner, 3);
        let id = reg.register(r);

        assert!(dom.destroy_entity(inner));
        assert!(dom.destroy_entity(doc));

        reg.with_range(id, &dom, |range, _| {
            // owner_document destroyed too — container identity stays
            // (won't redirect to a destroyed entity) but offset
            // zeroes.
            assert_eq!(range.start_offset, 0);
            assert_eq!(range.end_offset, 0);
        })
        .expect("range present");
    }

    #[test]
    fn for_each_entity_visits_three_per_range() {
        let (mut reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let doc = elem(&mut dom, "html");
        let n1 = elem(&mut dom, "div");
        let n2 = elem(&mut dom, "p");

        let mut r = Range::new_with_owner(n1, doc);
        r.set_end(n2, 0);
        reg.register(r);

        let mut entities = Vec::new();
        reg.for_each_entity(|e| entities.push(e));
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&n1));
        assert!(entities.contains(&n2));
        assert!(entities.contains(&doc));
    }

    #[test]
    fn clear_drops_all_ranges_and_pending() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "p");
        let _ = dom.append_child(parent, child);

        let _id = reg.register(Range::new(parent));
        bridge.after_remove(child, parent, 0);

        assert_eq!(reg.len(), 1);
        reg.clear();
        assert!(reg.is_empty());
        // Pending was cleared too: finalize is a no-op now.
        reg.finalize_pending(&dom);
    }

    #[test]
    fn bridge_send_sync_marker() {
        // Compile-time check: Bridge must be Send + Sync per the
        // `MutationHook: Send + Sync` supertrait. If a future field
        // breaks this (e.g. switching to Rc<RefCell<>>), this fails
        // to compile.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Bridge>();
        assert_send_sync::<LiveRangeRegistry>();
    }
}
