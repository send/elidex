//! `IntersectionObserver` API (Intersection Observer §3).
//!
//! Observes visibility changes of elements relative to a root element or viewport.
//!
//! ECS-native model: each observation lives as an `IntersectionObservedBy`
//! component on the observed target entity (carrying the per-observer
//! last-reported ratio for threshold-crossing detection), so a despawned entity
//! drops its observations automatically. The registry holds the id counter plus
//! the per-observer `IntersectionObserverInit` configuration.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::Rect;

/// Geometry source for `gather_observations`: returns the current bounding rect
/// for an entity. Takes `&EcsDom` so it can read layout without aliasing the
/// `&mut EcsDom` write-back borrow.
type RectProvider<'a> = dyn Fn(&EcsDom, Entity) -> Option<Rect> + 'a;

/// A unique identifier for an intersection observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionObserverId(u64);

impl IntersectionObserverId {
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

/// Options for creating an `IntersectionObserver`.
#[derive(Debug, Clone, Default)]
pub struct IntersectionObserverInit {
    /// The root element (None = viewport).
    pub root: Option<Entity>,
    /// Root margin (CSS margin syntax, e.g. "10px 20px").
    pub root_margin: String,
    /// Thresholds at which to trigger (0.0 to 1.0).
    pub threshold: Vec<f64>,
}

/// An intersection observation entry.
#[derive(Debug, Clone)]
pub struct IntersectionObserverEntry {
    /// The observed element.
    pub target: Entity,
    /// The intersection ratio (0.0 to 1.0).
    pub intersection_ratio: f64,
    /// Whether the target is intersecting.
    pub is_intersecting: bool,
}

/// A single observation on a node (one per observer watching this entity).
#[derive(Debug, Clone)]
struct IntersectionObservation {
    observer: IntersectionObserverId,
    /// Last reported intersection ratio (for threshold-crossing detection).
    last_ratio: Option<f64>,
}

/// Per-node component listing the intersection observers watching this entity.
/// Dropped automatically when the entity is despawned.
#[derive(Debug, Default)]
struct IntersectionObservedBy(Vec<IntersectionObservation>);

/// Registry for active intersection observers.
#[derive(Debug, Default)]
pub struct IntersectionObserverRegistry {
    next_id: u64,
    init: std::collections::HashMap<IntersectionObserverId, IntersectionObserverInit>,
}

impl IntersectionObserverRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new intersection observer, returning its ID.
    pub fn register(&mut self, init: IntersectionObserverInit) -> IntersectionObserverId {
        let id = IntersectionObserverId(self.next_id);
        self.next_id += 1;
        self.init.insert(id, init);
        id
    }

    /// Start observing a target (Intersection Observer §2.2 `observe(target)`).
    /// Re-observing the same target is a no-op.
    pub fn observe(&mut self, dom: &mut EcsDom, id: IntersectionObserverId, target: Entity) {
        if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(target) {
            if !comp.0.iter().any(|o| o.observer == id) {
                comp.0.push(IntersectionObservation {
                    observer: id,
                    last_ratio: None,
                });
            }
            return;
        }
        let _ = dom.world_mut().insert_one(
            target,
            IntersectionObservedBy(vec![IntersectionObservation {
                observer: id,
                last_ratio: None,
            }]),
        );
    }

    /// Stop observing a specific target (Intersection Observer §2.2 `unobserve(target)`).
    pub fn unobserve(&mut self, dom: &mut EcsDom, id: IntersectionObserverId, target: Entity) {
        let mut now_empty = false;
        if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(target) {
            comp.0.retain(|o| o.observer != id);
            now_empty = comp.0.is_empty();
        }
        if now_empty {
            let _ = dom.world_mut().remove_one::<IntersectionObservedBy>(target);
        }
    }

    /// Stop observing all targets for this observer (Intersection Observer §2.2 `disconnect()`).
    pub fn disconnect(&mut self, dom: &mut EcsDom, id: IntersectionObserverId) {
        let mut emptied: Vec<Entity> = Vec::new();
        for (entity, comp) in &mut dom.world().query::<(Entity, &mut IntersectionObservedBy)>() {
            comp.0.retain(|o| o.observer != id);
            if comp.0.is_empty() {
                emptied.push(entity);
            }
        }
        for entity in emptied {
            let _ = dom.world_mut().remove_one::<IntersectionObservedBy>(entity);
        }
    }

    /// Remove the observer entirely (drops its registrations and config).
    pub fn unregister(&mut self, dom: &mut EcsDom, id: IntersectionObserverId) {
        self.disconnect(dom, id);
        self.init.remove(&id);
    }

