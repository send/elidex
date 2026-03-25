//! Observer API methods for `HostBridge`.
//!
//! Provides access to `MutationObserver`, `ResizeObserver`, and
//! `IntersectionObserver` registries, plus callback/object storage.

use boa_engine::JsObject;
use elidex_api_observers::intersection::IntersectionObserverRegistry;
use elidex_api_observers::mutation::MutationObserverRegistry;
use elidex_api_observers::resize::ResizeObserverRegistry;

use super::HostBridge;

impl HostBridge {
    /// Access the mutation observer registry mutably.
    pub fn with_mutation_observers<R>(
        &self,
        f: impl FnOnce(&mut MutationObserverRegistry) -> R,
    ) -> R {
        f(&mut self.inner.borrow_mut().mutation_observers)
    }

    /// Access the resize observer registry mutably.
    pub fn with_resize_observers<R>(&self, f: impl FnOnce(&mut ResizeObserverRegistry) -> R) -> R {
        f(&mut self.inner.borrow_mut().resize_observers)
    }

    /// Access the intersection observer registry mutably.
    pub fn with_intersection_observers<R>(
        &self,
        f: impl FnOnce(&mut IntersectionObserverRegistry) -> R,
    ) -> R {
        f(&mut self.inner.borrow_mut().intersection_observers)
    }

    /// Store a JS callback for an observer.
    pub fn store_observer_callback(
        &self,
        observer_id: u64,
        callback: JsObject,
        observer_obj: JsObject,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner.observer_callbacks.insert(observer_id, callback);
        inner.observer_objects.insert(observer_id, observer_obj);
    }

    /// Get the JS callback for an observer.
    pub fn get_observer_callback(&self, observer_id: u64) -> Option<JsObject> {
        self.inner
            .borrow()
            .observer_callbacks
            .get(&observer_id)
            .cloned()
    }

    /// Get the JS observer wrapper object.
    pub fn get_observer_object(&self, observer_id: u64) -> Option<JsObject> {
        self.inner
            .borrow()
            .observer_objects
            .get(&observer_id)
            .cloned()
    }

    /// Remove an observer's callback and wrapper.
    pub fn remove_observer(&self, observer_id: u64) {
        let mut inner = self.inner.borrow_mut();
        inner.observer_callbacks.remove(&observer_id);
        inner.observer_objects.remove(&observer_id);
    }
}
