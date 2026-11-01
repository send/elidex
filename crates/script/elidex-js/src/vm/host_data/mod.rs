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

/// The S5-6a cross-context effect queues ([`HostEffectQueues`]) + their
/// enqueue/take methods on `HostData` — carved out of this file as a bounded
/// cohesion seam.
#[cfg(feature = "engine")]
mod effect_queues;

#[cfg(feature = "engine")]
mod engine_feature {
    use super::super::host::cache::CacheBackend;
    use super::super::host::observer_common::ObserverBinding;
    use super::super::value::ObjectId;
    use super::super::wrapper_intern::WrapperKey;
    // Doc-link target only (`HostData::wrapper_store` field docs).
    #[cfg(doc)]
    use super::super::wrapper_intern::WrapperKind;
    use super::effect_queues::HostEffectQueues;
    use elidex_ecs::Entity;
    use elidex_script_session::ListenerId;
    // A2: the Web Storage backends are `Legacy`-only — dropped from `App` builds.
    #[cfg(feature = "compat-webapi")]
    use elidex_storage_core::{SessionStorageState, WebStorageManager};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    mod bind;
    mod context_config;
    mod entity_kind;
    mod object_roots;
    mod realtime;

    pub use entity_kind::PrototypeKind;
    pub use realtime::{BinaryType, EventSourceState, WebSocketState};

    /// Per-process counter for opaque-origin sentinels (e.g. `about:blank`,
    /// `data:` URLs).  Each `HostData` claims one ID at construction time
    /// and uses it as the identity-preserving storage-partition key for
    /// origin-scoped surfaces (`localStorage` AND IndexedDB versionchange —
    /// see `VmInner::storage_origin_key`) so two opaque-origin VMs do not
    /// alias.  Resets on process restart.  NOT `compat-webapi`-gated: the
    /// IndexedDB broadcast key needs it in `engine`-only builds too.
    static OPAQUE_ORIGIN_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Prefix on per-VM opaque-origin sentinel strings.  Distinct
    /// from any `url::Origin::ascii_serialization()` output (which is
    /// always `scheme://host[:port]`) so the sentinel cannot collide
    /// with a real origin.
    const OPAQUE_ORIGIN_PREFIX: &str = "opaque-origin:";

