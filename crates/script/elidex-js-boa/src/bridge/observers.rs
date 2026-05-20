//! Observer API methods for `HostBridge`.
//!
//! Provides access to `MutationObserver`, `ResizeObserver`, and
//! `IntersectionObserver` registries, plus callback/object storage.

use boa_engine::JsObject;
use elidex_api_observers::intersection::IntersectionObserverRegistry;
use elidex_api_observers::mutation::MutationObserverRegistry;
use elidex_api_observers::resize::ResizeObserverRegistry;
use elidex_ecs::EcsDom;

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

    /// Access the mutation observer registry together with the bound
    /// `EcsDom`, for `observe` / `disconnect` (which mutate the per-entity
    /// `MutationObservedBy` component). The `RefCell` borrow is held for
    /// the whole closure, so `f` must not call back into other
    /// `HostBridge` methods. Must be called while bound (native-fn path).
    ///
    /// # Panics
    /// Panics if the bridge is not bound.
    pub fn with_mutation_observers_and_dom<R>(
        &self,
        f: impl FnOnce(&mut EcsDom, &mut MutationObserverRegistry) -> R,
    ) -> R {
        let mut inner = self.inner.borrow_mut();
        assert!(
            !inner.dom_ptr.is_null(),
            "with_mutation_observers_and_dom() called while unbound"
        );
        // SAFETY: `dom_ptr` is valid for the bind/unbind bracket; the
        // `EcsDom` allocation is disjoint from `inner`'s registry storage,
        // so the `&mut EcsDom` and `&mut inner.mutation_observers` cannot
        // alias. Holding the `RefMut` for the closure prevents any
        // re-entrant bridge access (RefCell double-borrow would panic).
        #[allow(unsafe_code)]
        let dom = unsafe { &mut *inner.dom_ptr };
        f(dom, &mut inner.mutation_observers)
    }

    /// Like [`Self::with_mutation_observers_and_dom`] for the resize registry.
    ///
    /// # Panics
    /// Panics if the bridge is not bound.
    pub fn with_resize_observers_and_dom<R>(
        &self,
        f: impl FnOnce(&mut EcsDom, &mut ResizeObserverRegistry) -> R,
    ) -> R {
        let mut inner = self.inner.borrow_mut();
        assert!(
            !inner.dom_ptr.is_null(),
            "with_resize_observers_and_dom() called while unbound"
        );
        // SAFETY: see `with_mutation_observers_and_dom`.
        #[allow(unsafe_code)]
        let dom = unsafe { &mut *inner.dom_ptr };
        f(dom, &mut inner.resize_observers)
    }

    /// Like [`Self::with_mutation_observers_and_dom`] for the intersection registry.
    ///
    /// # Panics
    /// Panics if the bridge is not bound.
    pub fn with_intersection_observers_and_dom<R>(
        &self,
        f: impl FnOnce(&mut EcsDom, &mut IntersectionObserverRegistry) -> R,
    ) -> R {
        let mut inner = self.inner.borrow_mut();
        assert!(
            !inner.dom_ptr.is_null(),
            "with_intersection_observers_and_dom() called while unbound"
        );
        // SAFETY: see `with_mutation_observers_and_dom`.
        #[allow(unsafe_code)]
        let dom = unsafe { &mut *inner.dom_ptr };
        f(dom, &mut inner.intersection_observers)
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
