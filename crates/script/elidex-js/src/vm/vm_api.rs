//! Public `Vm` API — thin wrappers that delegate to `VmInner`.
//!
//! Split out of `mod.rs` to keep that file under the project's
//! 1000-line convention.  All business logic lives in `VmInner`; this
//! file owns nothing but delegation.

use crate::bytecode::compiled::CompiledFunction;

use super::value::{self, FuncId, JsValue, Object, ObjectId, StringId, UpvalueId, VmError};
use super::{host_data, Vm};

impl Vm {
    // -- Public API: all delegate to VmInner --------------------------------

    /// Parse, compile, and execute JavaScript source code.
    pub fn eval(&mut self, source: &str) -> Result<JsValue, VmError> {
        self.inner.eval(source)
    }

    /// Load and execute a compiled script.
    pub fn run_script(
        &mut self,
        script: crate::bytecode::compiled::CompiledScript,
    ) -> Result<JsValue, VmError> {
        self.inner.run_script(script)
    }

    /// Call a JS function object with the given `this` and arguments.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.inner.call(func_obj_id, this, args)
    }

    /// Push `value` onto the VM stack as a temporary GC root and
    /// return an RAII guard that restores the stack on drop.
    ///
    /// Thin wrapper over [`VmInner::push_temp_root`] — see that for
    /// the rooting contract (RAII Drop + length/slot-identity
    /// asserts + panic-safe).
    ///
    /// Use this when an allocation has just produced a `JsValue` not
    /// yet reachable from any other root (a freshly created event
    /// object, a one-shot intermediate before being installed into a
    /// property, etc.) and you need it to survive a GC cycle
    /// triggered by user JS that runs while the guard is alive.
    ///
    /// ```rust,ignore
    /// let mut g = vm.push_temp_root(JsValue::Object(id));
    /// let _ = g.call(func_id, this, &[arg]);
    /// // g drops here; stack restored to pre-push length
    /// ```
    #[cfg(feature = "engine")]
    pub(crate) fn push_temp_root(&mut self, value: JsValue) -> super::VmTempRoot<'_> {
        self.inner.push_temp_root(value)
    }

    /// Intern a string, returning its `StringId`.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.inner.strings.intern(s)
    }

    /// Look up an interned string by its ID, returning WTF-16 code units.
    #[inline]
    pub fn get_string_u16(&self, id: StringId) -> &[u16] {
        self.inner.strings.get(id)
    }

    /// Look up an interned string by its ID, returning a UTF-8 `String`.
    #[inline]
    pub fn get_string(&self, id: StringId) -> String {
        self.inner.strings.get_utf8(id)
    }

    /// Allocate an object, returning its `ObjectId`.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.inner.alloc_object(obj)
    }

    /// Get a reference to an object.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.inner.get_object(id)
    }

    /// Get a mutable reference to an object.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.inner.get_object_mut(id)
    }

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        self.inner.register_function(func)
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        self.inner.get_compiled(id)
    }

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        self.inner.alloc_upvalue(uv)
    }

    /// Captured console output as `(level, message)` pairs in emission
    /// order — the retrievable oracle over the bounded per-VM buffer the
    /// console print natives tee into (the tee mirrors WHATWG Console §2.3 Printer; the S5-6 B26
    /// test-oracle accessor, replacing the boa runtime's
    /// `ConsoleOutput::messages()`). Marshal-scale surface: read by
    /// embedder tests, not by page script.
    #[must_use]
    pub fn console_messages(&self) -> Vec<(String, String)> {
        // The buffer stores the level as the natives' `&'static str` literal;
        // owned pairs are built only here, at the test-oracle read.
        self.inner
            .console_capture
            .iter()
            .map(|(level, message)| ((*level).to_string(), message.clone()))
            .collect()
    }

    /// Install a `HostData` instance for browser shell integration.
    /// Call once, typically at `ElidexJsEngine` construction.
    ///
    /// # Panics
    ///
    /// Panics if a `HostData` is already installed, to prevent accidentally
    /// dropping caches (listener_store, wrapper_cache) from a prior bind.
    pub fn install_host_data(&mut self, hd: host_data::HostData) {
        assert!(
            self.inner.host_data.is_none(),
            "HostData already installed; use host_data() to access or a fresh Vm to reinstall"
        );
        self.inner.host_data = Some(Box::new(hd));
    }

    /// Access the host data (if installed).
    pub fn host_data(&mut self) -> Option<&mut host_data::HostData> {
        self.inner.host_data.as_deref_mut()
    }

    /// The bound DOM (`Some` only while a batch bracket holds the host
    /// pointers), reconstructed from the installed `HostData`'s `dom_ptr`.
    ///
    /// The engine exposes this via [`ScriptEngine::bound_dom_mut`] so the
    /// shared event-dispatch loop resolves its dom through the single
    /// `dom_ptr` derivation chain (slot `#11-bound-safe-dispatch-dom-aliasing`)
    /// — see [`host_data::HostData::bound_dom_mut`].
    ///
    /// [`ScriptEngine::bound_dom_mut`]: elidex_script_session::ScriptEngine::bound_dom_mut
    #[cfg(feature = "engine")]
    pub fn bound_dom_mut(&mut self) -> Option<&mut elidex_ecs::EcsDom> {
        self.inner
            .host_data
            .as_deref_mut()
            .and_then(host_data::HostData::bound_dom_mut)
    }

    /// Set the URL surfaced by `document.referrer` (WHATWG HTML §3.1.4).
    /// Pass `None` to clear the slot back to the empty-string default.
    /// The shell calls this once before each post-navigation `bind`
    /// cycle when a previous Document URL is known and the referrer
    /// policy permits its disclosure to script.
    ///
    /// The argument is sanitised before storage — fragment + userinfo
    /// are stripped to match the WHATWG Fetch §3.2.5 referrer
    /// serialisation rules (Referer header and `document.referrer`
    /// share the same exposure surface).  Callers therefore do not
    /// need to pre-strip the URL themselves.
    #[cfg(feature = "engine")]
    pub fn set_navigation_referrer(&mut self, referrer: Option<url::Url>) {
        self.inner.navigation.referrer = referrer.map(|mut url| {
            url.set_fragment(None);
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url
        });
    }

    /// Bind host pointers for a JS execution call.  No-op if `HostData` is absent.
    ///
    /// # Safety
    ///
    /// See [`host_data::HostData::bind`]: pointers must remain valid (and not
    /// be aliased via any Rust reference) until `unbind()` is called.
    #[cfg(feature = "engine")]
    #[allow(unsafe_code)]
    pub unsafe fn bind(
        &mut self,
        session: *mut elidex_script_session::SessionCore,
        dom: *mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        // Snapshot `global_object` once up front (Copy) so the
        // inner `hd` scope can use it without a re-borrow of
        // `self.inner` that would conflict with the active
        // `host_data` borrow.
        let global_id = self.inner.global_object;

        // Scope the `HostData` borrow to the bind + window-entity
        // resolution + wrapper-cache population.  All three
        // operations live under the same `hd` binding so the
        // cache-populate call does not need a second
        // `as_deref_mut` re-borrow (which would rely on NLL to
        // end the first borrow — fragile across future edits).
        let window_entity = {
            let Some(hd) = self.inner.host_data.as_deref_mut() else {
                return;
            };
            unsafe { hd.bind(session, dom, document) };
            // Resolve the Window ECS entity backing `globalThis`.
            // First bind allocates via `dom().create_window_root()`;
            // subsequent binds reuse the stored entity so identity
            // (and the entity's `EventListeners` component)
            // survives across bind → unbind → bind cycles — see
            // `HostData::window_entity`.
            let we = if let Some(e) = hd.window_entity() {
                e
            } else {
                let e = hd.dom().create_window_root();
                hd.set_window_entity(e);
                e
            };
            // Cache (window_entity → global_object) in
            // wrapper_cache so any later
            // `create_element_wrapper(window_entity)` call returns
            // the canonical Window wrapper instead of allocating a
            // fresh `HostObject` via the `OtherNode` prototype
            // path.  Without this, `dispatch_script_event` at a
            // Window target (e.g. `window.postMessage` /
            // `window.dispatchEvent`) seeds `event.target` with a
            // distinct wrapper and breaks `event.target ===
            // window`.  Idempotent across bind→unbind→bind
            // cycles: the first bind populates, subsequent binds
            // skip via the pre-check.
            if hd.get_cached_wrapper(we).is_none() {
                hd.cache_wrapper(we, global_id);
            }
            // D-8 PR-A2 — install `MutationBridge` on `EcsDom` so
            // engine-side mutations adjust live Range boundaries
            // and NodeIterator pre-removing-steps synchronously.
            //
            // Install order (plan-v4 §A-NI-1):
            // 1. Take the existing (initial) `LiveRangeRegistry`
            //    out of HostData via `mem::take` (it was created
            //    empty in `HostData::new` and is being replaced by
            //    the bridge-paired registry).
            // 2. Pair a fresh `LiveRangeRegistry` with
            //    `MutationBridge` using HostData's shared
            //    `node_iterator_states_shared` `Arc<Mutex<>>` so
            //    bridge's hook-fire can access the iterator map.
            // 3. Install the bridge on `EcsDom` via
            //    `set_mutation_hook`.  Round 1 Arch CRIT-1: the
            //    displaced previous hook MUST be `None` (single-
            //    hook constraint per `#11-mutation-hook-multiplexer`
            //    defer slot).
            let iter_shared = hd.node_iterator_states_shared.clone();
            // Copilot R6: preserve the monotonic `next_id` across
            // bind cycles.  Retained `Range` JS wrappers carry their
            // old `range_id` in `ObjectKind::Range`; resetting the
            // counter to 0 would collide with their stale IDs on
            // the next `register` call.  `unregister`-then-recycle
            // is explicitly forbidden by [`RangeId`] doc.
            let prev_next_id = hd.live_range_registry.next_id_marker();
            let (mut registry, live_range) = elidex_dom_api::LiveRangeRegistry::new_pair();
            registry.restore_next_id_marker(prev_next_id);
            hd.live_range_registry = registry;
            let node_iter = elidex_dom_api::NodeIteratorAdjuster::new(iter_shared);
            // D-17 `#11-custom-elements-vm`: clone the HostData-owned
            // `Arc<Mutex<>>` handles for the CE registry + reaction
            // queue so the consumer's `handle` writes land in the same
            // state that the `customElements.*` natives read.
            let custom_elements = elidex_custom_elements::CustomElementReactionConsumer::new(
                std::sync::Arc::clone(&hd.ce_registry),
                std::sync::Arc::clone(&hd.ce_reaction_queue),
            );
            // D-31: typed `ConsumerDispatcher` replaces the v4-era
            // `MutationBridge` 2-consumer composer (the
            // `#11-mutation-hook-multiplexer` slot is closed by this
            // structural shift).  Dispatch order = field declaration
            // order — see `consumer_dispatcher.rs` for the
            // authoritative 7-field list (live_range → node_iter →
            // base_url → form_control → event_handler_attrs → canvas
            // → custom_elements).
            let mut dispatcher = crate::vm::consumer_dispatcher::ConsumerDispatcher::new(
                live_range,
                node_iter,
                custom_elements,
                self.inner.spec_level_policy,
            );
            // D-31 init pass: pre-bind tree state (e.g. parser-
            // created `<base href>`) never went through
            // `MutationEvent::Insert`, so the `BaseUrlMaintainer`
            // consumer never attached `BaseFrozenUrl` to those
            // entities and `DocumentBaseUrl` is stuck at
            // `about_blank`.  Walk the existing tree once BEFORE
            // installing the dispatcher so post-bind reads of
            // `document.baseURI` / `Node.baseURI` and relative URL
            // resolution see the real `<base href>` immediately
            // (rather than waiting for the next mutation), and so
            // removing a pre-bind `<base>` triggers recompute as
            // intended.  Other consumers (`live_range`, `node_iter`)
            // do not derive ECS state from pre-bind tree structure
            // and are no-ops here; see
            // `ConsumerDispatcher::initialize_consumers`.
            dispatcher.initialize_consumers(hd.dom());
            let displaced = hd.dom().set_mutation_dispatcher(Box::new(dispatcher));
            debug_assert!(
                displaced.is_none(),
                "Vm::bind: EcsDom already had a MutationDispatcher installed — \
                 bind/unbind paired-teardown invariant violated"
            );
            we
            // `hd` drops here so the subsequent
            // `self.inner.get_object_mut` does not conflict.
        };

        // Thread the Window entity through to the `globalThis`
        // `HostObject`.  `entity_from_this` reads `entity_bits`
        // and passes it to `Entity::from_bits` — non-zero values
        // reconstruct the Window entity so
        // `window.addEventListener(...)` records the listener
        // against the correct ECS target (distinct from document).
        //
        // Skip the write on rebinds when `entity_bits` already
        // equals the target — saves a (very cheap) store but
        // also keeps the object's storage cache-line clean for
        // the common rebind path.
        let target_bits = window_entity.to_bits().get();
        if let super::value::ObjectKind::HostObject {
            ref mut entity_bits,
        } = self.inner.get_object_mut(global_id).kind
        {
            if *entity_bits != target_bits {
                *entity_bits = target_bits;
            }
        }
        // Refresh the `document` global so JS code (and listener
        // bodies) sees the just-bound document entity.  Wrapper
        // identity is preserved across bind/unbind cycles via
        // `HostData::wrapper_cache` — repeated binds with the
        // same document entity return the same ObjectId.
        self.install_document_global();

        // S5-2 (Codex R6-D): seed the VisualViewport producer's diff prior to the
        // load-time viewport on the FIRST bind — BEFORE any resize turn mutates
        // `ViewportState`. Anchoring the baseline at lazy wrapper allocation
        // instead would let a `window.resize` handler that defers the first
        // `visualViewport` read until after the new size is pushed capture the
        // post-resize size and self-cancel the producer diff. Seeded once (the
        // prior survives unbind, the BATCH-BIND model); the per-turn producer
        // advances it thereafter.
        self.inner.seed_visual_viewport_baseline_if_unseeded();
    }

    /// Bind a dedicated-worker VM against its (empty) `EcsDom` + worker-scope
    /// entity (WHATWG HTML §10.2.1.1).
    ///
    /// Unlike [`Vm::bind`] there is no `document` global and no
    /// `ConsumerDispatcher` — a worker has no DOM tree to expose or mutate.
    /// `globalThis`'s `entity_bits` are pointed at the `NodeKind::Worker`
    /// entity so `self.addEventListener(...)` / `self.onmessage = fn` record
    /// against it and `dispatch_worker_message` fires there. The `document`
    /// entity is required only to satisfy `HostData::bind` (the worker creates
    /// an empty document root) and is never surfaced to script.
    ///
    /// # Safety
    ///
    /// As [`Vm::bind`]: the `session` / `dom` pointers must stay valid and
    /// unaliased while the VM is bound. The worker VM is dropped (with its
    /// `HostData`) before its owning `dom` / `session` at thread teardown and
    /// is never `unbind`-ed (it installs no dispatcher to tear down).
    #[cfg(feature = "engine")]
    #[allow(unsafe_code)]
    pub unsafe fn bind_worker(
        &mut self,
        session: *mut elidex_script_session::SessionCore,
        dom: *mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        let global_id = self.inner.global_object;
        let scope_entity = {
            let Some(hd) = self.inner.host_data.as_deref_mut() else {
                return;
            };
            unsafe { hd.bind(session, dom, document) };
            let we = if let Some(e) = hd.dom().worker_scope_entity() {
                e
            } else {
                hd.dom().create_worker_global_scope_root()
            };
            if hd.get_cached_wrapper(we).is_none() {
                hd.cache_wrapper(we, global_id);
            }
            we
        };
        let target_bits = scope_entity.to_bits().get();
        if let super::value::ObjectKind::HostObject {
            ref mut entity_bits,
        } = self.inner.get_object_mut(global_id).kind
        {
            *entity_bits = target_bits;
        }
    }

    /// Resolve an ECS `Entity` to its shared JS wrapper `ObjectId`,
    /// allocating on the first lookup and reusing the cached wrapper
    /// on every subsequent call.  See `vm/host/elements.rs` module
    /// doc for the identity contract.
    ///
    /// **Bench-only hook.**  The returned `ObjectId` can only be kept
    /// GC-alive by rooting machinery that is not yet public
    /// (`push_temp_root` / `HostData::wrapper_cache`), so this is not
    /// safe for external callers to persist across allocations.  Kept
    /// `pub` + `#[doc(hidden)]` so `benches/event_dispatch.rs` can
    /// construct test fixtures without reaching into `VmInner`.
    /// Do not rely on this for anything beyond bench scaffolding.
    #[cfg(feature = "engine")]
    #[doc(hidden)]
    pub fn create_element_wrapper(&mut self, entity: elidex_ecs::Entity) -> ObjectId {
        self.inner.create_element_wrapper(entity)
    }

    /// Build a JS event object for a single listener invocation.
    ///
    /// **Bench-only hook** (same reasoning as
    /// [`Vm::create_element_wrapper`]).  Thin wrapper over
    /// `vm/host/events.rs::create_event_object`; the caller must
    /// supply pre-resolved target/currentTarget `HostObject` wrappers.
    /// Returned `ObjectId` is unrooted — not safe to persist across
    /// subsequent allocations from external code.
    #[cfg(feature = "engine")]
    #[doc(hidden)]
    pub fn create_event_object(
        &mut self,
        event: &elidex_script_session::event_dispatch::DispatchEvent,
        target: ObjectId,
        current_target: ObjectId,
        passive: bool,
    ) -> ObjectId {
        self.inner
            .create_event_object(event, target, current_target, passive)
    }

    /// Clear host pointers after JS execution.  No-op if unbound.
    #[allow(clippy::too_many_lines)] // bookkeeping over many side tables — splitting would just add forwarding noise
    pub fn unbind(&mut self) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            // D-8 PR-A2 — clear the `MutationBridge` from `EcsDom`
            // BEFORE HostData::unbind (which null-zeros `dom_ptr`).
            // Order: bridge drop releases its Arc<Mutex<>> halves
            // (ranges + node_iterators) so HostData becomes sole
            // owner; subsequent `live_range_registry.clear()` +
            // `node_iterator_states_shared.lock().clear()` then
            // run on uniquely-owned state.
            //
            // Plan-v4 §A-NI-1 Round 1 IMP-1: post-clear invariant
            // is `Arc::strong_count(&node_iterator_states_shared)
            // == 1` (HostData's clone is the sole owner).
            // Skip cleanup if HostData was never bound (e.g. test
            // path that constructed but never bound the VM).
            if hd.is_bound() {
                // `clear_mutation_hook` returns the displaced hook
                // (currently `()`); we don't read the result — Drop
                // on the boxed bridge handles cleanup.
                hd.dom().clear_mutation_dispatcher();
                // Scrub transient MutationObserver registrations (DOM §4.2.3
                // step-15 entries) from the still-bound world. They are
                // delivery-cycle-ephemeral (the notify microtask, §4.3 step 6.3,
                // normally clears them), so dropping the cycle at unbind must
                // clear them too — otherwise a same-DOM rebind could deliver a
                // future detached-subtree mutation through a stale transient. The
                // `dom_ptr` is zeroed by `hd.unbind()` below, so this must run
                // here while bound. Permanent registrations are left as-is (they
                // despawn with the outgoing world, or persist for a same-DOM
                // rebind). (Codex PR413 R3.)
                elidex_api_observers::mutation::clear_all_transient_observers(hd.dom());
                hd.live_range_registry.clear();
                hd.node_iterator_states_shared
                    .lock()
                    .expect("NodeIterator state mutex poisoned")
                    .clear();
                hd.tree_walker_states.clear();
                hd.range_instances.clear();
                hd.tree_walker_instances.clear();
                hd.node_iterator_instances.clear();
                // Selection singleton state + cached wrapper + pending
                // dispatch flag are all bound to the per-DOM session
                // lifetime; reset on unbind so the next bind cycle
                // starts from a clean Selection.
                hd.selection_state = None;
                hd.selection_instance = None;
                hd.selectionchange_pending = false;
                debug_assert_eq!(
                    std::sync::Arc::strong_count(&hd.node_iterator_states_shared),
                    1,
                    "Vm::unbind: lingering Bridge-side Arc reference after clear_mutation_hook"
                );
            }
            hd.unbind();
        }
        // Reset the `globalThis` `HostObject`'s `entity_bits` to the
        // sentinel `0` so that post-unbind **`entity_from_this`
        // consumers** — `addEventListener` / `removeEventListener` /
        // `dispatchEvent` on `window`, and any future method that
        // resolves its Window entity from `this` — fall into the
        // `None` silent no-op path instead of dereferencing
        // `host_data.dom()` on a stale pointer (which would panic).
        //
        // Window-specific methods that do **not** consult
        // `entity_bits` (viewport getters `innerWidth` / `scrollX` /
        // …; scroll mutators `scrollTo` / `scrollBy`) still run after
        // unbind because they only read/write
        // `VmInner::viewport` — a purely VM-side struct with no
        // bound-state dependency.  That is intentional: scripts that
        // cache viewport values across a rebind should observe
        // continuous state, and none of these methods can
        // dereference a null pointer.
        //
        // `HostData::window_entity` itself is retained so the next
        // `bind` restores identity.
        #[cfg(feature = "engine")]
        {
            let global_id = self.inner.global_object;
            if let super::value::ObjectKind::HostObject {
                ref mut entity_bits,
            } = self.inner.get_object_mut(global_id).kind
            {
                *entity_bits = 0;
            }
            // Drop live-collection state so retained wrappers cannot
            // surface entries from the previous DOM after a rebind to
            // a different `EcsDom`. Two `EcsDom::new()` worlds produce
            // overlapping internal entity indices, so a stored
            // `Entity` from doc1 silently aliases a real entity in
            // doc2 and the cached filter would walk doc2's tree.
            // Clearing here keeps the post-unbind contract observable:
            // `_coll.length` reads `0`, `_coll.item(i)` reads `null`,
            // identical to the JS-still-bound-but-empty-tree case.
            self.inner.live_collection_states.clear();
            // `#11-wrapper-identity-seam` — clear every NON-`Node`
            // interned wrapper from the unified store, keeping the
            // primary `Node` wrapper.  The Entity-keyed and
            // ObjectId-keyed secondaries (classList / dataset / Attr /
            // inline+CSSOM style / the `[SameObject]` collections /
            // `<input>.files` FileList / DataTransferItem / …) all face
            // the cross-DOM Entity-index aliasing risk (lesson #195):
            // two `EcsDom::new()` worlds share entity-index space, so a
            // retained `el2.classList` after a rebind could surface the
            // previous DOM's cached wrapper.  Clearing them keeps
            // post-rebind lookups allocate-fresh.
            //
            // The primary `Node` wrapper is INTENTIONALLY retained —
            // node-wrapper identity (notably Window → `global_object`,
            // see the `entity_bits = 0` reset above) must persist across
            // bind→unbind→bind; the `bind_epoch` mechanism invalidates
            // stale retained node wrappers instead of dropping them.
            //
            // The Scope-owned `ServiceWorkerRegistration` / `ServiceWorker`
            // wrappers are ALSO retained (`#11-per-batch-unbind-document-
            // lifetime-state`; Codex #459 R2): they are document-lifetime
            // (their `sw_registrations` backing state + brand maps survive
            // per-turn), so a page's retained registration must stay
            // `reg === getRegistration()` across a per-turn unbind (SW §3.2
            // service-worker object map, `host/service_worker/mod.rs`). Unlike
            // the Entity-keyed getter wrappers dropped below, these are
            // `WrapperKey::scope` (String-keyed) → NO cross-DOM aliasing risk,
            // so retaining them is safe without world_id / agent-scoped EcsDom.
            // Released at `teardown_document` (the whole registration unit —
            // data + brand + wrapper — clears together at document destruction).
            //
            // This one retain also covers caches the prior per-field
            // clears OMITTED — `validity_state` / `options_collection` /
            // `form_controls_collection` (Entity-keyed) and the FileList
            // (ObjectId-keyed) were never cleared on unbind despite
            // carrying the identical cross-DOM aliasing risk.  Folding
            // them in is a net cross-DOM-safety improvement, not a
            // behaviour regression.
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                use super::wrapper_intern::WrapperKind;
                hd.wrapper_store.retain(|key, _| {
                    matches!(
                        key.kind,
                        WrapperKind::Node
                            | WrapperKind::ServiceWorkerRegistration
                            | WrapperKind::ServiceWorker
                    )
                });
            }
            // D-9 events-modern-input (slot
            // `#11-events-modern-input`).  Three state tables hold
            // cross-DOM references and must be cleared on unbind:
            // - `data_transfer_states`: `drag_image_entity` is raw
            //   `entity_bits` from the previous EcsDom (`EcsDom::new()`
            //   worlds share Entity index space).
            // - `touch_states`: `target` ObjectId can be a HostObject
            //   wrapping an Entity from the previous EcsDom.
            // - `touch_list_states`: items list references Touch
            //   wrappers whose `target` faces the same cross-DOM risk.
            // (The DataTransferItem identity cache is cleared by the
            // unified `wrapper_store.retain` above.)
            self.inner.data_transfer_states.clear();
            self.inner.touch_states.clear();
            self.inner.touch_list_states.clear();
            // IndexedDB (D-20 `#11-indexed-db-vm`).  The IDB wrapper state
            // maps hold per-VM identity handles (handler / listener / result
            // `ObjectId`s — cross-DOM-aliasing per the side-store→component
            // rule exception (a)), so they must be cleared on unbind.  But
            // first roll back any still-open SQLite transaction: the backend
            // `IdbTransaction` has NO `Drop` rollback (only an explicit
            // `abort`), so dropping the state map alone would leave the
            // shared connection mid-transaction and block the next bind's
            // operations.  `idb_backend` itself is the per-origin resource
            // and is RETAINED (network_handle parity — the embedder manages
            // re-install on rebind to a new origin).
            if let Some(backend) = self.inner.idb_backend.clone() {
                for state in self.inner.idb_transaction_states.values_mut() {
                    if let Some(mut txn) = state.backend_txn.take() {
                        let _ = txn.abort(backend.conn());
                    }
                }
            }
            self.inner.idb_request_states.clear();
            self.inner.idb_transaction_states.clear();
            self.inner.idb_database_states.clear();
            self.inner.idb_object_store_states.clear();
            self.inner.idb_key_range_states.clear();
            self.inner.idb_index_states.clear();
            self.inner.idb_cursor_states.clear();
            // Cache API (D-19 PR-1): drop the per-`Cache`-handle name
            // tuples.  The shared origin backend handle (`HostData::cache_backend`)
            // is origin-keyed / cross-thread, so retained `Cache` wrappers
            // must not observe the prior bind's store after a rebind — the
            // wrappers resolve to `cache_handle_states` which is now empty,
            // and a fresh `caches.open(...)` re-registers post-rebind.
            self.inner.cache_handle_states.clear();
            // Service Worker realm (D-19 PR-2): drop the SW event / client
            // side-stores + the client snapshot + any unflushed outbound IPC.
            // The events are per-dispatch-transient, but a rebind must not let
            // a retained `Client` wrapper observe the prior bind's snapshot.
            self.inner.fetch_event_states.clear();
            self.inner.extendable_event_states.clear();
            self.inner.client_states.clear();
            self.inner.sw_clients.clear();
            self.inner.sw_outgoing.clear();
            // The `navigator.serviceWorker` CLIENT state is document-lifetime
            // (SW §3.4 ServiceWorkerContainer) and is cleared in
            // `teardown_document`, NOT here — so a `register()` staged inside a
            // script batch SURVIVES the per-batch unbind and reaches the
            // out-of-bracket event-loop drain (`drain_sw_client_requests`), and
            // a page's retained `ServiceWorkerRegistration` / `ServiceWorker`
            // wrapper stays a valid receiver across batches
            // (`#11-per-batch-unbind-document-lifetime-state`).  Survival is
            // cross-DOM-safe: the keys are per-VM `ObjectId` / `String` and a
            // live `Vm` only ever rebinds the SAME `EcsDom`.
            //
            // The wrapper-brand maps `sw_registration_states` /
            // `service_worker_states` (`ObjectId → scope`) SURVIVE too, in
            // lockstep with the `ServiceWorkerRegistration` / `ServiceWorker`
            // WRAPPERS the `wrapper_store.retain` below now keeps (the whole
            // registration unit — data + brand + wrapper — is document-lifetime,
            // released together at `teardown_document`).  So a JS-retained
            // registration stays a valid receiver (`require_registration_scope`)
            // AND `reg === getRegistration()` across batches; clearing the brand
            // per-turn would instead break a retained wrapper (illegal receiver).
            // The GC sweep still prunes a brand entry if its wrapper `ObjectId`
            // is ever collected (`gc/collect.rs`), a harmless backstop now that
            // the wrapper is retained.
            //
            // The SW WORKER-side per-dispatch event state above
            // (`fetch_event_states` / `client_states` / `sw_clients` /
            // `sw_outgoing`) DOES stay a per-turn scrub — transient, must not
            // let a retained `Client` wrapper read a prior dispatch's snapshot.
            // NB: the container singleton + the three interface prototypes are
            // NOT cleared — like `navigator` / `clients_prototype` they are
            // realm-structural and persist across a rebind (so a post-rebind
            // deliver still finds the container); only the per-bind state above
            // resets.
            // D-8 PR-A2 — Range / TreeWalker / NodeIterator state
            // clearing on unbind.  These live on `HostData` (not
            // `VmInner`) because the bridge pair-install happens
            // there; the `clear` happens via `HostData::unbind` in
            // the block below alongside `dom.clear_mutation_dispatcher()`
            // and the bridge teardown.  See plan-v4 §A-NI-1 Vm::unbind
            // install-order recap.
            // `mutation_observers.clear_pending_records()` drains every
            // observer's pending record queue so a post-rebind `notify`
            // cannot deliver records that reference `Entity` values from
            // the outgoing `EcsDom` world.  The observation target lists
            // need no scrub: they live as `MutationObservedBy` components
            // on entities, which are despawned with the outgoing world —
            // so the old Entity-index-collision hazard cannot occur.
            // Observer IDs themselves stay live in the registry so brand
            // checks on retained JS instances continue to succeed.
            //
            // `mutation_observer_bindings` (and its
            // `resize_observer_bindings` / `intersection_observer_bindings`
            // siblings) are intentionally NOT cleared here — they are
            // keyed by per-registry monotonic `observer_id` (not by
            // `Entity` or recycled `ObjectId`), so cross-DOM aliasing
            // does not apply, and a retained `mo` / `ro` / `io` that
            // re-observes after a rebind needs its callback intact to
            // fire.  Retained-but-idle bindings are no longer a
            // for-life leak: the GC keepalive seam keeps a binding
            // only per its liveness predicate (canonical statement =
            // `gc/keepalive.rs` `keepalive_survivors`), and the sweep
            // (`gc/collect.rs`) prunes collected binding rows and
            // `retire_collected`s their registry entries — including
            // after a rebind, once the World is readable again (while
            // unbound, bindings are kept fail-safe).
            //
            // Internal-config `Entity` references inside each registry
            // ARE cross-DOM-aliasing risks though: `IntersectionObserverInit
            // .root: Option<Entity>` lives on the retained
            // `RegisteredObserver`, so a script that constructs
            // `new IntersectionObserver(cb, { root: X })`, survives an
            // `unbind` (e.g. via global retention), and observes again
            // after rebind would otherwise have `root` point at a recycled
            // entity in the new world.  Scrub here to `None` (implicit
            // viewport) — same defensive pattern as
            // `clear_pending_records`.  Resize / Mutation registries
            // store target references as per-entity components, which
            // drop automatically on entity despawn (no scrub needed).
            // The world_id discriminator
            // (`#11-wrapper-cache-cross-dom-discriminator`) will
            // eventually subsume this.
            // ⚠ SUPERSEDED 2026-06-30: world_id retracted → agent-scoped
            // EcsDom World (PR #434 docs/plans/2026-06-agent-scoped-ecsdom-world.md
            // §6); interim form unchanged until B1.
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                hd.mutation_observers.clear_pending_records();
                hd.intersection_observers.clear_root_entities();
                // Custom-Elements REACTION QUEUE stays a per-turn scrub:
                // it is a transient queue drained at every script /
                // event / microtask checkpoint by `flush_ce_reactions`
                // (empty at bracket-end in the well-behaved case) and
                // every variant holds an `Entity`, so it rides the
                // per-DOM Entity-scrub class alongside
                // `clear_pending_records` above (`#11-custom-elements-vm`).
                //
                // The authoritative CE REGISTRY (`ce_registry` /
                // `ce_constructors` / `ce_constructor_to_id` /
                // `ce_when_defined_promises` / the id counter) is
                // document-lifetime state and is cleared in
                // `teardown_document`, NOT here — so a
                // `customElements.define()` SURVIVES the per-batch
                // (BATCH-BIND) unbind and is visible to a later batch's
                // upgrade / `whenDefined` (HTML §4.13.4/§4.13.5).
                // Survival is cross-DOM-safe by construction: a live
                // `Vm` only ever rebinds the SAME `EcsDom` (navigation
                // allocates a NEW `Vm`, see `host/media_query.rs`), so
                // the per-VM ctor `ObjectId`s ride the object heap
                // validly across a same-DOM turn.
                // (`#11-per-batch-unbind-document-lifetime-state`; the
                // grain migration to per-realm components rides
                // agent-scoped EcsDom,
                // docs/plans/2026-06-agent-scoped-ecsdom-world.md §5.)
                hd.ce_reaction_queue
                    .lock()
                    .expect("CE reaction queue mutex poisoned")
                    .clear();
            }
            // (The Attr identity cache — keyed by `(Entity, StringId)`,
            // same cross-DOM aliasing risk — is cleared by the unified
            // `wrapper_store.retain` above.)
            // Drop any signal-slots queued from the previous DOM —
            // their entities live in the old world, so firing
            // slotchange post-rebind would either resolve to a
            // recycled slot or panic in `dom_shared().contains`.
            // Also strip any stale `NotifyMutationObservers`
            // microtask from the queue: if it remained, a new
            // `slot.assign()` in the rebound VM would land its
            // signal behind a pre-existing notify task, and that
            // stale task would dispatch the new signal at the
            // wrong queue position (ahead of any Promise reactions
            // the new tick has registered).  Clearing the
            // coalescing flag in addition lets the first signal
            // after rebind enqueue a FRESH notify-MO microtask in
            // the correct queue slot.
            self.inner.pending_slot_change_signals.clear();
            self.inner.mutation_observer_microtask_queued = false;
            self.inner.microtask_queue.retain(|task| {
                !matches!(
                    task,
                    super::natives_promise::Microtask::NotifyMutationObservers
                )
            });
            // Cached `localStorage` / `sessionStorage` Storage
            // wrappers carry no per-DOM Entity, but the area-side
            // origin lookup goes through `VmInner::navigation` which
            // is bound-state-independent.  Clearing the instance
            // cache prevents a retained `localStorage` reference
            // from continuing to serve the previous origin's data
            // after a rebind to a document with a different origin
            // (cross-origin data leak).  `sessionStorage` is also
            // cleared because its data lives on `HostData::session_storage`,
            // which is per-VM by spec — see the `session_storage.clear()`
            // call below. A2: gated with the `Legacy` Web Storage glue (the
            // instance-cache fields are always-`None` in `App` builds).
            #[cfg(feature = "compat-webapi")]
            self.inner.clear_storage_instance_cache();
            // Cached `crypto` / `crypto.subtle` singletons.  Wrappers
            // are stateless (every method reuses the global OS CSPRNG /
            // hashes the input directly) and carry no per-DOM or
            // per-origin payload, so the clear here is a hygiene
            // measure — drops the GC roots so the wrappers can be
            // collected and re-allocated lazily after the next bind.
            self.inner.clear_crypto_instance_cache();
            // `screen` / `visualViewport` singletons + the VisualViewport
            // event-producer diff prior (S5-2) are deliberately NOT cleared here.
            // `unbind` closes every BATCH (script-exec / UA-event / frame-drain),
            // not only a navigation (the BATCH-BIND model, `HostDriver` doc), so
            // clearing them would break their `[SameObject]` identity across
            // batches AND drop a `visualViewport` resize listener registered in an
            // earlier batch — the next frame-drain producer would fire at a
            // freshly-allocated, listener-less singleton (Codex R4-B). Unlike
            // `localStorage` (cleared above for cross-ORIGIN data-leak safety),
            // these wrappers carry no per-origin / per-document payload in their
            // internal brand slots (script-attached expandos are the exception —
            // see Codex R11-2 below), so there is no internal state to scrub. The
            // cross-DOM identity reset on an actual navigation is the world-id
            // discriminator's job (`#11-wrapper-cache-cross-dom-discriminator`),
            // not a per-batch cache clear.
            // ⚠ SUPERSEDED 2026-06-30: world_id retracted → agent-scoped EcsDom
            // World (PR #434 docs/plans/2026-06-agent-scoped-ecsdom-world.md §6);
            // interim form unchanged until B1.
            // Codex R6-A: a script-registered `visualViewport` resize listener
            // therefore also survives unbind — but this is the SAME engine-wide
            // property `window.addEventListener('resize', …)` already has: the
            // Window global's `ObjectId` is the realm global (stable across the
            // `unbind doc1 → bind doc2` navigation rebind, `HostData::window_entity`),
            // and `vm_event_listeners` / `listener_store` are never cleared on
            // unbind (GC-pruned only). So a navigation-time listener scrub for
            // stable-identity globals (window + the payload-free singletons) is
            // the SAME navigation-vs-batch discriminator the world-id slot owns
            // (`#11-wrapper-cache-cross-dom-discriminator`) — NOT a VisualViewport-
            // only unbind clear, which would re-drop the cross-batch listener
            // (R4-B) and be a lone-outlier. S5-6 (the flip that first drives the
            // VM producer in production) is the hard gate for landing that
            // engine-wide scrub before the producer goes live.
            // ⚠ SUPERSEDED 2026-06-30: nav-scrub-as-S5-6-hard-gate is RETRACTED
            // (the flip is cross-DOM-neutral) — PR #434
            // docs/plans/2026-06-agent-scoped-ecsdom-world.md §6.2.
            // Codex R11-2: these singletons are `extensible` (spec-correct —
            // `screen` / `visualViewport` accept expandos), so a script-attached
            // own property (`screen.token = …`) ALSO survives unbind. That is
            // per-document JS state, not a payload-free read — but it is again the
            // SAME engine-wide leak `window.foo = …` has (the realm global
            // survives the rebind), so it folds into the SAME world-id
            // navigation-scrub (reset identity → drop expandos + listeners on a
            // real navigation), not a screen/VV-only clear that would wipe the
            // page's own state every batch.
            // ⚠ SUPERSEDED 2026-06-30: nav-scrub-as-S5-6-hard-gate is RETRACTED
            // (the flip is cross-DOM-neutral) — PR #434
            // docs/plans/2026-06-agent-scoped-ecsdom-world.md §6.2.
            // D-17 `#11-custom-elements-vm` — drop the cached
            // `customElements` singleton wrapper so it can be re-
            // allocated lazily on the next bind. The registry state
            // itself (registered constructors, reaction queue) is
            // scrubbed alongside the observer registries above.
            self.inner.custom_element_registry_instance = None;
            // sessionStorage is per-VM and per-browsing-context.  An
            // unbind boundary expresses the browsing-context
            // teardown — drop entries so a rebind cannot observe
            // stale data and so memory use stays bounded across
            // long-lived VMs that churn many sessions.
            // `fallback_local_storage` is also cleared (it's the
            // in-memory stand-in for localStorage when no backend
            // is installed; treating it as session-storage-shaped
            // matches its tests-only purpose).
            // A2: `compat-webapi`-gated — the fields are absent in `App` builds.
            #[cfg(feature = "compat-webapi")]
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                hd.session_storage.clear();
                hd.fallback_local_storage.clear();
            }
            // D-16 `#11-wasm-vm` — clear all 6 WebAssembly side-store
            // maps + the `wasm_backed_buffers` reverse-lookup so a
            // post-rebind VM cannot inherit per-VM identity-handle
            // wasm wrappers from the previous DOM session.  All payloads
            // carry engine-bridge handles whose `WasmStoreHandle` clone
            // (F1 D-ii) is per-VM; instance / exported-function call
            // adapters also carry a `ScriptHostBinding { session, dom,
            // document }` triple sourced from the outgoing bind, so
            // retaining them across an unbind would surface stale
            // host-binding closures.  `wasm_backed_buffers` is keyed by
            // ArrayBuffer ObjectIds whose entity space is rebuilt on
            // rebind; cross-DOM rebind invalidates per CLAUDE.md
            // side-store→component rule "per-VM identity handle
            // (一時的例外)".
            //
            // `wasm_runtime: OnceCell<Arc<WasmRuntime>>` is INTENTIONALLY
            // NOT cleared — the runtime owns its own
            // `Arc<DomHandlerRegistry>` + `Arc<CssomHandlerRegistry>`
            // internally (runtime-internal, not per-DOM-session) and is
            // cross-DOM reusable per "shared cross-cutting state
            // (恒久的例外)".  See `wasm_runtime` field doc on
            // [`super::VmInner`].
            self.inner.wasm_module_storage.clear();
            self.inner.wasm_instance_storage.clear();
            self.inner.wasm_memory_storage.clear();
            self.inner.wasm_table_storage.clear();
            self.inner.wasm_global_storage.clear();
            self.inner.wasm_exported_func_storage.clear();
            self.inner.wasm_backed_buffers.clear();
            // `CryptoKey` side store (`#11-crypto-subtle-full`).  Holds
            // secret key material → clear on unbind so it does not leak
            // into the next bind cycle (same data-class as
            // `wasm_module_storage`; distinct from the payload-free
            // Crypto/SubtleCrypto singleton clear below).  The cached
            // `algorithm` / `usages` wrappers (`crypto_key_js_cache`) hold
            // ObjectIds into the per-VM object space and must clear with
            // the key state so a stale id can't alias the next cycle.
            self.inner.crypto_key_states.clear();
            self.inner.crypto_key_js_cache.clear();
        }
    }

    /// Release the browsing-context-scoped resources this document owns —
    /// force-close every live WebSocket / EventSource connection and terminate
    /// every dedicated worker — then `unbind`.  This is the WHATWG HTML §10.2.4
    /// "terminate a worker" / WebSockets connection-teardown-on-Document-
    /// destruction moment (document unloading / pipeline replacement), NOT a
    /// per-turn event: the per-turn [`unbind`](Self::unbind) re-establishment
    /// boundary deliberately KEEPS these connections + workers alive across the
    /// bracket storm (their lifetime is the document, bounded by this call or
    /// the engine-Drop backstop, not the turn).
    ///
    /// Runs the close/terminate **while still bound** (both need the live
    /// `NetworkHandle` / worker registry + wrapper access), then calls
    /// `unbind()` as its final step.  Idempotent: after the first call the
    /// realtime side-tables + worker registry are empty, so a second call
    /// (explicit-then-Drop backstop) sends no `Close` and terminates no worker.
    #[cfg(feature = "engine")]
    pub fn teardown_document(&mut self) {
        // D-12 `#11-net-ws-sse` (CRIT-A): snapshot the active
        // realtime conn_ids BEFORE clearing HostData side-tables
        // so we can emit a `WebSocketClose` / `EventSourceClose`
        // per conn through the outgoing handle (mirror of
        // `reject_pending_fetches_with_error` shape at
        // `vm/host/fetch_tick.rs:82-131`).  Without the explicit
        // teardown, the broker's per-conn I/O thread would only
        // observe its `command_tx`'s `request_rx` drop when the
        // renderer Drops the `NetworkHandle` itself — which can be
        // much later than document teardown if the embedder keeps
        // the handle around for a subsequent `bind`.  Sending the
        // Close eagerly bounds the I/O thread's lifetime to the
        // document.
        //
        // Held in a temporary so the broker `send` calls don't
        // interleave with the `HostData::*` clears (clean borrow
        // split: snapshot first, send after).
        let realtime_teardown: Option<(Vec<u64>, Vec<u64>)> =
            self.inner.host_data.as_deref_mut().and_then(|hd| {
                if hd.is_bound() {
                    Some(hd.drain_realtime_for_unbind())
                } else {
                    None
                }
            });
        if let Some((ws_conns, sse_conns)) = realtime_teardown {
            if let Some(handle) = self.inner.network_handle.as_ref() {
                for conn_id in ws_conns {
                    let _ = handle.send(elidex_net::broker::RendererToNetwork::WebSocketClose(
                        conn_id,
                    ));
                }
                for conn_id in sse_conns {
                    let _ = handle.send(elidex_net::broker::RendererToNetwork::EventSourceClose(
                        conn_id,
                    ));
                }
            }
        }

        // Terminate every dedicated worker spawned by this document (WHATWG
        // HTML §10.2.4 "terminate a worker" runs from document teardown) and
        // uncache their `Worker` wrappers while still bound.
        self.inner.teardown_workers();

        // Custom-Elements registry + `navigator.serviceWorker` client are
        // document-lifetime state (HTML §4.13.4 The CustomElementRegistry
        // interface / SW §3.4 ServiceWorkerContainer) — released here at
        // document destruction, NOT on the per-turn `unbind`, so they survive
        // the BATCH-BIND unbind between script batches
        // (`#11-per-batch-unbind-document-lifetime-state`).  Pure map clears
        // with no emit side-effect, so a double-fire (explicit call then the
        // engine-Drop backstop) is a trivial no-op.  NOT cleared here (they
        // stay on the per-turn scrub in `unbind`, called last below):
        // `ce_reaction_queue` + the SW worker-side per-dispatch state.  The SW
        // wrapper-brand maps (`sw_registration_states` / `service_worker_states`)
        // ARE released here (they are document-lifetime + GC-sweep-pruned, so a
        // retained wrapper stays a valid receiver across batches).
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            hd.ce_registry
                .lock()
                .expect("CE registry mutex poisoned")
                .clear();
            hd.ce_constructors.clear();
            hd.ce_constructor_to_id.clear();
            hd.ce_when_defined_promises.clear();
            hd.ce_next_constructor_id = 0;
        }
        self.inner.pending_registration_promises.clear();
        self.inner.pending_unregister_promises.clear();
        self.inner.sw_ready_promise = None;
        self.inner.sw_registrations.clear();
        self.inner.sw_registration_states.clear();
        self.inner.service_worker_states.clear();
        self.inner.sw_controller_scope = None;
        self.inner.sw_messages_enabled = false;
        self.inner.sw_message_buffer.clear();
        self.inner.sw_client_outgoing.clear();

        // Un-bind the pointers + drop the per-turn caches as the final step.
        self.unbind();
    }

    /// Deliver session-level `MutationRecord`s to every registered
    /// `MutationObserver` (WHATWG DOM §4.3).
    ///
    /// This is an **embedder API** — the VM does not auto-deliver
    /// mutation records.  Embedders call this once per script-task
    /// boundary so callbacks fire as part of the WHATWG "queue a
    /// mutation observer microtask" semantics.  Standalone tests
    /// must call this explicitly between mutating the DOM and
    /// asserting on observer side effects.
    ///
    /// Each session record is fed to the registry via
    /// `MutationObserverRegistry::notify`, with a closure that
    /// walks `EcsDom::get_parent` to test subtree-ancestry
    /// matches.  After every record is queued, observers with
    /// pending records are drained one at a time, their records
    /// marshalled into JS via `mutation_record_to_js`, and their
    /// callback invoked with `(records, observer)`.  Re-entrant
    /// `mo.observe(other, ...)` / `mo.disconnect()` from inside a
    /// callback is supported because the observer-id list is
    /// captured up front and registry access between iterations is
    /// always a fresh borrow (no nested mutation in a single
    /// borrow).
    ///
    /// Trailing microtask checkpoint runs so any
    /// `Promise.resolve().then(...)` queued from a callback fires
    /// before this call returns — matches the `eval` /
    /// `tick_network` policy.
    ///
    /// While bound, the trailing microtask checkpoint runs
    /// unconditionally — even when no records are queued and no
    /// observers have pending records on entry — to keep the
    /// embedder API uniform across script-task boundaries (the
    /// cost of an empty drain is negligible).  Post-unbind the
    /// implementation early-returns before any work, including the
    /// microtask drain, because no JS executes while the VM is
    /// unbound.  Callbacks that throw are reported via `eprintln!`
    /// and do not propagate (matches the boa-side behaviour and
    /// "report" semantics in HTML §8.1.4.6).
    #[cfg(feature = "engine")]
    pub fn deliver_mutation_records(&mut self, records: &[elidex_script_session::MutationRecord]) {
        self.inner.deliver_mutation_records(records);
    }

    /// Deliver per-frame resize observations to every registered
    /// `ResizeObserver` (W3C Resize Observer §2 "broadcast active
    /// resize observations").
    ///
    /// Same embedder-API contract as [`Self::deliver_mutation_records`]:
    /// the VM does not auto-deliver — the shell main loop calls this
    /// once per layout/paint cycle so callbacks fire as part of the
    /// "broadcast" step.  Unlike `deliver_mutation_records`, no input
    /// list is needed: the observation algorithm runs inside the
    /// engine-independent
    /// [`elidex_api_observers::resize::ResizeObserverRegistry::gather_observations`]
    /// against the bound `EcsDom`'s current `LayoutBox` components.
    ///
    /// Trailing microtask checkpoint runs so any `.then` chained from
    /// a callback fires before this call returns.  Post-unbind early-
    /// returns before any work.  Callbacks that throw are reported via
    /// `eprintln!` and do not propagate.
    ///
    /// Currently a cutover-ready API: the boa-driven shell still
    /// invokes the boa-side
    /// `JsRuntime::deliver_resize_observations`; the VM-side wiring
    /// lands with the boa→VM cutover (M4-12 D-26 / PR7).
    #[cfg(feature = "engine")]
    pub fn deliver_resize_observations(&mut self) {
        self.inner.deliver_resize_observations();
    }

    /// Deliver per-frame intersection observations to every registered
    /// `IntersectionObserver` (W3C Intersection Observer §4 "notify
    /// intersection observers").
    ///
    /// Same contract as [`Self::deliver_resize_observations`].  The
    /// implicit root rect (`window.innerWidth` / `innerHeight` /
    /// `scrollX` / `scrollY`) and broadcast `time`
    /// (`performance.now()`) are both sourced from VM state: shell
    /// maintains the viewport slots through the usual
    /// `Window.scrollTo` / `resize` paths, so a separate arg would
    /// just be redundant state.
    #[cfg(feature = "engine")]
    pub fn deliver_intersection_observations(&mut self) {
        self.inner.deliver_intersection_observations();
    }

    /// Drain pending network events (broker `FetchResponse` replies)
    /// and dispatch them to the JS side.  For each reply, settles
    /// the associated pending Promise — fulfil with a
    /// freshly-constructed `Response` on success, reject with a
    /// `TypeError("Failed to fetch: ...")` on broker-side failure.
    /// Late replies for fetches whose Promise was already settled by
    /// an abort fan-out (`controller.abort()` between dispatch and
    /// reply) are silently dropped because their entry in
    /// `VmInner::pending_fetches` was already removed.
    ///
    /// Runs a microtask checkpoint at the end so `.then` reactions
    /// fire before this call returns.
    ///
    /// Idempotent and cheap when no events are pending — the shell
    /// event loop calls this every tick; tests that need to observe
    /// Promise settlement after a mock fetch call this explicitly
    /// between dispatch and assertion.
    #[cfg(feature = "engine")]
    pub fn tick_network(&mut self) {
        self.inner.tick_network();
    }

    /// Drain every registered dedicated worker's outbound channel and fire the
    /// resulting `message` / `error` / `messageerror` events on the matching
    /// `Worker` objects (the parent's event-loop step of WHATWG HTML §10.2.4
    /// "run a worker"). The shell main loop calls this each frame, exactly as
    /// it calls [`Self::tick_network`]; a no-op when no workers are registered.
    #[cfg(feature = "engine")]
    pub fn drain_worker_messages(&mut self) {
        self.inner.drain_worker_messages();
    }

    /// Deliver an inbound `navigator.serviceWorker` back-channel update
    /// (DR-B'; WHATWG SW §3.1/§3.4, D-19 PR-3): settle `register()` /
    /// `unregister()` promises and fire `statechange` / `updatefound` /
    /// `controllerchange` / `message`.  The window-realm twin of PR-2's
    /// SW-thread recv loop and the 7th member of this `deliver_*` family —
    /// runs a trailing microtask checkpoint, silent no-op post-unbind.
    ///
    /// Harness-driven over the engine-independent `SwClientUpdate` contract;
    /// the `content/event_loop.rs`→VM consumer wire (mapping
    /// `BrowserToContent` SW variants 1:1 onto `SwClientUpdate`) is the D-26
    /// boa→VM cutover, like PR-2's `ContentToSw`/`SwToContent` harness.
    #[cfg(feature = "engine")]
    pub fn deliver_sw_client_update(&mut self, update: elidex_api_sw::SwClientUpdate) {
        self.inner.deliver_sw_client_update(update);
    }

    /// Take the outbound `navigator.serviceWorker` client requests staged by
    /// the `register` / `update` / `unregister` / `postMessage` natives (D-19
    /// PR-3).  The content event loop forwards these to the coordinator at the
    /// D-26 cutover; tests assert on them directly.
    #[cfg(feature = "engine")]
    pub fn drain_sw_client_requests(&mut self) -> Vec<elidex_api_sw::SwClientRequest> {
        self.inner.drain_sw_client_requests()
    }

    /// Seed the initial `navigator.serviceWorker` controller + registrations a
    /// page is controlled by AT navigation (WHATWG SW §3.4.1, F2
    /// construction-init seed), before any runtime
    /// [`deliver_sw_client_update`](Self::deliver_sw_client_update).  The shell
    /// populates this at document creation (D-26 cutover); tests call it
    /// directly.  An uncontrolled page passes `None` + an empty slice.
    #[cfg(feature = "engine")]
    pub fn seed_sw_client(
        &mut self,
        controller: Option<url::Url>,
        registrations: &[(url::Url, elidex_api_sw::SwWorkerSnapshot)],
    ) {
        let controller = controller.map(|u| u.as_str().to_owned());
        let regs = registrations
            .iter()
            .map(|(u, w)| (u.as_str().to_owned(), w.clone()))
            .collect();
        self.inner.seed_sw_client(controller, regs);
    }

    /// Flush every dirty `<canvas>` (HTML §4.12.5 "The 2D rendering context"):
    /// copy each 2D context's pixels into its [`ImageData`] component (the
    /// display-list source `elidex-render` composites) and clear the dirty
    /// marker. The shell main loop calls this each frame, exactly as it calls
    /// [`Self::tick_network`]; a no-op when unbound or no canvas is dirty.
    ///
    /// [`ImageData`]: elidex_ecs::ImageData
    #[cfg(feature = "engine")]
    pub fn sync_dirty_canvases(&mut self) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            if hd.is_bound() {
                elidex_api_canvas::sync_dirty_canvases(hd.dom());
            }
        }
    }

    /// Install the `NetworkHandle` used by the `fetch()` host
    /// global.  Without a handle, every `fetch()` call rejects
    /// with a `TypeError` (matches `NetworkHandle::disconnected()`
    /// semantics — the embedder simply has no live broker).
    ///
    /// Callers typically construct the handle from the Network
    /// Process broker (`NetworkProcessHandle::create_renderer_handle()`)
    /// or — for self-contained tests — from
    /// `NetworkHandle::mock_with_responses()` behind the
    /// `elidex-net/test-hooks` feature.
    ///
    /// Replaces any previously installed handle.  Dropping the
    /// `Vm` (or calling this with a fresh handle) releases the
    /// previous `Rc`.  Any in-flight async fetches against the
    /// previous handle are rejected with `TypeError("Failed to
    /// fetch: NetworkHandle replaced while request in flight")`
    /// before the swap, since their broker-reply channel becomes
    /// unreachable; without this the Promises would be
    /// permanently un-settleable (R3.3).  Reactions attached via
    /// `.then` / `.catch` fire on the next microtask drain (next
    /// `eval` / `tick_network` call), not synchronously here.
    ///
    /// Re-installing the *same* `Rc<NetworkHandle>` (pointer-equal
    /// to the one already stored) is a no-op — pending fetches
    /// are preserved because the broker-reply channel is the same
    /// physical handle (R6.1).  This keeps benign re-install
    /// patterns (e.g. an embedder cloning + re-installing through
    /// a shared accessor) from spuriously cancelling in-flight
    /// requests.
    #[cfg(feature = "engine")]
    pub fn install_network_handle(
        &mut self,
        handle: std::rc::Rc<elidex_net::broker::NetworkHandle>,
    ) {
        if let Some(ref current) = self.inner.network_handle {
            if std::rc::Rc::ptr_eq(current, &handle) {
                return;
            }
        }
        self.inner
            .reject_pending_fetches_with_error("NetworkHandle replaced while request in flight");
        self.inner.network_handle = Some(handle);
    }

    /// Install the per-origin IndexedDB backend (slot `#11-indexed-db-vm`).
    ///
    /// The embedder / session layer constructs an [`elidex_indexeddb::IdbBackend`]
    /// from the origin's `OriginStorageManager` `SqliteConnection` and installs
    /// it here for persistent per-origin storage.  When none is installed, the
    /// `indexedDB` host code lazily creates an in-memory backend on first use
    /// (`VmInner::ensure_idb_backend`, mirroring the boa bridge default), so
    /// IndexedDB works out of the box for tests / unconfigured VMs.
    ///
    /// Shared cross-cutting session resource (`!Send`/`!Sync` SQLite handle) —
    /// stored on the internal `VmInner::idb_backend` field.
    #[cfg(feature = "engine")]
    pub fn install_idb_backend(&mut self, backend: std::rc::Rc<elidex_indexeddb::IdbBackend>) {
        // Re-installing the *same* backend (pointer-equal to the one already
        // stored) is a no-op — its live `backend_txn` handles and the
        // request / transaction / database / store / key-range side stores
        // are all tied to this very connection, so the take + rollback +
        // clear below would strand in-flight transactions against a backend
        // that is in fact unchanged.  Mirrors `install_network_handle`'s
        // pointer-equality guard.
        if let Some(ref current) = self.inner.idb_backend {
            if std::rc::Rc::ptr_eq(current, &backend) {
                return;
            }
        }
        // If a DIFFERENT backend is already installed with live IDB state, the
        // existing `IdbTransactionState.backend_txn` handles are tied to the
        // OLD connection — swapping the backend would strand them (a later
        // commit/abort would target the NEW connection).  Roll them back
        // against the old backend, then tear down the connection-scoped state
        // before replacing it (the IDB portion of `unbind`).  Normal bind
        // installs onto empty state, so this is a defensive no-op there; it
        // makes a mid-session swap safe rather than connection-stranding.
        if let Some(old) = self.inner.idb_backend.take() {
            for state in self.inner.idb_transaction_states.values_mut() {
                if let Some(mut txn) = state.backend_txn.take() {
                    let _ = txn.abort(old.conn());
                }
            }
            // Abort still-pending requests IN PLACE (Done + AbortError) rather
            // than dropping their state: a held `IDBRequest` wrapper must not
            // hang at `readyState === 'pending'` forever once its backend is
            // gone.  Retaining the request states lets the wrappers resolve;
            // their queued `IdbDeliver` tasks then no-op (cleared outcome).
            self.inner.abort_pending_idb_requests(
                "IndexedDB backend replaced while a request was pending",
            );
            // The transaction / database / store / key-range / index / cursor
            // stores are connection-scoped and have no meaning against the new
            // backend.
            self.inner.idb_transaction_states.clear();
            self.inner.idb_database_states.clear();
            self.inner.idb_object_store_states.clear();
            self.inner.idb_key_range_states.clear();
            self.inner.idb_index_states.clear();
            self.inner.idb_cursor_states.clear();
        }
        self.inner.idb_backend = Some(backend);
    }

    /// Install a new global variable.
    ///
    /// Reusing a name is normally a bug — shell host globals and JS-visible
    /// built-ins must not collide — so this convenience method ignores any
    /// previous value.  Use [`Vm::set_global_checked`] if the caller needs
    /// to detect replacement explicitly.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        let _ = self.set_global_checked(name, value);
    }

    /// Install a new global variable and return the previous value, if any.
    pub fn set_global_checked(&mut self, name: &str, value: JsValue) -> Option<JsValue> {
        let id = self.inner.strings.intern(name);
        self.inner.globals.insert(id, value)
    }

    /// Get a global variable.
    pub fn get_global(&self, name: &str) -> Option<JsValue> {
        let sid = self.inner.strings.lookup(name)?;
        self.inner.globals.get(&sid).copied()
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
