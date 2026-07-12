use super::*;
use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord};

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

#[test]
fn notify_child_list() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        parent,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    let session_record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target: parent,
        added_nodes: vec![child],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };

    reg.notify(&dom, &session_record);

    assert!(reg.has_pending_records());
    let records = reg.take_records(id);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].mutation_type, "childList");
    assert_eq!(records[0].added_nodes, vec![child]);
}

#[test]
fn notify_attribute_with_filter() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        el,
        MutationObserverInit {
            attributes: true,
            attribute_filter: Some(vec!["class".to_string()]),
            attribute_old_value: true,
            ..Default::default()
        },
    );

    // "class" attribute should match.
    let record_class = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::Attribute,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: Some("old".to_string()),
    };
    reg.notify(&dom, &record_class);

    // "id" attribute should NOT match the filter.
    let record_id = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::Attribute,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("id".to_string()),
        old_value: Some("test".to_string()),
    };
    reg.notify(&dom, &record_id);

    let records = reg.take_records(id);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].attribute_name, Some("class".to_string()));
    assert_eq!(records[0].old_value, Some("old".to_string()));
}

#[test]
fn disconnect_clears_records_and_targets() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        el,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    reg.notify(&dom, &record);

    reg.disconnect(&mut dom, id);
    assert!(!reg.has_pending_records());
    assert!(reg.take_records(id).is_empty());

    // Targets cleared: a post-disconnect notify matches nothing.
    reg.notify(&dom, &record);
    assert!(!reg.has_pending_records());
}

#[test]
fn despawn_auto_cleans_targets() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        el,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    // Despawning the entity drops its MutationObservedBy component, so the
    // observer no longer matches — no manual scrub needed.
    let _ = dom.destroy_entity(el);

    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    reg.notify(&dom, &record);
    assert!(!reg.has_pending_records());
}

#[test]
fn subtree_observer() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");
    let _ = dom.append_child(parent, child);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        parent,
        MutationObserverInit {
            attributes: true,
            subtree: true,
            ..Default::default()
        },
    );

    // Record on the child is matched via inclusive-ancestor walk to the
    // parent's subtree observer.
    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: None,
    };
    reg.notify(&dom, &record);

    let records = reg.take_records(id);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].target, child);
}

#[test]
fn two_observers_same_target() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let id_a = reg.register();
    let id_b = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        ..Default::default()
    };
    reg.observe(&mut dom, id_a, el, init.clone());
    reg.observe(&mut dom, id_b, el, init);

    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    reg.notify(&dom, &record);

    assert_eq!(reg.take_records(id_a).len(), 1);
    assert_eq!(reg.take_records(id_b).len(), 1);
}

#[test]
fn observe_unregistered_id_is_noop() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    // `ghost` is never register()'d — observe must not attach a stale
    // component nor leave registry state inconsistent.
    let ghost = MutationObserverId::from_raw(999);
    reg.observe(
        &mut dom,
        ghost,
        el,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );
    assert!(
        dom.world().get::<&MutationObservedBy>(el).is_err(),
        "observe on an unregistered id must not attach a component"
    );
}

#[test]
fn take_pending_observers_drains_in_append_order() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let init = MutationObserverInit {
        child_list: true,
        ..Default::default()
    };
    let mut ids = Vec::new();
    for _ in 0..4 {
        let id = reg.register();
        reg.observe(&mut dom, id, el, init.clone());
        ids.push(id.raw());
    }

    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target: el,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    };
    reg.notify(&dom, &record);

    // One record on one target enqueues all four in registered-observer-list
    // (= walk = registration) order; here that is also `ids`. The id-vs-append
    // *divergence* is covered by `pending_observers_preserve_append_order_not_id_order`.
    let got: Vec<u64> = reg
        .take_pending_observers()
        .into_iter()
        .map(MutationObserverId::raw)
        .collect();
    assert_eq!(got, ids, "delivery follows append (registration) order");
    // Draining the notifySet leaves it empty (step 3 "empty pending").
    assert!(reg.take_pending_observers().is_empty());
}

