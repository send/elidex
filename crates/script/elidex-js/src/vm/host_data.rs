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
    use super::super::host::cache::CacheBackend;
    use super::super::host::observer_common::ObserverBinding;
    use super::super::value::{ObjectId, StringId};
    use super::super::wrapper_intern::{WrapperKey, WrapperKind};
    use elidex_ecs::{Entity, NodeKind};
    use elidex_script_session::ListenerId;
    // A2: the Web Storage backends are `Legacy`-only — dropped from `App` builds.
    #[cfg(feature = "compat-webapi")]
    use elidex_storage_core::{SessionStorageState, WebStorageManager};
    use std::collections::{HashMap, HashSet};
    // Used only by the (compat-webapi-gated) opaque-origin counter.
    #[cfg(feature = "compat-webapi")]
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// Per-process counter for opaque-origin sentinels (e.g. `about:blank`,
    /// `data:` URLs).  Each `HostData` claims one ID at construction time
    /// and uses it to scope `localStorage` entries so two opaque-origin
    /// VMs do not see each other's data through the manager's
    /// origin-keyed registry.  Resets on process restart.
    /// A2: used only for `localStorage` scoping — `compat-webapi`-gated.
    #[cfg(feature = "compat-webapi")]
    static OPAQUE_ORIGIN_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Prefix on per-VM opaque-origin sentinel strings.  Distinct
    /// from any `url::Origin::ascii_serialization()` output (which is
    /// always `scheme://host[:port]`) so the sentinel cannot collide
    /// with a real origin.
    #[cfg(feature = "compat-webapi")]
    const OPAQUE_ORIGIN_PREFIX: &str = "opaque-origin:";

    #[cfg(feature = "compat-webapi")]
    fn next_opaque_origin_id() -> String {
        let n = OPAQUE_ORIGIN_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{OPAQUE_ORIGIN_PREFIX}{n}")
    }

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
        /// `NodeKind::DocumentFragment` — chains via
        /// `DocumentFragment.prototype → Node.prototype → …`.
        /// Carries the ParentNode mixin (`prepend` / `append` /
        /// `replaceChildren`) per WHATWG DOM §4.7.
        DocumentFragment,
        /// Entity carries an `elidex_ecs::ShadowRoot` component —
        /// chains via `ShadowRoot.prototype → DocumentFragment.prototype
        /// → Node.prototype → …`.  Brand check for ShadowRoot
        /// receivers is the engine-component lookup
        /// `world.get::<&elidex_ecs::ShadowRoot>(entity).is_ok()`,
        /// uniform with how other ECS-backed wrapper kinds are
        /// dispatched here ([feedback_objectkind-resolution-uniformity]).
        ShadowRoot,
        /// Document or anything without a recognised `NodeKind`.
        /// Chains directly to `Node.prototype`.
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
        /// A stable `EcsDom::world_id()` discriminator is tracked
        /// as defer slot `#11-wrapper-cache-cross-dom-discriminator`
        /// (no current milestone — opens when Web Workers VM port
        /// land OR an observable cross-DOM aliasing bug surfaces);
        /// until then the invariant is a caller contract.
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
        /// Monotonic bind-cycle counter — incremented on every
        /// `unbind()`.  Used by `StaticRange.isValid()` to detect
        /// retained instances whose captured `Entity` bits became
        /// stale across rebind (a fresh `EcsDom` can reuse the
        /// same slot for a different entity).  Copilot R9.
        pub(crate) bind_epoch: u32,
        pub(crate) listener_store: HashMap<ListenerId, ObjectId>,
        /// Unified wrapper-identity store (`#11-wrapper-identity-seam`) —
        /// every `[SameObject]` DOM wrapper identity (node wrappers + the 23
        /// former per-purpose caches: classList / dataset / style / attr /
        /// cssRules / collections / FileList / DataTransferItem) keyed by
        /// [`WrapperKey`].  GC mark/sweep dispatch via
        /// [`WrapperKind::mark_agent`] / [`WrapperKind::retain`].  Cleared on
        /// `unbind` (per-VM, world_id-independent — see module docs).
        pub(crate) wrapper_store: HashMap<WrapperKey, ObjectId>,
        /// Page-visibility state of this document's top-level browsing
        /// context (WHATWG HTML §6.2): `true` when the tab/window is
        /// hidden (background tab, minimized, occluded).  Drives
        /// `document.hidden` / `document.visibilityState`.  A per-VM
        /// browsing-context fact driven by the embedding shell — not a
        /// per-entity DOM fact — so it lives on `HostData` (the shared
        /// cross-cutting (b) exception in the side-store→component rule),
        /// not as an ECS component.  Defaults visible.
        pub(crate) tab_hidden: bool,
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
        /// Iframe sandbox flags for this document's browsing context
        /// (WHATWG HTML §7.1.5 Sandboxing).  `None` for top-level /
        /// unsandboxed documents — the cutover's shell installs parsed
        /// `IframeSandboxFlags` (from the iframe `sandbox=""` attribute)
        /// via [`Self::set_sandbox_flags`] when a document loads inside a
        /// sandboxed iframe.  A per-browsing-context security fact (not a
        /// per-entity DOM fact) → the CLAUDE.md side-store rule (b)
        /// shared-cross-cutting exception, like `cookie_jar` above.  Read
        /// by [`Self::scripts_allowed`] (the eval gate; S1b adds the
        /// `forms`/`popups`/`modals` accessors + their consumer wiring).
        sandbox_flags: Option<elidex_plugin::IframeSandboxFlags>,
        /// The document's security origin override (WHATWG HTML §7.1.1).
        /// `None` until the embedder's load path installs it via
        /// [`Self::set_origin`]; while `None`,
        /// [`super::VmInner::document_origin`] derives the origin from
        /// `navigation.current_url` (the spec default: a document's origin is
        /// its URL's origin unless overridden).  The shell installs
        /// `Some(opaque)` for a sandboxed iframe (HTML §7.1.5), so the
        /// document reports `"null"` on the settings-object-origin surfaces
        /// (window.postMessage / WebSocket + EventSource `Origin` /
        /// localStorage partition).
        ///
        /// **ECS-native placement (interim, not the ideal).** A document's
        /// origin is strictly a per-Document fact (HTML §7.1.1), so the
        /// CLAUDE.md side-store rule points at an ECS component on the document
        /// entity, not this per-VM store — it is *not* genuinely the (b)
        /// shared-cross-cutting exception that `cookie_jar` is. S1b keeps it
        /// here as the **consistent interim**: the whole current-document-state
        /// cluster (`current_url`/`NavigationState`, `sandbox_flags`, and this)
        /// is per-VM today, so moving only the origin to a component would be a
        /// strangler (One-issue-one-way). The ideal — migrating the cluster to
        /// per-entity components, with navigation creating a fresh document
        /// entity (so cleanup/rebind come from ECS despawn rather than the
        /// shell remembering to overwrite) — is a model-wide redesign coupled
        /// to the navigation back-channel (S1c) and spanning the merged S1a
        /// `sandbox_flags`. Tracked as the ECS-native side-store→component
        /// program → slot `#11-browsing-context-state-ecs-components` (sibling
        /// of `#11-wrapper-identity-component-migration`, same world_id gate).
        /// (NB `location.origin` does NOT read this — HTML §7.2.4 returns the
        /// *URL's* origin, which can differ from the document origin for a
        /// sandboxed doc.)
        document_origin_override: Option<elidex_plugin::SecurityOrigin>,
        /// Per-VM **stable** opaque origin returned by
        /// [`super::VmInner::document_origin`] when no override is installed and
        /// `navigation.current_url` is itself opaque (e.g. the standalone /
        /// `about:blank` pipeline path, where the shell never calls
        /// [`Self::set_origin`] because `current_url` is `None`).  A document's
        /// origin is stable document state (HTML §7.1.1), so the resolver must
        /// not re-mint a fresh `Opaque(n)` per read — that would make
        /// `iframe/lifecycle.rs`'s parent→child origin propagation
        /// (`bridge().origin()` feeds the child's load context) non-deterministic
        /// and diverge from boa's single stored default.  Minted once at
        /// `HostData::new`.  (Distinct from `opaque_origin_sentinel`, which is
        /// the storage-partition *string* key for the same opaque situation.)
        fallback_opaque_origin: elidex_plugin::SecurityOrigin,
        /// The iframe nesting depth of this document's browsing context
        /// (`0` for top-level).  Installed by the shell's iframe load path
        /// ([`Self::set_iframe_depth`]); read to cap runaway `<iframe>`
        /// nesting.  Per-browsing-context fact, (b) exception like above.
        iframe_depth: usize,
        /// `MutationObserver` registry (WHATWG DOM §4.3.1) — owns the
        /// per-observer pending-record queues. The observation targets +
        /// options live as `MutationObservedBy` components on the
        /// observed entities (WHATWG DOM §4.3 registered observer
        /// list), not in the registry. Held here (rather than on
        /// `VmInner`) so the registry's lifetime tracks the bound DOM
        /// world: `Vm::unbind` drains pending records via
        /// `MutationObserverRegistry::clear_pending_records` (they hold
        /// outgoing-world `Entity` refs), while observer IDs stay live so
        /// a JS reference held to a `MutationObserver` instance across
        /// the unbind boundary continues to brand-check.
        pub(crate) mutation_observers: elidex_api_observers::mutation::MutationObserverRegistry,
        /// `(callback, instance)` JS-identity binding per `MutationObserver`
        /// ID.  Keyed by the raw `MutationObserverId` u64 (matches the
        /// inline `ObjectKind::Observer { kind: Mutation, observer_id }`
        /// payload).  Both `ObjectId`s in each [`ObserverBinding`] are
        /// rooted via [`Self::gc_root_object_ids`] so the callback +
        /// instance wrapper survive any GC cycle while the observer is
        /// alive.
        ///
        /// **Retained across `Vm::unbind`** — keyed by VM-monotonic
        /// `observer_id`, not by `Entity` or recycled `ObjectId`, so
        /// cross-DOM aliasing does not apply.  A retained `mo`
        /// reference can re-`observe` after a rebind (same or
        /// different DOM) and have its callback fire.  Trade-off:
        /// this map grows monotonically with `new MutationObserver()`
        /// calls and is never shrunk — `disconnect()` per spec only
        /// clears observation targets, not the binding, and
        /// `Vm::unbind` intentionally retains the map.  Long-lived
        /// VMs that churn many observers accumulate dead entries;
        /// weak-rooting / sweep-time cleanup is tracked at
        /// `#11-mutation-observer-extras`.
        pub(crate) mutation_observer_bindings: HashMap<u64, ObserverBinding>,
        /// `ResizeObserver` registry (W3C Resize Observer §3) — owns
        /// the monotonic observer ID counter; observation target lists
        /// live as `ResizeObservedBy` components on the observed
        /// entities (same target-tracking model as
        /// `MutationObservedBy`).  Same `Vm::unbind` contract as
        /// [`Self::mutation_observers`]: the registry survives unbind
        /// (its ID space is VM-monotonic, no cross-DOM aliasing), so a
        /// retained `ro` reference re-observes after a rebind.
        /// Target-list scrubbing is implicit because the components live
        /// on entities that are despawned with the outgoing world.
        pub(crate) resize_observers: elidex_api_observers::resize::ResizeObserverRegistry,
        /// `(callback, instance)` binding per `ResizeObserver` ID.
        /// Same shape / retain-across-unbind contract as
        /// [`Self::mutation_observer_bindings`]; rooted via
        /// [`Self::gc_root_object_ids`].
        pub(crate) resize_observer_bindings: HashMap<u64, ObserverBinding>,
        /// `IntersectionObserver` registry (W3C Intersection Observer §3) —
        /// owns the monotonic observer ID counter + per-observer
        /// `IntersectionObserverInit` (root / rootMargin / thresholds).
        /// Same `Vm::unbind` contract as [`Self::mutation_observers`] +
        /// [`Self::resize_observers`]: registry survives unbind so
        /// retained `io` references can re-observe after rebind.
        pub(crate) intersection_observers:
            elidex_api_observers::intersection::IntersectionObserverRegistry,
        /// `(callback, instance)` binding per `IntersectionObserver` ID.
        /// Same contract as [`Self::resize_observer_bindings`].
        pub(crate) intersection_observer_bindings: HashMap<u64, ObserverBinding>,
        /// Origin-scoped `localStorage` backend (WHATWG HTML §11.2).
        /// Wrapped in `Arc` so multiple `HostData` instances (e.g. one
        /// per browsing-context VM) can share a single per-process
        /// manager + on-disk JSON tree.  `None` until the embedder
        /// calls [`Self::install_web_storage`] — the JS-visible
        /// `localStorage` natives operate on a private stub when no
        /// backend is installed (no panic, but the data is per-VM
        /// in-memory and lost on `Vm::unbind`, matching Chrome's
        /// "cookie-averse" fallback for `document.cookie`).
        /// A2: `Legacy` Web Storage backend — absent in `App` builds.
        #[cfg(feature = "compat-webapi")]
        web_storage: Option<Arc<WebStorageManager>>,
        /// Origin-keyed Cache API backend (WHATWG Service Workers §5,
        /// `#11-cache-api-vm` / D-19 PR-1, DR-A).  Wrapped in `Arc` so the
        /// same origin connection is shared across browsing-context VMs —
        /// and, in PR-2, handed to the service-worker thread (the wrapper
        /// is `Send + Sync`).  `None` until the shell calls
        /// [`Self::install_cache_storage`]; the `caches` natives then
        /// lazily mint an in-memory backend
        /// ([`super::super::VmInner::ensure_cache_backend`], boa parity) so
        /// headless / unit-test VMs still work (per-VM, lost on
        /// `Vm::unbind`).
        cache_backend: Option<Arc<CacheBackend>>,
        /// Per-VM `sessionStorage` backing (WHATWG HTML §11.2). In
        /// memory only; cleared on `Vm::unbind` (the spec models
        /// sessionStorage as scoped to a browsing context, so the
        /// existing bind/unbind cycle expresses that boundary).
        #[cfg(feature = "compat-webapi")]
        pub(crate) session_storage: SessionStorageState,
        /// Stable per-`HostData` opaque-origin sentinel — used for
        /// localStorage scoping when `Vm::navigation`'s URL has an
        /// opaque origin (`about:blank`, `data:`, …).  Generated at
        /// `HostData::new` via [`OPAQUE_ORIGIN_COUNTER`] so two such
        /// VMs in the same process do not alias on the manager's
        /// origin-keyed registry.
        /// A2: only `localStorage` scoping reads it — `compat-webapi`-gated.
        #[cfg(feature = "compat-webapi")]
        opaque_origin_sentinel: String,
        /// Fallback in-memory `localStorage` used when no
        /// `WebStorageManager` is installed.  Same `IndexMap` shape
        /// as the disk-backed path so `setItem` / `getItem` / `key`
        /// observe identical insertion-order semantics.
        ///
        /// Ephemeral by design: cleared on `Vm::unbind` (matching
        /// `session_storage` lifetime).  Tests that exercise
        /// localStorage persistence install a real
        /// `WebStorageManager`.
        #[cfg(feature = "compat-webapi")]
        pub(crate) fallback_local_storage: SessionStorageState,
        // -------------------------------------------------------------
        // D-8 PR-A2: Range / TreeWalker / NodeIterator live state
        // -------------------------------------------------------------
        /// HostData-side `LiveRangeRegistry` (WHATWG DOM §4.4 +
        /// §5.5).  Shares the `Arc<Mutex<HashMap<RangeId, Range>>>`
        /// hash with the `MutationBridge` installed via
        /// `EcsDom::set_mutation_hook` at `Vm::bind`.  Hook fires
        /// from engine-side mutations (insert / remove / text-change /
        /// replace-data / split-text / normalize-merge) apply
        /// boundary adjustments synchronously through the shared
        /// `Arc`.  See `crates/dom/elidex-dom-api/src/range/live.rs`
        /// for the consumer-side machinery and
        /// `crates/dom/elidex-dom-api/src/mutation_bridge.rs` for
        /// the multi-consumer wrapper that pairs with this field.
        ///
        /// **Persisted across unbind cycles?**  No — cleared by
        /// `clear` on `Vm::unbind` (range identity does not span
        /// rebinds; the JS-side Range wrappers re-register via
        /// `document.createRange()` after rebind).
        pub(crate) live_range_registry: elidex_dom_api::LiveRangeRegistry,
        /// Shared `Arc<Mutex<HashMap<NodeIteratorId, NodeIteratorState>>>`
        /// held jointly with the `MutationBridge` installed on
        /// `EcsDom`.  The shared half lets the bridge's
        /// `after_remove_with_descendants` callback apply WHATWG
        /// §6.1 pre-removing-steps to every registered iterator
        /// synchronously inside the engine fire site.
        ///
        /// **Lock ordering** (plan-v4 §A-NI-1 Round 2 IMP-3): the
        /// bridge acquires `live_range_registry`'s inner `ranges`
        /// lock FIRST, releases, then acquires this map's lock —
        /// **no nested locking**.  Enforced syntactically by
        /// disjoint-block scoping inside
        /// `MutationBridge::after_remove_with_descendants`.
        pub(crate) node_iterator_states_shared:
            std::sync::Arc<std::sync::Mutex<HashMap<u64, elidex_dom_api::NodeIteratorState>>>,
        /// Per-iterator wrapper `ObjectId` cache.  Mirrors the
        /// `mutation_observer_instances` pattern — keyed by the
        /// monotonic `iterator_id: u64` carried inline in
        /// `ObjectKind::NodeIterator { iterator_id }`.  Sweep tail
        /// prunes dead entries (`unregister`s the iterator from
        /// `node_iterator_states_shared`).
        pub(crate) node_iterator_instances: HashMap<u64, ObjectId>,
        /// Monotonic counter for `NodeIterator` IDs.  Per lesson
        /// #217 (post-unbind static sentinel stability): does NOT
        /// reset on `Vm::unbind` — retained references to a
        /// `NodeIterator` re-bind across DOM swaps without
        /// collision.
        pub(crate) next_node_iterator_id: u64,
        /// VM-local `TreeWalker` state map.  NOT shared with the
        /// bridge — WHATWG DOM §6.4 has **no** pre-removing-steps
        /// for `TreeWalker` (only §6.1 NodeIterator does); the
        /// walker's `currentNode` is allowed to go stale and
        /// recovers lazily on the next traversal call.  See plan
        /// v4 §A-NI-1 Round 2 IMP-2 verification.
        pub(crate) tree_walker_states: HashMap<u64, TreeWalkerState>,
        /// Per-walker wrapper `ObjectId` cache.  Sweep tail prunes
        /// dead entries.
        pub(crate) tree_walker_instances: HashMap<u64, ObjectId>,
        /// Monotonic counter for `TreeWalker` IDs (lesson #217 —
        /// does NOT reset on `Vm::unbind`).
        pub(crate) next_tree_walker_id: u64,
        /// Per-Range wrapper `ObjectId` cache.  Keyed by the inner
        /// `RangeId` bits.  Sweep tail prunes dead entries +
        /// unregisters the Range from
        /// `live_range_registry.ranges`.
        pub(crate) range_instances: HashMap<u64, ObjectId>,
        /// Per-document `Selection` singleton state (Selection API §3,
        /// formerly WHATWG HTML §7.5.5).  M4-12 single-document VM
        /// models exactly one Selection per VM, so `Option<...>` is the
        /// right shape — promote to `HashMap<EntityBits, SelectionState>`
        /// when multi-document arrives (D-15 ShadowRoot / iframe).
        ///
        /// Lifecycle: created lazily on first `window.getSelection()` /
        /// `document.getSelection()` call (or first internal mutation
        /// via the Selection prototype dispatchers).  Cleared on
        /// `Vm::unbind` because the registered `RangeId` references
        /// live in `live_range_registry`, which is also cleared.
        pub(crate) selection_state: Option<elidex_dom_api::SelectionState>,
        /// Canonical `[SameObject]` wrapper for the per-document
        /// Selection.  `window.getSelection()` and
        /// `document.getSelection()` both return THIS `ObjectId` —
        /// identity is preserved per spec §2 / Chrome behaviour.
        /// GC sweep tail clears this slot when the wrapper becomes
        /// unreachable; the next `getSelection()` call materialises a
        /// fresh wrapper backed by the same `selection_state`.
        pub(crate) selection_instance: Option<ObjectId>,
        /// "Selection task source" dirty flag per HTML §8.1.7.1.  Set
        /// on any user-script Selection / Range mutation; the
        /// pending-task drain checks the flag at eval boundary and
        /// fires a coalesced `selectionchange` at the document per
        /// Selection API §3.4 (one event per microtask checkpoint
        /// regardless of how many discrete mutations happened).
        pub(crate) selectionchange_pending: bool,
        // -------------------------------------------------------------
        // D-12 #11-net-ws-sse: WebSocket / EventSource side-tables
        // -------------------------------------------------------------
        /// Per-`WebSocket` instance out-of-band state — see
        /// [`WebSocketState`] for the per-instance fields (4-state
        /// readyState, url, protocol, extensions, bufferedAmount,
        /// binaryType, broker conn_id, 4 on* handler `ObjectId`s).
        /// Lives on HostData (not VmInner) per plan v1.1 IMP-7
        /// because the underlying broker I/O thread dies on
        /// `Vm::unbind` (along with the `network_handle`), so
        /// JS-visible WS state must die with it.  `Vm::unbind`
        /// snapshots the conn_ids here BEFORE clearing the map so
        /// it can emit `WebSocketClose` per conn_id to the
        /// outgoing handle (CRIT-A: mirror
        /// `reject_pending_fetches_with_error`'s teardown order).
        pub(crate) websocket_states: HashMap<ObjectId, WebSocketState>,
        /// Reverse lookup from broker `conn_id` to the instance
        /// `ObjectId` — populated at ctor time and consumed by the
        /// extended `tick_network` drain so incoming `WsEvent`s can
        /// route back to the right wrapper.  Cleared alongside
        /// `websocket_states` on `Vm::unbind`.
        pub(crate) ws_conn_to_object: HashMap<u64, ObjectId>,
        /// Monotonic per-VM WS connection ID counter.  Resets on
        /// `Vm::unbind` (the broker assigns its own
        /// `WsId` internally; renderers and broker maintain
        /// independent counters that meet only via the
        /// renderer-assigned `conn_id` carried in
        /// `RendererToNetwork::WebSocketOpen` / `WebSocketSend` /
        /// `WebSocketClose`).
        pub(crate) ws_next_conn_id: u64,
        /// Per-`EventSource` instance out-of-band state — see
        /// [`EventSourceState`] for the per-instance fields (3-state
        /// readyState, url, withCredentials, sticky lastEventId,
        /// broker conn_id, 3 on* handler `ObjectId`s, per-instance
        /// `addEventListener` registry for spec-mandated named-event
        /// delivery).  Same HostData rationale as
        /// `websocket_states`.
        pub(crate) event_source_states: HashMap<ObjectId, EventSourceState>,
        /// Reverse lookup from broker `conn_id` to the instance
        /// `ObjectId` for SSE event routing.  Same `Vm::unbind`
        /// contract as `ws_conn_to_object`.
        pub(crate) sse_conn_to_object: HashMap<u64, ObjectId>,
        /// Monotonic per-VM SSE connection ID counter; resets on
        /// `Vm::unbind`.
        pub(crate) sse_next_conn_id: u64,
        // -------------------------------------------------------------
        // D-17 `#11-custom-elements-vm`: Custom Elements v1 state
        // (WHATWG HTML §4.13)
        // -------------------------------------------------------------
        /// Per-realm custom element registry (HTML §4.13.4) — owns the
        /// `name → CustomElementDefinition` map + the per-name pending-
        /// upgrade entity queue. Shared via `Arc<Mutex<>>` with
        /// [`elidex_custom_elements::CustomElementReactionConsumer`]
        /// (which only reads `observed_attributes`). Cleared on
        /// `Vm::unbind` because each `CustomElementDefinition`
        /// references its constructor by a `u64` ID that aliases the
        /// per-VM `custom_element_constructors` map below — neither
        /// survives an unbind crossing.
        pub(crate) ce_registry:
            std::sync::Arc<std::sync::Mutex<elidex_custom_elements::CustomElementRegistry>>,
        /// Per-VM reaction queue (HTML §4.13.6) — pushed to by the
        /// `CustomElementReactionConsumer` on Insert / Remove /
        /// AttributeChange, drained at script-execution / event-
        /// dispatch / microtask checkpoints by `flush_ce_reactions`.
        /// Shared via `Arc<Mutex<>>` with the consumer.
        pub(crate) ce_reaction_queue: std::sync::Arc<
            std::sync::Mutex<
                std::collections::VecDeque<elidex_custom_elements::CustomElementReaction>,
            >,
        >,
        /// VM-monotonic constructor ID counter — assigned in `define()`
        /// and stored on the `CustomElementDefinition::constructor_id`.
        pub(crate) ce_next_constructor_id: u64,
        /// `constructor_id → constructor ObjectId` map. The constructor
        /// `ObjectId` is per-VM identity (HostData exception (a) — see
        /// `feedback_boa-hostbridge-port-is-not-a-registry.md`), so this
        /// stays on HostData not as an ECS component. Cleared on unbind.
        pub(crate) ce_constructors: HashMap<u64, ObjectId>,
        /// Reverse of [`Self::ce_constructors`]: `constructor ObjectId →
        /// constructor_id`. Populated + cleared in lockstep with the
        /// forward map. This is the **single SoT** for reverse-mapping
        /// `new.target` back to its registered `constructor_id` inside
        /// [`super::custom_elements::html_element::native_html_element_ctor`]
        /// (\[C1\] §3.2.3 step 5). Keeping the map host-side instead of
        /// stamping a JS-visible property on the ctor makes spoofing
        /// impossible by construction — user code with access to a
        /// registered ctor can no longer copy a brand to a different
        /// object to impersonate the registered definition. No extra GC
        /// roots: the values (ObjectIds) are the same set as
        /// `ce_constructors.values()` already rooted in `cf_ce_roots`.
        pub(crate) ce_constructor_to_id: HashMap<ObjectId, u64>,
        /// Cached pending `whenDefined()` Promise per CE name — returns
        /// the same Promise for repeated calls with the same name (per
        /// WHATWG §4.13.4 step 3 — "Set promise to a new promise" runs
        /// once; later calls return the previously stored promise).
        pub(crate) ce_when_defined_promises: HashMap<String, ObjectId>,
    }

    /// `WebSocket.binaryType` enum (WHATWG WebSockets §9.3).
    ///
    /// Default is `Blob` per spec.  WebIDL `enum BinaryType {
    /// "blob", "arraybuffer" }` — assignment to any other string
    /// throws TypeError (handled in the setter, not here — Phase 2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum BinaryType {
        /// Default: incoming binary frames are delivered as `Blob`
        /// instances on the next `MessageEvent.data`.
        #[default]
        Blob,
        /// Incoming binary frames are delivered as `ArrayBuffer`
        /// instances.  Set via `ws.binaryType = "arraybuffer"`.
        ArrayBuffer,
    }

    /// Per-`WebSocket` instance out-of-band state.  Held on
    /// `HostData` in the `websocket_states` map keyed by the
    /// instance `ObjectId`.
    ///
    /// Lives on HostData (not VmInner) because the broker handle's
    /// per-conn_id state dies on `Vm::unbind`; JS-visible state has
    /// to die with it.  `transition_to` enforces WHATWG §9.3 state
    /// monotonicity (CONNECTING → OPEN → CLOSING → CLOSED, no
    /// backward moves, no CONNECTING → CLOSED direct without an
    /// intermediate CLOSING — same-state is a no-op).
    #[derive(Debug)]
    pub struct WebSocketState {
        /// Current readyState bucket (matches the JS-visible
        /// `WebSocket.readyState` integer).
        pub ready_state: elidex_api_ws::WsReadyState,
        /// Constructor URL after normalization + validation (so
        /// `ws.url` echoes back the post-promotion `ws://` /
        /// `wss://` form per WHATWG §9.3.4 step 2 "URL serializer").
        pub url: String,
        /// Pre-interned WebSocket-URL origin (per WHATWG §9.3.7 the
        /// `MessageEvent.origin` for incoming frames is the server's
        /// origin, NOT the page origin).  Cached once at
        /// constructor time so per-message dispatch reads a
        /// `StringId` from the side-table rather than re-parsing
        /// `url` + serialising the origin on every event.  Opaque
        /// origins serialise to the literal `"null"`.
        pub origin_sid: StringId,
        /// Negotiated sub-protocol — `""` until `WsEvent::Connected`
        /// supplies the value, then frozen.
        pub protocol: String,
        /// Negotiated extensions — `""` until Connected, then frozen.
        pub extensions: String,
        /// Bytes queued for transmission but not yet flushed.  JS
        /// `send()` increments via `saturating_add`; broker
        /// `WsEvent::BytesSent(n)` decrements via `saturating_sub`.
        /// `u64` to match the spec's `unsigned long long`.
        pub buffered_amount: u64,
        /// Current binaryType — see [`BinaryType`].
        pub binary_type: BinaryType,
        /// Broker connection ID — paired with HostData's
        /// `ws_conn_to_object` reverse map for event routing and
        /// emitted on `WsCommand` / `WebSocketClose` messages.
        pub conn_id: u64,
        // `onopen` / `onmessage` / `onerror` / `onclose` are NOT stored
        // here: since `#11-realtime-event-listeners` they live (with all
        // `addEventListener` listeners) in the unified
        // `VmInner::vm_event_listeners` home, dispatched through the
        // shared §2.9 VmObject core.
    }

    impl WebSocketState {
        /// Apply a `readyState` transition.  Per WHATWG §9.3 the
        /// state machine is strictly monotonic — once OPEN the
        /// only legal next states are CLOSING / CLOSED, and once
        /// CLOSING the only legal next state is CLOSED.  Same-state
        /// transitions are accepted as no-ops (idempotent
        /// `close()` calls land here).
        ///
        /// Returns `Err(&'static str)` on an illegal transition
        /// AND fires a `debug_assert!` — the wrong direction is
        /// always a code bug, never user-driven.  Production
        /// release builds receive the Result so the caller can
        /// short-circuit gracefully (e.g. drop a stale broker
        /// event after `Vm::unbind` snapshotted the state).
        pub fn transition_to(
            &mut self,
            new: elidex_api_ws::WsReadyState,
        ) -> Result<(), &'static str> {
            use elidex_api_ws::WsReadyState::{Closed, Closing, Connecting, Open};
            // Legal transitions per WHATWG §9.3 (CONNECTING →
            // anything; OPEN → forward-or-same; CLOSING →
            // forward-or-same; CLOSED → CLOSED only).  Same-state
            // pairs are idempotent no-ops folded into each arm.
            // Wildcard `_` returns `false` for backward /
            // terminal-exit pairs (Open→Connecting, Closing→
            // Connecting / Open, Closed→Connecting / Open /
            // Closing).
            let ok = matches!(
                (self.ready_state, new),
                (Connecting, _)
                    | (Open, Open | Closing | Closed)
                    | (Closing, Closing | Closed)
                    | (Closed, Closed)
            );
            if ok {
                self.ready_state = new;
                Ok(())
            } else {
                debug_assert!(
                    false,
                    "illegal WebSocket readyState transition {:?} → {:?}",
                    self.ready_state, new
                );
                Err("illegal WebSocket readyState transition")
            }
        }
    }

    /// Per-`EventSource` instance out-of-band state.  Held on
    /// `HostData` in the `event_source_states` map keyed by the
    /// instance `ObjectId`.
    ///
    /// SSE has a 3-state machine (no CLOSING — error / fatal /
    /// close all land directly).  The transient `Error` event
    /// moves OPEN → CONNECTING during automatic reconnect, then
    /// `Connected` snaps back to OPEN.  `FatalError` and JS
    /// `close()` are terminal.
    #[derive(Debug)]
    pub struct EventSourceState {
        pub ready_state: elidex_api_ws::SseReadyState,
        /// Constructor URL after parse + relative-resolution.  No
        /// scheme promotion (SSE is HTTP-only).
        pub url: String,
        /// Pre-interned origin used as `MessageEvent.origin` on
        /// every dispatched server event.  Seeded at constructor
        /// time from the ctor URL's origin as a defensive default
        /// for the (unreachable-in-practice) pre-Connected window,
        /// then refreshed at every `Connected` dispatch
        /// (`vm::host::event_source_dispatch::dispatch_sse_connected`)
        /// from the post-redirect final URL's origin per WHATWG
        /// HTML §9.2 "Dispatch the event".  Opaque-origin
        /// serialisation returns the literal `"null"`, but SSE's
        /// HTTP(S) scheme gate (`connect.rs`) makes that path
        /// unreachable in practice.
        pub origin_sid: StringId,
        /// `init.withCredentials` echo.
        pub with_credentials: bool,
        /// Sticky lastEventId — broker emits cumulative value per
        /// HTML §9.2 (IMP-5).  Tracked here so the JS surface
        /// `lastEventId` accessor (not in this PR — handler-only)
        /// + reconnect's `Last-Event-ID` header are kept in sync.
        pub last_event_id: String,
        /// Broker connection ID — paired with HostData's
        /// `sse_conn_to_object` reverse map.
        pub conn_id: u64,
        // `onopen` / `onmessage` / `onerror` and the bespoke per-type
        // `event_listeners` registry are NOT stored here: since
        // `#11-realtime-event-listeners` every listener (on* handlers +
        // named-event `addEventListener` registrations) lives in the
        // unified `VmInner::vm_event_listeners` home, dispatched through
        // the shared §2.9 VmObject core.  Named-event delivery + the
        // §9.2.6 "message vs named" fan-out are emergent from it.
    }

    impl EventSourceState {
        /// Apply an SSE `readyState` transition.  Legal moves:
        /// - `Connecting → Open` (handshake completed)
        /// - `Open → Connecting` (transient `SseEvent::Error`,
        ///   auto-reconnect IMP-3)
        /// - `Connecting → Closed` / `Open → Closed` (JS
        ///   `close()` or `SseEvent::FatalError`)
        /// - Same-state = idempotent
        ///
        /// Illegal moves trip `debug_assert!` and return Err.
        pub fn transition_to(
            &mut self,
            new: elidex_api_ws::SseReadyState,
        ) -> Result<(), &'static str> {
            use elidex_api_ws::SseReadyState::{Closed, Connecting, Open};
            // Legal transitions per WHATWG HTML §9.2 — CONNECTING
            // and OPEN can flow to any state (OPEN→CONNECTING is
            // the legitimate auto-reconnect path on transient
            // `SseEvent::Error` per IMP-3), CLOSED is terminal.
            let ok = matches!(
                (self.ready_state, new),
                (Connecting | Open, _) | (Closed, Closed)
            );
            if ok {
                self.ready_state = new;
                Ok(())
            } else {
                debug_assert!(
                    false,
                    "illegal EventSource readyState transition {:?} → {:?}",
                    self.ready_state, new
                );
                Err("illegal EventSource readyState transition")
            }
        }
    }

    /// VM-local TreeWalker state.  WHATWG DOM §6.4.
    ///
    /// Holds NodeFilter callback as opaque `Option<u64>` (ObjectId
    /// bits) per the engine-indep layering precedent in
    /// `NodeIteratorState` (Round 2 IMP-2).
    #[derive(Debug, Clone)]
    pub struct TreeWalkerState {
        /// `root` per spec §6.4 — never mutates after construction.
        pub root: Entity,
        /// `whatToShow` bitmask per spec §6.3.
        pub what_to_show: u32,
        /// VM-side filter callback `ObjectId` bits.  `None` for
        /// "no filter" (every node ACCEPTed without callback).
        pub filter_object_id: Option<u64>,
        /// `currentNode` per spec §6.4.
        pub current: Entity,
        /// Active-flag for filter re-entrancy detection (§6.3 step 2).
        pub active: bool,
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
            // D-8 PR-A2: create the shared `Arc<Mutex<HashMap>>` that
            // binds HostData to the future `MutationBridge` via the
            // canonical pair-construction in `Vm::bind`.  Held empty
            // here; populated by VM-side NodeIterator ctor +
            // adjusted by the bridge's
            // `after_remove_with_descendants` hook.  The
            // `LiveRangeRegistry` factory below produces the half
            // owned by HostData; `Vm::bind` swaps it for the
            // multi-consumer pair when wiring the hook.
            let (live_range_registry, _initial_bridge_unused) =
                elidex_dom_api::LiveRangeRegistry::new_pair();
            Self {
                session_ptr: std::ptr::null_mut(),
                dom_ptr: std::ptr::null_mut(),
                document_entity: None,
                window_entity: None,
                document_methods_installed: HashSet::new(),
                bind_epoch: 0,
                listener_store: HashMap::new(),
                wrapper_store: HashMap::new(),
                tab_hidden: false,
                cookie_jar: None,
                sandbox_flags: None,
                document_origin_override: None,
                fallback_opaque_origin: elidex_plugin::SecurityOrigin::opaque(),
                iframe_depth: 0,
                mutation_observers: elidex_api_observers::mutation::MutationObserverRegistry::new(),
                mutation_observer_bindings: HashMap::new(),
                resize_observers: elidex_api_observers::resize::ResizeObserverRegistry::new(),
                resize_observer_bindings: HashMap::new(),
                intersection_observers:
                    elidex_api_observers::intersection::IntersectionObserverRegistry::new(),
                intersection_observer_bindings: HashMap::new(),
                #[cfg(feature = "compat-webapi")]
                web_storage: None,
                cache_backend: None,
                #[cfg(feature = "compat-webapi")]
                session_storage: SessionStorageState::new(),
                #[cfg(feature = "compat-webapi")]
                opaque_origin_sentinel: next_opaque_origin_id(),
                #[cfg(feature = "compat-webapi")]
                fallback_local_storage: SessionStorageState::new(),
                live_range_registry,
                node_iterator_states_shared: std::sync::Arc::new(std::sync::Mutex::new(
                    HashMap::new(),
                )),
                node_iterator_instances: HashMap::new(),
                next_node_iterator_id: 0,
                tree_walker_states: HashMap::new(),
                tree_walker_instances: HashMap::new(),
                next_tree_walker_id: 0,
                range_instances: HashMap::new(),
                selection_state: None,
                selection_instance: None,
                selectionchange_pending: false,
                websocket_states: HashMap::new(),
                ws_conn_to_object: HashMap::new(),
                ws_next_conn_id: 0,
                event_source_states: HashMap::new(),
                sse_conn_to_object: HashMap::new(),
                sse_next_conn_id: 0,
                ce_registry: std::sync::Arc::new(std::sync::Mutex::new(
                    elidex_custom_elements::CustomElementRegistry::new(),
                )),
                ce_reaction_queue: std::sync::Arc::new(std::sync::Mutex::new(
                    std::collections::VecDeque::new(),
                )),
                ce_next_constructor_id: 0,
                ce_constructors: HashMap::new(),
                ce_constructor_to_id: HashMap::new(),
                ce_when_defined_promises: HashMap::new(),
            }
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

        /// Install the shell-owned `WebStorageManager` (idempotent
        /// replace).  Tests that exercise `localStorage` persistence
        /// across VM lifetimes pass a `tempfile::tempdir()`-based
        /// manager here; production embedders share one
        /// `Arc<WebStorageManager>` across browsing-context VMs.
        #[cfg(feature = "compat-webapi")]
        pub fn install_web_storage(&mut self, manager: Arc<WebStorageManager>) {
            self.web_storage = Some(manager);
        }

        /// Borrow the installed `WebStorageManager`, if any.  Used by
        /// the `vm/host/storage.rs` natives to route `getItem` /
        /// `setItem` / `removeItem` / `clear` / `length` / `key` for
        /// `localStorage`.  Returns `None` when no backend is
        /// installed; callers fall back to
        /// [`Self::fallback_local_storage`] in that case.
        #[cfg(feature = "compat-webapi")]
        pub(crate) fn web_storage(&self) -> Option<&Arc<WebStorageManager>> {
            self.web_storage.as_ref()
        }

        /// Install the shell-owned origin Cache API backend (DR-A,
        /// `#11-cache-api-vm`).  Production embedders share one
        /// `Arc<CacheBackend>` per origin across browsing-context VMs (and,
        /// in PR-2, the service-worker thread) so the `caches` store is
        /// consistent within a session; tests pass an in-memory one.
        /// Idempotent replace.
        ///
        /// `pub(crate)` for PR-1 (window-realm only — the lazy in-memory
        /// fallback in `VmInner::ensure_cache_backend` is the sole caller).
        /// PR-2 / D-26 promotes this to `pub` + re-exports `CacheBackend`
        /// when the shell wires the origin-shared handle across the
        /// service-worker spawn boundary (§4.1).
        pub(crate) fn install_cache_storage(&mut self, backend: Arc<CacheBackend>) {
            self.cache_backend = Some(backend);
        }

        /// Borrow the installed Cache API backend, if any.  `None` until the
        /// shell installs one or [`super::super::VmInner::ensure_cache_backend`]
        /// lazily mints an in-memory fallback.
        pub(crate) fn cache_backend(&self) -> Option<&Arc<CacheBackend>> {
            self.cache_backend.as_ref()
        }

        /// Stable per-VM opaque-origin string (e.g. `"opaque-origin:7"`).
        /// Used by `vm/host/storage.rs` for `localStorage` scoping when
        /// the current navigation URL's origin is opaque.
        #[cfg(feature = "compat-webapi")]
        pub(crate) fn opaque_origin_sentinel(&self) -> &str {
            &self.opaque_origin_sentinel
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

        /// Install this document's iframe sandbox flags (WHATWG HTML
        /// §7.1.5).  Pass `None` for a top-level / unsandboxed document
        /// (the default).  The shell parses the iframe `sandbox=""`
        /// attribute into `IframeSandboxFlags` and installs them here when
        /// a document loads inside a sandboxed iframe.
        pub fn set_sandbox_flags(&mut self, flags: Option<elidex_plugin::IframeSandboxFlags>) {
            self.sandbox_flags = flags;
        }

        /// Whether scripting is enabled for this browsing context
        /// (WHATWG HTML §8.1.3.4 "scripting is disabled" — gated by the
        /// §7.1.5 sandboxed scripts flag).  `true` when not sandboxed or
        /// when the sandbox grants `allow-scripts`; the eval gate
        /// short-circuits to a silent success when this is `false`.
        pub(crate) fn scripts_allowed(&self) -> bool {
            self.sandbox_flags
                .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS))
        }

        /// The sandbox flags for this document's browsing context, if it is
        /// sandboxed (`None` for top-level / unsandboxed).  Read by the shell
        /// (via the `ElidexJsEngine` forwarder) to gate `target="_blank"`
        /// navigation; the per-capability `allow-*` predicates below are the
        /// preferred entry points.
        pub(crate) fn sandbox_flags(&self) -> Option<elidex_plugin::IframeSandboxFlags> {
            self.sandbox_flags
        }

        /// Whether form submission is allowed (sandbox `allow-forms`; WHATWG
        /// HTML §7.1.5).  `true` when not sandboxed or when the flag is
        /// granted — the same `is_none_or` shape as [`Self::scripts_allowed`].
        pub(crate) fn forms_allowed(&self) -> bool {
            self.sandbox_flags
                .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_FORMS))
        }

        /// Whether popups are allowed (sandbox `allow-popups`; WHATWG HTML
        /// §7.1.5).  `true` when not sandboxed or when the flag is granted.
        pub(crate) fn popups_allowed(&self) -> bool {
            self.sandbox_flags
                .is_none_or(|f| f.contains(elidex_plugin::IframeSandboxFlags::ALLOW_POPUPS))
        }

        /// Install the document's security origin (WHATWG HTML §7.1.1).  The
        /// embedder's load path computes it (`SecurityOrigin::from_url`, or the
        /// opaque sandbox origin via the shell's `apply_sandbox_origin_from_flags`)
        /// and installs it before scripts run; [`super::VmInner::document_origin`]
        /// reads it (falling back to the `current_url`-derived origin when unset).
        pub(crate) fn set_origin(&mut self, origin: elidex_plugin::SecurityOrigin) {
            self.document_origin_override = Some(origin);
        }

        /// The installed document-origin override, if any.  `None` ⇒ the
        /// resolver derives the origin from `current_url`
        /// (see [`super::VmInner::document_origin`]).
        pub(crate) fn document_origin_override(&self) -> Option<&elidex_plugin::SecurityOrigin> {
            self.document_origin_override.as_ref()
        }

        /// The per-VM stable opaque fallback origin (see the field docs) — used
        /// by [`super::VmInner::document_origin`] for the no-override +
        /// opaque-`current_url` case so the origin stays identity-stable.
        pub(crate) fn fallback_opaque_origin(&self) -> &elidex_plugin::SecurityOrigin {
            &self.fallback_opaque_origin
        }

        /// Set the iframe nesting depth (`0` = top-level).
        pub(crate) fn set_iframe_depth(&mut self, depth: usize) {
            self.iframe_depth = depth;
        }

        /// The iframe nesting depth of this document's browsing context.
        pub(crate) fn iframe_depth(&self) -> usize {
            self.iframe_depth
        }

        /// Set the page-visibility state (WHATWG HTML §6.2), driven by the
        /// embedding shell on tab show/hide.  `visible = false` ⇒ hidden.
        pub(crate) fn set_visibility(&mut self, visible: bool) {
            self.tab_hidden = !visible;
        }

        /// Whether this document's top-level browsing context is hidden
        /// (WHATWG HTML §6.2) — backs `document.hidden`.
        pub(crate) fn is_tab_hidden(&self) -> bool {
            self.tab_hidden
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

        /// Engine-component brand check: `entity` carries an
        /// `elidex_ecs::ShadowRoot` component in the currently-bound
        /// DOM.  Sibling of [`Self::is_element_entity`] with identical
        /// aliasing contract.
        ///
        /// Used to distinguish ShadowRoot wrappers from other DOM
        /// wrappers post-H-migration ([feedback_objectkind-resolution-uniformity]):
        /// both kinds are `ObjectKind::HostObject { entity_bits }` and
        /// the ECS component is the only discriminator.
        #[allow(unsafe_code)]
        pub fn is_shadow_root_entity(&self, entity: Entity) -> bool {
            if !self.is_bound() {
                return false;
            }
            // SAFETY: see `is_element_entity` — same contract.
            let dom = unsafe { &*self.dom_ptr };
            dom.world().get::<&elidex_ecs::ShadowRoot>(entity).is_ok()
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
        /// prototype.  Probes a small set of ECS components
        /// (`TagType` → `Element` fast path for the ~99% case, then
        /// `ShadowRoot` to disambiguate shadow-root DF entities, then
        /// `node_kind` for the remaining non-Element node families)
        /// with a legacy-payload fallback (`TextContent` / `CommentData`
        /// / `DocTypeData`) for entities lacking an explicit `NodeKind`
        /// component.  Returns [`PrototypeKind::OtherNode`] when the
        /// `HostData` is not bound (pre-bind wrapper allocation paths).
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
            // behaviour of `is_element_entity` short-circuit).  Shadow
            // root entities lack `TagType` so this probe naturally
            // short-circuits to the non-shadow path for the ~99% case;
            // the `ShadowRoot` probe below disambiguates SR-vs-plain-DF
            // only for the DF-typed minority without paying for an
            // extra lookup on every Element wrapper allocation.
            if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                return PrototypeKind::Element;
            }
            // ShadowRoot entities carry `NodeKind::DocumentFragment`
            // but must route through `ShadowRoot.prototype` (not
            // `DocumentFragment.prototype`) for accurate brand checks
            // (`shadowRoot instanceof ShadowRoot === true`).  Probe
            // after the Element fast-path so non-shadow DF allocations
            // also benefit from the early TagType short-circuit.
            if dom.world().get::<&elidex_ecs::ShadowRoot>(entity).is_ok() {
                return PrototypeKind::ShadowRoot;
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
                Some(NodeKind::DocumentFragment) => PrototypeKind::DocumentFragment,
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
            self.wrapper_store
                .get(&WrapperKey::entity(entity, WrapperKind::Node))
                .copied()
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
            let prev = self
                .wrapper_store
                .insert(WrapperKey::entity(entity, WrapperKind::Node), obj);
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
            self.wrapper_store
                .remove(&WrapperKey::entity(entity, WrapperKind::Node))
        }

        pub fn gc_root_object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
            // Copilot R8: `TreeWalker` / `NodeIterator` filters are
            // NOT rooted here — doing so created a leak cycle when
            // the filter closure captured the wrapper (filter root →
            // closure trace → wrapper marked → state table entry
            // preserved → filter stays rooted → ...).  Filters are
            // instead reached via per-wrapper trace fan-out (see
            // `trace_work_list`'s `TreeWalker` / `NodeIterator`
            // arms), so an unreachable wrapper drops its filter
            // root naturally on the same sweep.
            //
            // Node wrappers (the former `wrapper_cache`) are NOT chained
            // here: they are strong-marked by the unified wrapper-store mark
            // loop in `gc/roots.rs` via the `MarkAgent::StrongRoot` arm
            // (`#11-wrapper-identity-seam`).
            // Three per-kind binding maps × 2 ObjectIds per entry
            // — `[b.callback, b.instance]` is flat-mapped inline so
            // adding a 4th observer kind is a single entry in the
            // binding-map array here rather than a new pair of
            // `chain(...)` calls.
            self.listener_store
                .values()
                .copied()
                .chain(
                    [
                        &self.mutation_observer_bindings,
                        &self.resize_observer_bindings,
                        &self.intersection_observer_bindings,
                    ]
                    .into_iter()
                    .flat_map(|m| m.values())
                    .flat_map(|b| [b.callback, b.instance]),
                )
                // D-17 `#11-custom-elements-vm`: every registered CE
                // constructor + cached whenDefined Promise must stay
                // GC-rooted for the registry's lifetime — otherwise an
                // upgrade after a major GC cycle would dereference a
                // freed `ObjectId`. Both maps are cleared on
                // `Vm::unbind` so the roots release on rebind.
                .chain(self.ce_constructors.values().copied())
                .chain(self.ce_when_defined_promises.values().copied())
        }

        /// GC trace fan-out accessor for `TreeWalker.filter_object_id`
        /// lookup.  Copilot R8 — filter is reached only when the
        /// wrapper itself is reachable via this state table.
        pub(crate) fn tree_walker_states_ref(&self) -> &HashMap<u64, TreeWalkerState> {
            &self.tree_walker_states
        }
        pub(crate) fn tree_walker_instances_ref(&self) -> &HashMap<u64, ObjectId> {
            &self.tree_walker_instances
        }
        pub(crate) fn node_iterator_states_shared_ref(
            &self,
        ) -> &std::sync::Arc<std::sync::Mutex<HashMap<u64, elidex_dom_api::NodeIteratorState>>>
        {
            &self.node_iterator_states_shared
        }
        pub(crate) fn node_iterator_instances_ref(&self) -> &HashMap<u64, ObjectId> {
            &self.node_iterator_instances
        }

        /// `[SameObject]` Range wrapper cache accessor for the GC trace
        /// fan-out (Selection → current Range wrapper).
        pub(crate) fn range_instances_ref(&self) -> &HashMap<u64, ObjectId> {
            &self.range_instances
        }

        /// Selection singleton wrapper ObjectId, if a `getSelection()`
        /// call has already materialised one.
        pub(crate) fn selection_instance_id(&self) -> Option<ObjectId> {
            self.selection_instance
        }

        /// Read access to the per-document Selection state for the GC
        /// trace fan-out — returns the currently-active `RangeId.bits()`
        /// when a range is set.
        pub(crate) fn selection_active_range_id_bits(&self) -> Option<u64> {
            self.selection_state
                .as_ref()
                .and_then(elidex_dom_api::SelectionState::current_range_id)
                .map(|rid| rid.0)
        }

        // -------------------------------------------------------------
        // D-12 #11-net-ws-sse: ID allocation + side-table accessors
        // -------------------------------------------------------------

        /// Allocate a fresh per-VM WebSocket connection ID.  Counter
        /// resets on `Vm::unbind` so connection IDs are scoped to
        /// the current bind cycle, matching the broker handle's
        /// own lifetime.
        pub(crate) fn alloc_ws_conn_id(&mut self) -> u64 {
            let id = self.ws_next_conn_id;
            self.ws_next_conn_id = self.ws_next_conn_id.wrapping_add(1);
            id
        }

        /// Allocate a fresh per-VM SSE connection ID.  Same scope
        /// contract as [`Self::alloc_ws_conn_id`].
        pub(crate) fn alloc_sse_conn_id(&mut self) -> u64 {
            let id = self.sse_next_conn_id;
            self.sse_next_conn_id = self.sse_next_conn_id.wrapping_add(1);
            id
        }

        /// Drain the WebSocket / EventSource side-tables and
        /// produce the broker `conn_id` lists that
        /// `Vm::unbind` must close BEFORE clearing state, per
        /// CRIT-A.  Returns `(ws_conn_ids, sse_conn_ids)`; the
        /// caller emits `WebSocketClose` / `EventSourceClose` per
        /// id and then this method's *post-drain* state is the
        /// cleared baseline for the next `bind`.
        ///
        /// Counter fields (`ws_next_conn_id` / `sse_next_conn_id`)
        /// also reset so the next bind starts fresh — matches the
        /// broker handle's lifetime contract.
        pub(crate) fn drain_realtime_for_unbind(&mut self) -> (Vec<u64>, Vec<u64>) {
            let ws_conns: Vec<u64> = self.ws_conn_to_object.keys().copied().collect();
            let sse_conns: Vec<u64> = self.sse_conn_to_object.keys().copied().collect();
            self.websocket_states.clear();
            self.ws_conn_to_object.clear();
            self.ws_next_conn_id = 0;
            self.event_source_states.clear();
            self.sse_conn_to_object.clear();
            self.sse_next_conn_id = 0;
            (ws_conns, sse_conns)
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
#[cfg(feature = "engine")]
pub use engine_feature::TreeWalkerState;
#[cfg(feature = "engine")]
pub use engine_feature::{BinaryType, EventSourceState, WebSocketState};
