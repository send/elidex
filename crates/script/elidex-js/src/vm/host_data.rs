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

        /// # Panics
        ///
        /// Panics if `HostData` is already bound.  Double-bind indicates a
        /// missing `unbind()` call (e.g. exception recovery bug); silently
        /// overwriting would abandon the caller's prior borrow.
        ///
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
            assert!(
                !self.is_bound(),
                "HostData::bind called while already bound; missing unbind()?"
            );
            // Non-null enforcement in release builds too: a null session
            // or dom pointer would make subsequent session()/dom() deref
            // immediate UB.
            assert!(
                !session.is_null() && !dom.is_null(),
                "HostData::bind requires non-null session and dom pointers"
            );
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

        /// # Panics
        ///
        /// Panics if the `ListenerId` is already registered.  `ListenerId`
        /// values are expected to be unique per `addEventListener` call;
        /// a duplicate would silently orphan the prior `ObjectId` and
        /// drop it from `gc_root_object_ids` — a recipe for a
        /// use-after-free if any JS-side reference to the old listener
        /// still exists.  Enforced in release too.
        pub fn store_listener(&mut self, id: ListenerId, func: ObjectId) {
            let prev = self.listener_store.insert(id, func);
            assert!(prev.is_none(), "duplicate ListenerId {id:?}");
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

        /// # Panics
        ///
        /// Panics if the Entity already has a cached wrapper.  Wrapper cache
        /// identity (`el === el`) requires the caller to check
        /// `get_cached_wrapper()` first; silently overwriting would
        /// orphan the prior wrapper ObjectId, dropping it from
        /// `gc_root_object_ids` while live JS references may still
        /// reach it.  Enforced in release too.
        pub fn cache_wrapper(&mut self, entity: Entity, obj: ObjectId) {
            let prev = self.wrapper_cache.insert(entity.to_bits().get(), obj);
            assert!(
                prev.is_none(),
                "wrapper already cached for Entity {entity:?}"
            );
        }

        /// Drop the cached wrapper for `entity`, returning the prior
        /// `ObjectId` if any.  Called when an entity is destroyed
        /// (DOM mutation removed it) so its wrapper becomes eligible
        /// for GC instead of leaking via the cache root.
        ///
        /// PR3 introduces the API; the DOM-mutation hook that calls
        /// it lives in PR4 alongside the rest of the tree-mutation
        /// surface (`removeChild`, `replaceWith`, etc.).  Until then
        /// wrappers for destroyed entities stay rooted — a known but
        /// bounded leak (capped by the number of distinct entities
        /// the page ever observes).
        pub fn remove_wrapper(&mut self, entity: Entity) -> Option<ObjectId> {
            self.wrapper_cache.remove(&entity.to_bits().get())
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

    // Raw pointers (`*mut T`) are `!Send` and `!Sync` in Rust by default
    // (<https://doc.rust-lang.org/nomicon/send-and-sync.html>), so the
    // compiler automatically infers `HostData: !Send + !Sync` from the
    // `session_ptr` / `dom_ptr` fields.  We deliberately do NOT add an
    // `unsafe impl Send`; moving a bound VM across threads would race with
    // the caller's `&mut SessionCore` / `&mut EcsDom`.  When Worker threads
    // are introduced (PR2+), each worker will own its own VM and the Send
    // invariant will be designed explicitly (e.g., split unbound cache +
    // bound non-Send guard).
    //
    // REGRESSION GUARD: if the raw pointer fields are ever replaced with
    // `Send` types (e.g. `NonNull<T>` wrapped in `Arc`), add an explicit
    // `PhantomData<*const ()>` marker field to preserve `!Send + !Sync`.
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
