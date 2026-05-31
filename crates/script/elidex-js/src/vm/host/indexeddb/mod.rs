//! IndexedDB host bindings (W3C Indexed Database API 3.0) — slot
//! `#11-indexed-db-vm` (D-20).
//!
//! ```text
//! indexedDB (ObjectKind::IdbFactory singleton)
//! IDBRequest / IDBOpenDBRequest (ObjectKind::IdbRequest)  → IDBRequest.prototype
//!   → EventTarget.prototype  (vm/host/event_target.rs)
//! IDBDatabase  (ObjectKind::IdbDatabase)    → IDBDatabase.prototype → EventTarget.prototype
//! IDBTransaction (ObjectKind::IdbTransaction) → ... → EventTarget.prototype
//! IDBObjectStore (ObjectKind::IdbObjectStore)
//! IDBKeyRange  (ObjectKind::IdbKeyRange)
//! ```
//!
//! ## Layering (CLAUDE.md Layering mandate)
//!
//! This module is marshalling + the §5.x event-loop orchestration ONLY.
//! Every record / key / index / cursor / range algorithm lives in the
//! engine-independent `elidex-indexeddb` backend crate
//! (`key.rs` / `ops.rs` / `index.rs` / `cursor.rs` / `key_range.rs`).
//! host/ converts `JsValue` ↔ `IdbKey` / value, fires events on the VM
//! event loop, and runs the transaction lifecycle state machine — all
//! engine-bound concerns.
//!
//! ## Async model (W3C IDB §5.6 / §5.9 / §2.7.1)
//!
//! `IDBRequest` is a non-Node `EventTarget` (NOT a Promise).  A request's
//! operation runs synchronously against the SQLite backend, but its result
//! is delivered via a **database task** (§5.6 step 5.6 "queue a database
//! task") — [`super::pending_tasks::PendingTask::IdbDeliver`] drained at the
//! `drain_tasks` tail — so the success/error event fires after control
//! returns to the event loop, never inline (the boa bridge fired inline =
//! bug, NOT copied).  Transactions auto-commit when their request list
//! empties after event dispatch (§5.9 step 8.3) or, for a zero-request txn,
//! via the post-turn sweep ([`super::super::VmInner::idb_autocommit_sweep`]).

#![cfg(feature = "engine")]

use std::collections::HashMap;

use super::super::shape::PropertyAttrs;
use super::super::value::{
    CallMode, JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyValue, StringId,
    VmError,
};
use super::super::VmInner;
use super::events::{set_event_slot_raw, EventInit, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_TARGET};

pub(crate) mod database;
pub(crate) mod factory;
pub(crate) mod key_range;
pub(crate) mod object_store;
mod register;
pub(crate) mod request;
pub(crate) mod txn;
mod value;

/// `IDBRequest.readyState` (W3C IDB §4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum IdbReadyState {
    /// Operation in flight; result not yet available.
    #[default]
    Pending,
    /// Operation finished; `result` / `error` populated.
    Done,
}

impl IdbReadyState {
    /// JS string form (`"pending"` / `"done"`) for the `readyState` getter.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            IdbReadyState::Pending => "pending",
            IdbReadyState::Done => "done",
        }
    }
}

/// Transaction lifecycle state (W3C IDB §2.7.1).
///
/// `Active` during the creating script's synchronous run and during an
/// event dispatch from one of its requests; `Inactive` after control
/// returns to the event loop; `Committing` once it starts to commit;
/// `Finished` after commit / abort.  Requests may only be issued while
/// `Active`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum IdbTxnState {
    #[default]
    Active,
    Inactive,
    Committing,
    Finished,
}

/// Outcome of a request's backend operation, staged in
/// [`IdbRequestState::deferred`] between issue (synchronous backend call)
/// and delivery (the `IdbDeliver` database task that fires the event).
#[derive(Debug)]
pub(crate) enum DeferredOutcome {
    /// Result value to assign to `request.result` (already marshalled to a
    /// `JsValue` at issue time; a heap `ObjectId` inside is GC-rooted by the
    /// owning [`IdbRequestState`]'s trace step).
    Success(JsValue),
    /// `DOMException` wrapper `ObjectId` to assign to `request.error`.
    Error(ObjectId),
}

/// Per-`IDBRequest` / `IDBOpenDBRequest` state, keyed in
/// [`super::super::VmInner::idb_request_states`] by the instance `ObjectId`.
#[derive(Debug)]
pub(crate) struct IdbRequestState {
    pub(crate) ready_state: IdbReadyState,
    /// `request.result` after delivery (Done).  `Undefined` while Pending or
    /// on error.
    pub(crate) result: JsValue,
    /// `request.error` (`DOMException` `ObjectId`) on failure, else `None`.
    pub(crate) error: Option<ObjectId>,
    /// `request.source` — the IDBObjectStore / IDBIndex / IDBCursor that
    /// produced this request, or `None` (factory `open` / `deleteDatabase`).
    pub(crate) source: Option<ObjectId>,
    /// Owning `IDBTransaction` `ObjectId` (`None` for a factory open request
    /// until an upgrade transaction is associated).
    pub(crate) transaction: Option<ObjectId>,
    /// Staged backend outcome awaiting its `IdbDeliver` database task.
    pub(crate) deferred: Option<DeferredOutcome>,
    /// `on*` handler attributes keyed by interned attr-name SID
    /// (`onsuccess` / `onerror` / `onupgradeneeded` / `onblocked`).
    pub(crate) handlers: HashMap<StringId, ObjectId>,
    /// `addEventListener` callbacks (in-VM listener store; non-Node
    /// EventTarget, AbortSignal precedent).
    pub(crate) listeners: Vec<IdbListener>,
}

