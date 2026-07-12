use super::*;
use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord};

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}
/// Codex R1 (P2) + R8-L1: observing a node that holds ONLY a transient
/// registration matches it in §4.3.1 step 7 (a transient IS a registered
/// observer), so step 8 does NOT add a spurious permanent — the transient is
/// updated in place and lapses on the next microtask. The step-7.1 clear is
/// source-keyed, so an unrelated removed subtree's transient is NOT cleared.
#[test]
fn observe_on_transient_only_node_keeps_other_transients() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "div");
    let _ = dom.append_child(root, a);
    let _ = dom.append_child(root, b);

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

    // Remove both children (coalesced removal) → each gets a transient for id.
    assert!(dom.remove_child(root, a));
    assert!(dom.remove_child(root, b));
    reg.add_transient_observers(&mut dom, root, &[a, b]);
    assert_eq!(count_entries(&dom, a, id, true), 1);
    assert_eq!(count_entries(&dom, b, id, true), 1);

    // Observe `a` (which has ONLY a transient for id) before the microtask.
    reg.observe(
        &mut dom,
        id,
        a,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );
    // §4.3.1 step 7 matches `a`'s transient → step 8 does NOT add a permanent;
    // `a` stays transient (and lapses next microtask). `b`'s unrelated transient
    // (different source) is NOT cleared.
    assert_eq!(
        count_entries(&dom, a, id, false),
        0,
        "no spurious permanent added (step 7 matched the transient)"
    );
    assert_eq!(
        count_entries(&dom, a, id, true),
        1,
        "a's transient updated in place"
    );
    assert_eq!(
        count_entries(&dom, b, id, true),
        1,
        "b's transient survives"
    );
}

// ---- Transient registered observers (DOM §4.3 / §4.2.3 step 15) ----

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

/// Count entries on `node`'s registered observer list matching `observer`
/// and `transient`-ness.
fn count_entries(
    dom: &EcsDom,
    node: Entity,
    observer: MutationObserverId,
    transient: bool,
) -> usize {
    dom.world().get::<&MutationObservedBy>(node).map_or(0, |c| {
        c.0.iter()
            .filter(|o| o.observer == observer && o.transient == transient)
            .count()
    })
}

/// Headline §1 scenario: remove a subtree-observed mid-node, then mutate the
/// detached subtree — the mutation still reaches the ancestor's observer via
/// the transient, and is cleared after delivery.
#[test]
fn transient_keeps_detached_subtree_observed_until_cleared() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        grandparent,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );

    // removeChild(mid): detach, then create transients (the notify_one order).
    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);

    // mid carries one transient for `id` carrying the original subtree init.
    {
        let comp = dom
            .world()
            .get::<&MutationObservedBy>(mid)
            .expect("transient appended to removed node");
        let t = comp
            .0
            .iter()
            .find(|o| o.observer == id && o.transient)
            .expect("transient entry for the ancestor observer");
        assert!(t.init.subtree, "transient carries original subtree:true");
        assert!(t.init.child_list);
    }

    // Mutating the detached subtree (mid.appendChild(x)) reaches `id`.
    let x = elem(&mut dom, "span");
    let record = child_list_added(mid, vec![x]);
    assert!(reg.notify(&dom, &record));
    assert_eq!(reg.take_records(id).len(), 1);

    // Clearing the transient (delivery step 6.3) stops further delivery.
    clear_transient_observers(&mut dom, id);
    assert_eq!(count_entries(&dom, mid, id, true), 0);
    assert!(!reg.notify(&dom, &record));
}

/// The transient is appended to the removed node ONLY (not its descendants);
/// a mutation deep inside the detached subtree still reaches the observer
/// because `notify` walks the mutated node's inclusive ancestors up through
/// the removed node, which now carries the transient (subtree:true).
#[test]
fn transient_on_removed_node_catches_deep_descendant_mutation() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    let grandchild = elem(&mut dom, "div");
    let _ = dom.append_child(root, mid);
    let _ = dom.append_child(mid, child);
    let _ = dom.append_child(child, grandchild);

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

    assert!(dom.remove_child(root, mid));
    reg.add_transient_observers(&mut dom, root, &[mid]);
    // Only `mid` carries the transient — child/grandchild do not.
    assert_eq!(count_entries(&dom, mid, id, true), 1);
    assert_eq!(count_entries(&dom, child, id, true), 0);
    assert_eq!(count_entries(&dom, grandchild, id, true), 0);

    // A mutation two levels below the removed node still reaches the observer.
    let x = elem(&mut dom, "span");
    let record = child_list_added(grandchild, vec![x]);
    assert!(reg.notify(&dom, &record));
    assert_eq!(reg.take_records(id).len(), 1);
}

