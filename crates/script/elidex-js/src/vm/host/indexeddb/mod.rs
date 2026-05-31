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

use super::super::value::{JsValue, ObjectId, StringId, VmError};
use super::super::VmInner;

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

    /// W3C IndexedDB §2.7.1 "cleanup Indexed Database transactions": commit
    /// every still-`Active` transaction whose request list is empty.  Run at
    /// the END of every microtask checkpoint (`drain_microtasks`), the exact
    /// HTML "perform a microtask checkpoint" step 5 seam — i.e. once the task
    /// that created (or last activated) the transaction has completed, BEFORE
    /// any later task can observe it.  The common case still commits earlier
    /// via §5.9 step 8.3 (after a request's success event); this is the
    /// fallback for a zero-request transaction that never fired an event.
    /// Eligible ids are collected first so `commit_transaction` (which mutates
    /// the entry in place and queues a task, but never inserts or removes map
    /// entries) cannot invalidate the iteration.  De-dup with §5.9 step 8.3
    /// and the previous checkpoint's sweep: a txn already committed is
    /// `Committing` / `Finished`, so the `Active` filter skips it.
    pub(crate) fn idb_autocommit_sweep(&mut self) {
        let eligible: Vec<ObjectId> = self
            .idb_transaction_states
            .iter()
            .filter(|(_, st)| {
                // §2.7.1 cleanup applies only to transactions with a "cleanup
                // event loop" — i.e. those created by a script call to
                // `transaction()`.  A versionchange (upgrade) transaction
                // (`upgrade_request.is_some()`) has no cleanup event loop: it
                // is created by the `open` algorithm in one task but ACTIVATED
                // by a later `IdbUpgrade` task, so it must NOT be committed at
                // the checkpoint between those two tasks.  Its lifecycle runs
                // via `run_post_dispatch` after `upgradeneeded` instead.
                st.state == IdbTxnState::Active
                    && st.request_list.is_empty()
                    && st.upgrade_request.is_none()
            })
            .map(|(id, _)| *id)
            .collect();
        for id in eligible {
            txn::commit_transaction(self, id);
        }
    }
}

// EventTarget model + §2.9/§2.10 dispatch live in `dispatch` (split for the
// ~1000-line convention).  Re-exported so callers keep using `super::*`.
pub(crate) mod dispatch;
pub(crate) use dispatch::{
    fire_idb_event, fire_version_change_event, native_idb_add_event_listener,
    native_idb_dispatch_event, native_idb_handler_get, native_idb_handler_set,
    native_idb_remove_event_listener, remove_idb_listeners_for_signal, FireResult,
};