impl Default for IdbRequestState {
    fn default() -> Self {
        IdbRequestState {
            ready_state: IdbReadyState::Pending,
            result: JsValue::Undefined,
            error: None,
            source: None,
            transaction: None,
            deferred: None,
            handlers: HashMap::new(),
            listeners: Vec::new(),
        }
    }
}

/// A registered `addEventListener` callback (in-VM store).
#[derive(Debug, Clone)]
pub(crate) struct IdbListener {
    /// Interned event-type SID (`success` / `error` / `complete` / `abort` /
    /// `upgradeneeded` / `versionchange` / `blocked` / `close`).
    pub(crate) event_type: StringId,
    /// The callback object `ObjectId`.
    pub(crate) callback: ObjectId,
    /// `once` flag (WHATWG DOM `AddEventListenerOptions`).
    pub(crate) once: bool,
    /// The `AbortSignal` `ObjectId` from `{signal}`, if any (WHATWG DOM §2.7.3).
    /// When that signal aborts, [`remove_idb_listeners_for_signal`] drops this
    /// listener.  GC-traced (alongside `callback`) so the signal `ObjectId`
    /// cannot be collected + recycled while the listener references it — a
    /// recycled id would otherwise make a later unrelated abort remove this
    /// listener (identity-reuse hazard).
    pub(crate) signal: Option<ObjectId>,
}

/// Per-`IDBTransaction` state, keyed in
/// [`super::super::VmInner::idb_transaction_states`].
pub(crate) struct IdbTransactionState {
    pub(crate) state: IdbTxnState,
    pub(crate) mode: elidex_indexeddb::IdbTransactionMode,
    pub(crate) db_name: String,
    /// Store names in scope (§4.10 `objectStoreNames`).
    pub(crate) scope: Vec<String>,
    /// Owning `IDBDatabase` `ObjectId`.
    pub(crate) db: Option<ObjectId>,
    /// Open backend SQLite transaction handle.  `None` once committed /
    /// aborted.  Has no `Drop` rollback (backend `IdbTransaction` exposes
    /// only an explicit `abort`), so [`super::super::VmInner::unbind`] must
    /// explicitly abort any still-open handle (plan §4.5).
    pub(crate) backend_txn: Option<elidex_indexeddb::IdbTransaction>,
    /// §5.6 "transaction's request list" — request `ObjectId`s in issue
    /// order.  Drives auto-commit: emptied list after event dispatch →
    /// commit (§5.9 step 8.3).
    pub(crate) request_list: Vec<ObjectId>,
    /// `oncomplete` / `onerror` / `onabort` handler attributes.
    pub(crate) handlers: HashMap<StringId, ObjectId>,
    pub(crate) listeners: Vec<IdbListener>,
    /// The `DOMException` that caused an abort (§4.10 `error`), else `None`.
    pub(crate) error: Option<ObjectId>,
    /// For an upgrade transaction, the associated open request `ObjectId`
    /// (so commit can fire `success` at it and clear `request.transaction`,
    /// §5.4 step 2.5.4).  `None` for a normal transaction.
    pub(crate) upgrade_request: Option<ObjectId>,
    /// Backend database handle for an upgrade transaction (needed by
    /// `finish_upgrade` / `abort_upgrade`).  `None` for a normal txn.
    pub(crate) upgrade_handle: Option<elidex_indexeddb::IdbDatabaseHandle>,
    /// Old version for `abort_upgrade` rollback (§5.8).
    pub(crate) upgrade_old_version: u64,
}

impl IdbTransactionState {
    /// A freshly-`Active` transaction over `backend_txn` with the upgrade
    /// fields cleared (a normal `db.transaction(...)`).  The upgrade flow
    /// builds on this with `..IdbTransactionState::new_active(...)` + its
    /// `upgrade_*` overrides, so the 13-field literal lives in one place.
    pub(super) fn new_active(
        mode: elidex_indexeddb::IdbTransactionMode,
        db: ObjectId,
        db_name: &str,
        scope: Vec<String>,
        backend_txn: elidex_indexeddb::IdbTransaction,
    ) -> Self {
        IdbTransactionState {
            state: IdbTxnState::Active,
            mode,
            db_name: db_name.to_string(),
            scope,
            db: Some(db),
            backend_txn: Some(backend_txn),
            request_list: Vec::new(),
            handlers: HashMap::new(),
            listeners: Vec::new(),
            error: None,
            upgrade_request: None,
            upgrade_handle: None,
            upgrade_old_version: 0,
        }
    }
}

/// Per-`IDBDatabase` connection state, keyed in
/// [`super::super::VmInner::idb_database_states`].
#[derive(Debug, Default)]
pub(crate) struct IdbDatabaseState {
    pub(crate) db_name: String,
    pub(crate) version: u64,
    /// `true` after `close()` (or a `versionchange` that closed it).
    pub(crate) closed: bool,
    /// The active upgrade (`versionchange`) transaction `ObjectId` while an
    /// `upgradeneeded` handler runs, else `None` — `createObjectStore` /
    /// `deleteObjectStore` operate against it (§5.7).  Set by the factory
    /// open flow; cleared when the upgrade transaction finishes.
    pub(crate) upgrade_txn: Option<ObjectId>,
    /// `onversionchange` / `onclose` / `onabort` handler attributes.
    pub(crate) handlers: HashMap<StringId, ObjectId>,
    pub(crate) listeners: Vec<IdbListener>,
}

