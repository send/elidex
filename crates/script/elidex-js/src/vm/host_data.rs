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
    /// `VmInner::create_element_wrapper`.  The
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
        /// Created on the first [`Vm::bind`](super::super::Vm::bind) via the
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
        /// Explicitly cleared by `HTMLElement.blur()` via
        /// [`Self::invalidate_focus_if`].  Detached entities may
        /// remain stored here internally; `document.activeElement`
        /// and `document.hasFocus()` filter them out at read time
        /// (connectedness walk via `get_parent`).
        pub(crate) focused_entity: Option<Entity>,
        /// Shared cookie storage owned by the embedding shell
        /// (PR6).  Populated via [`Self::install_cookie_jar`]
        /// (typically reached from the shell as
        /// `vm.host_data().install_cookie_jar(...)`) once at startup
        /// and persists across bind/unbind cycles — browsing
        /// contexts share the same cookie jar across navigations
        /// within a profile.  `None` in tests and standalone
        /// harnesses that have not opted into cookie storage;
        /// `document.cookie` getter returns the empty string and
        /// the setter is a no-op in that case (the
        /// "cookie-averse" path of WHATWG §6.5.2).
        cookie_jar: Option<std::sync::Arc<elidex_net::CookieJar>>,
        /// `MutationObserver` registry (WHATWG DOM §4.3) — owns the
        /// per-observer target list, options, and pending records.
        /// Held here (rather than on `VmInner`) so the registry's
        /// lifetime tracks the bound DOM world: `Vm::unbind` clears
        /// per-observer target lists via
        /// `MutationObserverRegistry::clear_all_targets` to avoid
        /// cross-`EcsDom` Entity aliasing on rebind, while observer
        /// IDs and registrations themselves stay live so a JS
        /// reference held to a `MutationObserver` instance across
        /// the unbind boundary continues to brand-check.
        pub(crate) mutation_observers: elidex_api_observers::mutation::MutationObserverRegistry,
        /// JS callback `ObjectId` per observer ID.  Keyed by the
        /// raw `MutationObserverId` u64 (matches the inline
        /// `ObjectKind::MutationObserver { observer_id }` payload)
        /// and rooted via [`Self::gc_root_object_ids`] so the
        /// callback survives any GC cycle while the observer is
        /// alive.
        ///
        /// **Retained across `Vm::unbind`** — the map is keyed by
        /// VM-monotonic `observer_id`, not by `Entity` or recycled
        /// `ObjectId`, so cross-DOM aliasing does not apply.  A
        /// retained `mo` reference can re-`observe` after a rebind
        /// (same or different DOM) and have its callback fire.
        /// Trade-off: this map (and its sibling
        /// `mutation_observer_instances`) grows monotonically with
        /// the count of `new MutationObserver()` calls and is never
        /// shrunk — `disconnect()` does not remove the entry, and
        /// `Vm::unbind` intentionally retains it.  Long-lived VMs
        /// that churn many observers would accumulate dead entries;
        /// weak-rooting / sweep-time cleanup is tracked at
        /// `#11-mutation-observer-extras`.
        pub(crate) mutation_observer_callbacks: HashMap<u64, ObjectId>,
        /// Reverse lookup from observer ID to the JS instance
        /// `ObjectId`.  Needed at delivery time so the embedder can
        /// pass the same `MutationObserver` JS object back as the
        /// callback's `this` and second argument (WHATWG DOM §4.3.4).
        /// Also rooted via [`Self::gc_root_object_ids`] — without
        /// this root, a user that calls `new
        /// MutationObserver(cb)`-and-immediately-drops would let the
        /// instance be collected before its first delivery; the
        /// registry-side `observer_id` is just `u64`, so the
        /// per-spec "registered observer keeps target alive"
        /// reference cannot pin the JS wrapper.  Same retain-across-
        /// unbind contract as [`Self::mutation_observer_callbacks`].
        pub(crate) mutation_observer_instances: HashMap<u64, ObjectId>,
    }

    /// Returns `true` when two raw pointers share their base
    /// address (`session as *const u8 == dom as *const u8`) — a
    /// minimal sanity check for `bind`'s "disjoint allocations"
    /// safety contract that **does not prove non-overlap**: two
    /// pointers can still alias by referring into the same backing
    /// allocation at different offsets (distinct fields of a
    /// containing struct, transmute-derived aliases, etc.), and
    /// that case slips past this comparison.  The full
    /// no-overlap invariant remains the caller's responsibility.
    /// Pure, side-effect-free, never derefs the pointers — safe to
    /// unit-test directly without invoking `bind`'s unsafe
    /// preconditions.
    fn pointers_alias_at_base<S, D>(session: *const S, dom: *const D) -> bool {
        session.cast::<u8>() == dom.cast::<u8>()
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
                cookie_jar: None,
                mutation_observers: elidex_api_observers::mutation::MutationObserverRegistry::new(),
                mutation_observer_callbacks: HashMap::new(),
                mutation_observer_instances: HashMap::new(),
            }
        }

        /// Install the shell-owned cookie jar.  Idempotent —
        /// installing the same `Arc` twice is fine; replacing an
        /// existing jar swaps the pointer (cookies in the previous
        /// jar are not migrated).  Tests typically call this with a
        /// fresh `CookieJar::new()` after `bind_vm` so
        /// `document.cookie` round-trips can be observed.
        pub fn install_cookie_jar(&mut self, jar: std::sync::Arc<elidex_net::CookieJar>) {
            self.cookie_jar = Some(jar);
        }

        /// Borrow the installed cookie jar, or `None` when the shell
        /// did not call [`Self::install_cookie_jar`] (cookie-averse
        /// fallback, WHATWG §6.5.2).
        pub(crate) fn cookie_jar(&self) -> Option<&std::sync::Arc<elidex_net::CookieJar>> {
            self.cookie_jar.as_ref()
        }

        /// Set the focused Element (called from `HTMLElement.focus()`).
        pub(crate) fn set_focused_entity(&mut self, entity: Entity) {
            self.focused_entity = Some(entity);
        }

        /// Clear focus if the currently-focused entity equals `entity`.
        /// Called from `HTMLElement.blur()`.  `Document.activeElement`
        /// enforces the "live, connected" side of the invariant at
        /// read time by walking `get_parent` back to the document
        /// root, so detached elements fall back to `<body>` without
        /// needing an ECS-level detach hook.
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
        /// - `session` and `dom` MUST point at **disjoint allocations**
        ///   — i.e. no part of the `SessionCore` storage overlaps the
        ///   `EcsDom` storage.  [`Self::with_session_and_dom`] creates
        ///   two simultaneous `&mut` borrows from these raw pointers in
        ///   the same call frame; if the regions ever aliased, that
        ///   would violate Rust's no-overlapping-mutable-borrow rule
        ///   and produce immediate UB.  In practice this is upheld
        ///   because `SessionCore` and `EcsDom` are independent
        ///   stack-or-heap-resident structs (the typical caller
        ///   `pin`s a `&mut SessionCore` and a `&mut EcsDom` from
        ///   distinct local variables and converts each via
        ///   `ptr::from_mut`).
        /// - The caller MUST NOT access `session` or `dom` via any other
        ///   reference (Stacked-Borrows: raw-pointer aliasing with a live
        ///   `&mut` is UB).  Typical usage: caller holds `&mut`, calls
        ///   `bind(ptr_from_mut)`, invokes VM, calls `unbind()`, then
        ///   resumes using the `&mut`.
        ///
        /// # Test-fixture panic safety (regression guard)
        ///
        /// Only `tests_dom_handler_dispatch.rs::with_doc_vm` (added
        /// in `#11-arch-hoist-a`) wraps the bound VM in an
        /// `UnbindOnDrop` guard so a panic inside the closure still
        /// runs `unbind()` before the `session` / `dom` locals
        /// expire.  ~36 sibling test fixtures across `vm/tests/`
        /// instead call `vm.unbind()` only on the success path.
        ///
        /// That gap is **safe today** because no `Drop` impl on
        /// `Vm` / `VmInner` / `HostData` ever derefs `session_ptr`
        /// or `dom_ptr` — the field-drop chain only touches `Copy`
        /// raw pointers and self-contained ECMA state, so dangling
        /// pointers left by a panic are never read.
        ///
        /// **If you add a manual `Drop` impl to any of `Vm`,
        /// `VmInner`, or `HostData`, and the body reaches host
        /// state through `is_bound()` / `session()` / `dom()` /
        /// `with_session_and_dom` / `dom_shared()` (or any future
        /// gated accessor), every test fixture must adopt the
        /// `UnbindOnDrop` pattern before that change can land.**
        /// Plan memo `m4-12-pr-arch-hoist-a-plan.md` §L records this
        /// trigger; the sweep is a ~30-60 min mechanical edit
        /// across the existing fixtures.
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
            // Pointer-disjointness sanity check: the two raw pointers
            // must refer to non-overlapping allocations because
            // `with_session_and_dom` creates a `&mut SessionCore` and
            // a `&mut EcsDom` simultaneously from them.  See
            // [`pointers_alias_at_base`] for what this check covers
            // (equal base addresses) and what it does *not* cover
            // (different offsets into the same allocation — still UB,
            // still the caller's responsibility per the safety
            // contract above).  `debug_assert!` keeps release builds
            // branch-free; in debug builds a misuse fires before any
            // deref happens.
            debug_assert!(
                !pointers_alias_at_base(session, dom),
                "HostData::bind requires session and dom to point at \
                 disjoint allocations (same base address detected)"
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

        /// Returns `true` only when **both** `session_ptr` and
        /// `dom_ptr` are non-null.  The two pointers are set together
        /// in `bind` and cleared together in `unbind`, so an
        /// asymmetric state (one set, the other null) indicates a
        /// partial-bind / partial-unbind bug — never a valid
        /// runtime state.  Requiring both here means callers that
        /// follow a successful `is_bound()` check with a `dom_ptr` /
        /// `session_ptr` deref cannot run into a one-sided null on
        /// the path between the check and the deref.
        #[inline]
        pub fn is_bound(&self) -> bool {
            !self.session_ptr.is_null() && !self.dom_ptr.is_null()
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

        /// Borrow the bound `SessionCore` and `EcsDom` simultaneously.
        ///
        /// Required by the `DomApiHandler::invoke()` dispatch path
        /// (see `vm/host/dom_bridge.rs::invoke_dom_api`) which takes
        /// both `&mut session` and `&mut dom` in the same call —
        /// neither `Self::session` nor `Self::dom` alone suffices,
        /// because each takes `&mut self` and so cannot be live
        /// alongside the other.
        ///
        /// # Safety
        ///
        /// The two `&mut` borrows produced here alias *different*
        /// allocations, by [`Self::bind`]'s third caller-enforced
        /// invariant: `bind` requires `session` and `dom` to point
        /// at disjoint allocations.  Creating both `&mut`s at once
        /// therefore does not violate Rust's
        /// no-overlapping-mutable-borrow rule.  This contract mirrors
        /// the boa-side pattern (`HostBridge::with(|session, dom|
        /// ...)`) which the VM is replacing.
        #[allow(unsafe_code)]
        pub fn with_session_and_dom<F, R>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut elidex_script_session::SessionCore, &mut elidex_ecs::EcsDom) -> R,
        {
            assert!(self.is_bound(), "HostData accessed while unbound");
            // Defence-in-depth in debug/test builds: re-check
            // pointer disjointness here so a same-base-address
            // misuse not caught at `bind` time (e.g. introduced via
            // a direct field assignment that bypasses `bind`'s own
            // assert) still trips before any deref happens.  This
            // and the bind-side `debug_assert!` are both compiled
            // out in release; release-mode enforcement is the
            // documented safety contract on `bind`, not this
            // assert.
            debug_assert!(
                !pointers_alias_at_base(self.session_ptr, self.dom_ptr),
                "HostData::with_session_and_dom: session_ptr and dom_ptr \
                 must point at disjoint allocations (set by bind())"
            );
            // SAFETY: see method-level safety comment.  `session_ptr`
            // and `dom_ptr` point at distinct allocations supplied by
            // the most recent `bind()`; the `debug_assert!` above
            // catches misuse in test/debug builds.
            let session = unsafe { &mut *self.session_ptr };
            let dom = unsafe { &mut *self.dom_ptr };
            f(session, dom)
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
        pub(crate) fn dom_shared(&self) -> &elidex_ecs::EcsDom {
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

        /// Borrow the bound DOM (shared) and the
        /// [`elidex_api_observers::mutation::MutationObserverRegistry`]
        /// (exclusive) simultaneously via disjoint field projection.
        ///
        /// Lets [`super::super::Vm::deliver_mutation_records`] hand
        /// [`elidex_ecs::EcsDom::is_ancestor_or_self`] to the
        /// registry's subtree-ancestry callback while keeping the
        /// `&mut MutationObserverRegistry` borrowed for
        /// [`elidex_api_observers::mutation::MutationObserverRegistry::notify`].
        ///
        /// Without this disjoint projection, the natural form
        /// `let dom = host.dom(); host.mutation_observers.notify(...)`
        /// would conflict — `host.dom()` re-borrows `&mut self` to
        /// hand back a `&mut EcsDom`, and the closure inside `notify`
        /// would alias that `&mut`.
        ///
        /// # Safety
        ///
        /// Same `dom_ptr` aliasing contract as [`Self::dom_shared`] —
        /// callers must not invoke any sibling `host()` / `host().dom()`
        /// path while either of the returned references is live.  The
        /// `EcsDom` allocation is disjoint from the `HostData`'s
        /// registry storage by `bind`'s "disjoint allocations"
        /// contract, so the `&EcsDom` and `&mut MutationObserverRegistry`
        /// cannot alias.
        #[allow(unsafe_code)]
        pub(crate) fn split_dom_and_observers(
            &mut self,
        ) -> (
            &elidex_ecs::EcsDom,
            &mut elidex_api_observers::mutation::MutationObserverRegistry,
        ) {
            assert!(self.is_bound(), "HostData accessed while unbound");
            // SAFETY: see method-level safety comment.  `dom_ptr` is
            // the bound `&mut EcsDom` supplied by the most recent
            // `bind()`; we only synthesise a shared ref here, and
            // the returned `&mut MutationObserverRegistry` projects
            // a disjoint field (the registry lives inside
            // `HostData` itself, not behind `dom_ptr`).
            let dom = unsafe { &*self.dom_ptr };
            (dom, &mut self.mutation_observers)
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
        /// [`Vm::bind`](super::super::Vm::bind) has never run on this `HostData`.
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

        /// Record the Window entity allocated by [`Vm::bind`](super::super::Vm::bind).
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
                .chain(self.mutation_observer_callbacks.values().copied())
                .chain(self.mutation_observer_instances.values().copied())
        }
    }

    impl Default for HostData {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use elidex_ecs::EcsDom;
        use elidex_script_session::SessionCore;

        /// Disjoint allocations — bind() must accept normal usage.
        #[test]
        fn bind_accepts_disjoint_pointers() {
            let mut hd = HostData::new();
            let mut session = SessionCore::new();
            let mut dom = EcsDom::new();
            let doc = dom.create_document_root();
            let session_ptr: *mut SessionCore = &raw mut session;
            let dom_ptr: *mut EcsDom = &raw mut dom;
            assert_ne!(
                session_ptr.cast::<u8>(),
                dom_ptr.cast::<u8>(),
                "test setup invariant: separate locals must have distinct addresses"
            );
            #[allow(unsafe_code)]
            unsafe {
                hd.bind(session_ptr, dom_ptr, doc);
            }
            assert!(hd.is_bound());
            hd.unbind();
        }

        /// `pointers_alias_at_base` flags the same numeric address
        /// even when it's been cast to two different typed pointers
        /// — that's the canary `bind`'s `debug_assert!` relies on
        /// to catch the most common misuse (passing the same
        /// pointer twice).  Pure comparison, never derefs.
        #[test]
        fn pointers_alias_at_base_detects_same_address() {
            // Cast the same numeric address directly to each typed
            // pointer (rather than going through `*const u8 ->
            // .cast::<T>()`, which clippy flags as `cast_ptr_alignment`
            // because a 1-byte-aligned `*const u8` can't safely be
            // upgraded to an 8-byte-aligned `*const SessionCore` /
            // `*const EcsDom` in general).  We only ever compare the
            // numeric addresses — no deref happens, so the alignment
            // is irrelevant for the test, but going `usize -> *const T`
            // directly keeps the lint clean.
            let session = 0x1234_usize as *const SessionCore;
            let dom = 0x1234_usize as *const EcsDom;
            assert!(pointers_alias_at_base(session, dom));
        }

        /// Two distinct stack locals never share a base address
        /// (with the modest assumption the compiler does not pun
        /// them onto the same slot, which it doesn't for these
        /// non-overlapping borrows).  This is the happy-path
        /// counterpart to the alias test above.
        #[test]
        fn pointers_alias_at_base_distinguishes_distinct_addresses() {
            let session = SessionCore::new();
            let dom = EcsDom::new();
            assert!(!pointers_alias_at_base(&raw const session, &raw const dom,));
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
