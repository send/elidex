use super::*;
use elidex_ecs::Attributes;

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

/// Setup a shadow DOM: root > host > shadow root > shadow child.
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values // Test setup calls dom.append_child() etc. without checking return values
fn setup_shadow_dom(mode: elidex_ecs::ShadowRootMode) -> (EcsDom, Entity, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);
    let sr = dom.attach_shadow(host, mode).unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);
    (dom, root, host, sr, shadow_child)
}

/// Add an event listener to an entity, creating or extending its `EventListeners` component.
fn add_listener(dom: &mut EcsDom, entity: Entity, event_type: &str, capture: bool) -> ListenerId {
    let mut listeners = dom
        .world()
        .get::<&EventListeners>(entity)
        .ok()
        .map(|l| (*l).clone())
        .unwrap_or_default();
    let id = listeners.add(event_type, capture);
    dom.world_mut().insert_one(entity, listeners).unwrap();
    id
}

#[test]
fn propagation_path_single_node() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    let path = build_propagation_path(&dom, e, true);
    assert_eq!(path, vec![e]);
}

#[test]
fn propagation_path_deep() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child = elem(&mut dom, "p");
    let grandchild = elem(&mut dom, "span");
    let _ = dom.append_child(root, child);
    let _ = dom.append_child(child, grandchild);

    let path = build_propagation_path(&dom, grandchild, true);
    assert_eq!(path, vec![root, child, grandchild]);
}

#[test]
fn propagation_path_detached() {
    let mut dom = EcsDom::new();
    let e = elem(&mut dom, "div");
    // Detached node — should just return itself.
    let path = build_propagation_path(&dom, e, true);
    assert_eq!(path, vec![e]);
}

#[test]
fn dispatch_capture_phase() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let target = elem(&mut dom, "span");
    let _ = dom.append_child(root, target);

    let lid = add_listener(&mut dom, root, "click", true);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut invoked = Vec::new();
    dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
        invoked.push((id, entity));
    });

    assert_eq!(invoked.len(), 1);
    assert_eq!(invoked[0], (lid, root));
}

#[test]
fn dispatch_bubble_phase() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let target = elem(&mut dom, "span");
    let _ = dom.append_child(root, target);

    let lid = add_listener(&mut dom, root, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut invoked = Vec::new();
    dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
        invoked.push((id, entity));
    });

    assert_eq!(invoked.len(), 1);
    assert_eq!(invoked[0], (lid, root));
}

#[test]
fn dispatch_no_bubble() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let target = elem(&mut dom, "span");
    let _ = dom.append_child(root, target);

    add_listener(&mut dom, root, "focus", false);

    let mut event = DispatchEvent::new("focus", target);
    event.bubbles = false;

    let mut invoked = Vec::new();
    dispatch_event(&dom, &mut event, &mut |id, entity, _ev| {
        invoked.push((id, entity));
    });

    // Bubble listener should NOT fire for non-bubbling event.
    assert!(invoked.is_empty());
}

#[test]
fn dispatch_stop_propagation() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let mid = elem(&mut dom, "p");
    let target = elem(&mut dom, "span");
    let _ = dom.append_child(root, mid);
    let _ = dom.append_child(mid, target);

    add_listener(&mut dom, root, "click", true);
    add_listener(&mut dom, mid, "click", true);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut count = 0;
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        count += 1;
        ev.flags.propagation_stopped = true;
    });

    assert_eq!(count, 1);
}

#[test]
fn dispatch_stop_immediate_propagation() {
    let mut dom = EcsDom::new();
    let target = elem(&mut dom, "span");

    // Two listeners on target.
    add_listener(&mut dom, target, "click", false);
    add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut count = 0;
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        count += 1;
        ev.flags.immediate_propagation_stopped = true;
    });

    // Only the first listener should fire.
    assert_eq!(count, 1);
}

#[test]
fn dispatch_prevent_default() {
    let mut dom = EcsDom::new();
    let target = elem(&mut dom, "span");

    add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let prevented = dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        ev.flags.default_prevented = true;
    });

    assert!(prevented);
}

