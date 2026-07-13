//! `Vm` document-lifecycle API — `unbind` / `teardown_document`.
//!
//! Carved out of `vm_api.rs` as a standalone prereq split: the
//! `unbind` / `teardown_document` pair is the document-lifecycle
//! cohesion seam (per-turn host-pointer clear + browsing-context
//! resource teardown) distinct from the `bind` / `eval` / API
//! surface that stays in `vm_api.rs`.  Pure move — no logic change.
//! Folds into the `#11-host-data-full-decomposition` sibling debt.

use super::Vm;

impl Vm {
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
            // `reg === getRegistration()` across a per-turn unbind (SW §3.2.1
            // service worker registration object map, `host/service_worker/mod.rs`). Unlike
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
            // ── Wrapper-lifetime taxonomy (Codex #459 R1–R4; the SoT so each
            // field's class is not re-litigated per review round) ──
            //  1. REALM-STRUCTURAL singleton — identity owned by an install-
            //     once, never-re-installed `globalThis` data property (or a
            //     realm prototype): `customElements`, `sw_container`,
            //     `navigator`, `crypto`, the prototypes. Only its mutable
            //     BACKING DATA is document-scoped. The cache slot MUST NOT be
            //     cleared where re-minting can DIVERGE from the property — the
            //     `alloc_or_cached_*` + identity-compare case (`customElements`
            //     via `convert_custom_element_registry_member`): splitting slot
            //     from property yields two sources of truth for one identity and
            //     a `Foreign` misclassification. Where there is NO such compare
            //     (`crypto`), a per-turn slot-clear is redundant-but-benign (the
            //     property keeps the wrapper rooted; no divergence observable) —
            //     `sw_container` / `navigator` / prototypes are simply never
            //     cleared.
            //  2. DOCUMENT-LIFETIME dynamic wrapper — minted per-op, String/
            //     Scope-keyed, NOT globalThis-reachable (SW per-scope
            //     `ServiceWorkerRegistration` / `ServiceWorker`): survives a
            //     per-turn unbind, dropped at `teardown_document` with its data
            //     + brand rows.
            //  3. PER-TURN / Entity-keyed transient — cleared every unbind
            //     (Node getter wrappers, validity/touch/IDB caches, the CE
            //     reaction queue).
            //
            // D-17 `#11-custom-elements-vm` — the cached `customElements`
            // singleton wrapper is CLASS 1 (realm-structural), so it SURVIVES
            // the per-turn unbind AND `teardown_document` (Codex #459 R3-1 +
            // R4). `globalThis.customElements` is installed ONCE as an eager
            // data property (`register_globals` at `Vm::new`, never re-run per
            // bind), so it permanently points at the original wrapper; dropping
            // the `custom_element_registry_instance` slot (at unbind OR
            // teardown) would let `alloc_or_cached_custom_element_registry` mint
            // a SECOND wrapper, and `convert_custom_element_registry_member`
            // would then classify the page's own `customElements` as `Foreign`
            // and reject it (`createElement(x, { customElementRegistry:
            // customElements })` throwing NotSupportedError). Only the
            // `ce_registry` DATA is document-lifetime (cleared at teardown); the
            // wrapper reads it, so after teardown it presents an empty registry.
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
        // retained wrapper stays a valid receiver across batches).  The
        // document-lifetime, per-scope SW WRAPPERS that `unbind` retains — the
        // Scope-keyed `ServiceWorkerRegistration` / `ServiceWorker`
        // `wrapper_store` entries — are dropped here too, in lockstep with
        // their data + brand rows (Codex #459 R3-2), so that per-registration
        // identity unit clears together.  (The CE registry SINGLETON wrapper is
        // NOT dropped here — it is realm-structural; see its note below the
        // `ce_*` clears — Codex #459 R4.)
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
        // The CE registry WRAPPER (`custom_element_registry_instance`) is NOT
        // cleared here — it is **realm-structural**, not document-lifetime
        // (Codex #459 R4, correcting R3-1's teardown over-reach). Only its
        // BACKING DATA (`ce_registry` etc., cleared above) is document-scoped;
        // the wrapper itself is an install-once `globalThis.customElements`
        // singleton (class 1 in the wrapper-lifetime taxonomy at `unbind`) that
        // lives as long as the realm global, exactly like `sw_container` /
        // `navigator` / the prototypes. Nulling the slot would free NOTHING
        // (the `globalThis.customElements` data property keeps the wrapper
        // rooted) while DESYNCing the cached id from that property — so a
        // teardown-then-rebind of the same `Vm` would re-mint a second wrapper
        // and classify the page's own `customElements` as `Foreign`. After
        // teardown the surviving wrapper simply reads the now-empty
        // `ce_registry` = an empty registry, which is correct.
        self.inner.pending_registration_promises.clear();
        self.inner.pending_unregister_promises.clear();
        self.inner.sw_ready_promise = None;
        // Drop the surviving SW registration/worker WRAPPERS in lockstep with
        // their data + brand rows (Codex #459 R3-2). The per-turn `unbind`
        // RETAINS `WrapperKind::ServiceWorkerRegistration`/`ServiceWorker`
        // (document-lifetime, so a page's `reg === getRegistration()` holds
        // across batches); at document destruction the whole unit clears
        // together. Leaving the Scope-keyed `wrapper_store` entries behind
        // would let a later same-`Vm` re-`register()` of the same scope hit
        // `intern_wrapper`'s cached `ObjectId`, skip the allocation closure
        // that repopulates `sw_registration_states`/`service_worker_states`,
        // and return a registration that fails its own brand check.
        {
            use super::wrapper_intern::{WrapperKey, WrapperKind};
            let scope_sids: Vec<_> = self
                .inner
                .sw_registrations
                .values()
                .map(|entry| entry.scope_sid)
                .collect();
            for scope_sid in scope_sids {
                let _ = self.inner.remove_wrapper_keyed(WrapperKey::scope(
                    scope_sid,
                    WrapperKind::ServiceWorkerRegistration,
                ));
                let _ = self
                    .inner
                    .remove_wrapper_keyed(WrapperKey::scope(scope_sid, WrapperKind::ServiceWorker));
            }
        }
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
}
