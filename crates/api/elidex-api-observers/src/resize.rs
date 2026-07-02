//! `ResizeObserver` API (Resize Observer §3).
//!
//! Observes changes to the content box / border box size of elements.
//!
//! ECS-native model: each observation lives as a `ResizeObservedBy` component
//! on the observed target entity (carrying the per-observer last-reported size
//! used for change detection), so a despawned entity drops its observations
//! automatically. The registry holds only the monotonic id counter.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{Rect, Size};

/// Geometry source for `gather_observations`: returns the current
/// `(content_rect, border_box_size)` for an entity, where `content_rect` is the
/// content box rect in the element's own coordinate space (origin = padding
/// offsets per the Resize Observer `contentRect` legacy field) and its `.size`
/// is the content box size. Takes `&EcsDom` so it can read layout without
/// aliasing the `&mut EcsDom` write-back borrow. `None` = box-less / unrendered.
type SizeProvider<'a> = dyn Fn(&EcsDom, Entity) -> Option<(Rect, Size)> + 'a;

/// A unique identifier for a resize observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResizeObserverId(u64);

impl ResizeObserverId {
    /// Create an ID from a raw u64 value.
    #[must_use]
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw u64 value.
    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Which box model to observe.
#[derive(Debug, Clone, Copy, Default)]
pub enum ResizeObserverBoxOptions {
    /// Observe the content box (default).
    #[default]
    ContentBox,
    /// Observe the border box.
    BorderBox,
    /// Observe the device-pixel content box.
    DevicePixelContentBox,
}

impl ResizeObserverBoxOptions {
    /// Map a WebIDL enum string (Resize Observer §3) to the variant, or
    /// `None` for an unrecognised value. The spec strings live with the
    /// enum (crate-side) per the Layering mandate — host marshalling
    /// translates a TypeError on `None`, but the string→variant choice
    /// is API surface, not engine machinery.
    #[must_use]
    pub fn from_webidl(s: &str) -> Option<Self> {
        match s {
            "content-box" => Some(Self::ContentBox),
            "border-box" => Some(Self::BorderBox),
            "device-pixel-content-box" => Some(Self::DevicePixelContentBox),
            _ => None,
        }
    }
}

/// Options for `ResizeObserver.observe()`.
#[derive(Debug, Clone, Default)]
pub struct ResizeObserverOptions {
    /// Which box to observe.
    pub box_model: ResizeObserverBoxOptions,
}

/// A resize observation entry delivered to the callback (Resize Observer §2.3).
#[derive(Debug, Clone)]
pub struct ResizeObserverEntry {
    /// The observed element.
    pub target: Entity,
    /// `contentRect`: the content box rect (origin = padding offsets).
    /// The spec's `contentBoxSize` FrozenArray is derived from
    /// `content_rect.size` at host-marshal time — keeping it as a
    /// derivation removes the "two fields, must stay in sync" foot-gun.
    pub content_rect: Rect,
    /// Border box size.
    pub border_box_size: Size,
}

/// A single observation on a node (one per observer watching this entity).
#[derive(Debug, Clone)]
struct ResizeObservation {
    observer: ResizeObserverId,
    #[allow(dead_code)] // retained for box-model fidelity; gather currently keys on content size
    options: ResizeObserverOptions,
    /// Last reported content box size (for change detection).
    last_size: Option<Size>,
}

/// Per-node component listing the resize observers watching this entity.
/// Dropped automatically when the entity is despawned.
#[derive(Debug, Default)]
struct ResizeObservedBy(Vec<ResizeObservation>);

/// Registry for active resize observers.
///
/// Tracks live observer ids so `observe` can reject an unregistered id and
/// `unregister` truly retires it — mirroring the mutation registry's `records`
/// keys and the intersection registry's `init` keys (ResizeObserver carries no
/// other per-observer state, hence a bare id set).
#[derive(Debug, Default)]
pub struct ResizeObserverRegistry {
    next_id: u64,
    registered: std::collections::HashSet<ResizeObserverId>,
}

impl ResizeObserverRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new resize observer, returning its ID.
    pub fn register(&mut self) -> ResizeObserverId {
        let id = ResizeObserverId(self.next_id);
        self.next_id += 1;
        self.registered.insert(id);
        id
    }