/// Codex R1 (P2): the §4.3 notifySet is the pending-mutation-observers set,
/// NOT derived from record-queue contents — so `takeRecords()` draining an
/// observer's queue before the microtask must NOT drop it from `notifySet`
/// (else its step-6.3 transient clear is skipped and the transient leaks).
#[test]
fn pending_observer_survives_take_records() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        el,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    let child = elem(&mut dom, "span");
    let record = child_list_added(el, vec![child]);
    reg.notify(&dom, &record);

    // The page calls takeRecords() before the microtask — empties the queue
    // but must not remove the observer from the pending notifySet.
    assert_eq!(reg.take_records(id).len(), 1);
    assert!(!reg.has_pending_records(), "record queue drained");
    assert_eq!(
        reg.take_pending_observers(),
        vec![id],
        "observer is still in the notifySet after takeRecords()"
    );
}

/// S5-3c pending-records keepalive clause: after `notify` queues a record for an
/// observer, `observers_with_pending_records` contains its id; after
/// `take_records` drains the queue, it does not. This is the engine-indep truth
/// source for the GC-keepalive "has ≥1 pending undelivered record" disjunct.
#[test]
fn observers_with_pending_records_tracks_nonempty_queue() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());

    // No record yet → not pending.
    assert!(
        reg.observers_with_pending_records().is_empty(),
        "a registered observer with an empty queue is not pending"
    );

    let child = elem(&mut dom, "span");
    reg.notify(&dom, &child_list_added(el, vec![child]));
    let pending = reg.observers_with_pending_records();
    assert!(
        pending.contains(&id.raw()),
        "an observer with a queued record is pending"
    );
    assert_eq!(pending.len(), 1);

    // Drain the queue (models `takeRecords()`): the record is gone → not pending,
    // even though the observer stays in the `pending` notifySet (keyed on
    // NON-EMPTY `records`, not `pending` membership).
    assert_eq!(reg.take_records(id).len(), 1);
    assert!(
        reg.observers_with_pending_records().is_empty(),
        "after take_records the queue is empty ⇒ no longer pending (not over-kept on stale `pending`)"
    );
    // The observer IS still in the pending notifySet (takeRecords doesn't remove
    // it) — proving the two signals are distinct.
    assert_eq!(
        reg.take_pending_observers(),
        vec![id],
        "takeRecords leaves the observer in the pending notifySet"
    );
}

/// S5-3c registry-side leak fix: `retire_collected` drops the registry-internal
/// `records` row (and any `pending` membership) for a GC-collected observer, so
/// the registry does not grow monotonically alongside the (already-pruned)
/// HostData binding row. Dom-free — a collected observer is guaranteed
/// non-observing, so no target-list scrub is needed.
#[test]
fn retire_collected_drops_records_row() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());
    assert_eq!(
        reg.records_len(),
        1,
        "the registered observer has a records row"
    );

    // The GC-sweep retirement (id captured from a pruned binding row).
    reg.retire_collected(id);
    assert_eq!(
        reg.records_len(),
        0,
        "retire_collected drops the registry-internal records row (no residual)"
    );
    // Idempotent + monotonic id preserved: re-registering yields a FRESH id
    // (next_id untouched), and a redundant retire of an already-gone id is a no-op.
    reg.retire_collected(id);
    let id2 = reg.register();
    assert_ne!(id2.raw(), id.raw(), "next_id stays monotonic across retire");
}

/// `retire_collected` also clears a stale `pending` (notifySet) membership for
/// the collected observer, so a later delivery cycle cannot iterate a retired id.
#[test]
fn retire_collected_clears_pending_membership() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());
    let child = elem(&mut dom, "span");
    reg.notify(&dom, &child_list_added(el, vec![child]));
    // Drain the record queue but leave the observer in the pending notifySet
    // (models takeRecords()), then retire it.
    let _ = reg.take_records(id);
    reg.retire_collected(id);
    assert!(
        reg.take_pending_observers().is_empty(),
        "retire_collected removes the observer from the pending notifySet"
    );
    assert_eq!(reg.records_len(), 0);
}