    fn next_opaque_origin_id() -> String {
        let n = OPAQUE_ORIGIN_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{OPAQUE_ORIGIN_PREFIX}{n}")
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
        /// **⚠ SUPERSEDED (2026-06-30, PR #434
        /// `docs/plans/2026-06-agent-scoped-ecsdom-world.md`)**: the
        /// `world_id` discriminator is retracted. The cross-DOM
        /// resolution is the **agent-scoped `EcsDom` World** (1 agent =
        /// 1 World = 1 Vm), under which this aliasing is unconstructable
        /// by construction. The caller contract stays the interim
        /// invariant; the full removal lands with the B1 implementation.
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
        /// ⚠ SUPERSEDED 2026-06-30: world_id retracted → agent-scoped EcsDom
        /// World (PR #434 `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §6);
        /// interim form unchanged until B1.
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
        /// "cookie-averse" path of WHATWG HTML §3.1.4).
        cookie_jar: Option<std::sync::Arc<elidex_net::CookieJar>>,
        /// Sandboxing flag set for this document's browsing context
        /// (WHATWG HTML §7.1.5 Sandboxing).  `unsandboxed()` (the empty set)
        /// for top-level / unsandboxed documents — the cutover's shell
        /// installs the parsed `SandboxingFlagSet` (from the iframe
        /// `sandbox=""` attribute) via [`Self::set_sandbox_flags`] when a
        /// document loads inside a sandboxed iframe.  A member of the current-document-state cluster (cf. the
        /// `document_origin_override` note below); under B1 it migrates to an ECS
        /// component at its spec-correct grain per the decision's grain rule (PR #434
        /// §5 req 5), *not* a permanent CLAUDE.md side-store (b) exception like
        /// `cookie_jar`.  Read
        /// by [`Self::scripts_allowed`] (the eval gate) and the
        /// per-capability [`Self::forms_allowed`] / [`Self::popups_allowed`] /
        /// [`Self::modals_allowed`] accessors.
        sandbox_flags: elidex_plugin::SandboxingFlagSet,
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
        /// ⚠ SUPERSEDED 2026-06-30: this slot is FOLDED into the agent-scoped
        /// World decision (PR #434 §5 req 5 / §6.1).
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
        /// nesting.  Per-context interim; its B1 ECS grain follows the
        /// decision's grain rule (PR #434 §5 req 5), not asserted here.
        iframe_depth: usize,
        /// The S5-6a cross-context effect queues ([`HostEffectQueues`] —
        /// `localStorage` broadcasts / IndexedDB versionchange requests /
        /// `window.focus()` flag / iframe→parent `postMessage` intents):
        /// transient FIFO intents the shell drains per turn via the
        /// `HostDriver` drain group (the `pending_history` /
        /// `pending_window_open` event-queue standing; B1-neutral).  Carved
        /// into the `effect_queues` submodule; enqueue/take methods stay on
        /// `HostData`.  NOT cleared on `Vm::unbind`: like the navigation
        /// back-channel, the shell drains these after the batch bracket
        /// closes, so they must survive it.
        pub(super) effect_queues: HostEffectQueues,
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
        /// rooted by the keepalive seam's **active-observation predicate**
        /// (`gc/keepalive.rs` `keepalive_survivors`, S5-3c) — kept iff the
        /// observer has ≥1 active observation (DOM §4.3 registered-observer-list
        /// membership) **OR ≥1 pending undelivered `MutationRecord`** (a non-empty
        /// record queue awaiting the §4.3 "notify mutation observers" microtask —
        /// so a record queued just before its sole target despawned / the observer
        /// `disconnect()`ed still survives to deliver; the second disjunct is
        /// MutationObserver-only, RO/IO have no persistent entry queue). So the
        /// callback + instance wrapper survive a GC **while the observer observes
        /// or still has a record to deliver**, and become collectible only once
        /// its last observation ends (`disconnect()` / `unobserve()` of the last
        /// target / despawn of the sole observed entity) **AND** its record queue
        /// is drained, unless independently JS-rooted.
        ///
        /// **Retained across `Vm::unbind`** — keyed by VM-monotonic
        /// `observer_id`, not by `Entity` or recycled `ObjectId`, so
        /// cross-DOM aliasing does not apply.  A retained `mo`
        /// reference can re-`observe` after a rebind (same or
        /// different DOM) and have its callback fire.  The binding **row** no
        /// longer leaks (S5-3c): `gc/collect.rs` sweep-prunes each row whose
        /// `instance` mark bit is clear, so a `disconnect()`ed / never-observed
        /// unreferenced observer's row is reclaimed at the next GC (the prior
        /// grow-monotonically-until-unbind leak is fixed for the collected case).
        /// The residual `#11-mutation-observer-extras` scope is the **non-
        /// keepalive** extras only (primitive→`ToObject` init parsing,
        /// `Symbol.iterator` filter handling, etc.), **not** the binding leak.
        ///
        /// **The engine-independent registry row is retired alongside the binding
        /// row.** The `gc/collect.rs` sweep captures each pruned observer id and
        /// calls `retire_collected` on its registry — dropping the
        /// `MutationObserverRegistry::records` empty-`Vec` row + `pending` slot,
        /// the `ResizeObserverRegistry` `registered` id, and the
        /// `IntersectionObserverRegistry` per-observer config
        /// (`IntersectionObserverInit` / parsed rootMargin). So **no registry-side
        /// residual remains** for a GC-collected observer either; the leak is
        /// fully closed for the collected case, not just the binding half.
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
        /// [`Self::mutation_observer_bindings`]; rooted by the keepalive seam's
        /// active-observation predicate and sweep-pruned by row (S5-3c) —
        /// Resize Observer §3.5 "Lifetime".
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
        /// origin-keyed registry.  Read by both `localStorage` scoping and
        /// the IndexedDB versionchange broadcast key (via
        /// `VmInner::storage_origin_key`), so NOT `compat-webapi`-gated.
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
        /// `mutation_observer_bindings` per-id pattern — keyed by the
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
        /// `name → CustomElementDefinition` map (+ the `constructor_id →
        /// name` reverse index); it holds NO `Entity` (awaiting-upgrade
        /// state is the per-entity `CustomElementState` component, which
        /// dies with the world). Shared via `Arc<Mutex<>>` with
        /// [`elidex_custom_elements::CustomElementReactionConsumer`]
        /// (which only reads `observed_attributes`).
        ///
        /// Document-lifetime authoritative registry: it **survives** a
        /// per-turn (BATCH-BIND) `Vm::unbind` — cleared only at
        /// `Vm::teardown_document` (document destruction) — so a
        /// `customElements.define()` in one script batch is visible to a
        /// later batch's upgrade / `whenDefined` (HTML §4.13.4/§4.13.5).
        /// Survival is cross-DOM-safe by construction: a live `Vm` only
        /// ever rebinds the SAME `EcsDom` (navigation allocates a new
        /// `Vm`), so the per-VM constructor `ObjectId`s in
        /// `ce_constructors` below ride the object heap validly across a
        /// same-DOM turn (`#11-per-batch-unbind-document-lifetime-state`;
        /// the grain migration to a per-realm component rides agent-scoped
        /// EcsDom, docs/plans/2026-06-agent-scoped-ecsdom-world.md §5).
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
        /// **Document-lifetime**: reset to 0 ONLY at `Vm::teardown_document`,
        /// NEVER on a per-turn `Vm::unbind` — resetting per-turn would recycle
        /// ids against the surviving `ce_constructors` entries and mis-map a
        /// later `new.target` to a stale definition
        /// (`#11-per-batch-unbind-document-lifetime-state`).
        pub(crate) ce_next_constructor_id: u64,
        /// `constructor_id → constructor ObjectId` map. The constructor
        /// `ObjectId` is per-VM identity (HostData exception (a) — see
        /// `feedback_boa-hostbridge-port-is-not-a-registry.md`), so this
        /// stays on HostData not as an ECS component. Document-lifetime:
        /// cleared at `Vm::teardown_document` (survives a per-turn unbind,
        /// in lockstep with `ce_registry` above).
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
        /// **Document-lifetime**: cleared at `Vm::teardown_document`
        /// (survives a per-turn unbind, in lockstep with `ce_registry` —
        /// `#11-per-batch-unbind-document-lifetime-state`).
        pub(crate) ce_when_defined_promises: HashMap<String, ObjectId>,
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
                sandbox_flags: elidex_plugin::SandboxingFlagSet::unsandboxed(),
                document_origin_override: None,
                fallback_opaque_origin: elidex_plugin::SecurityOrigin::opaque(),
                iframe_depth: 0,
                effect_queues: HostEffectQueues::default(),
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
#[cfg(feature = "engine")]
pub use engine_feature::TreeWalkerState;
#[cfg(feature = "engine")]
pub use engine_feature::{BinaryType, EventSourceState, WebSocketState};
