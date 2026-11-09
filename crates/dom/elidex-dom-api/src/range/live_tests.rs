use super::*;
use elidex_ecs::Attributes;

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element_ownerless_fixture(tag, Attributes::default())
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
    let t = dom.create_text_ownerless_fixture("hello");

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
    dom.set_mutation_dispatcher(crate::range::make_range_only_test_dispatcher(bridge));
    let parent = elem(&mut dom, "p");
    let t = dom.create_text_ownerless_fixture("hello");
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
    let t = dom.create_text_ownerless_fixture("hello");

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
    let t = dom.create_text_ownerless_fixture("hello world");
    let new_t = dom.create_text_ownerless_fixture("");
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
    let t = dom.create_text_ownerless_fixture("hello world");
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

    bridge.after_split_text(
        t,
        dom.create_text_ownerless_fixture(""),
        5,
        Some(parent),
        Some(0),
    );

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
    let t = dom.create_text_ownerless_fixture("hello world");
    let new_t = dom.create_text_ownerless_fixture("");
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
    let prev = dom.create_text_ownerless_fixture("helloworld");
    let merged = dom.create_text_ownerless_fixture("");

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
    let prev = dom.create_text_ownerless_fixture("helloworld"); // post-merge state
    let merged = dom.create_text_ownerless_fixture(""); // post-merge empty, pre-detach
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
    let grandchild = dom.create_text_ownerless_fixture("inner");
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
    dom.set_mutation_dispatcher(crate::range::make_range_only_test_dispatcher(bridge));
    let parent = elem(&mut dom, "div");
    let target = elem(&mut dom, "section");
    let descendant = dom.create_text_ownerless_fixture("hello");
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
    let inner = dom.create_text_ownerless_fixture("hello");
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
    let inner = dom.create_text_ownerless_fixture("hello");

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
    // Compile-time check: LiveRangeBridge must be Send + Sync per
    // the `MutationDispatcher: Send + Sync` supertrait that
    // `ConsumerDispatcher` (the actual dispatcher impl) inherits.
    // If a future field breaks this (e.g. switching to
    // Rc<RefCell<>>), this fails to compile.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LiveRangeBridge>();
    assert_send_sync::<LiveRangeRegistry>();
}