/// A non-`subtree` ancestor observer creates no transient (step 15 gates on
/// `options["subtree"]`).
#[test]
fn no_transient_for_non_subtree_ancestor() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(parent, mid);

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

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    assert!(
        dom.world().get::<&MutationObservedBy>(mid).is_err(),
        "no transient component on the removed node"
    );
}

/// A removed node's OWN registered observer is left permanent — only the
/// inherited ancestor observation is appended as transient.
#[test]
fn own_observer_not_made_transient() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id_own = reg.register();
    let id_anc = reg.register();
    reg.observe(
        &mut dom,
        id_own,
        mid,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );
    reg.observe(
        &mut dom,
        id_anc,
        grandparent,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);

    assert_eq!(
        count_entries(&dom, mid, id_own, false),
        1,
        "own stays permanent"
    );
    assert_eq!(
        count_entries(&dom, mid, id_own, true),
        0,
        "own not transient"
    );
    assert_eq!(
        count_entries(&dom, mid, id_anc, true),
        1,
        "ancestor is transient"
    );
}

/// `clear_transient_observers` removes only the named observer's transients.
#[test]
fn clear_targets_only_the_named_observer() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id_a = reg.register();
    let id_b = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        subtree: true,
        ..Default::default()
    };
    reg.observe(&mut dom, id_a, parent, init.clone());
    reg.observe(&mut dom, id_b, parent, init);

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    assert_eq!(count_entries(&dom, mid, id_a, true), 1);
    assert_eq!(count_entries(&dom, mid, id_b, true), 1);

    clear_transient_observers(&mut dom, id_a);
    assert_eq!(count_entries(&dom, mid, id_a, true), 0, "a cleared");
    assert_eq!(count_entries(&dom, mid, id_b, true), 1, "b kept");
}

/// A coalesced removal (replaceChildren / replaceChild) carries its removed
/// set on one record — every removed subtree-root gets a transient (§2.4).
#[test]
fn coalesced_removal_creates_transient_per_removed_node() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let c1 = elem(&mut dom, "div");
    let c2 = elem(&mut dom, "div");
    let _ = dom.append_child(parent, c1);
    let _ = dom.append_child(parent, c2);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    reg.observe(
        &mut dom,
        id,
        parent,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );

    // replaceChildren(): both children removed under one coalesced record.
    assert!(dom.remove_child(parent, c1));
    assert!(dom.remove_child(parent, c2));
    reg.add_transient_observers(&mut dom, parent, &[c1, c2]);

    assert_eq!(count_entries(&dom, c1, id, true), 1);
    assert_eq!(count_entries(&dom, c2, id, true), 1);
}

/// §4.3.2 step 1 collapse: a removed node holding both a permanent and a
/// transient registration for the SAME observer yields ONE record.
#[test]
fn notify_collapses_permanent_and_transient_same_observer() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    // Same observer registered on the ancestor (subtree) AND directly on mid.
    reg.observe(
        &mut dom,
        id,
        grandparent,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );
    reg.observe(
        &mut dom,
        id,
        mid,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    // mid now has permanent + transient entries for `id`.
    assert_eq!(count_entries(&dom, mid, id, false), 1);
    assert_eq!(count_entries(&dom, mid, id, true), 1);

    let x = elem(&mut dom, "span");
    let record = child_list_added(mid, vec![x]);
    assert!(reg.notify(&dom, &record));
    assert_eq!(
        reg.take_records(id).len(),
        1,
        "duplicate matches collapse to one record per observer"
    );
}