    /// Start observing a target (Resize Observer §2.1 `observe(target, options)`).
    /// Re-observing the same target is a no-op (matches the existing registration).
    pub fn observe(
        &mut self,
        dom: &mut EcsDom,
        id: ResizeObserverId,
        target: Entity,
        options: ResizeObserverOptions,
    ) {
        // Ignore observe for an unregistered (or unregister()'d) id so a stale
        // `ResizeObservedBy` can't accumulate. Restores the pre-refactor
        // registry-lookup guard.
        if !self.registered.contains(&id) {
            return;
        }
        if let Ok(mut comp) = dom.world_mut().get::<&mut ResizeObservedBy>(target) {
            if !comp.0.iter().any(|o| o.observer == id) {
                comp.0.push(ResizeObservation {
                    observer: id,
                    options,
                    last_size: None,
                });
            }
            return;
        }
        let _ = dom.world_mut().insert_one(
            target,
            ResizeObservedBy(vec![ResizeObservation {
                observer: id,
                options,
                last_size: None,
            }]),
        );
    }

    /// Stop observing a specific target (Resize Observer §2.1 `unobserve(target)`).
    pub fn unobserve(&mut self, dom: &mut EcsDom, id: ResizeObserverId, target: Entity) {
        let mut now_empty = false;
        if let Ok(mut comp) = dom.world_mut().get::<&mut ResizeObservedBy>(target) {
            comp.0.retain(|o| o.observer != id);
            now_empty = comp.0.is_empty();
        }
        if now_empty {
            let _ = dom.world_mut().remove_one::<ResizeObservedBy>(target);
        }
    }

    /// Stop observing all targets for this observer (Resize Observer §2.1 `disconnect()`).
    pub fn disconnect(&mut self, dom: &mut EcsDom, id: ResizeObserverId) {
        let mut emptied: Vec<Entity> = Vec::new();
        for (entity, comp) in &mut dom.world_mut().query::<(Entity, &mut ResizeObservedBy)>() {
            comp.0.retain(|o| o.observer != id);
            if comp.0.is_empty() {
                emptied.push(entity);
            }
        }
        for entity in emptied {
            let _ = dom.world_mut().remove_one::<ResizeObservedBy>(entity);
        }
    }

    /// Remove the observer entirely: drop its observations and retire the id so
    /// a later `observe` with the same id is a no-op.
    pub fn unregister(&mut self, dom: &mut EcsDom, id: ResizeObserverId) {
        self.disconnect(dom, id);
        self.registered.remove(&id);
    }

    /// Drop the registry-internal bookkeeping for an observer whose JS wrapper
    /// was GC-collected (binding row swept). Dom-free: a GC-collected observer is
    /// guaranteed non-observing (its observation components are already gone), so
    /// no target-list scrub is needed — mirrors the mutation registry's
    /// `clear_pending_records` rationale. Called only from the `gc/collect.rs`
    /// binding-row sweep. Retires only the `registered` id set (RO holds no other
    /// per-observer state; observations live on the `ResizeObservedBy`
    /// components, already gone); `next_id` stays monotonic.
    pub fn retire_collected(&mut self, id: ResizeObserverId) {
        self.registered.remove(&id);
    }

    /// Number of retained observer ids. VM-integration + test oracle for the
    /// GC-sweep [`Self::retire_collected`] retirement — `registered` has no public
    /// reader. `#[doc(hidden)]` (not part of the supported API surface), mirroring
    /// the mutation registry's `clear_pending_records` VM-integration marking.
    #[doc(hidden)]
    #[must_use]
    pub fn registered_len(&self) -> usize {
        self.registered.len()
    }