/// Per-`IDBObjectStore` handle state, keyed in
/// [`super::super::VmInner::idb_object_store_states`].  Metadata
/// (`keyPath` / `autoIncrement` / `indexNames`) is read on demand from the
/// backend so it never drifts from the schema.
#[derive(Debug, Default)]
pub(crate) struct IdbObjectStoreState {
    pub(crate) db_name: String,
    pub(crate) store_name: String,
    /// Owning `IDBTransaction` `ObjectId`.
    pub(crate) transaction: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Backend lifecycle + auto-commit sweep (impl VmInner)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Return the per-origin IndexedDB backend, lazily creating an
    /// in-memory one on first use when the embedder installed none (boa
    /// bridge `ensure_idb_backend` parity).  `None` only if in-memory
    /// SQLite creation fails — the caller surfaces that to JS.
    pub(crate) fn ensure_idb_backend(
        &mut self,
    ) -> Option<std::rc::Rc<elidex_indexeddb::IdbBackend>> {
        if self.idb_backend.is_none() {
            match elidex_indexeddb::IdbBackend::open_in_memory() {
                Ok(backend) => self.idb_backend = Some(std::rc::Rc::new(backend)),
                Err(_) => return None,
            }
        }
        self.idb_backend.clone()
    }

    /// [`Self::ensure_idb_backend`] or a thrown `TypeError` — the
    /// backend-unavailable path is identical at every call site, so the
    /// message lives here once.
    pub(crate) fn require_idb_backend(
        &mut self,
    ) -> Result<std::rc::Rc<elidex_indexeddb::IdbBackend>, VmError> {
        self.ensure_idb_backend()
            .ok_or_else(|| VmError::type_error("IndexedDB backend unavailable"))
    }

    /// Abort every still-pending `IDBRequest` IN PLACE with an `AbortError`:
    /// set `readyState = "done"`, `error = AbortError`, and clear any staged
    /// outcome (so its queued `IdbDeliver` task no-ops) and transaction link.
    /// The request states are RETAINED — a held wrapper then resolves to
    /// `done` + `request.error` instead of hanging at `pending` forever, and
    /// GC reaps the entries with their wrappers.  Used when the backend is
    /// replaced out from under in-flight requests ([`Vm::install_idb_backend`]).
    pub(crate) fn abort_pending_idb_requests(&mut self, message: &str) {
        let pending: Vec<ObjectId> = self
            .idb_request_states
            .iter()
            .filter(|(_, s)| s.ready_state == IdbReadyState::Pending)
            .map(|(id, _)| *id)
            .collect();
        if pending.is_empty() {
            return;
        }
        let abort_sid = self.strings.intern("AbortError");
        let exc = match self.build_dom_exception(abort_sid, message) {
            JsValue::Object(id) => Some(id),
            _ => None,
        };
        for rid in pending {
            if let Some(s) = self.idb_request_states.get_mut(&rid) {
                s.ready_state = IdbReadyState::Done;
                s.result = JsValue::Undefined;
                s.error = exc;
                s.deferred = None;
                s.transaction = None;
            }
        }
    }

    /// W3C IndexedDB §2.7.1 auto-commit fallback: commit every still-`Active`
    /// transaction whose request list is empty.  Run at the `drain_tasks`
    /// tail (the "control returns to the event loop" seam).  Eligible ids
    /// are collected first so `commit_transaction` (which mutates the entry
    /// in place and queues a task, but never inserts or removes map
    /// entries) cannot invalidate the iteration.  De-dup with §5.9 step 8.3: a txn already
    /// committed there is `Committing`, so the `Active` filter skips it.
    pub(crate) fn idb_autocommit_sweep(&mut self) {
        let eligible: Vec<ObjectId> = self
            .idb_transaction_states
            .iter()
            .filter(|(_, st)| st.state == IdbTxnState::Active && st.request_list.is_empty())
            .map(|(id, _)| *id)
            .collect();
        for id in eligible {
            txn::commit_transaction(self, id);
        }
    }
}

// ---------------------------------------------------------------------------
// Shared event firing (non-Node EventTarget; AbortSignal precedent)
// ---------------------------------------------------------------------------

/// Outcome of dispatching an IDB event, consumed by the §5.9 / §5.10
/// transaction lifecycle steps.
pub(super) struct FireResult {
    /// A handler / listener threw (§5.9 step 8.2 / §5.10 step 8.2 →
    /// abort the transaction with an `"AbortError"`).
    pub(super) threw: bool,
    /// `event.preventDefault()` was called during dispatch (§5.10 step 8.3
    /// canceled-flag check — when false the error aborts the transaction).
    pub(super) canceled: bool,
}

/// Snapshot the `on*` handler + matching `addEventListener` callbacks for
/// `event_type` from the target's side-store, each paired with its `once`
/// flag.  Does NOT prune — `once` listeners are removed individually at the
/// point each is about to be invoked ([`remove_idb_once_listener`]), per
/// WHATWG DOM §2.10: a `once` listener that the walk never reaches (e.g. a
/// `stopPropagation()` halted bubbling before its node) must survive for a
/// later event.  The three IDB EventTarget state structs share field names
/// (`handlers` / `listeners`), so one macro covers all three.
fn collect(
    vm: &VmInner,
    target: ObjectId,
    event_type: StringId,
    handler_attr: Option<StringId>,
) -> (Option<ObjectId>, Vec<(ObjectId, bool)>) {
    // Reduce to a small `Copy` discriminant under the shared borrow.
    let Some(kind) = idb_target_kind(vm, target) else {
        return (None, Vec::new());
    };
    macro_rules! pull {
        ($map:ident) => {{
            match vm.$map.get(&target) {
                Some(st) => {
                    let handler = handler_attr.and_then(|h| st.handlers.get(&h).copied());
                    let cbs: Vec<(ObjectId, bool)> = st
                        .listeners
                        .iter()
                        .filter(|l| l.event_type == event_type)
                        .map(|l| (l.callback, l.once))
                        .collect();
                    (handler, cbs)
                }
                None => (None, Vec::new()),
            }
        }};
    }
    match kind {
        IdbTargetKind::Request => pull!(idb_request_states),
        IdbTargetKind::Transaction => pull!(idb_transaction_states),
        IdbTargetKind::Database => pull!(idb_database_states),
    }
}

