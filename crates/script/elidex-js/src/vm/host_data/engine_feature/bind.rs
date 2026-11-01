//! `HostData` bind lifecycle and bound-pointer access.
//!
//! `bind` / `unbind` / `bind_epoch` / `is_bound`, the raw-pointer
//! accessors (`session` / `dom` / `bound_dom_mut` / `dom_shared` /
//! `with_session_and_dom`), the disjoint-borrow projections
//! (`split_dom_*`) that rest on `bind`'s "disjoint allocations" safety
//! contract, and the bind-scoped document / Window entity accessors.
//! Carved out of `host_data/mod.rs` as a cohesion seam (pure move).

use super::HostData;
use elidex_ecs::Entity;

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
        // Copilot R9: bump bind epoch so retained `StaticRange`
        // wrappers (which captured the prior epoch) invalidate
        // on `isValid()` even if their stored `Entity` bits
        // collide with a new slot in a rebound `EcsDom`.
        self.bind_epoch = self.bind_epoch.wrapping_add(1);
    }

    /// Current bind epoch — incremented on every `Vm::unbind` so
    /// retained `StaticRange` wrappers can detect stale entity
    /// bits after a rebind.  See [`Self::unbind`].
    #[inline]
    pub fn bind_epoch(&self) -> u32 {
        self.bind_epoch
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

    /// The bound DOM as `Some(&mut EcsDom)` while a batch bracket holds
    /// the host pointers, else `None` — the Option-returning sibling of
    /// [`Self::dom`] (which asserts bound).
    ///
    /// This is the single-derivation-chain source for the bound-safe
    /// dispatch path (slot `#11-bound-safe-dispatch-dom-aliasing`): the
    /// shared `script_dispatch_event_core` routes ALL its dom access
    /// through the engine's bound `dom_ptr` via here, never a fresh
    /// `ctx.dom` reborrow, so no `&mut EcsDom` derived off `ctx` can
    /// invalidate the engine's raw `dom_ptr` under Stacked Borrows.
    #[allow(unsafe_code)]
    pub fn bound_dom_mut(&mut self) -> Option<&mut elidex_ecs::EcsDom> {
        // SAFETY: `is_bound()` guarantees `dom_ptr` is non-null and
        // references the `EcsDom` from the most recent `bind()`, kept
        // valid + unaliased by the caller's `bind` contract until
        // `unbind`.  Same aliasing contract as [`Self::dom`].
        self.is_bound().then(|| unsafe { &mut *self.dom_ptr })
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

    /// Borrow `EcsDom` (shared) and `LiveRangeRegistry`
    /// (exclusive) simultaneously via disjoint field projection.
    ///
    /// Mirrors [`Self::split_dom_and_observers`] (line 577) —
    /// VM-side Range accessors that need both `finalize_pending`
    /// AND a `Range` read/mutate in one call cannot get
    /// `&EcsDom` and `&mut LiveRangeRegistry` through separate
    /// `dom_shared` / `live_range_registry_mut` accessors
    /// without conflicting borrows on `&mut self`.
    ///
    /// # Safety
    ///
    /// Same `dom_ptr` aliasing contract as
    /// [`Self::dom_shared`]: callers MUST NOT invoke any
    /// sibling `host()` / `host().dom()` path while either of
    /// the returned references is live.  The `EcsDom`
    /// allocation is disjoint from the `HostData`'s
    /// `live_range_registry` storage by `bind`'s "disjoint
    /// allocations" contract, so the `&EcsDom` and `&mut
    /// LiveRangeRegistry` cannot alias.
    ///
    /// Plan-v4 §A6 Round 1 Arch CRIT-2 — doc-comment carries
    /// the same caller-contract as `split_dom_and_observers`.
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_and_live_ranges(
        &mut self,
    ) -> (&elidex_ecs::EcsDom, &mut elidex_dom_api::LiveRangeRegistry) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: see method-level safety comment.  `dom_ptr`
        // is the bound `&mut EcsDom` supplied by the most
        // recent `bind()`; we only synthesise a shared ref here,
        // and the returned `&mut LiveRangeRegistry` projects a
        // disjoint owned field (live_range_registry lives
        // inside `HostData` itself, not behind `dom_ptr`).
        let dom = unsafe { &*self.dom_ptr };
        (dom, &mut self.live_range_registry)
    }

    /// Three-way borrow split: `&mut EcsDom`, `&mut LiveRangeRegistry`,
    /// and `&mut Option<SelectionState>` — used by
    /// `deleteFromDocument` which delegates to the engine-indep
    /// `SelectionState::delete_from_document` (Copilot R1 IMP-2:
    /// the engine-indep impl owns the Phase 1/2/3 spec algorithm,
    /// so the VM-side just hands all three borrows over).
    ///
    /// **Safety contract** (identical to
    /// [`Self::split_dom_and_live_ranges`]): all three return
    /// values borrow from `&mut self`; they are disjoint because
    /// `dom_ptr` is the bound `&mut EcsDom` exclusively held by
    /// HostData while bound, and the other two are owned fields
    /// of `HostData`.
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_mut_live_ranges_and_selection(
        &mut self,
    ) -> (
        &mut elidex_ecs::EcsDom,
        &mut elidex_dom_api::LiveRangeRegistry,
        &mut Option<elidex_dom_api::SelectionState>,
    ) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: same as `split_dom_and_live_ranges` — `dom_ptr`
        // references the currently-bound `&mut EcsDom` exclusively
        // owned by HostData; we synthesise the `&mut` here.  The
        // other two are owned fields of `HostData`.
        let dom = unsafe { &mut *self.dom_ptr };
        (
            dom,
            &mut self.live_range_registry,
            &mut self.selection_state,
        )
    }

    /// Three-way borrow split: `&EcsDom`, `&mut LiveRangeRegistry`,
    /// and `&mut Option<SelectionState>` — the canonical access
    /// shape for `Selection.prototype` methods that need DOM read,
    /// registry mutate, and selection-singleton mutate concurrently.
    ///
    /// **Safety contract** (identical to
    /// [`Self::split_dom_and_live_ranges`]): callers MUST observe
    /// the "no Range / Selection allocations between split and
    /// drop" rule.  All three return values borrow from `&mut
    /// self`; they are disjoint because `dom_ptr` is the bound
    /// `&mut EcsDom` (synthesised here as a shared ref) and the
    /// other two are owned fields of `HostData` itself.
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_live_ranges_and_selection(
        &mut self,
    ) -> (
        &elidex_ecs::EcsDom,
        &mut elidex_dom_api::LiveRangeRegistry,
        &mut Option<elidex_dom_api::SelectionState>,
    ) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: `dom_ptr` references the currently-bound
        // `&mut EcsDom`; we synthesise a shared ref to it here.
        // The `&mut LiveRangeRegistry` and `&mut
        // Option<SelectionState>` project disjoint owned fields
        // of `HostData` (neither lives behind `dom_ptr`).
        let dom = unsafe { &*self.dom_ptr };
        (
            dom,
            &mut self.live_range_registry,
            &mut self.selection_state,
        )
    }

    /// Borrow the bound DOM (shared) and the
    /// [`elidex_api_observers::mutation::MutationObserverRegistry`]
    /// (exclusive) simultaneously via disjoint field projection.
    ///
    /// Lets [`super::super::super::Vm::deliver_mutation_records`] hand
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

    /// Like [`Self::split_dom_and_observers`] but yields an
    /// exclusive `&mut EcsDom` for observation mutations
    /// (`observe` / `disconnect` insert/remove the per-node
    /// `MutationObservedBy` component on target entities).
    ///
    /// # Safety
    ///
    /// Same `dom_ptr` aliasing contract as [`Self::split_dom_and_observers`]
    /// / [`Self::dom_shared`]: callers must not invoke any sibling
    /// `host()` / `host().dom()` path while either returned reference is
    /// live. The `EcsDom` allocation is disjoint from the `HostData`'s
    /// registry storage by `bind`'s "disjoint allocations" contract, so the
    /// `&mut EcsDom` and `&mut MutationObserverRegistry` cannot alias.
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_mut_and_observers(
        &mut self,
    ) -> (
        &mut elidex_ecs::EcsDom,
        &mut elidex_api_observers::mutation::MutationObserverRegistry,
    ) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: `dom_ptr` is the bound `&mut EcsDom` from the most
        // recent `bind()`; the returned `&mut MutationObserverRegistry`
        // projects a disjoint `HostData` field (the registry lives in
        // `HostData`, not behind `dom_ptr`), so the two `&mut` cannot alias.
        let dom = unsafe { &mut *self.dom_ptr };
        (dom, &mut self.mutation_observers)
    }

    /// Like [`Self::split_dom_mut_and_observers`] but for the
    /// `ResizeObserver` registry (W3C Resize Observer §3) — used by
    /// the VM `resize_observer.rs` host bindings to dispatch
    /// `observe` / `unobserve` / `disconnect` into the engine-indep
    /// registry while it inserts / removes per-target
    /// `ResizeObservedBy` components on the DOM.  Same `dom_ptr`
    /// aliasing contract as [`Self::split_dom_mut_and_observers`].
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_mut_and_resize_observers(
        &mut self,
    ) -> (
        &mut elidex_ecs::EcsDom,
        &mut elidex_api_observers::resize::ResizeObserverRegistry,
    ) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: same `dom_ptr` aliasing contract as
        // `split_dom_mut_and_observers`; `resize_observers` is a
        // disjoint owned field of `HostData`.
        let dom = unsafe { &mut *self.dom_ptr };
        (dom, &mut self.resize_observers)
    }

    /// Like [`Self::split_dom_mut_and_observers`] but for the
    /// `IntersectionObserver` registry (W3C Intersection Observer §3).
    /// Same `dom_ptr` aliasing contract.
    #[allow(unsafe_code)]
    pub(crate) fn split_dom_mut_and_intersection_observers(
        &mut self,
    ) -> (
        &mut elidex_ecs::EcsDom,
        &mut elidex_api_observers::intersection::IntersectionObserverRegistry,
    ) {
        assert!(self.is_bound(), "HostData accessed while unbound");
        // SAFETY: same `dom_ptr` aliasing contract as
        // `split_dom_mut_and_observers`; `intersection_observers` is
        // a disjoint owned field of `HostData`.
        let dom = unsafe { &mut *self.dom_ptr };
        (dom, &mut self.intersection_observers)
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
    /// [`Vm::bind`](super::super::super::Vm::bind) has never run on this `HostData`.
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

    /// Record the Window entity allocated by [`Vm::bind`](super::super::super::Vm::bind).
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
