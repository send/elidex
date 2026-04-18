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
//! 2. **Prototype chain dispatched by node kind** — entities carrying
//!    a `TagType` component receive `Element.prototype` as
//!    their prototype; Text / Comment nodes and the document /
//!    window roots fall through to `EventTarget.prototype` directly.
//!    Both chains terminate at `Object.prototype`, so Node-level
//!    members (`parentNode`, `nodeType`, `textContent`, …) remain
//!    accessible on every DOM wrapper via the shared tail.
//!
//! The wrapper carries only `ObjectKind::HostObject { entity_bits }`
//! and its prototype slot — no properties are installed at creation.
//! Per-interface methods (e.g. `getAttribute`, `textContent`) are
//! installed on the shared prototypes rather than duplicated onto
//! each wrapper.

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

        // Pick the prototype based on whether the entity is an
        // Element (TagType present) or a non-Element (Text, Comment,
        // Document, Window, …):
        //
        // - Element  → `Element.prototype` (chained to
        //              EventTarget.prototype).
        // - Non-elem → `EventTarget.prototype` directly.
        //
        // Both prototypes are populated during `register_globals`
        // — missing at that point would mean the VM skipped
        // initialisation entirely (a bug worth panicking on).  When
        // `HostData` is not yet bound (pre-bind wrapper allocation)
        // we fall back to the non-element path; method calls on that
        // wrapper still go through `entity_from_this`, which short-
        // circuits to a no-op while unbound.
        let is_element = self
            .host_data
            .as_deref()
            .is_some_and(|hd| hd.is_element_entity(entity));
        let proto = if is_element {
            self.element_prototype
                .expect("create_element_wrapper called before register_element_prototype")
        } else {
            self.event_target_prototype
                .expect("create_element_wrapper called before register_event_target_prototype")
        };
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