/// One propagation-path node's dispatch snapshot: `(node, on*-handler,
/// [(listener-callback, once)])`, as produced by [`collect`].
type NodeDispatch = (ObjectId, Option<ObjectId>, Vec<(ObjectId, bool)>);

/// Remove the `once` listener `(event_type, callback)` from `node`'s in-VM
/// store at the point it is about to be invoked (WHATWG DOM §2.10: a `once`
/// listener is removed before its callback runs, so a re-entrant dispatch in
/// the callback can't re-fire it, while a never-reached `once` listener is
/// left in place by [`collect`] not pruning up front).
fn remove_idb_once_listener(
    vm: &mut VmInner,
    node: ObjectId,
    event_type: StringId,
    callback: ObjectId,
) {
    let Some(kind) = idb_target_kind(vm, node) else {
        return;
    };
    macro_rules! rm {
        ($map:ident) => {
            if let Some(st) = vm.$map.get_mut(&node) {
                st.listeners
                    .retain(|l| !(l.event_type == event_type && l.callback == callback && l.once));
            }
        };
    }
    match kind {
        IdbTargetKind::Request => rm!(idb_request_states),
        IdbTargetKind::Transaction => rm!(idb_transaction_states),
        IdbTargetKind::Database => rm!(idb_database_states),
    }
}

/// Fire `event_type` at `target` (W3C IDB §5.9 / §5.10 dispatch step):
/// build a fresh `Event`, set `target` / `currentTarget`, invoke the
/// `on*` handler attribute then every matching `addEventListener`
/// callback.  Returns whether a listener threw + whether the default was
/// prevented so the caller can run the transaction lifecycle steps.
pub(super) fn fire_idb_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    handler_attr: StringId,
    cancelable: bool,
    bubbles: bool,
) -> FireResult {
    fire_idb_event_with_props(
        ctx,
        target,
        event_type,
        handler_attr,
        cancelable,
        bubbles,
        None,
        &[],
    )
}

/// Fire an `IDBVersionChangeEvent` (§4.2) at `target` — a base `Event`
/// with own `oldVersion` / `newVersion` data properties + the
/// `IDBVersionChangeEvent.prototype`.  Used for `upgradeneeded` /
/// `versionchange` / `blocked`.  `new_version` is `null` for a
/// `deleteDatabase` versionchange.
pub(super) fn fire_version_change_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    handler_attr: StringId,
    old_version: u64,
    new_version: Option<u64>,
) -> FireResult {
    #[allow(clippy::cast_precision_loss)]
    let new_v = new_version.map_or(JsValue::Null, |v| JsValue::Number(v as f64));
    let old_sid = ctx.vm.well_known.old_version;
    let new_sid = ctx.vm.well_known.new_version;
    #[allow(clippy::cast_precision_loss)]
    let props = [
        (old_sid, JsValue::Number(old_version as f64)),
        (new_sid, new_v),
    ];
    let proto = ctx.vm.idb_version_change_event_prototype;
    fire_idb_event_with_props(
        ctx,
        target,
        event_type,
        handler_attr,
        false,
        false,
        proto,
        &props,
    )
}

/// The bubbling ancestor chain for an IDB event target (WHATWG DOM §2.5
/// propagation path, specialized to the IDB hierarchy): an `IDBRequest`
/// bubbles to its transaction then that transaction's database; an
/// `IDBTransaction` bubbles to its database; an `IDBDatabase` has no ancestor.
/// Used so a request `error` (or a transaction `abort`) reaches the
/// `tx.onerror` / `db.onerror` handlers and an ancestor `preventDefault()`
/// can cancel the automatic abort (§5.10).
fn idb_event_ancestors(vm: &VmInner, target: ObjectId) -> Vec<ObjectId> {
    let mut chain = Vec::new();
    match idb_target_kind(vm, target) {
        Some(IdbTargetKind::Request) => {
            if let Some(tid) = vm
                .idb_request_states
                .get(&target)
                .and_then(|s| s.transaction)
            {
                chain.push(tid);
                if let Some(db) = vm.idb_transaction_states.get(&tid).and_then(|s| s.db) {
                    chain.push(db);
                }
            }
        }
        Some(IdbTargetKind::Transaction) => {
            if let Some(db) = vm.idb_transaction_states.get(&target).and_then(|s| s.db) {
                chain.push(db);
            }
        }
        Some(IdbTargetKind::Database) | None => {}
    }
    chain
}