/// §4.3.2 step 3.2.3 old-value union under collapse: when one observer
/// matches via a registration that requests the old value AND one that does
/// not, the collapsed record carries the old value regardless of walk order
/// (a non-requesting match never resets a value a requesting match set).
#[test]
fn notify_collapse_old_value_union_is_order_independent() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, child);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    // Walk order is child→parent→grandparent. The requesting registration is
    // on `parent` (processed FIRST), the non-requesting on `grandparent`
    // (processed SECOND) — so this exercises "later non-requesting match must
    // not reset the already-set old value".
    reg.observe(
        &mut dom,
        id,
        parent,
        MutationObserverInit {
            attributes: true,
            attribute_old_value: true,
            subtree: true,
            ..Default::default()
        },
    );
    reg.observe(
        &mut dom,
        id,
        grandparent,
        MutationObserverInit {
            attributes: true,
            subtree: true,
            ..Default::default()
        },
    );

    let record = SessionRecord {
        parent_was_connected: false,
        kind: MutationKind::Attribute,
        target: child,
        added_nodes: vec![],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: Some("class".to_string()),
        old_value: Some("old".to_string()),
    };
    assert!(reg.notify(&dom, &record));
    let records = reg.take_records(id);
    assert_eq!(records.len(), 1, "collapsed to one record");
    assert_eq!(
        records[0].old_value,
        Some("old".to_string()),
        "the requesting registration's old value survives the non-requesting match"
    );
}

/// §4.3.2 step 1 collapse (pre-existing dup fix): one observer registered on
/// two inclusive ancestors of the target yields ONE record per mutation.
#[test]
fn notify_collapses_observer_on_two_ancestors() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, child);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        subtree: true,
        ..Default::default()
    };
    reg.observe(&mut dom, id, grandparent, init.clone());
    reg.observe(&mut dom, id, parent, init);

    let x = elem(&mut dom, "span");
    let record = child_list_added(child, vec![x]);
    assert!(reg.notify(&dom, &record));
    assert_eq!(reg.take_records(id).len(), 1);
}

/// Re-observing a target clears that observer's outstanding transients
/// (§4.3.1 observe step 7.1, source→observer-id collapse).
#[test]
fn reobserve_clears_outstanding_transients() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        subtree: true,
        ..Default::default()
    };
    reg.observe(&mut dom, id, parent, init.clone());

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    assert_eq!(count_entries(&dom, mid, id, true), 1);

    // Re-observe (any target carrying a registration for `id` triggers 7.1).
    reg.observe(&mut dom, id, parent, init);
    assert_eq!(
        count_entries(&dom, mid, id, true),
        0,
        "re-observe cleared the observer's transients"
    );
}

/// Codex R3 (P2): observe() step 7.1 is **source-keyed**. When one observer
/// holds permanent registrations on two inclusive ancestors, a removed node
/// gets two transients with different sources; re-observing ONE ancestor must
/// clear only that ancestor's transient and preserve the other-sourced one.
#[test]
fn reobserve_preserves_other_source_transient() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, mid);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        subtree: true,
        ..Default::default()
    };
    // Same observer registered on BOTH inclusive ancestors of `mid`.
    reg.observe(&mut dom, id, grandparent, init.clone());
    reg.observe(&mut dom, id, parent, init.clone());

    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    // Two transients for `id` on `mid` — sourced from `parent` and `grandparent`.
    assert_eq!(count_entries(&dom, mid, id, true), 2);

    // Re-observe `parent` only → clears the `parent`-sourced transient,
    // leaves the `grandparent`-sourced one.
    reg.observe(&mut dom, id, parent, init);
    assert_eq!(
        count_entries(&dom, mid, id, true),
        1,
        "only the re-observed ancestor's transient is cleared (source-keyed §7.1)"
    );
    // A mutation in the still-detached subtree still reaches `id` via the
    // surviving grandparent-sourced transient.
    let x = elem(&mut dom, "span");
    assert!(reg.notify(&dom, &child_list_added(mid, vec![x])));
    assert_eq!(reg.take_records(id).len(), 1);
}

