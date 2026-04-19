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
//!    a `TagType` component receive `Element.prototype`; other Node
//!    entities (Text / Comment / Document / DocumentFragment) fall
//!    through to `Node.prototype` directly.  Both chains terminate
//!    at `Object.prototype` via `Node.prototype → EventTarget.prototype`,
//!    so Node-level members (`parentNode`, `nodeType`, `textContent`,
//!    …) are visible on every DOM wrapper.  Window is wrapped
//!    independently (see `vm/globals.rs`) and does *not* chain
//!    through `Node.prototype` — Window is an EventTarget but not
//!    a Node per WHATWG.
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

        // Pick the prototype based on the entity's DOM node kind.
        // `prototype_kind_for` centralises the Element / Text /
        // Comment / other-Node dispatch for wrapper creation:
        //
        // - Element             → `Element.prototype`
        //                         (→ Node.prototype → EventTarget.prototype).
        // - Text                → `Text.prototype`
        //                         (→ CharacterData.prototype → Node.prototype).
        // - Comment / PI / CDATA → `CharacterData.prototype`
        //                         (→ Node.prototype).
        // - Document / DocumentFragment / DocumentType / unbound
        //                       → `Node.prototype` directly.
        //
        // Pre-bind / unbound wrapper allocation falls through to the
        // OtherNode branch (Node.prototype); method calls on that
        // wrapper route through `entity_from_this`, which
        // short-circuits to a no-op while unbound.
        //
        // `Window` is NOT wrapped via this path — it gets an
        // independent `HostObject` allocated in `register_globals`
        // whose prototype chain skips `Node.prototype` so Node
        // members do not appear on `window` (WHATWG: Window is an
        // EventTarget but not a Node).
        let kind = self
            .host_data
            .as_deref()
            .map_or(super::super::host_data::PrototypeKind::OtherNode, |hd| {
                hd.prototype_kind_for(entity)
            });
        let proto = match kind {
            super::super::host_data::PrototypeKind::Element => self
                .element_prototype
                .expect("create_element_wrapper called before register_element_prototype"),
            super::super::host_data::PrototypeKind::Text => {
                // Text wrappers chain `Text.prototype →
                // CharacterData.prototype`; fall back to
                // `CharacterData.prototype` during the narrow
                // bootstrap window after CharacterData is registered
                // but before `register_text_prototype` runs.
                self.text_prototype
                    .or(self.character_data_prototype)
                    .expect(
                        "create_element_wrapper called before register_character_data_prototype",
                    )
            }
            super::super::host_data::PrototypeKind::OtherCharacterData => self
                .character_data_prototype
                .expect("create_element_wrapper called before register_character_data_prototype"),
            super::super::host_data::PrototypeKind::OtherNode => self
                .node_prototype
                .expect("create_element_wrapper called before register_node_prototype"),
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