/// Shared event-build + dispatch.  `proto_override` reparents the event to
/// a subclass prototype (e.g. `IDBVersionChangeEvent.prototype`);
/// `extra_props` installs own data properties on the event before
/// dispatch.  A `bubbles` event propagates along the IDB ancestor chain
/// ([`idb_event_ancestors`]): each node's `on*` handler + matching
/// `addEventListener` callbacks run with `currentTarget` set to that node
/// (`target` stays the original), and `preventDefault()` from any node sets
/// the returned `canceled` flag.
#[allow(clippy::too_many_arguments)]
fn fire_idb_event_with_props(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    handler_attr: StringId,
    cancelable: bool,
    bubbles: bool,
    proto_override: Option<ObjectId>,
    extra_props: &[(StringId, JsValue)],
) -> FireResult {
    // Build the event lazily (only when a node actually has a handler /
    // listener — see `dispatch_idb_event`) so a fire at an unobserved
    // target allocates nothing.
    dispatch_idb_event(
        ctx,
        target,
        event_type,
        Some(handler_attr),
        bubbles,
        |ctx| {
            let shape = ctx
                .vm
                .precomputed_event_shapes
                .as_ref()
                .expect("precomputed_event_shapes missing — register_globals did not run")
                .core;
            let init = EventInit {
                bubbles,
                cancelable,
                composed: false,
            };
            let event_id = ctx.vm.create_fresh_event_object(
                JsValue::Undefined,
                event_type,
                init,
                shape,
                Vec::new(),
                true,
                CallMode::Call,
            );
            if let Some(proto) = proto_override {
                ctx.vm.get_object_mut(event_id).prototype = Some(proto);
            }
            for &(key, value) in extra_props {
                ctx.vm.define_shaped_property(
                    event_id,
                    PropertyKey::String(key),
                    PropertyValue::Data(value),
                    PropertyAttrs::BUILTIN,
                );
            }
            event_id
        },
    )
}

/// Whether `stopPropagation()` (or `stopImmediatePropagation()`, which
/// implies it) has been called on `event_id` — stops bubbling to ancestors.
fn idb_event_propagation_stopped(vm: &VmInner, event_id: ObjectId) -> bool {
    matches!(
        vm.get_object(event_id).kind,
        ObjectKind::Event {
            propagation_stopped: true,
            ..
        }
    )
}

/// Whether `stopImmediatePropagation()` has been called on `event_id` —
/// stops the remaining listeners on the current node as well.
fn idb_event_immediate_stopped(vm: &VmInner, event_id: ObjectId) -> bool {
    matches!(
        vm.get_object(event_id).kind,
        ObjectKind::Event {
            immediate_propagation_stopped: true,
            ..
        }
    )
}

