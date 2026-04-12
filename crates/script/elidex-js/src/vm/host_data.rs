//! Host-provided data for bridging the VM to the browser shell.
//!
//! `HostData` carries raw pointers to `SessionCore` and `EcsDom`, plus
//! caches for event listeners, DOM wrappers, and timers.  It follows the
//! same bind/unbind lifecycle as boa's `HostBridge`: the shell calls
//! `bind()` before `eval`/`call_listener`, and `unbind()` after.
//!
//! Boundness is derived from pointer nullness — no redundant flag.  Between
//! `unbind()` and the next `bind()` the pointers are null; `session()`/`dom()`
//! panic in that state.
//!
//! # Safety
//!
//! The raw pointers are valid only between `bind()` and `unbind()`.

#[cfg(feature = "engine")]
mod engine_feature {
    use super::super::value::ObjectId;
    use elidex_ecs::Entity;
    use elidex_script_session::ListenerId;
    use std::collections::HashMap;

    pub struct HostData {
        session_ptr: *mut elidex_script_session::SessionCore,
        dom_ptr: *mut elidex_ecs::EcsDom,
        document_entity: Option<Entity>,
        pub(crate) listener_store: HashMap<ListenerId, ObjectId>,
        pub(crate) wrapper_cache: HashMap<u64, ObjectId>,
    }

    impl HostData {
        pub fn new() -> Self {
            Self {
                session_ptr: std::ptr::null_mut(),
                dom_ptr: std::ptr::null_mut(),
                document_entity: None,
                listener_store: HashMap::new(),
                wrapper_cache: HashMap::new(),
            }
        }

        /// # Safety
        ///
        /// - `session` and `dom` must point to valid, uniquely-owned
        ///   instances until `unbind()` is called.
        /// - The caller MUST NOT access `session` or `dom` via any other
        ///   reference (Stacked-Borrows: raw-pointer aliasing with a live
        ///   `&mut` is UB).  Typical usage: caller holds `&mut`, calls
        ///   `bind(ptr_from_mut)`, invokes VM, calls `unbind()`, then
        ///   resumes using the `&mut`.
        #[allow(unsafe_code)]
        pub unsafe fn bind(
            &mut self,
            session: *mut elidex_script_session::SessionCore,
            dom: *mut elidex_ecs::EcsDom,
            document: Entity,
        ) {
            debug_assert!(!session.is_null() && !dom.is_null());
            self.session_ptr = session;
            self.dom_ptr = dom;
            self.document_entity = Some(document);
        }

        pub fn unbind(&mut self) {
            self.session_ptr = std::ptr::null_mut();
            self.dom_ptr = std::ptr::null_mut();
            self.document_entity = None;
        }

        #[inline]
        pub fn is_bound(&self) -> bool {
            !self.session_ptr.is_null()
        }

        #[allow(unsafe_code)]
        pub fn session(&mut self) -> &mut elidex_script_session::SessionCore {
            assert!(self.is_bound(), "HostData accessed while unbound");
            unsafe { &mut *self.session_ptr }
        }

        #[allow(unsafe_code)]
        pub fn dom(&mut self) -> &mut elidex_ecs::EcsDom {
            assert!(self.is_bound(), "HostData accessed while unbound");
            unsafe { &mut *self.dom_ptr }
        }

        pub fn document(&self) -> Entity {
            assert!(self.is_bound(), "HostData accessed while unbound");
            self.document_entity.unwrap()
        }

        pub fn store_listener(&mut self, id: ListenerId, func: ObjectId) {
            self.listener_store.insert(id, func);
        }

        pub fn get_listener(&self, id: ListenerId) -> Option<ObjectId> {
            self.listener_store.get(&id).copied()
        }

        pub fn remove_listener(&mut self, id: ListenerId) -> Option<ObjectId> {
            self.listener_store.remove(&id)
        }

        pub fn get_cached_wrapper(&self, entity: Entity) -> Option<ObjectId> {
            self.wrapper_cache.get(&entity.to_bits().get()).copied()
        }

        pub fn cache_wrapper(&mut self, entity: Entity, obj: ObjectId) {
            self.wrapper_cache.insert(entity.to_bits().get(), obj);
        }

        pub fn gc_root_object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
            self.listener_store
                .values()
                .copied()
                .chain(self.wrapper_cache.values().copied())
        }
    }

    impl Default for HostData {
        fn default() -> Self {
            Self::new()
        }
    }

    // Intentionally no `Send`/`Sync` impl:
    // `HostData` holds raw pointers to caller-owned host state while bound.
    // Moving a bound VM across threads (or sharing `&HostData` across them)
    // would race with the caller's owned `&mut SessionCore` / `&mut EcsDom`.
    // The invariant is not expressible in Rust's type system, so we keep
    // `HostData` `!Send`/`!Sync` to prevent accidental cross-thread transfer.
    // When Worker threads are introduced (PR2+), each worker owns its own VM;
    // the Send invariant will be designed explicitly (e.g., split unbound
    // cache + bound non-Send guard).
}

#[cfg(not(feature = "engine"))]
mod engine_feature {
    use super::super::value::ObjectId;

    /// Stub: without the `engine` feature, `HostData` carries no state and
    /// provides only the GC-root iterator (always empty).
    #[derive(Default)]
    pub struct HostData;

    impl HostData {
        pub fn new() -> Self {
            Self
        }

        pub fn unbind(&mut self) {}

        pub fn is_bound(&self) -> bool {
            false
        }

        pub fn gc_root_object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
            std::iter::empty()
        }
    }
}

pub use engine_feature::HostData;
