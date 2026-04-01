//! `ResizeObserver` API (Resize Observer §2).
//!
//! Observes changes to the content box / border box size of elements.

use elidex_ecs::Entity;
use elidex_plugin::Size;

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

/// Registry for active resize observers.
#[derive(Debug, Default)]
pub struct ResizeObserverRegistry {
    next_id: u64,
    observers: Vec<ResizeObserverState>,
}

/// Stored per-observer, per-entity last known sizes for change detection.
type LastSizes = std::collections::HashMap<Entity, Size>;

#[derive(Debug)]
struct ResizeObserverState {
    id: ResizeObserverId,
    targets: Vec<(Entity, ResizeObserverOptions)>,
    /// Last reported content box sizes (entity → (width, height)).
    last_sizes: LastSizes,
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
        self.observers.push(ResizeObserverState {
            id,
            targets: Vec::new(),
            last_sizes: LastSizes::new(),
        });
        id
    }

    /// Start observing a target.
    pub fn observe(
        &mut self,
        id: ResizeObserverId,
        target: Entity,
        options: ResizeObserverOptions,
    ) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            if !entry.targets.iter().any(|(e, _)| *e == target) {
                entry.targets.push((target, options));
            }
        }
    }

    /// Stop observing a specific target.
    pub fn unobserve(&mut self, id: ResizeObserverId, target: Entity) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            entry.targets.retain(|(e, _)| *e != target);
        }
    }

    /// Stop observing all targets for this observer.
    pub fn disconnect(&mut self, id: ResizeObserverId) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            entry.targets.clear();
        }
    }

    /// Remove the observer entirely.
    pub fn unregister(&mut self, id: ResizeObserverId) {
        self.observers.retain(|e| e.id != id);
    }

    /// Remove a destroyed entity from all observer target lists.
    ///
    /// Call this when an entity is removed from the DOM to prevent stale references.
    pub fn remove_entity(&mut self, entity: Entity) {
        for entry in &mut self.observers {
            entry.targets.retain(|(e, _)| *e != entity);
            entry.last_sizes.remove(&entity);
        }
    }

    /// Gather resize observations by comparing current sizes against last known sizes.
    ///
    /// `size_fn` provides the current `(content_box_size, border_box_size)` for each
    /// observed entity. Returns a list of `(observer_id, entries)` pairs for
    /// observers that have at least one changed target.
    pub fn gather_observations(
        &mut self,
        size_fn: &dyn Fn(Entity) -> Option<(Size, Size)>,
    ) -> Vec<(ResizeObserverId, Vec<ResizeObserverEntry>)> {
        let mut result = Vec::new();
        for observer in &mut self.observers {
            let mut entries = Vec::new();
            for &(target, _) in &observer.targets {
                let Some((content_size, border_size)) = size_fn(target) else {
                    continue;
                };
                let changed = observer.last_sizes.get(&target).is_none_or(|last| {
                    (last.width - content_size.width).abs() > f32::EPSILON
                        || (last.height - content_size.height).abs() > f32::EPSILON
                });
                if changed {
                    observer.last_sizes.insert(target, content_size);
                    entries.push(ResizeObserverEntry {
                        target,
                        content_box_size: content_size,
                        border_box_size: border_size,
                    });
                }
            }
            if !entries.is_empty() {
                result.push((observer.id, entries));
            }
        }
        result
    }

    /// Returns iterator over all observer IDs.
    pub fn observer_ids(&self) -> impl Iterator<Item = ResizeObserverId> + '_ {
        self.observers.iter().map(|e| e.id)
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
        reg.observe(id, el, ResizeObserverOptions::default());

        let observations = reg.gather_observations(&|e| {
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
        reg.observe(id, el, ResizeObserverOptions::default());

        // First gather — initial observation.
        reg.gather_observations(&|_| Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0))));

        // Same sizes — no observations.
        let observations =
            reg.gather_observations(&|_| Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0))));
        assert!(observations.is_empty());
    }

    #[test]
    fn gather_detects_size_change() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(id, el, ResizeObserverOptions::default());

        reg.gather_observations(&|_| Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0))));

        // Width changed.
        let observations =
            reg.gather_observations(&|_| Some((Size::new(200.0, 50.0), Size::new(210.0, 60.0))));
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].1[0].content_box_size,
            Size::new(200.0, 50.0)
        );
    }

    #[test]
    fn remove_entity_clears_last_sizes() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = ResizeObserverRegistry::new();
        let id = reg.register();
        reg.observe(id, el, ResizeObserverOptions::default());
        reg.gather_observations(&|_| Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0))));
        reg.remove_entity(el);

        // After removal, no observations should be produced.
        let observations =
            reg.gather_observations(&|_| Some((Size::new(100.0, 50.0), Size::new(110.0, 60.0))));
        assert!(observations.is_empty());
    }
}