/// The single dispatch core for the IDB in-VM EventTarget model — used by
/// every internal fire ([`fire_idb_event_with_props`]) AND the script-facing
/// [`native_idb_dispatch_event`], so both observe identical WHATWG DOM §2.9
/// semantics (specialized to the IDB ancestor chain).  Putting the full §2.9
/// bookkeeping here — not in the `dispatchEvent` wrapper — keeps internal
/// fires and `dispatchEvent` correct by construction (one dispatch algorithm,
/// not two parallel ones):
///
/// 1. Build the propagation path: `target`, then its [`idb_event_ancestors`]
///    when `bubbles`.
/// 2. Snapshot each node's `on*` handler + matching listeners (with their
///    `once` flag, NOT pruned up front — see [`collect`]). When nothing is
///    registered anywhere on the path, return early: `make_event` is never
///    called, so a fire at an unobserved target allocates no event object.
/// 3. Bracket the event in [`VmInner::dispatched_events`] for the whole walk
///    (§2.9 step 1 dispatch flag) so a handler that captures the event and
///    re-dispatches it re-entrantly throws `InvalidStateError`.
/// 4. GC-root the event + every collected callback on the VM stack for the
///    dispatch duration (a listener that allocates can trip the
///    `alloc_object` GC threshold; an invoked `once` listener is removed from
///    its side-store, so the snapshot becomes its only reference).
/// 5. Invoke each node's handler then listeners with `currentTarget` set to
///    that node — removing a `once` listener immediately before it runs —
///    honoring `stopImmediatePropagation` (stop the node's remaining
///    listeners) and `stopPropagation` (stop bubbling to ancestors).
/// 6. Finalize (§2.9 steps 27-31): clear `currentTarget` + the propagation
///    flags so a captured event reads the "no longer dispatching" state and a
///    later dispatch starts clean (`target` stays set; `defaultPrevented` is
///    the canceled bit and is preserved).
///
/// Returns whether any listener threw (so §5.9/§5.10 step 8.2 can abort) and
/// whether the default was prevented (`canceled`).
fn dispatch_idb_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    handler_attr: Option<StringId>,
    bubbles: bool,
    make_event: impl FnOnce(&mut NativeContext<'_>) -> ObjectId,
) -> FireResult {
    // Propagation path: the target, then its ancestors when bubbling.
    let mut path = vec![target];
    if bubbles {
        path.extend(idb_event_ancestors(ctx.vm, target));
    }
    // Snapshot each node's handler + listeners (with `once` flags) up front.
    let collected: Vec<NodeDispatch> = path
        .iter()
        .map(|&node| {
            let (handler, listeners) = collect(ctx.vm, node, event_type, handler_attr);
            (node, handler, listeners)
        })
        .collect();
    if collected
        .iter()
        .all(|(_, handler, listeners)| handler.is_none() && listeners.is_empty())
    {
        return FireResult {
            threw: false,
            canceled: false,
        };
    }
    let event_id = make_event(ctx);
    set_event_slot_raw(ctx.vm, event_id, EVENT_SLOT_TARGET, JsValue::Object(target));
    // §2.9 step 1 dispatch flag: bracket the event for the whole walk so a
    // re-entrant `dispatchEvent(thisEvent)` from a handler throws
    // `InvalidStateError` (checked in `native_idb_dispatch_event`).
    ctx.vm.dispatched_events.insert(event_id);
    // GC-root the live dispatch values on the VM stack for the duration of the
    // loop: the event object — held only in `event_id` here, reachable from no
    // rooted owner — and every collected handler / listener callback (an
    // invoked `once` listener is removed from its side-store before it runs,
    // so the snapshot becomes its ONLY reference).
    let mut frame = ctx.vm.push_stack_scope();
    frame.stack.push(JsValue::Object(event_id));
    for (_, handler, listeners) in &collected {
        if let Some(h) = handler {
            frame.stack.push(JsValue::Object(*h));
        }
        for &(cb, _once) in listeners {
            frame.stack.push(JsValue::Object(cb));
        }
    }
    let mut sub_ctx = NativeContext::new_call(&mut frame);
    let mut threw = false;
    // Errors swallowed per WHATWG event-handler-attribute semantics
    // (uncaught exceptions log, don't propagate) but recorded so §5.9
    // step 8.2 / §5.10 step 8.2 can abort the transaction.
    'walk: for (node, handler, listeners) in collected {
        set_event_slot_raw(
            sub_ctx.vm,
            event_id,
            EVENT_SLOT_CURRENT_TARGET,
            JsValue::Object(node),
        );
        if let Some(h) = handler {
            if sub_ctx
                .call_function(h, JsValue::Object(node), &[JsValue::Object(event_id)])
                .is_err()
            {
                threw = true;
            }
            if idb_event_immediate_stopped(sub_ctx.vm, event_id) {
                break 'walk;
            }
        }
        for (cb, once) in listeners {
            // §2.10: remove a `once` listener immediately before invoking it.
            if once {
                remove_idb_once_listener(sub_ctx.vm, node, event_type, cb);
            }
            if sub_ctx
                .call_function(cb, JsValue::Object(node), &[JsValue::Object(event_id)])
                .is_err()
            {
                threw = true;
            }
            if idb_event_immediate_stopped(sub_ctx.vm, event_id) {
                break 'walk;
            }
        }
        // `stopPropagation()` lets this node's listeners finish but halts
        // bubbling to the remaining ancestors.
        if idb_event_propagation_stopped(sub_ctx.vm, event_id) {
            break 'walk;
        }
    }
    let canceled = matches!(
        sub_ctx.vm.get_object(event_id).kind,
        ObjectKind::Event {
            default_prevented: true,
            ..
        }
    );
    // §2.9 steps 27-31 finalize: clear `currentTarget` + the propagation-stop
    // flags (a captured event must read "not dispatching"; a re-dispatch must
    // start clean), then unset the dispatch flag.  `target` stays set;
    // `defaultPrevented` is intentionally preserved (it is the canceled bit).
    set_event_slot_raw(
        sub_ctx.vm,
        event_id,
        EVENT_SLOT_CURRENT_TARGET,
        JsValue::Null,
    );
    if let ObjectKind::Event {
        propagation_stopped,
        immediate_propagation_stopped,
        ..
    } = &mut sub_ctx.vm.get_object_mut(event_id).kind
    {
        *propagation_stopped = false;
        *immediate_propagation_stopped = false;
    }
    sub_ctx.vm.dispatched_events.remove(&event_id);
    // `sub_ctx` then `frame` drop at scope end; `frame`'s drop truncates the
    // VM stack back to its pre-dispatch length, releasing the temp roots.
    FireResult { threw, canceled }
}

// ---------------------------------------------------------------------------
// Shared EventTarget natives (handler attrs + addEventListener family)
//
// IDBRequest / IDBDatabase / IDBTransaction are non-Node `EventTarget`s whose
// listener + handler stores live in-VM (the AbortSignal model), so they
// shadow the inherited `EventTarget.prototype` methods.  One backend fn per
// member dispatches on the receiver's `ObjectKind` to the matching side-store
// (the three state structs share `handlers` / `listeners` field names).
// ---------------------------------------------------------------------------

/// Which IDB EventTarget side-store a receiver maps to.
enum IdbTargetKind {
    Request,
    Transaction,
    Database,
}

fn idb_target_kind(vm: &VmInner, id: ObjectId) -> Option<IdbTargetKind> {
    match &vm.get_object(id).kind {
        ObjectKind::IdbRequest => Some(IdbTargetKind::Request),
        ObjectKind::IdbTransaction => Some(IdbTargetKind::Transaction),
        ObjectKind::IdbDatabase => Some(IdbTargetKind::Database),
        _ => None,
    }
}

/// Brand-check that `this` is one of the IDB EventTargets.
fn require_idb_event_target(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(ObjectId, IdbTargetKind), VmError> {
    if let JsValue::Object(id) = this {
        if let Some(kind) = idb_target_kind(ctx.vm, id) {
            return Ok((id, kind));
        }
    }
    Err(VmError::type_error(format!(
        "EventTarget.prototype.{method} called on a non-IndexedDB EventTarget"
    )))
}

/// Shared `on*` handler-attribute getter (bound-key keyed; WebIDL §3.7.6).
pub(crate) fn native_idb_handler_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key = ctx
        .bound_key()
        .expect("IDB event-handler accessor missing bound_key");
    let (id, kind) = require_idb_event_target(ctx, this, "on<event>")?;
    let handler = match kind {
        IdbTargetKind::Request => ctx.vm.idb_request_states.get(&id).map(|s| &s.handlers),
        IdbTargetKind::Transaction => ctx.vm.idb_transaction_states.get(&id).map(|s| &s.handlers),
        IdbTargetKind::Database => ctx.vm.idb_database_states.get(&id).map(|s| &s.handlers),
    }
    .and_then(|h| h.get(&key).copied());
    Ok(handler.map_or(JsValue::Null, JsValue::Object))
}