    /// Gather intersection observations by computing intersection ratios
    /// (Intersection Observer §3.2.8 "Run the Update Intersection Observations
    /// Steps").
    ///
    /// `rect_fn` provides the current bounding rect for an entity, taking
    /// `&EcsDom` so it can read layout without aliasing the `&mut EcsDom`
    /// write-back borrow. `viewport` is the root viewport rect. Returns
    /// `(observer_id, entries)` pairs for observers with threshold crossings.
    pub fn gather_observations(
        &mut self,
        dom: &mut EcsDom,
        rect_fn: &RectProvider<'_>,
        viewport: Rect,
    ) -> Vec<(IntersectionObserverId, Vec<IntersectionObserverEntry>)> {
        let mut grouped: std::collections::HashMap<
            IntersectionObserverId,
            Vec<IntersectionObserverEntry>,
        > = std::collections::HashMap::new();
        let mut changes: Vec<(Entity, IntersectionObserverId, f64)> = Vec::new();

        // Phase A: read observations + compute ratios (shared borrows only —
        // the `&IntersectionObservedBy` query and `rect_fn`'s layout reads
        // touch disjoint components, so they coexist).
        for (entity, comp) in &mut dom.world().query::<(Entity, &IntersectionObservedBy)>() {
            let Some(target_rect) = rect_fn(&*dom, entity) else {
                continue;
            };
            for obs in &comp.0 {
                let Some(init) = self.init.get(&obs.observer) else {
                    continue;
                };
                let root_rect = init
                    .root
                    .and_then(|r| rect_fn(&*dom, r))
                    .unwrap_or(viewport);
                let thresholds = if init.threshold.is_empty() {
                    &[0.0][..]
                } else {
                    &init.threshold
                };
                let ratio = compute_intersection_ratio(target_rect, root_rect);
                let last = obs.last_ratio.unwrap_or(-1.0);
                if crossed_threshold(last, ratio, thresholds) {
                    changes.push((entity, obs.observer, ratio));
                    grouped
                        .entry(obs.observer)
                        .or_default()
                        .push(IntersectionObserverEntry {
                            target: entity,
                            intersection_ratio: ratio,
                            is_intersecting: ratio > 0.0,
                        });
                }
            }
        }

        // Phase B: write back last ratios (exclusive borrow).
        for (entity, observer, ratio) in changes {
            if let Ok(mut comp) = dom.world_mut().get::<&mut IntersectionObservedBy>(entity) {
                if let Some(obs) = comp.0.iter_mut().find(|o| o.observer == observer) {
                    obs.last_ratio = Some(ratio);
                }
            }
        }

        grouped.into_iter().collect()
    }
}

/// Compute the intersection ratio between a target rect and root rect.
fn compute_intersection_ratio(target: Rect, root: Rect) -> f64 {
    let target_area = target.size.area_f64();
    if target_area <= 0.0 {
        return 0.0;
    }

    let intersection_area = target
        .intersection(&root)
        .map_or(0.0, |inter| inter.size.area_f64());

    (intersection_area / target_area).clamp(0.0, 1.0)
}

/// Check if the ratio transition crosses any threshold.
fn crossed_threshold(old: f64, new: f64, thresholds: &[f64]) -> bool {
    for &t in thresholds {
        let old_above = old >= t;
        let new_above = new >= t;
        if old_above != new_above {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn intersection_ratio_fully_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(10.0, 10.0, 100.0, 100.0), // target
            Rect::new(0.0, 0.0, 1024.0, 768.0),  // viewport
        );
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn intersection_ratio_half_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(50.0, 0.0, 1000.0, 1000.0), // viewport starts at x=50
        );
        assert!((ratio - 0.5).abs() < 0.001);
    }

    #[test]
    fn intersection_ratio_not_visible() {
        let ratio = compute_intersection_ratio(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(200.0, 200.0, 100.0, 100.0),
        );
        assert!((ratio - 0.0).abs() < 0.001);
    }

    #[test]
    fn threshold_crossing_detection() {
        assert!(crossed_threshold(-1.0, 0.5, &[0.0]));
        assert!(!crossed_threshold(0.5, 0.6, &[0.0]));
        assert!(crossed_threshold(0.4, 0.6, &[0.5]));
        assert!(crossed_threshold(0.6, 0.4, &[0.5]));
    }

    #[test]
    fn gather_initial_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit {
            threshold: vec![0.0],
            ..Default::default()
        };
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init);
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        // Element fully inside viewport.
        let observations = reg.gather_observations(
            &mut dom,
            &|_, e| {
                if e == el {
                    Some(Rect::new(10.0, 10.0, 100.0, 100.0))
                } else {
                    None
                }
            },
            viewport,
        );
        // Initial ratio -1.0 → 1.0 crosses threshold 0.0.
        assert_eq!(observations.len(), 1);
        assert!(observations[0].1[0].is_intersecting);
    }

    #[test]
    fn no_crossing_no_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit {
            threshold: vec![0.5],
            ..Default::default()
        };
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init);
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);

        // First: fully visible → ratio 1.0, crosses 0.5.
        reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );

        // Still fully visible — same ratio, no crossing.
        let observations = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(20.0, 20.0, 100.0, 100.0)),
            viewport,
        );
        assert!(observations.is_empty());
    }

    #[test]
    fn despawn_clears_observation() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit::default();
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init);
        reg.observe(&mut dom, id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );

        let _ = dom.destroy_entity(el);
        let observations = reg.gather_observations(
            &mut dom,
            &|_, _| Some(Rect::new(10.0, 10.0, 100.0, 100.0)),
            viewport,
        );
        assert!(observations.is_empty());
    }
}
