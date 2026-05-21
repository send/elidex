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

    /// Set the URL surfaced by `document.referrer` (WHATWG HTML §3.1.5).
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
            // D-31: typed `ConsumerDispatcher` replaces the v4-era
            // `MutationBridge` 2-consumer composer (the
            // `#11-mutation-hook-multiplexer` slot is closed by this
            // structural shift).  Dispatch order = field declaration
            // order = (live_range → node_iter → base_url →
            // form_control_reconciler).
            let mut dispatcher =
                crate::vm::consumer_dispatcher::ConsumerDispatcher::new(live_range, node_iter);
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
    pub fn unbind(&mut self) {
        // D-12 `#11-net-ws-sse` (CRIT-A): snapshot the active
        // realtime conn_ids BEFORE clearing HostData side-tables
        // so we can emit a `WebSocketClose` / `EventSourceClose`
        // per conn through the outgoing handle (mirror of
        // `reject_pending_fetches_with_error` shape at
        // `vm/host/fetch_tick.rs:82-131`).  Without the explicit
        // teardown, the broker's per-conn I/O thread would only
        // observe its `command_tx`'s `request_rx` drop when the
        // renderer Drops the `NetworkHandle` itself — which can be
        // much later than `unbind` if the embedder keeps the
        // handle around for a subsequent `bind`.  Sending the
        // Close eagerly bounds the I/O thread's lifetime to the
        // bind cycle.
        //
        // Held in a temporary so the broker `send` calls don't
        // interleave with the `HostData::*` clears below (clean
        // borrow split: snapshot first, send after, clear last).
        #[cfg(feature = "engine")]
        let realtime_teardown: Option<(Vec<u64>, Vec<u64>)> =
            self.inner.host_data.as_deref_mut().and_then(|hd| {
                if hd.is_bound() {
                    Some(hd.drain_realtime_for_unbind())
                } else {
                    None
                }
            });
        #[cfg(feature = "engine")]
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
        #[cfg(feature = "engine")]
        self.inner.teardown_workers();

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
            // This one retain also covers caches the prior per-field
            // clears OMITTED — `validity_state` / `options_collection` /
            // `form_controls_collection` (Entity-keyed) and the FileList
            // (ObjectId-keyed) were never cleared on unbind despite
            // carrying the identical cross-DOM aliasing risk.  Folding
            // them in is a net cross-DOM-safety improvement, not a
            // behaviour regression.
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                hd.wrapper_store
                    .retain(|key, _| key.kind == super::wrapper_intern::WrapperKind::Node);
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
            // `mutation_observer_callbacks` /
            // `mutation_observer_instances` are intentionally NOT
            // cleared here — they are keyed by VM-monotonic
            // `observer_id` (not by `Entity` or recycled `ObjectId`),
            // so cross-DOM aliasing does not apply, and a retained
            // `mo` that re-observes after a rebind needs its callback
            // intact to fire.  The trade-off is a bounded leak per
            // `new MutationObserver()` call (callback + instance
            // wrapper rooted until the VM drops); cleanup belongs to
            // a future weak-rooting design tracked in
            // `#11-mutation-observer-extras`.
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                hd.mutation_observers.clear_pending_records();
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
            // call below.
            self.inner.clear_storage_instance_cache();
            // Cached `crypto` / `crypto.subtle` singletons.  Wrappers
            // are stateless (every method reuses the global OS CSPRNG /
            // hashes the input directly) and carry no per-DOM or
            // per-origin payload, so the clear here is a hygiene
            // measure — drops the GC roots so the wrappers can be
            // collected and re-allocated lazily after the next bind.
            self.inner.clear_crypto_instance_cache();
            // sessionStorage is per-VM and per-browsing-context.  An
            // unbind boundary expresses the browsing-context
            // teardown — drop entries so a rebind cannot observe
            // stale data and so memory use stays bounded across
            // long-lived VMs that churn many sessions.
            // `fallback_local_storage` is also cleared (it's the
            // in-memory stand-in for localStorage when no backend
            // is installed; treating it as session-storage-shaped
            // matches its tests-only purpose).
            if let Some(hd) = self.inner.host_data.as_deref_mut() {
                hd.session_storage.clear();
                hd.fallback_local_storage.clear();
            }
        }
    }

    /// Deliver session-level `MutationRecord`s to every registered
    /// `MutationObserver` (WHATWG DOM §4.3.4).
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
    /// "report" semantics in HTML §8.1.5).
    #[cfg(feature = "engine")]
    pub fn deliver_mutation_records(&mut self, records: &[elidex_script_session::MutationRecord]) {
        self.inner.deliver_mutation_records(records);
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