    /// Gather resize observations by comparing current sizes against last known
    /// sizes (Resize Observer §3.4.1 "Gather active observations at depth" /
    /// §3.4.5 "Broadcast active observations").
    ///
    /// `size_fn` provides the current `(content_box_size, border_box_size)` for
    /// an entity, taking `&EcsDom` so it can read layout without aliasing the
    /// `&mut EcsDom` write-back borrow. Returns `(observer_id, entries)` pairs
    /// for observers with at least one changed target.
    pub fn gather_observations(
        &mut self,
        dom: &mut EcsDom,
        size_fn: &SizeProvider<'_>,
    ) -> Vec<(ResizeObserverId, Vec<ResizeObserverEntry>)> {
        // BTreeMap keyed by the monotonic observer id → iteration (and the
        // returned Vec) is in registration order by construction.
        let mut grouped: std::collections::BTreeMap<ResizeObserverId, Vec<ResizeObserverEntry>> =
            std::collections::BTreeMap::new();
        let mut writebacks: Vec<(Entity, ResizeObserverId, Size)> = Vec::new();

        // Phase A: read observations + current sizes (shared borrows only —
        // the `&ResizeObservedBy` query and `size_fn`'s layout reads touch
        // disjoint components, so they coexist).
        //
        // A box-less target (`size_fn` → None: display:none / pre-layout) is
        // NOT skipped: per Resize Observer §2.1 (observe) the first broadcast
        // must deliver an initial 0×0 entry, so the missing box is treated as a
        // zero content rect and runs the same change-detection logic.
        for (entity, comp) in &mut dom.world().query::<(Entity, &ResizeObservedBy)>() {
            let (content_rect, border_size) =
                size_fn(&*dom, entity).unwrap_or((Rect::default(), Size::ZERO));
            let content_size = content_rect.size;
            for obs in &comp.0 {
                let changed = obs.last_size.is_none_or(|last| {
                    (last.width - content_size.width).abs() > f32::EPSILON
                        || (last.height - content_size.height).abs() > f32::EPSILON
                });
                if changed {
                    writebacks.push((entity, obs.observer, content_size));
                    grouped
                        .entry(obs.observer)
                        .or_default()
                        .push(ResizeObserverEntry {
                            target: entity,
                            content_rect,
                            border_box_size: border_size,
                        });
                }
            }
        }

        // Phase B: write back last sizes (exclusive borrow).
        for (entity, observer, size) in writebacks {
            if let Ok(mut comp) = dom.world_mut().get::<&mut ResizeObservedBy>(entity) {
                if let Some(obs) = comp.0.iter_mut().find(|o| o.observer == observer) {
                    obs.last_size = Some(size);
                }
            }
        }

        grouped.into_iter().collect()
    }
}

