//! `ResizeObserver` API (Resize Observer §3).
//!
//! Observes changes to the content box / border box size of elements.
//!
//! ECS-native model: each observation lives as a `ResizeObservedBy` component
//! on the observed target entity (carrying the per-observer last-reported size
//! used for change detection), so a despawned entity drops its observations
//! automatically. The registry holds only the monotonic id counter.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::Size;

/// Geometry source for `gather_observations`: returns the current
/// `(content_box_size, border_box_size)` for an entity. Takes `&EcsDom` so it
/// can read layout without aliasing the `&mut EcsDom` write-back borrow.
type SizeProvider<'a> = dyn Fn(&EcsDom, Entity) -> Option<(Size, Size)> + 'a;

/// A unique identifier for a resize observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Options for `ResizeObserver.observe()`.
#[derive(Debug, Clone, Default)]
pub struct ResizeObserverOptions {
    /// Which box to observe.
    pub box_model: ResizeObserverBoxOptions,
}

/// A resize observation entry delivered to the callback.
#[derive(Debug, Clone)]
pub struct ResizeObserverEntry {
    /// The observed element.
    pub target: Entity,
    /// Content box size.
    pub content_box_size: Size,
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
#[derive(Debug, Default)]
pub struct ResizeObserverRegistry {
    next_id: u64,
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
        for (entity, comp) in &mut dom.world().query::<(Entity, &mut ResizeObservedBy)>() {
            comp.0.retain(|o| o.observer != id);
            if comp.0.is_empty() {
                emptied.push(entity);
            }
        }
        for entity in emptied {
            let _ = dom.world_mut().remove_one::<ResizeObservedBy>(entity);
        }
    }

    /// Remove the observer entirely (equivalent to disconnect; no registry-side
    /// state beyond the id counter).
    pub fn unregister(&mut self, dom: &mut EcsDom, id: ResizeObserverId) {
        self.disconnect(dom, id);
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
        let mut grouped: std::collections::HashMap<ResizeObserverId, Vec<ResizeObserverEntry>> =
            std::collections::HashMap::new();
        let mut writebacks: Vec<(Entity, ResizeObserverId, Size)> = Vec::new();

        // Phase A: read observations + current sizes (shared borrows only —
        // the `&ResizeObservedBy` query and `size_fn`'s layout reads touch
        // disjoint components, so they coexist).
        for (entity, comp) in &mut dom.world().query::<(Entity, &ResizeObservedBy)>() {
            let Some((content_size, border_size)) = size_fn(&*dom, entity) else {
                continue;
            };
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
                            content_box_size: content_size,
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
                Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
            } else {
                None
            }
        });

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].1.len(), 1);
        assert_eq!(
            observations[0].1[0].content_box_size,
            Size::new(100.0, 50.0)
        );
        assert_eq!(observations[0].1[0].border_box_size, Size::new(110.0, 60.0));
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
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
        });

        // Same sizes — no observations.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
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
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
        });

        // Width changed.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Size::new(200.0, 50.0), Size::new(210.0, 60.0)))
        });
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].1[0].content_box_size,
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
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
        });
        let _ = dom.destroy_entity(el);

        // After despawn, the observation component is gone — no observations.
        let observations = reg.gather_observations(&mut dom, &|_, _| {
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
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
            Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0)))
        });
        assert!(observations.is_empty());
    }
}
