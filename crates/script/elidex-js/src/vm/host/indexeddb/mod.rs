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
};
use super::super::VmInner;
use super::events::{set_event_slot_raw, EventInit, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_TARGET};

pub(crate) mod database;
pub(crate) mod factory;
pub(crate) mod key_range;
pub(crate) mod object_store;
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
    /// `true` for `IDBOpenDBRequest` (gates `onupgradeneeded` / `onblocked`).
    pub(crate) is_open: bool,
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
            is_open: false,
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
/// `event_type` from the target's side-store, removing any `once`
/// listeners of that type (WHATWG DOM dispatch removes them before the
/// callback runs).  The three IDB EventTarget state structs share field
/// names (`handlers` / `listeners`), so one macro covers all three.
fn collect_and_prune(
    vm: &mut VmInner,
    target: ObjectId,
    event_type: StringId,
    handler_attr: StringId,
) -> (Option<ObjectId>, Vec<ObjectId>) {
    // `ObjectKind` is not `Copy` (Vec/Box payloads), so reduce to a small
    // discriminant under the shared borrow before taking the `&mut` map
    // borrow below.
    enum Which {
        Request,
        Transaction,
        Database,
        Other,
    }
    let which = match &vm.get_object(target).kind {
        ObjectKind::IdbRequest => Which::Request,
        ObjectKind::IdbTransaction => Which::Transaction,
        ObjectKind::IdbDatabase => Which::Database,
        _ => Which::Other,
    };
    macro_rules! pull {
        ($map:ident) => {{
            match vm.$map.get_mut(&target) {
                Some(st) => {
                    let handler = st.handlers.get(&handler_attr).copied();
                    let cbs: Vec<ObjectId> = st
                        .listeners
                        .iter()
                        .filter(|l| l.event_type == event_type)
                        .map(|l| l.callback)
                        .collect();
                    st.listeners
                        .retain(|l| !(l.event_type == event_type && l.once));
                    (handler, cbs)
                }
                None => (None, Vec::new()),
            }
        }};
    }
    match which {
        Which::Request => pull!(idb_request_states),
        Which::Transaction => pull!(idb_transaction_states),
        Which::Database => pull!(idb_database_states),
        Which::Other => (None, Vec::new()),
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

/// Shared event-build + dispatch.  `proto_override` reparents the event to
/// a subclass prototype (e.g. `IDBVersionChangeEvent.prototype`);
/// `extra_props` installs own data properties on the event before
/// dispatch.
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
    let (handler, listeners) = collect_and_prune(ctx.vm, target, event_type, handler_attr);
    if handler.is_none() && listeners.is_empty() {
        return FireResult {
            threw: false,
            canceled: false,
        };
    }
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
    set_event_slot_raw(ctx.vm, event_id, EVENT_SLOT_TARGET, JsValue::Object(target));
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_CURRENT_TARGET,
        JsValue::Object(target),
    );
    let mut threw = false;
    // Errors swallowed per WHATWG event-handler-attribute semantics
    // (uncaught exceptions log, don't propagate) but recorded so §5.9
    // step 8.2 / §5.10 step 8.2 can abort the transaction.
    if let Some(h) = handler {
        if ctx
            .call_function(h, JsValue::Object(target), &[JsValue::Object(event_id)])
            .is_err()
        {
            threw = true;
        }
    }
    for cb in listeners {
        if ctx
            .call_function(cb, JsValue::Object(target), &[JsValue::Object(event_id)])
            .is_err()
        {
            threw = true;
        }
    }
    let canceled = matches!(
        ctx.vm.get_object(event_id).kind,
        ObjectKind::Event {
            default_prevented: true,
            ..
        }
    );
    FireResult { threw, canceled }
}