#[test]
fn dispatch_at_target_fires_both_capture_and_bubble() {
    let mut dom = EcsDom::new();
    let target = elem(&mut dom, "span");

    let cap_id = add_listener(&mut dom, target, "click", true);
    let bub_id = add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut invoked = Vec::new();
    dispatch_event(&dom, &mut event, &mut |id, _entity, _ev| {
        invoked.push(id);
    });

    // Both should fire at target.
    assert_eq!(invoked, vec![cap_id, bub_id]);
}

#[test]
fn dispatch_full_lifecycle() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let target = elem(&mut dom, "span");
    let _ = dom.append_child(root, target);

    let cap_id = add_listener(&mut dom, root, "click", true);
    let bub_id = add_listener(&mut dom, root, "click", false);
    let tgt_id = add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut phases = Vec::new();
    dispatch_event(&dom, &mut event, &mut |id, entity, ev| {
        phases.push((id, entity, ev.phase));
    });

    // Capture on root, at-target on target, bubble on root.
    assert_eq!(phases.len(), 3);
    assert_eq!(phases[0], (cap_id, root, EventPhase::Capturing));
    assert_eq!(phases[1], (tgt_id, target, EventPhase::AtTarget));
    assert_eq!(phases[2], (bub_id, root, EventPhase::Bubbling));
}

// --- Shadow DOM event dispatch tests ---

#[test]
fn shadow_retarget_to_host() {
    let (mut dom, root, host, _sr, shadow_child) =
        setup_shadow_dom(elidex_ecs::ShadowRootMode::Open);

    add_listener(&mut dom, root, "click", false);

    let mut event = DispatchEvent::new_composed("click", shadow_child);
    let mut seen_targets = Vec::new();
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        seen_targets.push(ev.target);
    });

    // Root listener should see the host as target (retargeted).
    assert!(!seen_targets.is_empty());
    assert_eq!(seen_targets[0], host);
}

#[test]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
fn slotted_element_not_retargeted() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);

    let light = elem(&mut dom, "span");
    dom.append_child(host, light);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let slot = elem(&mut dom, "slot");
    dom.append_child(sr, slot);

    // Assign light to slot.
    dom.world_mut()
        .insert_one(
            slot,
            elidex_ecs::SlotAssignment {
                assigned_nodes: vec![light],
            },
        )
        .unwrap();
    dom.world_mut()
        .insert_one(light, elidex_ecs::SlottedMarker)
        .unwrap();

    add_listener(&mut dom, root, "click", false);

    let mut event = DispatchEvent::new_composed("click", light);
    let mut seen_targets = Vec::new();
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        seen_targets.push(ev.target);
    });

    // Slotted elements are not retargeted — target should be light.
    assert!(!seen_targets.is_empty());
    assert_eq!(seen_targets[0], light);
}

#[test]
fn non_composed_stops_at_shadow_boundary() {
    let (mut dom, root, _host, _sr, shadow_child) =
        setup_shadow_dom(elidex_ecs::ShadowRootMode::Open);

    add_listener(&mut dom, root, "click", false);

    let mut event = DispatchEvent::new("click", shadow_child);
    // composed defaults to false for custom events.

    let mut count = 0;
    dispatch_event(&dom, &mut event, &mut |_id, _entity, _ev| {
        count += 1;
    });

    // Non-composed event should not reach root listener.
    assert_eq!(count, 0);
}

#[test]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
fn nested_shadow_dom_retarget() {
    // Nested shadow: root > host1 > [shadow1] > host2 > [shadow2] > target
    // Listener on root should see host1 as target (retarget through both boundaries).
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let host1 = elem(&mut dom, "div");
    dom.append_child(root, host1);

    let sr1 = dom
        .attach_shadow(host1, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let host2 = elem(&mut dom, "div");
    dom.append_child(sr1, host2);

    let sr2 = dom
        .attach_shadow(host2, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let deep_target = elem(&mut dom, "span");
    dom.append_child(sr2, deep_target);

    add_listener(&mut dom, root, "click", false);
    add_listener(&mut dom, host2, "click", false);

    let mut event = DispatchEvent::new_composed("click", deep_target);
    let mut seen: Vec<(Entity, Entity)> = Vec::new(); // (listener_entity, seen_target)
    dispatch_event(&dom, &mut event, &mut |_id, entity, ev| {
        seen.push((entity, ev.target));
    });

    // host2 listener (inside shadow1) sees host2 as target (retargeted from shadow2).
    let host2_entry = seen.iter().find(|(e, _)| *e == host2);
    assert!(host2_entry.is_some());
    assert_eq!(host2_entry.unwrap().1, host2);

    // root listener (outside all shadows) sees host1 as target (retargeted through both).
    let root_entry = seen.iter().find(|(e, _)| *e == root);
    assert!(root_entry.is_some());
    assert_eq!(root_entry.unwrap().1, host1);
}

#[test]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
fn composed_path_populated() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let child = elem(&mut dom, "p");
    let target = elem(&mut dom, "span");
    dom.append_child(root, child);
    dom.append_child(child, target);

    add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    let mut seen_path = Vec::new();
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        seen_path = ev.composed_path.clone();
    });

    assert_eq!(seen_path, vec![root, child, target]);
}