/// The raw ids of the resize observers that currently have ≥1 active
/// observation — Resize Observer §3.5 "Lifetime" ("the observer is not
/// observing any targets" ⇒ collectible) mechanised by the `[[observationTargets]]`
/// slot (§3.2.2), here the per-entity `ResizeObservedBy` components. Derived in
/// one hecs archetype query, flat-mapping every observation's `observer` id into
/// the set.
///
/// This is the GC-keepalive predicate `elidex-js` marshals (S5-3c): a
/// `ResizeObserver` wrapper stays rooted iff its id is in this set. **Despawn-
/// safe by construction** — a despawned entity's `ResizeObservedBy` is gone with
/// the entity, so its (observer, target) pair is never scanned.
///
/// Unlike `MutationObserver`, ResizeObserver has NO second "pending undelivered
/// entries" keepalive clause: its delivery is **synchronous** within
/// `deliver_resize_observations` (§3.4.5 "Broadcast active resize observations" runs
/// inside "update the rendering" — `gather_observations` gathers the active
/// entries and delivers them in the SAME call), and this registry keeps NO
/// persistent per-observer entry queue that survives a GC-able window between
/// "entry queued" and "delivered". Active entries are recomputed each frame from
/// the live `ResizeObservedBy` components (size-change diff), so there is no
/// cross-checkpoint pending state to root. Active-observation membership is the
/// sole keepalive signal.
#[must_use]
pub fn observing_observer_ids(dom: &EcsDom) -> std::collections::HashSet<u64> {
    let mut ids = std::collections::HashSet::new();
    for (_entity, comp) in &mut dom.world().query::<(Entity, &ResizeObservedBy)>() {
        for obs in &comp.0 {
            ids.insert(obs.observer.raw());
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn gather_detects_initial_size() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());

        let observations = reg.gather_observations(&mut dom, &|_, e| {
            if e == el {
                Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
            } else {
                None
            }
        });

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].1.len(), 1);
        assert_eq!(
            observations[0].1[0].content_rect.size,
            Size::new(100.0, 50.0)
        );
        assert_eq!(observations[0].1[0].border_box_size, Size::new(110.0, 60.0));
    }

    #[test]
    fn box_less_target_delivers_initial_zero() {
        // display:none / pre-layout target (size_fn → None) must still get the
        // mandated initial 0×0 broadcast, exactly once.
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());

        let first = reg.gather_observations(&mut dom, &|_, _| None);
        assert_eq!(first.len(), 1, "box-less target delivers once");
        assert_eq!(first[0].1[0].content_rect.size, Size::ZERO);
        assert_eq!(first[0].1[0].content_rect, Rect::default());

        // Still box-less → no re-delivery (last_size now 0×0).
        let second = reg.gather_observations(&mut dom, &|_, _| None);
        assert!(second.is_empty(), "no re-delivery while still box-less");
    }

    #[test]
    fn gather_no_change_no_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());

        // First gather — initial observation.
        reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });

        // Same sizes — no observations.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });
        assert!(observations.is_empty());
    }

    #[test]
    fn gather_detects_size_change() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());

        reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });

        // Width changed.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 200.0, 50.0), Size::new(210.0, 60.0)))
        });
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].1[0].content_rect.size,
            Size::new(200.0, 50.0)
        );
    }

    #[test]
    fn despawn_clears_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });
        let _ = dom.destroy_entity(el);

        // After despawn, the observation component is gone — no observations.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });
        assert!(observations.is_empty());
    }

    #[test]
    fn unobserve_stops_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        reg.unobserve(&mut dom, id, el);

        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });
        assert!(observations.is_empty());
    }

    #[test]
    fn gather_observations_is_id_sorted() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        for _ in 0..4 {
            let id = reg.register();
            reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        }

        let result = reg.gather_observations(&mut dom, &|_, _| {
            Some((Rect::new(0.0, 0.0, 100.0, 50.0), Size::new(110.0, 60.0)))
        });
        let got: Vec<u64> = result.iter().map(|(id, _)| id.raw()).collect();
        let mut sorted = got.clone();
        sorted.sort_unstable();
        assert_eq!(got.len(), 4);
        assert_eq!(got, sorted, "gather must deliver in id-sorted order");
    }

    #[test]
    fn observe_unregistered_id_is_noop() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let ghost = ResizeObserverId::from_raw(999);
        reg.observe(&mut dom, ghost, el, ResizeObserverOptions::default());
        assert!(
            dom.world().get::<&ResizeObservedBy>(el).is_err(),
            "observe on an unregistered id must not attach a component"
        );
    }

    #[test]
    fn unregister_retires_id() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        reg.unregister(&mut dom, id);

        // The id is retired: a later observe with it is a no-op.
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        assert!(
            dom.world().get::<&ResizeObservedBy>(el).is_err(),
            "a retired id must not be reusable for observe"
        );
    }

    #[test]
    fn retire_collected_drops_registered_id() {
        // S5-3c registry-side leak fix: `retire_collected` retires the observer's
        // `registered` id for a GC-collected wrapper (binding row pruned), so the
        // registry does not grow monotonically. Dom-free — a collected observer is
        // guaranteed non-observing, so no target-list scrub is needed.
        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        assert_eq!(reg.registered_len(), 1);

        reg.retire_collected(id);
        assert_eq!(
            reg.registered_len(),
            0,
            "retire_collected drops the registered id (no residual)"
        );
        // A retired id is not reusable for observe (mirrors unregister).
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        assert!(
            dom.world().get::<&ResizeObservedBy>(el).is_err(),
            "a retired id must not be reusable for observe"
        );
        // next_id stays monotonic across the retire.
        let id2 = reg.register();
        assert_ne!(id2.raw(), id.raw());
    }

    // --- observing_observer_ids (the S5-3c GC-keepalive membership query) ---

    #[test]
    fn observing_ids_empty_world_is_empty() {
        let dom = EcsDom::new();
        assert!(observing_observer_ids(&dom).is_empty());
    }

    #[test]
    fn observing_ids_present_after_observe() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());

        let ids = observing_observer_ids(&dom);
        assert!(ids.contains(&id.raw()));
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn observing_ids_absent_after_unobserve() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
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
        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
        reg.disconnect(&mut dom, id);
        assert!(observing_observer_ids(&dom).is_empty());
    }

    #[test]
    fn observing_ids_absent_after_despawn_of_sole_target() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(&mut dom, id, el, ResizeObserverOptions::default());
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
        let mut reg = ResizeObserverRegistry::new();
        let id_a = reg.register();
        let id_b = reg.register();
        reg.observe(&mut dom, id_a, a, ResizeObserverOptions::default());
        reg.observe(&mut dom, id_b, b, ResizeObserverOptions::default());

        let ids = observing_observer_ids(&dom);
        assert!(ids.contains(&id_a.raw()));
        assert!(ids.contains(&id_b.raw()));
        assert_eq!(ids.len(), 2);
    }
}