/// A `ChildList` mutation record on `target` adding `added`.
fn child_list_added(target: Entity, added: Vec<Entity>) -> SessionRecord {
    SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::ChildList,
        target,
        added_nodes: added,
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}

// --- observing_observer_ids (the S5-3c GC-keepalive membership query) -------

fn child_list_init() -> MutationObserverInit {
    MutationObserverInit {
        child_list: true,
        ..Default::default()
    }
}

#[test]
fn observing_ids_empty_world_is_empty() {
    let dom = EcsDom::new();
    assert!(
        observing_observer_ids(&dom).is_empty(),
        "no observations ⇒ empty membership set"
    );
}

#[test]
fn observing_ids_present_after_observe() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());

    let ids = observing_observer_ids(&dom);
    assert!(
        ids.contains(&id.raw()),
        "an actively-observing observer is a member"
    );
    assert_eq!(ids.len(), 1);
}

#[test]
fn observing_ids_absent_after_disconnect() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());
    reg.disconnect(&mut dom, id);

    assert!(
        observing_observer_ids(&dom).is_empty(),
        "disconnect ends the only observation ⇒ non-member (collectible)"
    );
}

#[test]
fn observing_ids_absent_after_despawn_of_sole_target() {
    // The despawn-safety proof at the unit level: a despawned entity's
    // `MutationObservedBy` vanishes with the entity, so the observer's
    // membership drops to zero with no registry decrement hook.
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(&mut dom, id, el, child_list_init());
    assert!(observing_observer_ids(&dom).contains(&id.raw()));

    let _ = dom.destroy_entity(el);
    assert!(
        observing_observer_ids(&dom).is_empty(),
        "despawn of the sole observed entity drops membership (despawn-safe by construction)"
    );
}

#[test]
fn observing_ids_include_transient_membership() {
    // A transient registered observer is a registered observer (§4.3): it counts
    // as membership automatically (the transient entry lives in the same
    // `MutationObservedBy` component the query flat-maps). A transient added onto
    // a node that carries NO permanent registration for `id` still makes `id` a
    // member; clearing that transient (notify step 6.3) drops it.
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let removed = elem(&mut dom, "span");
    let _ = dom.append_child(root, removed);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        root,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );
    // `add_transient_observers(root, [removed])` walks `removed`'s inclusive
    // ancestors, finds the `subtree:true` permanent on `root`, and appends a
    // transient copy onto `removed` (tagged `transient:true`). `removed` carries
    // ONLY that transient (no permanent registration of `id`).
    reg.add_transient_observers(&mut dom, root, &[removed]);
    {
        // Confirm the transient landed on `removed` and is the only registration
        // there (so its membership contribution is genuinely the transient's).
        let comp = dom
            .world()
            .get::<&MutationObservedBy>(removed)
            .expect("removed carries a transient registration");
        assert!(comp.0.iter().all(|o| o.transient && o.observer == id));
    }

    let ids = observing_observer_ids(&dom);
    assert!(
        ids.contains(&id.raw()),
        "a transient registered observer counts as membership automatically"
    );

    // Clear the transient (notify step 6.3): the transient on `removed` is gone,
    // but the permanent on `root` still anchors membership.
    clear_transient_observers(&mut dom, id);
    assert!(
        dom.world().get::<&MutationObservedBy>(removed).is_err(),
        "clearing removed the transient (and emptied removed's component)"
    );
    assert!(
        observing_observer_ids(&dom).contains(&id.raw()),
        "the permanent registration keeps membership after the transient clear"
    );

    // Drop the permanent too → no membership remains (fully collectible).
    reg.disconnect(&mut dom, id);
    assert!(observing_observer_ids(&dom).is_empty());
}

#[test]
fn observing_ids_two_observers_distinct_targets_both_present() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "section");
    let mut reg = MutationObserverRegistry::new();
    let id_a = reg.register();
    let id_b = reg.register();
    reg.observe(&mut dom, id_a, a, child_list_init());
    reg.observe(&mut dom, id_b, b, child_list_init());

    let ids = observing_observer_ids(&dom);
    assert!(ids.contains(&id_a.raw()));
    assert!(ids.contains(&id_b.raw()));
    assert_eq!(ids.len(), 2);
}
