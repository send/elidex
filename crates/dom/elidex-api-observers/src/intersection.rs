//! `IntersectionObserver` API (Intersection Observer §2).
//!
//! Observes visibility changes of elements relative to a root element or viewport.

use elidex_ecs::Entity;
use elidex_plugin::Rect;

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

/// Registry for active intersection observers.
#[derive(Debug, Default)]
pub struct IntersectionObserverRegistry {
    next_id: u64,
    observers: Vec<IntersectionObserverState>,
}

/// Stored per-observer, per-entity last known ratios for threshold crossing detection.
type LastRatios = std::collections::HashMap<Entity, f64>;

#[derive(Debug)]
struct IntersectionObserverState {
    id: IntersectionObserverId,
    /// Observer configuration (root, rootMargin, threshold).
    init: IntersectionObserverInit,
    targets: Vec<Entity>,
    /// Last reported intersection ratios (entity → ratio).
    last_ratios: LastRatios,
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
        self.observers.push(IntersectionObserverState {
            id,
            init,
            targets: Vec::new(),
            last_ratios: LastRatios::new(),
        });
        id
    }

    /// Start observing a target.
    pub fn observe(&mut self, id: IntersectionObserverId, target: Entity) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            if !entry.targets.contains(&target) {
                entry.targets.push(target);
            }
        }
    }

    /// Stop observing a specific target.
    pub fn unobserve(&mut self, id: IntersectionObserverId, target: Entity) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            entry.targets.retain(|e| *e != target);
        }
    }

    /// Stop observing all targets for this observer.
    pub fn disconnect(&mut self, id: IntersectionObserverId) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            entry.targets.clear();
        }
    }

    /// Remove the observer entirely.
    pub fn unregister(&mut self, id: IntersectionObserverId) {
        self.observers.retain(|e| e.id != id);
    }

    /// Remove a destroyed entity from all observer target lists.
    ///
    /// Call this when an entity is removed from the DOM to prevent stale references.
    pub fn remove_entity(&mut self, entity: Entity) {
        for entry in &mut self.observers {
            entry.targets.retain(|e| *e != entity);
            entry.last_ratios.remove(&entity);
        }
    }

    /// Gather intersection observations by computing intersection ratios.
    ///
    /// `rect_fn` provides the current bounding rect for an entity.
    /// `viewport` is the root viewport rect.
    /// Returns `(observer_id, entries)` pairs for observers with threshold crossings.
    pub fn gather_observations(
        &mut self,
        rect_fn: &dyn Fn(Entity) -> Option<Rect>,
        viewport: Rect,
    ) -> Vec<(IntersectionObserverId, Vec<IntersectionObserverEntry>)> {
        let mut result = Vec::new();
        for observer in &mut self.observers {
            let root_rect = observer.init.root.and_then(rect_fn).unwrap_or(viewport);
            let thresholds = if observer.init.threshold.is_empty() {
                &[0.0][..]
            } else {
                &observer.init.threshold
            };

            let mut entries = Vec::new();
            for &target in &observer.targets {
                let Some(target_rect) = rect_fn(target) else {
                    continue;
                };
                let ratio = compute_intersection_ratio(target_rect, root_rect);
                let last_ratio = observer.last_ratios.get(&target).copied().unwrap_or(-1.0);

                if crossed_threshold(last_ratio, ratio, thresholds) {
                    observer.last_ratios.insert(target, ratio);
                    entries.push(IntersectionObserverEntry {
                        target,
                        intersection_ratio: ratio,
                        is_intersecting: ratio > 0.0,
                    });
                }
            }
            if !entries.is_empty() {
                result.push((observer.id, entries));
            }
        }
        result
    }

    /// Access the init config for a given observer.
    #[must_use]
    pub fn get_init(&self, id: IntersectionObserverId) -> Option<&IntersectionObserverInit> {
        self.observers.iter().find(|e| e.id == id).map(|e| &e.init)
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
        reg.observe(id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        // Element fully inside viewport.
        let observations = reg.gather_observations(
            &|e| {
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
        reg.observe(id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);

        // First: fully visible → ratio 1.0, crosses 0.5.
        reg.gather_observations(&|_| Some(Rect::new(10.0, 10.0, 100.0, 100.0)), viewport);

        // Still fully visible — same ratio, no crossing.
        let observations =
            reg.gather_observations(&|_| Some(Rect::new(20.0, 20.0, 100.0, 100.0)), viewport);
        assert!(observations.is_empty());
    }

    #[test]
    fn remove_entity_clears_ratios() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let init = IntersectionObserverInit::default();
        let mut reg = IntersectionObserverRegistry::new();
        let id = reg.register(init);
        reg.observe(id, el);

        let viewport = Rect::new(0.0, 0.0, 1024.0, 768.0);
        reg.gather_observations(&|_| Some(Rect::new(10.0, 10.0, 100.0, 100.0)), viewport);

        reg.remove_entity(el);
        let observations =
            reg.gather_observations(&|_| Some(Rect::new(10.0, 10.0, 100.0, 100.0)), viewport);
        assert!(observations.is_empty());
    }
}
