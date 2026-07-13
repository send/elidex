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