/// Codex R3 (P2): `clear_all_transient_observers` (the unbind scrub) drops
/// every transient across the tree while leaving permanent registrations.
#[test]
fn clear_all_transient_observers_scrubs_transients_keeps_permanent() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let _ = dom.append_child(root, mid);

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
    assert!(dom.remove_child(root, mid));
    reg.add_transient_observers(&mut dom, root, &[mid]);
    assert_eq!(count_entries(&dom, mid, id, true), 1);
    assert_eq!(count_entries(&dom, root, id, false), 1);

    clear_all_transient_observers(&mut dom);
    assert_eq!(count_entries(&dom, mid, id, true), 0, "transient scrubbed");
    assert_eq!(
        count_entries(&dom, root, id, false),
        1,
        "permanent registration untouched"
    );
}

/// Codex R4-H1 + R8-L1: when one observer holds TWO registrations on a single
/// node (a permanent + a chained transient, both subtree), the transients they
/// spawn on a removed descendant carry distinct `source` reg-ids. Re-observing
/// that node matches BOTH registrations (§4.3.1 step 7 — a transient is a
/// registered observer), so step 7.1 clears the transients sourced from each →
/// both chained child transients are cleared. (The reg-id source key is what
/// makes per-registration step-7.1 possible at all; the ancestor-entity key
/// could not distinguish the two child transients, both `source == mid`.)
#[test]
fn reobserve_matching_both_registrations_clears_both_chained_sources() {
    let mut dom = EcsDom::new();
    let grandparent = elem(&mut dom, "div");
    let parent = elem(&mut dom, "div");
    let mid = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    let _ = dom.append_child(grandparent, parent);
    let _ = dom.append_child(parent, mid);
    let _ = dom.append_child(mid, child);

    let mut reg = MutationObserverRegistry::new();
    let id = reg.register();
    let init = MutationObserverInit {
        child_list: true,
        subtree: true,
        ..Default::default()
    };
    // Same observer on grandparent AND mid.
    reg.observe(&mut dom, id, grandparent, init.clone());
    reg.observe(&mut dom, id, mid, init);

    // Remove mid from parent → mid gets a transient sourced from grandparent's
    // registration (mid also keeps its own permanent registration).
    assert!(dom.remove_child(parent, mid));
    reg.add_transient_observers(&mut dom, parent, &[mid]);
    // Remove child from the now-detached mid → child gets TWO transients: one
    // sourced from mid's permanent registration, one from mid's transient.
    assert!(dom.remove_child(mid, child));
    reg.add_transient_observers(&mut dom, mid, &[child]);
    assert_eq!(count_entries(&dom, child, id, true), 2);

    // Re-observe mid → step 7 matches BOTH mid's permanent and mid's transient
    // (both have observer == id); step 7.1 clears the child transients sourced
    // from each → both cleared.
    reg.observe(
        &mut dom,
        id,
        mid,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );
    assert_eq!(
        count_entries(&dom, child, id, true),
        0,
        "both child transients (sourced from mid's permanent + transient) cleared"
    );
    // The doubly-detached child no longer reaches the observer.
    let x = elem(&mut dom, "span");
    assert!(!reg.notify(&dom, &child_list_added(child, vec![x])));
}

/// Codex R4 (P2, H2): the pending mutation observers set delivers in spec
/// **append order** (the order each observer first received a record this
/// cycle), NOT observer-id order — observable cross-observer callback ordering.
#[test]
fn pending_observers_preserve_append_order_not_id_order() {
    let mut dom = EcsDom::new();
    let el_a = elem(&mut dom, "div");
    let el_b = elem(&mut dom, "div");

    let mut reg = MutationObserverRegistry::new();
    let id_a = reg.register(); // lower id
    let id_b = reg.register(); // higher id
    let init = MutationObserverInit {
        child_list: true,
        ..Default::default()
    };
    reg.observe(&mut dom, id_a, el_a, init.clone());
    reg.observe(&mut dom, id_b, el_b, init);

    // The higher-id observer (B, on el_b) receives a record FIRST.
    let x = elem(&mut dom, "span");
    reg.notify(&dom, &child_list_added(el_b, vec![x]));
    let y = elem(&mut dom, "span");
    reg.notify(&dom, &child_list_added(el_a, vec![y]));

    assert_eq!(
        reg.take_pending_observers(),
        vec![id_b, id_a],
        "delivery follows append order (B then A), not id order (A then B)"
    );
}
