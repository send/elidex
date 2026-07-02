use super::*;
use elidex_ecs::{Attributes, EcsDom};

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

#[test]
fn retire_collected_drops_observer_config() {
    // S5-3c registry-side leak fix: `retire_collected` drops the observer's
    // `RegisteredObserver` config (init + parsed rootMargin) for a GC-collected
    // wrapper (binding row pruned), so the registry does not grow monotonically.
    // Dom-free — a collected observer is guaranteed non-observing, so no
    // target-list scrub is needed.
    let mut reg = IntersectionObserverRegistry::new();
    let id = reg
        .register(IntersectionObserverInit {
            root_margin: "10px".to_string(),
            threshold: vec![0.5],
            ..Default::default()
        })
        .expect("valid rootMargin");
    assert_eq!(reg.observers_len(), 1);

    reg.retire_collected(id);
    assert_eq!(
        reg.observers_len(),
        0,
        "retire_collected drops the per-observer config (no residual)"
    );
    // A retired id is not reusable for observe (mirrors unregister).
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    reg.observe(&mut dom, id, el);
    assert!(
        dom.world().get::<&IntersectionObservedBy>(el).is_err(),
        "a retired id must not be reusable for observe"
    );
    // next_id stays monotonic across the retire.
    let id2 = register_default(&mut reg);
    assert_ne!(id2.raw(), id.raw());
}

// --- observing_observer_ids (the S5-3c GC-keepalive membership query) ---

fn register_default(reg: &mut IntersectionObserverRegistry) -> IntersectionObserverId {
    reg.register(IntersectionObserverInit::default())
        .expect("default init has valid rootMargin")
}

#[test]
fn observing_ids_empty_world_is_empty() {
    let dom = EcsDom::new();
    assert!(observing_observer_ids(&dom).is_empty());
}

#[test]
fn observing_ids_present_after_observe() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = IntersectionObserverRegistry::new();
    let id = register_default(&mut reg);
    reg.observe(&mut dom, id, el);

    let ids = observing_observer_ids(&dom);
    assert!(ids.contains(&id.raw()));
    assert_eq!(ids.len(), 1);
}

#[test]
fn observing_ids_absent_after_unobserve() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = IntersectionObserverRegistry::new();
    let id = register_default(&mut reg);
    reg.observe(&mut dom, id, el);
    reg.unobserve(&mut dom, id, el);
    assert!(
        observing_observer_ids(&dom).is_empty(),
        "unobserve of the sole target ⇒ non-member (collectible)"
    );
}

#[test]
fn observing_ids_absent_after_disconnect() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = IntersectionObserverRegistry::new();
    let id = register_default(&mut reg);
    reg.observe(&mut dom, id, el);
    reg.disconnect(&mut dom, id);
    assert!(observing_observer_ids(&dom).is_empty());
}

#[test]
fn observing_ids_absent_after_despawn_of_sole_target() {
    let mut dom = EcsDom::new();
    let el = elem(&mut dom, "div");
    let mut reg = IntersectionObserverRegistry::new();
    let id = register_default(&mut reg);
    reg.observe(&mut dom, id, el);
    assert!(observing_observer_ids(&dom).contains(&id.raw()));

    let _ = dom.destroy_entity(el);
    assert!(
        observing_observer_ids(&dom).is_empty(),
        "despawn of the sole observed entity drops membership (despawn-safe)"
    );
}

#[test]
fn observing_ids_two_observers_distinct_targets_both_present() {
    let mut dom = EcsDom::new();
    let a = elem(&mut dom, "div");
    let b = elem(&mut dom, "section");
    let mut reg = IntersectionObserverRegistry::new();
    let id_a = register_default(&mut reg);
    let id_b = register_default(&mut reg);
    reg.observe(&mut dom, id_a, a);
    reg.observe(&mut dom, id_b, b);

    let ids = observing_observer_ids(&dom);
    assert!(ids.contains(&id_a.raw()));
    assert!(ids.contains(&id_b.raw()));
    assert_eq!(ids.len(), 2);
}
