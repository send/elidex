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

/// A `ChildList` mutation record on `target` adding `added`.
fn child_list_added(target: Entity, added: Vec<Entity>) -> SessionRecord {
    SessionRecord {
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
