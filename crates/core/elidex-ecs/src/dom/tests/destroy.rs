use super::*;

#[test]
fn destroy_entity_removes_from_world() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.destroy_entity(child);

    assert!(dom.children(parent).is_empty());
    assert!(!dom.contains(child));
}

#[test]
fn destroy_detached_entity() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    dom.destroy_entity(e);
    assert!(dom.query_by_tag("div").is_empty());
}

#[test]
fn append_destroyed_parent_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.destroy_entity(parent);
    assert!(!dom.append_child(parent, child));
}

#[test]
fn append_destroyed_child_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.destroy_entity(child);
    assert!(!dom.append_child(parent, child));
}

#[test]
fn remove_destroyed_child_returns_false() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "span");

    dom.append_child(parent, child);
    dom.destroy_entity(child);
    assert!(!dom.remove_child(parent, child));
}

#[test]
fn destroy_entity_returns_false_for_already_destroyed() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    assert!(dom.destroy_entity(e));
    assert!(!dom.destroy_entity(e));
}

#[test]
fn destroy_entity_orphans_children() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let a = elem(&mut dom, "span");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "span");

    dom.append_child(parent, a);
    dom.append_child(parent, b);
    dom.append_child(parent, c);

    dom.destroy_entity(parent);

    assert_eq!(dom.get_parent(a), None);
    assert_eq!(dom.get_parent(b), None);
    assert_eq!(dom.get_parent(c), None);
    assert_eq!(dom.get_next_sibling(a), None);
    assert_eq!(dom.get_prev_sibling(b), None);
    assert_eq!(dom.get_next_sibling(b), None);
    assert_eq!(dom.get_prev_sibling(c), None);
}

// --- Destroy + Shadow DOM interaction ---

#[test]
fn destroy_shadow_host_orphans_shadow_root() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    dom.destroy_entity(host);

    // Host is gone.
    assert!(!dom.contains(host));
    // Shadow root is orphaned but still exists (despawn only destroys the entity itself).
    assert!(dom.contains(sr));
    // Shadow child is detached from shadow root (orphaned by destroy_entity).
    assert!(dom.get_parent(sr).is_none());
}

#[test]
fn destroy_shadow_root_does_not_crash() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    // Destroying the shadow root: ShadowHost is cleaned up on the host.
    dom.destroy_entity(sr);

    assert!(!dom.contains(sr));
    assert!(dom.contains(host));
    // ShadowHost component should be removed (bidirectional cleanup).
    assert!(dom.world().get::<&crate::ShadowHost>(host).is_err());
    assert_eq!(dom.get_shadow_root(host), None);
}

// --- despawn_subtree ---

#[test]
fn despawn_subtree_destroys_whole_light_tree() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let a = elem(&mut dom, "section");
    let b = elem(&mut dom, "span");
    let c = elem(&mut dom, "em");
    dom.append_child(root, a);
    dom.append_child(a, b);
    dom.append_child(root, c);

    assert!(dom.despawn_subtree(root));

    for e in [root, a, b, c] {
        assert!(!dom.contains(e), "every node in the subtree is destroyed");
    }
    assert!(
        !dom.despawn_subtree(root),
        "returns false for a missing root"
    );
}

#[test]
fn despawn_subtree_destroys_shadow_root_unlike_destroy_entity() {
    // Contrast with `destroy_shadow_host_orphans_shadow_root`: a single
    // `destroy_entity(host)` leaves the shadow root alive (orphaned), but
    // `despawn_subtree` must tear the shadow root (and its shadow-tree
    // contents) out too, leaving no live remnant.
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    assert!(dom.despawn_subtree(root));

    assert!(!dom.contains(root));
    assert!(!dom.contains(host));
    assert!(
        !dom.contains(sr),
        "the shadow root entity must be despawned, not leaked"
    );
    assert!(!dom.contains(shadow_child));
}

#[test]
fn despawn_subtree_is_event_free_even_on_a_connected_shadow_host() {
    // `despawn_subtree` is a raw structural teardown, not a DOM removal: it
    // suppresses dispatch for the whole walk. So tearing a *connected* subtree
    // — even one with a shadow host whose shadow root is destroyed ahead of the
    // host — fires no `MutationEvent`, hence no mis-ordered Remove that could
    // miss the shadow tree's `disconnectedCallback`s. Removal-with-events is
    // the DOM remove algorithm's responsibility, not this primitive's.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Recorder {
        count: Arc<AtomicUsize>,
    }
    impl crate::dom::MutationDispatcher for Recorder {
        fn dispatch(&mut self, _event: &crate::dom::MutationEvent<'_>, _dom: &mut EcsDom) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let mut dom = EcsDom::new();
    // A `Document` ancestor makes the subtree connected (`is_connected` true),
    // so without suppression each `destroy_entity` would fire a Remove.
    let doc = dom.create_document_node();
    let root = elem(&mut dom, "div");
    dom.append_child(doc, root);
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    let count = Arc::new(AtomicUsize::new(0));
    dom.set_mutation_dispatcher(Box::new(Recorder {
        count: count.clone(),
    }));

    assert!(dom.despawn_subtree(root));

    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "raw structural teardown fires no mutation events"
    );
    assert!(
        dom.take_mutation_dispatcher().is_some(),
        "the dispatcher is restored after the teardown, not dropped"
    );
}