/// Shared `on*` handler-attribute setter: a callable installs the handler,
/// anything else clears it (WHATWG HTML event-handler IDL attribute).
pub(crate) fn native_idb_handler_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let key = ctx
        .bound_key()
        .expect("IDB event-handler accessor missing bound_key");
    let (id, kind) = require_idb_event_target(ctx, this, "on<event>")?;
    let new_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let callable =
        matches!(new_val, JsValue::Object(obj) if ctx.vm.get_object(obj).kind.is_callable());
    macro_rules! apply {
        ($map:ident) => {
            if let Some(st) = ctx.vm.$map.get_mut(&id) {
                match new_val {
                    JsValue::Object(obj) if callable => {
                        st.handlers.insert(key, obj);
                    }
                    _ => {
                        st.handlers.remove(&key);
                    }
                }
            }
        };
    }
    match kind {
        IdbTargetKind::Request => apply!(idb_request_states),
        IdbTargetKind::Transaction => apply!(idb_transaction_states),
        IdbTargetKind::Database => apply!(idb_database_states),
    }
    Ok(JsValue::Undefined)
}

/// `addEventListener(type, callback, options?)` (WHATWG DOM §2.7) over the
/// in-VM listener store.  Honors `once` + `{signal}`.
///
/// DEFERRED to `#11-eventtarget-dispatch-core` (the listener-lifecycle facets
/// where this in-VM `Vec<IdbListener>` model still diverges from the canonical
/// ECS-backed `EventListeners` — and which WebSocket / FileReader / EventSource
/// share, since each reimplements §2.9/§2.10 against the ECS-coupled
/// `dispatch_script_event`): `capture` as part of listener identity, per-listener
/// `passive` gating of `preventDefault`, and the §2.10 removed-flag so a
/// listener removed mid-dispatch (by an earlier listener or a `{signal}` abort)
/// is skipped.  The fix is to extract a listener-source-agnostic dispatch core
/// shared by all EventTargets, not to deepen this copy.
pub(crate) fn native_idb_add_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, kind) = require_idb_event_target(ctx, this, "addEventListener")?;
    let event_type = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let callback = match args.get(1).copied() {
        Some(JsValue::Object(cb)) if ctx.vm.get_object(cb).kind.is_callable() => cb,
        // null / undefined callback is a silent no-op (WHATWG DOM §2.7
        // "add an event listener").
        None | Some(JsValue::Null | JsValue::Undefined) => return Ok(JsValue::Undefined),
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'addEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ))
        }
    };
    // Reuse the shared `AddEventListenerOptions` parser (one option-parsing
    // path): IDB ignores `capture` / `passive` but honors `once` + `signal`.
    let options = super::event_target::parse_listener_options(
        ctx,
        args.get(2).copied().unwrap_or(JsValue::Undefined),
    )?;
    // WHATWG DOM §2.7.3 step 2: an already-aborted signal short-circuits —
    // the listener is never added (and so never bound for removal).
    if let Some(sig) = options.signal {
        if ctx
            .vm
            .abort_signal_states
            .get(&sig)
            .is_some_and(|s| s.aborted)
        {
            return Ok(JsValue::Undefined);
        }
    }
    let listener = IdbListener {
        event_type,
        callback,
        once: options.once,
        signal: options.signal,
    };
    macro_rules! add {
        ($map:ident) => {
            if let Some(st) = ctx.vm.$map.get_mut(&id) {
                // WHATWG DOM §2.7 "add an event listener": duplicate (type, callback, capture)
                // tuples are not added again (capture is always false here).
                if !st
                    .listeners
                    .iter()
                    .any(|l| l.event_type == event_type && l.callback == callback)
                {
                    st.listeners.push(listener);
                }
            }
        };
    }
    match kind {
        IdbTargetKind::Request => add!(idb_request_states),
        IdbTargetKind::Transaction => add!(idb_transaction_states),
        IdbTargetKind::Database => add!(idb_database_states),
    }
    Ok(JsValue::Undefined)
}

/// WHATWG DOM §2.7.3: when an `AbortSignal` bound via
/// `addEventListener(type, cb, {signal})` aborts, remove every IDB listener it
/// was attached to.  Called from [`super::abort::abort_signal`] alongside the
/// ECS-side `detach_bound_listeners`.  Scans the three IDB EventTarget stores
/// and drops listeners whose `signal` matches — authoritative (the listener
/// store is the source of truth, so there is no stale-id to reconcile).
pub(crate) fn remove_idb_listeners_for_signal(vm: &mut VmInner, signal_id: ObjectId) {
    let keep = |l: &IdbListener| l.signal != Some(signal_id);
    for st in vm.idb_request_states.values_mut() {
        st.listeners.retain(keep);
    }
    for st in vm.idb_transaction_states.values_mut() {
        st.listeners.retain(keep);
    }
    for st in vm.idb_database_states.values_mut() {
        st.listeners.retain(keep);
    }
}

/// `removeEventListener(type, callback)` (WHATWG DOM §2.7).
pub(crate) fn native_idb_remove_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (id, kind) = require_idb_event_target(ctx, this, "removeEventListener")?;
    let event_type = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let JsValue::Object(callback) = args.get(1).copied().unwrap_or(JsValue::Undefined) else {
        return Ok(JsValue::Undefined);
    };
    macro_rules! remove {
        ($map:ident) => {
            if let Some(st) = ctx.vm.$map.get_mut(&id) {
                st.listeners
                    .retain(|l| !(l.event_type == event_type && l.callback == callback));
            }
        };
    }
    match kind {
        IdbTargetKind::Request => remove!(idb_request_states),
        IdbTargetKind::Transaction => remove!(idb_transaction_states),
        IdbTargetKind::Database => remove!(idb_database_states),
    }
    Ok(JsValue::Undefined)
}

