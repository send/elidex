//! Element (and more generally, DOM) wrapper creation.
//!
//! `create_element_wrapper(entity)` is the single entry point used by
//! every host-side DOM API that needs to surface an Entity as a JS
//! object.  It enforces two invariants:
//!
//! 1. **Identity** — `el === el` across repeated lookups.  The
//!    wrapper `ObjectId` is cached in `HostData::wrapper_cache`, keyed
//!    by `Entity::to_bits().get()`.  A cache hit returns the existing
//!    ObjectId without allocating.
//! 2. **EventTarget inheritance** — the wrapper's prototype is
//!    `EventTarget.prototype` (PR3 C0), so
//!    `addEventListener` / `removeEventListener` / `dispatchEvent`
//!    resolve via the prototype chain instead of requiring per-wrapper
//!    method registration.
//!
//! The wrapper carries only `ObjectKind::HostObject { entity_bits }`
//! and its prototype slot — no properties are installed at creation.
//! Per-interface methods (e.g. `getAttribute`, `textContent`) arrive
//! in **PR4** when the full DOM method suite lands.

#[cfg(feature = "engine")]
use super::super::shape;
#[cfg(feature = "engine")]
use super::super::value::{Object, ObjectId, ObjectKind, PropertyStorage};
#[cfg(feature = "engine")]
use super::super::VmInner;

#[cfg(feature = "engine")]
impl VmInner {
    /// Return the shared JS wrapper ObjectId for `entity`, allocating a
    /// new `HostObject` on the first call and reusing the cached one on
    /// every subsequent call.
    ///
    /// # Panics
    ///
    /// Panics if `HostData` has not been *installed* via
    /// `Vm::install_host_data` (the cache lives on `HostData` so
    /// nowhere to cache the result), or if `event_target_prototype`
    /// has not been initialised (`register_globals` not yet run —
    /// should be impossible after `Vm::new` returns).
    ///
    /// Bind state is **irrelevant** here: the wrapper cache is a
    /// HashMap on `HostData`, not a session/dom dereference, so this
    /// function works after `Vm::unbind()` too — useful for code
    /// paths that build wrappers as part of pre-eval setup.  Calling
    /// methods on the returned wrapper that touch `dom()` does still
    /// require a bound HostData; see the per-native checks in
    /// `vm/host/event_target.rs`.
    ///
    /// # GC safety
    ///
    /// `alloc_object` may trigger a collection before the new object
    /// is installed.  The caller must not hold any `&Object` references
    /// across this call.  The freshly-returned `ObjectId` is rooted by
    /// `wrapper_cache` immediately after allocation; until that point
    /// the only reference is the local — no GC-traceable structure
    /// points at it, and no intervening allocation happens, so GC
    /// cannot run in that window.
    pub(crate) fn create_element_wrapper(&mut self, entity: elidex_ecs::Entity) -> ObjectId {
        // Fast path: identity cache hit.  `HostData` borrow is scoped
        // to this block so the subsequent `alloc_object` call (which
        // needs `&mut self`) is unblocked on miss.
        if let Some(existing) = self
            .host_data
            .as_deref()
            .and_then(|hd| hd.get_cached_wrapper(entity))
        {
            return existing;
        }

        // `event_target_prototype` is set by `register_globals` during
        // `Vm::new` — it should never be None here.  Use `expect` so a
        // future refactor that reorders init fails loudly in release
        // too, instead of silently creating a wrapper without
        // EventTarget.prototype (breaking method lookup via the chain).
        let proto = self
            .event_target_prototype
            .expect("create_element_wrapper called before register_event_target_prototype");
        let obj = self.alloc_object(Object {
            kind: ObjectKind::HostObject {
                entity_bits: entity.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });

        // Register in the wrapper cache so the next lookup for this
        // Entity returns the same ObjectId (and the object stays
        // rooted via `HostData::gc_root_object_ids`).
        self.host_data
            .as_deref_mut()
            .expect("create_element_wrapper requires installed HostData")
            .cache_wrapper(entity, obj);
        obj
    }
}
