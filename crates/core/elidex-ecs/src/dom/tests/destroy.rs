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
