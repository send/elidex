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
    use elidex_ecs::{Entity, NodeKind};
    use elidex_script_session::ListenerId;
    use std::collections::{HashMap, HashSet};

    /// Four-way partition of DOM wrapper prototype chains used by
    /// [`super::super::VmInner::create_element_wrapper`].  The
    /// [`PrototypeKind`] is derived from the entity's ECS components
    /// so the caller does not need to run independent
    /// `is_element_entity` / `is_character_data_entity` checks.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PrototypeKind {
        /// Entity carries a `TagType` — Element wrapper chain:
        /// `Element.prototype → Node.prototype → …`.
        Element,
        /// `NodeKind::Text` — chains via `Text.prototype →
        /// CharacterData.prototype → Node.prototype → …`.
        Text,
        /// Other CharacterData (currently `Comment`; future
        /// `ProcessingInstruction` / `CdataSection`) — chains via
        /// `CharacterData.prototype → Node.prototype → …`.
        OtherCharacterData,
        /// `NodeKind::DocumentType` — chains via
        /// `DocumentType.prototype → Node.prototype → …`.  Carries
        /// `name` / `publicId` / `systemId`.
        DocumentType,
        /// Document, DocumentFragment, ShadowRoot, or anything
        /// without a recognised `NodeKind`.  Chains directly to
        /// `Node.prototype`.
        OtherNode,
    }

    pub struct HostData {
        session_ptr: *mut elidex_script_session::SessionCore,
        dom_ptr: *mut elidex_ecs::EcsDom,
        document_entity: Option<Entity>,
        /// Entity backing `globalThis` / `window` (WHATWG HTML §7.2).
        ///
        /// Created on the first [`Vm::bind`](super::Vm::bind) via the
        /// bound `dom` and **retained across unbind cycles** — identity
        /// is stable for the whole lifetime of the `HostData`.
        ///
        /// # Single-world invariant
        ///
        /// This entity, along with `wrapper_cache`,
        /// `document_methods_installed`, and `listener_store`, are
        /// meaningful only within the `EcsDom` world that allocated
        /// them.  **Callers must not rebind a `HostData` to a
        /// different `EcsDom` world.**  Doing so would thread stale
        /// `Entity` bits into `globalThis` and the wrapper cache.
        ///
        /// We do not enforce this with a pointer assert because
        /// `EcsDom` is `!Pin` — the same world can legally move in
        /// memory between unbind → bind cycles (e.g. `Vec` grow,
        /// `mem::swap`), which would cause a false-positive panic.
        /// A stable `EcsDom::world_id()` will be introduced when
        /// Worker threads (PR5b) require per-world isolation; until
        /// then the invariant is a caller contract.
        window_entity: Option<Entity>,
        /// Document entities whose wrapper has already had the
        /// document-specific own-property suite (`getElementById` /
        /// `createElement` / `body` accessor / ...) installed.
        ///
        /// Tracked **per-entity** because a single `Vm` can be bound
        /// to multiple document entities over its lifetime (shell
        /// navigation: unbind doc1 → bind doc2 produces a fresh
        /// wrapper via `wrapper_cache`, and that wrapper needs its
        /// own method install).  A single VM-wide boolean would skip
        /// the install on every document after the first — see
        /// `vm/host/document.rs::install_document_methods_if_needed`.
        ///
        /// Bounded by the number of distinct documents a VM observes
        /// (typically 1 — at most a handful).
        pub(crate) document_methods_installed: HashSet<Entity>,
        pub(crate) listener_store: HashMap<ListenerId, ObjectId>,
        pub(crate) wrapper_cache: HashMap<u64, ObjectId>,
        /// Currently focused Element entity (WHATWG HTML §6.6.3).
        ///
        /// `None` when no Element is focused; `document.activeElement`
        /// falls back to the `<body>` element in that case.  Phase 2
        /// simplification: we track focus as a single Option (spec
        /// models a focus chain, but single-frame VM covers the
        /// primary use cases).
        ///
        /// Cleared automatically when the focused Entity is detached
        /// from the document — see [`Self::invalidate_focus_if`].
        pub(crate) focused_entity: Option<Entity>,
    }

    impl HostData {
        pub fn new() -> Self {
            Self {
                session_ptr: std::ptr::null_mut(),
                dom_ptr: std::ptr::null_mut(),
                document_entity: None,
                window_entity: None,
                document_methods_installed: HashSet::new(),
                listener_store: HashMap::new(),
                wrapper_cache: HashMap::new(),
                focused_entity: None,
            }
        }

        /// Set the focused Element (called from `HTMLElement.focus()`).
        pub(crate) fn set_focused_entity(&mut self, entity: Entity) {
            self.focused_entity = Some(entity);
        }

        /// Clear focus if the currently-focused entity equals `entity`.
        /// Called from `HTMLElement.blur()` and from the ECS detach
        /// hook to maintain the invariant that `focused_entity` always
        /// points to a live, connected Element.
        pub(crate) fn invalidate_focus_if(&mut self, entity: Entity) {
            if self.focused_entity == Some(entity) {
                self.focused_entity = None;
            }
        }

        /// Return the currently focused Element, if any.
        pub(crate) fn focused_entity(&self) -> Option<Entity> {
            self.focused_entity
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

        /// Shared-reference view of the bound DOM — used from
        /// property-lookup paths that hold a `&HostData` (through
        /// `&VmInner.host_data`) and cannot upgrade to `&mut`.
        ///
        /// # Safety
        ///
        /// Only safe to call while bound (asserted).  The returned
        /// reference aliases whatever exclusive borrow the caller
        /// of [`bind`] promised to keep quiescent — see `bind`'s
        /// safety contract.  Callers must not hold this `&EcsDom`
        /// alongside any `&mut EcsDom` produced by [`Self::dom`].
        #[allow(unsafe_code)]
        pub fn dom_shared(&self) -> &elidex_ecs::EcsDom {
            assert!(self.is_bound(), "HostData accessed while unbound");
            unsafe { &*self.dom_ptr }
        }

        /// Return `true` if this `HostData` is bound AND `entity` is
        /// an Element (has the `TagType` component) in the bound
        /// world.  Returns `false` when unbound so pre-bind or
        /// post-unbind wrapper allocation can still decide a
        /// reasonable fallback prototype (`Node.prototype`).
        ///
        /// Takes `&self` (not `&mut self`) because the lookup is
        /// read-only — callers holding a shared borrow of `HostData`
        /// (e.g. `create_element_wrapper`'s prototype branch) cannot
        /// otherwise reach the world.
        ///
        /// # Aliasing contract
        ///
        /// Callers must not hold a live `&mut EcsDom` produced via
        /// [`Self::dom`] when invoking this method.  Creating the
        /// shared reference while a mutable reference to the same
        /// `EcsDom` exists would be undefined behaviour under the
        /// Rust aliasing rules.  In practice this is guaranteed by
        /// `create_element_wrapper`'s borrow discipline — it calls
        /// `is_element_entity` through `self.host_data.as_deref()`,
        /// so no `&mut` borrow of the DOM can be live at that call
        /// site.  Future callers must preserve the same ordering.
        #[allow(unsafe_code)]
        pub fn is_element_entity(&self, entity: Entity) -> bool {
            if !self.is_bound() {
                return false;
            }
            // SAFETY: `is_bound` implies `dom_ptr` is non-null and
            // points at the `EcsDom` supplied by the most recent
            // `bind()`.  The pointer lifetime is tied to that bind
            // window; callers must not drop or move the `EcsDom`
            // between bind and unbind (documented on `bind` itself).
            // The aliasing contract above guarantees no `&mut`
            // reference to the same `EcsDom` is live at the call
            // site, so creating a `&` here cannot violate Rust
            // aliasing rules.
            let dom = unsafe { &*self.dom_ptr };
            dom.world().get::<&elidex_ecs::TagType>(entity).is_ok()
        }

        /// ASCII-case-insensitive tag-name match — used by
        /// `create_element_wrapper`'s per-tag prototype dispatch
        /// (e.g. `<iframe>` → `HTMLIFrameElement.prototype`).
        ///
        /// Aliasing contract mirrors [`Self::is_element_entity`]: no
        /// `&mut EcsDom` must be live at the call site.
        #[allow(unsafe_code)]
        pub fn tag_matches_ascii_case(&self, entity: Entity, tag: &str) -> bool {
            if !self.is_bound() {
                return false;
            }
            // SAFETY: see `is_element_entity`.
            let dom = unsafe { &*self.dom_ptr };
            dom.world()
                .get::<&elidex_ecs::TagType>(entity)
                .is_ok_and(|t| t.0.eq_ignore_ascii_case(tag))
        }

        /// Classify `entity` into a [`PrototypeKind`] used by
        /// `create_element_wrapper` to pick the appropriate wrapper
        /// prototype in a single ECS lookup.  Returns
        /// [`PrototypeKind::OtherNode`] when the `HostData` is not
        /// bound (pre-bind wrapper allocation paths).
        ///
        /// Aliasing contract mirrors [`Self::is_element_entity`] — the
        /// caller must not hold a live `&mut EcsDom` produced via
        /// [`Self::dom`] at the call site.
        #[allow(unsafe_code)]
        pub fn prototype_kind_for(&self, entity: Entity) -> PrototypeKind {
            if !self.is_bound() {
                return PrototypeKind::OtherNode;
            }
            // SAFETY: see `is_element_entity` — same contract.
            let dom = unsafe { &*self.dom_ptr };
            // Element has highest priority (matches the pre-PR4e
            // behaviour of `is_element_entity` short-circuit).
            if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                return PrototypeKind::Element;
            }
            match dom.node_kind(entity) {
                Some(NodeKind::Text) => PrototypeKind::Text,
                Some(NodeKind::Comment) => PrototypeKind::OtherCharacterData,
                // CDATA / ProcessingInstruction would also be
                // CharacterData per WHATWG §4.10; they are not
                // created by the current parser but are listed here
                // so future support is a one-line add.
                Some(NodeKind::CdataSection | NodeKind::ProcessingInstruction) => {
                    PrototypeKind::OtherCharacterData
                }
                Some(NodeKind::DocumentType) => PrototypeKind::DocumentType,
                None => {
                    // Defensive inference for legacy entities that
                    // carry CharacterData payload without an explicit
                    // `NodeKind` component.  Mirrors the same
                    // fallback in `EcsDom::clone_node_shallow`.
                    if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                        PrototypeKind::Element
                    } else if dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok() {
                        PrototypeKind::Text
                    } else if dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok() {
                        PrototypeKind::OtherCharacterData
                    } else if dom.world().get::<&elidex_ecs::DocTypeData>(entity).is_ok() {
                        PrototypeKind::DocumentType
                    } else {
                        PrototypeKind::OtherNode
                    }
                }
                _ => PrototypeKind::OtherNode,
            }
        }

        pub fn document(&self) -> Entity {
            assert!(self.is_bound(), "HostData accessed while unbound");
            self.document_entity.unwrap()
        }

        /// Same as [`Self::document`] but returns `None` when not
        /// bound instead of panicking.  Used from document-global
        /// natives that must silent-no-op when JS code calls a
        /// retained `document` reference across an unbind boundary.
        pub fn document_entity_opt(&self) -> Option<Entity> {
            if self.is_bound() {
                self.document_entity
            } else {
                None
            }
        }

        /// Return the cached Window entity, or `None` if
        /// [`Vm::bind`](super::Vm::bind) has never run on this `HostData`.
        ///
        /// Unlike [`Self::document`], this **does not** require the
        /// `HostData` to be currently bound — the Window entity is
        /// VM-owned (allocated by `Vm::bind` through
        /// `dom().create_window_root()`) and remains valid for the
        /// lifetime of the `HostData`, which is bound to a single
        /// `EcsDom` world by caller contract (see the single-world
        /// invariant documented on [`Self::window_entity`] field).
        pub fn window_entity(&self) -> Option<Entity> {
            self.window_entity
        }

        /// Record the Window entity allocated by [`Vm::bind`](super::Vm::bind).
        ///
        /// # Panics
        ///
        /// Panics if a Window entity is already stored — calling twice
        /// would silently orphan the prior entity (losing its
        /// `EventListeners` component) and is indicative of a missing
        /// lifecycle guard in `Vm::bind`.
        pub fn set_window_entity(&mut self, entity: Entity) {
            assert!(
                self.window_entity.is_none(),
                "HostData::set_window_entity called twice (already stored)"
            );
            self.window_entity = Some(entity);
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
#[cfg(feature = "engine")]
pub use engine_feature::PrototypeKind;
