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
//! - [`LiveRangeBridge`] (`EcsDom` dispatcher consumer) is a small
//!   struct that implements [`elidex_ecs::MutationDispatcher`] by
//!   matching on each [`elidex_ecs::MutationEvent`] variant and
//!   forwarding into the shared `ranges`
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

/// Test-only single-consumer `MutationDispatcher` adapter so legacy
/// tests can install a [`LiveRangeBridge`] directly on `EcsDom`
/// without constructing a full [`crate::ConsumerDispatcher`] (which
/// would require pairing a NodeIteratorAdjuster too).  Production
/// code installs the typed [`crate::ConsumerDispatcher`].
#[cfg(test)]
impl elidex_ecs::MutationDispatcher for LiveRangeBridge {
    fn dispatch(&mut self, event: &elidex_ecs::MutationEvent<'_>, dom: &mut EcsDom) {
        self.handle(event, dom);
    }
}

impl LiveRangeBridge {
    /// Single-method dispatch entry point invoked by
    /// [`crate::ConsumerDispatcher`].  Pattern-matches the
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
            } => self.after_remove_with_descendants(node, parent, removed_index, descendants, dom),
            MutationEvent::Insert {
                node,
                parent,
                index,
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
            MutationEvent::AttributeChange { .. } => {
                // Range live-tracking does not depend on attribute
                // mutations — Range boundaries are tree-positional.
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
    fn bridge_after_replace_data_with_overflow_count_uses_clamped_span() {
        // PR186 R3 #1 regression: `replace_text_data` must pass the
        // CLAMPED count to `after_replace_data` so the bridge
        // boundary-adjustment math operates on the actual spliced
        // span. With offset=1, count=99 (clamped to 4), replacement=""
        // on "hello": a boundary at off=5 (the OLD end) should shift
        // by `replacement_len - clamped_count = 0 - 4 = -4` →
        // off=1, NOT collapse to offset=1 via the "inside splice"
        // rule that would fire if the unclamped count=99 were passed.
        let mut dom = EcsDom::new();
        let (mut reg, bridge) = LiveRangeRegistry::new_pair();
        dom.set_mutation_dispatcher(Box::new(bridge));
        let parent = elem(&mut dom, "p");
        let t = dom.create_text("hello");
        let _ = dom.append_child(parent, t);

        let mut r = Range::new(t);
        r.set_start(t, 5);
        r.set_end(t, 5);
        let id = reg.register(r);

        // count=99 past end → clamped to 4 ("ello"). Result: "h" +
        // "" + "" = "h" (length 1). Boundary at 5 was past the
        // splice end; spec rule shifts it by `0 - 4 = -4` → 1.
        assert_eq!(dom.replace_text_data(t, 1, 99, ""), Some(1));

        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_offset, 1);
            assert_eq!(range.end_offset, 1);
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
        // `off > node_idx + 1`, so the LiveRangeBridge tops up the equality
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
    fn bridge_after_split_text_node_side_equality_stays_on_original() {
        // Spec §4.10 step 7.2 / 7.3 use strict-greater (`off > offset`)
        // to migrate boundaries to `new_node`. The equality case
        // (`off == offset`) stays on the original `node` at its new
        // end — a Range collapsed at the split point is preserved on
        // the original node, NOT migrated to `(new_node, 0)`.
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
            assert_eq!(range.start_container, t, "equality boundary stays on node");
            assert_eq!(range.start_offset, 5);
            assert_eq!(range.end_container, t);
            assert_eq!(range.end_offset, 5);
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
        // merge splice point. The LiveRangeBridge applies ONLY the equality
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

        // Apply after_remove synchronously with the snapshot. Since
        // the snapshot includes `child` + `grandchild`, the boundary
        // on `grandchild` collapses to (parent, 0) per §5.5 step 4.
        bridge.after_remove_with_descendants(child, parent, 0, &[child, grandchild], &dom);

        reg.with_range(id, &dom, |range, _| {
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
    fn destroy_entity_collapses_boundary_on_live_descendant() {
        // PR186 R2 #3 regression: `destroy_entity` fires the
        // `after_remove` hook BEFORE orphaning children, so a Range
        // boundary on a still-live descendant of the destroyed subtree
        // is reached via `is_ancestor_or_self` and collapsed to
        // `(parent, removed_index)` per WHATWG §5.5 remove step 4.
        let (mut reg, bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        dom.set_mutation_dispatcher(Box::new(bridge));
        let parent = elem(&mut dom, "div");
        let target = elem(&mut dom, "section");
        let descendant = dom.create_text("hello");
        let _ = dom.append_child(parent, target);
        let _ = dom.append_child(target, descendant);

        let mut r = Range::new(descendant);
        r.owner_document = parent;
        r.set_start(descendant, 2);
        r.set_end(descendant, 4);
        let id = reg.register(r);

        // Pre-condition: target is at index 0 inside parent.
        assert!(dom.destroy_entity(target));

        // Post-condition: boundary collapsed to (parent, 0) — the
        // removed_index of `target`. WITHOUT the destroy_entity
        // ordering fix, the descendant walk via `is_ancestor_or_self`
        // would not reach `target` from `descendant` (children's parent
        // links cleared pre-fire) and the boundary would silently stay
        // on the orphaned `descendant`.
        reg.with_range(id, &dom, |range, _| {
            assert_eq!(range.start_container, parent);
            assert_eq!(range.start_offset, 0);
            assert_eq!(range.end_container, parent);
            assert_eq!(range.end_offset, 0);
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
    fn clear_drops_all_ranges() {
        let (mut reg, mut bridge) = LiveRangeRegistry::new_pair();
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "p");
        let _ = dom.append_child(parent, child);

        let _id = reg.register(Range::new(parent));
        bridge.after_remove_with_descendants(child, parent, 0, &[child], &dom);

        assert_eq!(reg.len(), 1);
        reg.clear();
        assert!(reg.is_empty());
        // Post-clear, dangling-collapse on an empty range set is a
        // no-op.
        reg.finalize_pending(&dom);
    }

    #[test]
    fn bridge_send_sync_marker() {
        // Compile-time check: LiveRangeBridge must be Send + Sync per the
        // `MutationHook: Send + Sync` supertrait. If a future field
        // breaks this (e.g. switching to Rc<RefCell<>>), this fails
        // to compile.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LiveRangeBridge>();
        assert_send_sync::<LiveRangeRegistry>();
    }
}