/// `dispatchEvent(event)` (WHATWG DOM §2.9): dispatch a script-constructed
/// event through the in-VM listener store.  Reads the event's `type`, invokes
/// the matching `on*` handler then every registered listener of that type,
/// and returns `!event.defaultPrevented`.  `once` listeners are pruned.
pub(crate) fn native_idb_dispatch_event(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (target, _kind) = require_idb_event_target(ctx, this, "dispatchEvent")?;
    // WebIDL `Event event`: a non-Event argument (or none) is a TypeError
    // before any §2.9 dispatch logic — matching the shared
    // `EventTarget.prototype.dispatchEvent` (host/event_target.rs).
    let event_id = match args.first().copied() {
        Some(JsValue::Object(id))
            if matches!(ctx.vm.get_object(id).kind, ObjectKind::Event { .. }) =>
        {
            id
        }
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'dispatchEvent' on 'EventTarget': \
                 parameter 1 is not of type 'Event'.",
            ))
        }
    };
    // The event's `type` + `bubbles` select dispatch behaviour: an `on<type>`
    // attribute name only exists for the IDB event set, so a dispatch of an
    // unknown type runs only the registered listeners.  Both are read from the
    // immutable internal slots (not the JS data properties) so a user-side
    // `delete evt.type` / overridden accessor cannot hijack dispatch, per the
    // `ObjectKind::Event` slot semantics.
    let ObjectKind::Event {
        type_sid: event_type,
        bubbles,
        ..
    } = ctx.vm.get_object(event_id).kind
    else {
        // Unreachable: brand-checked as `ObjectKind::Event` above.
        unreachable!("dispatchEvent argument brand-checked as Event");
    };
    let handler_attr = on_handler_sid(ctx.vm, event_type);
    // WHATWG DOM §2.9 step 1: a re-entrant dispatch of an event already in
    // flight throws `InvalidStateError` (a sequential `dispatchEvent(e);
    // dispatchEvent(e);` is fine — the dispatch flag is bracketed across the
    // walk inside `dispatch_idb_event`).  This is the ONLY dispatchEvent-
    // specific step; the flag bracket + target / currentTarget / propagation
    // bookkeeping all live in the shared core, so internal fires get them too.
    // Reuses the canonical `dispatched_events` set so the IDB path and
    // `EventTarget.prototype.dispatchEvent` share the one membership store.
    if ctx.vm.dispatched_events.contains(&event_id) {
        let name_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
        return Err(super::dom_exception::invalid_state_error(
            name_sid,
            "EventTarget",
            "dispatchEvent",
            "The event is already being dispatched.",
        ));
    }
    // §2.9: `target` is set for the whole dispatch and stays observable after
    // it — set it up front so `event.target` is correct even when no listener
    // runs (the shared dispatcher's no-observer early-return never builds /
    // touches the event).
    set_event_slot_raw(ctx.vm, event_id, EVENT_SLOT_TARGET, JsValue::Object(target));
    // Route through the one shared dispatcher: dispatch-flag bracketed,
    // GC-rooted, honors `stopPropagation` / `stopImmediatePropagation`, bubbles
    // along the IDB ancestor chain when `event.bubbles`, and finalizes
    // `currentTarget` / propagation flags — identical to internal fires.  The
    // event already exists, so `make_event` just hands it back.
    let _ = dispatch_idb_event(ctx, target, event_type, handler_attr, bubbles, |_ctx| {
        event_id
    });
    // §2.9: `dispatchEvent` returns `false` iff the event's default was
    // prevented.  Read the event's FINAL `default_prevented` directly rather
    // than the `FireResult` — the latter reports `canceled: false` on the
    // no-listener early-return, which would wrongly return `true` for an event
    // that was already `preventDefault()`'d before dispatch.
    let not_canceled = !matches!(
        ctx.vm.get_object(event_id).kind,
        ObjectKind::Event {
            default_prevented: true,
            ..
        }
    );
    Ok(JsValue::Boolean(not_canceled))
}

/// Map an IDB event-type SID to its `on<type>` handler-attribute SID
/// (`success` → `onsuccess`, …).  Returns a sentinel for an unknown type so
/// `collect` finds no handler (only listeners run).
fn on_handler_sid(vm: &VmInner, event_type: StringId) -> Option<StringId> {
    let wk = &vm.well_known;
    // `None` for an event type with no `on<type>` attribute — so a dispatch of
    // e.g. `new Event('onsuccess')` runs only `addEventListener` callbacks, not
    // the stored `onsuccess` handler (the event type must NOT double as the
    // no-handler sentinel: it could equal a handler-attribute name).
    if event_type == wk.success {
        Some(wk.onsuccess)
    } else if event_type == wk.error {
        Some(wk.onerror)
    } else if event_type == wk.complete {
        Some(wk.oncomplete)
    } else if event_type == wk.abort {
        Some(wk.onabort)
    } else if event_type == wk.upgradeneeded {
        Some(wk.onupgradeneeded)
    } else if event_type == wk.versionchange {
        Some(wk.onversionchange)
    } else if event_type == wk.blocked {
        Some(wk.onblocked)
    } else if event_type == wk.close {
        Some(wk.onclose)
    } else {
        None
    }
}
