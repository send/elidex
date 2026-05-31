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

use super::super::value::{JsValue, ObjectId, StringId};

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