#[test]
fn despawn_subtree_tears_down_past_the_traversal_depth_cap() {
    // The shadow-inclusive descendant walker caps at `MAX_ANCESTOR_DEPTH`; a
    // teardown that inherited that cap would leak everything below it, breaking
    // the strict parser's "dom is pristine on failure" contract for a
    // maliciously deep fragment. `despawn_subtree` must reach every node.
    use crate::dom::MAX_ANCESTOR_DEPTH;

    let mut dom = EcsDom::new();
    let baseline = dom.world().len();
    // Build bottom-up — wrap the current top in a fresh parent each step — so
    // every `append_child` walks up from a parentless node (O(1)); a top-down
    // chain would re-walk the whole ancestor list per append (O(n²)).
    let deepest = elem(&mut dom, "div");
    let mut root = deepest;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        let parent = elem(&mut dom, "div");
        dom.append_child(parent, root);
        root = parent;
    }
    assert!(
        dom.contains(deepest),
        "the over-cap leaf exists pre-teardown"
    );

    assert!(dom.despawn_subtree(root));

    assert!(
        !dom.contains(deepest),
        "the node below the traversal depth cap is torn down, not leaked"
    );
    assert_eq!(
        dom.world().len(),
        baseline,
        "no entity in the over-deep subtree survives"
    );
}

#[test]
fn despawn_subtree_tears_down_past_the_breadth_cap() {
    // `children` / `children_iter` cap the *sibling* walk at
    // `MAX_ANCESTOR_DEPTH`; a teardown inheriting that cap would leak children
    // past it. `despawn_subtree` (via `child_list_uncapped`) must reach every
    // sibling, however wide.
    use crate::dom::MAX_ANCESTOR_DEPTH;

    let mut dom = EcsDom::new();
    let baseline = dom.world().len();
    let root = elem(&mut dom, "div");
    let mut last = root;
    for _ in 0..(MAX_ANCESTOR_DEPTH + 5) {
        last = elem(&mut dom, "span");
        dom.append_child(root, last); // O(1): links at the `last_child` pointer
    }
    assert_eq!(
        dom.child_list_uncapped(root).len(),
        MAX_ANCESTOR_DEPTH + 5,
        "the uncapped enumerator returns every child"
    );
    assert_eq!(
        dom.children(root).len(),
        MAX_ANCESTOR_DEPTH,
        "the capped enumerator truncates (contrast)"
    );

    assert!(dom.despawn_subtree(root));

    assert!(
        !dom.contains(last),
        "the child past the breadth cap is torn down, not leaked"
    );
    assert_eq!(
        dom.world().len(),
        baseline,
        "no wide sibling survives teardown"
    );
}

#[test]
fn despawn_subtree_bumps_only_the_external_parent_version() {
    // Per-node version propagation (`rev_version` walks all ancestors) is
    // suppressed during teardown so a deep subtree is not O(n²); the one
    // surviving live-tree effect is a single version bump on the root's
    // *external* parent, so live collections rooted at/above it invalidate.
    let mut dom = EcsDom::new();
    let outer = elem(&mut dom, "div");
    let root = elem(&mut dom, "section");
    dom.append_child(outer, root);
    let child = elem(&mut dom, "span");
    dom.append_child(root, child);

    let before = dom.inclusive_descendants_version(outer);
    assert!(dom.despawn_subtree(root));

    assert!(
        !dom.contains(root) && !dom.contains(child),
        "the subtree is torn down"
    );
    assert!(
        dom.inclusive_descendants_version(outer) > before,
        "the external parent's version is bumped so live collections invalidate"
    );
    assert!(
        dom.children(outer).is_empty(),
        "root is removed from the external parent's child list"
    );
}

#[test]
fn destroy_slot_clears_assignment() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Manually attach SlotAssignment.
    let _ = dom.world_mut().insert_one(
        slot,
        SlotAssignment {
            assigned_nodes: vec![light],
        },
    );

    dom.destroy_entity(slot);
    assert!(!dom.contains(slot));
    // Light child is still alive — just no longer assigned.
    assert!(dom.contains(light));
}

#[test]
fn destroy_assigned_node_leaves_no_dangling() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    let _ = dom.world_mut().insert_one(
        slot,
        SlotAssignment {
            assigned_nodes: vec![light],
        },
    );
    let _ = dom.world_mut().insert_one(light, crate::SlottedMarker);

    dom.destroy_entity(light);
    assert!(!dom.contains(light));
    // SlotAssignment still references the destroyed entity (stale ref).
    // This is documented behavior — redistribute should be called after mutations.
    let assign = dom.world().get::<&SlotAssignment>(slot).unwrap();
    assert_eq!(assign.assigned_nodes.len(), 1);
    assert!(!dom.contains(assign.assigned_nodes[0]));
}

#[test]
fn destroy_shadow_root_cleans_up_host() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert!(dom.world().get::<&ShadowHost>(host).is_ok());
    dom.destroy_entity(sr);
    // ShadowHost component should be removed from host.
    assert!(dom.world().get::<&ShadowHost>(host).is_err());
}

#[test]
fn destroy_host_cleans_up_shadow_root() {
    use crate::components::ShadowRoot;

    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    assert!(dom.world().get::<&ShadowRoot>(sr).is_ok());
    dom.destroy_entity(host);
    // ShadowRoot component should be removed from shadow root entity.
    assert!(dom.world().get::<&ShadowRoot>(sr).is_err());
}

#[test]
fn composed_children_fallback_after_shadow_root_destroy() {
    let mut dom = EcsDom::new();
    let host = elem(&mut dom, "div");
    let light_child = elem(&mut dom, "span");
    dom.append_child(host, light_child);
    let sr = dom.attach_shadow(host, ShadowRootMode::Open).unwrap();
    // Before destroy: composed_children returns shadow tree (empty here).
    assert!(dom.composed_children(host).is_empty());
    dom.destroy_entity(sr);
    // After destroy: ShadowHost cleaned up, falls through to normal children.
    assert_eq!(dom.composed_children(host), vec![light_child]);
}