#[test]
fn composed_path_non_composed_stops_at_shadow() {
    let (mut dom, root, host, sr, shadow_child) =
        setup_shadow_dom(elidex_ecs::ShadowRootMode::Open);

    add_listener(&mut dom, shadow_child, "custom", false);

    let mut event = DispatchEvent::new("custom", shadow_child);
    event.composed = false;

    let mut seen_path = Vec::new();
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        seen_path = ev.composed_path.clone();
    });

    // Non-composed path should stop at shadow root.
    assert!(seen_path.contains(&shadow_child));
    assert!(seen_path.contains(&sr));
    assert!(!seen_path.contains(&host));
    assert!(!seen_path.contains(&root));
}

#[test]
fn composed_event_crosses_shadow_boundary() {
    let (mut dom, root, _host, _sr, shadow_child) =
        setup_shadow_dom(elidex_ecs::ShadowRootMode::Open);

    add_listener(&mut dom, root, "click", false);

    let mut event = DispatchEvent::new_composed("click", shadow_child);
    // composed defaults to true.

    let mut count = 0;
    dispatch_event(&dom, &mut event, &mut |_id, _entity, _ev| {
        count += 1;
    });

    // Composed event should reach root listener exactly once.
    assert_eq!(count, 1);
}

#[test]
fn dispatch_flag_set_during_dispatch() {
    let mut dom = EcsDom::new();
    let target = elem(&mut dom, "div");
    add_listener(&mut dom, target, "click", false);

    let mut event = DispatchEvent::new_composed("click", target);
    assert!(!event.dispatch_flag, "flag should be false before dispatch");

    let mut flag_during = false;
    dispatch_event(&dom, &mut event, &mut |_id, _entity, ev| {
        flag_during = ev.dispatch_flag;
    });

    assert!(flag_during, "flag should be true during dispatch");
    assert!(!event.dispatch_flag, "flag should be false after dispatch");
}

#[test]
fn composed_path_for_js_empty_outside_dispatch() {
    let dom = EcsDom::new();
    let mut event = DispatchEvent::new_composed("click", Entity::DANGLING);
    event.composed_path = vec![Entity::DANGLING]; // simulate populated path
    event.dispatch_flag = false;

    let result = composed_path_for_js(&event, &dom);
    assert!(
        result.is_empty(),
        "should be empty when dispatch_flag is false"
    );
}

#[test]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
fn composed_path_for_js_filters_closed_shadow() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let host = elem(&mut dom, "div");
    dom.append_child(root, host);

    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
        .unwrap();
    let shadow_child = elem(&mut dom, "span");
    dom.append_child(sr, shadow_child);

    // Simulate dispatch: path includes shadow_child, sr, host, root.
    let mut event = DispatchEvent::new_composed("click", shadow_child);
    event.composed_path = vec![root, host, sr, shadow_child];
    event.dispatch_flag = true;
    // current_target is root (outside the closed shadow).
    event.current_target = Some(root);

    let filtered = composed_path_for_js(&event, &dom);
    // shadow_child and sr are inside the closed shadow root that doesn't
    // contain root, so they should be excluded.
    assert!(!filtered.contains(&shadow_child));
    assert!(!filtered.contains(&sr));
    assert!(filtered.contains(&root));
    assert!(filtered.contains(&host));
}

#[test]
fn custom_event_defaults_composed_false() {
    let target = elem(&mut EcsDom::new(), "div");
    let event = DispatchEvent::new("test", target);
    assert!(
        !event.composed,
        "custom events should default to composed=false"
    );

    let composed = DispatchEvent::new_composed("click", target);
    assert!(composed.composed, "UA events should be composed=true");
}
