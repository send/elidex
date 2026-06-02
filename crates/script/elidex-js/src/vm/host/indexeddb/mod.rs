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
//! via the microtask-checkpoint cleanup
//! ([`super::super::VmInner::idb_cleanup_transactions`]).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, ObjectId, VmError};
use super::super::VmInner;

pub(crate) mod cursor;
pub(crate) mod database;
pub(crate) mod factory;
pub(crate) mod index;
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
    /// Cursor iteration result (W3C IDB §6.7 "iterate a cursor").  Richer
    /// than a plain `JsValue`: at delivery the
    /// [`dispatch::commit_cursor_iteration`] step reads the cursor's backend
    /// position and either sets the §4.9 got-value flag + commits the
    /// `key` / `primaryKey` / `value` attribute snapshots and
    /// `request.result = cursor` (a record was found), or leaves got-value
    /// false with `request.result = null` (the cursor is exhausted).
    /// Staged by `openCursor` / `openKeyCursor` and re-staged on the SAME
    /// request by `continue` / `advance` / `continuePrimaryKey` (the
    /// iteration re-fire — plan §3).
    CursorIteration { cursor_id: ObjectId },
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
        }
    }
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
    /// Per-store-instance `IDBIndex` handle cache (§4.5 `index()` NOTE:
    /// `store.index("x") === store.index("x")`, and `=== createIndex("x",…)`).
    /// Keyed by index name → the `IDBIndex` wrapper `ObjectId`.  Dies with the
    /// store on sweep; the cache↔index `object_store` back-ref is an
    /// intentional cycle, collected together once the store is unreachable.
    pub(crate) index_handles: std::collections::HashMap<String, ObjectId>,
}

/// Per-`IDBIndex` handle state, keyed in
/// [`super::super::VmInner::idb_index_states`] (§4.6).  Identity tuple only —
/// `name` / `keyPath` / `unique` / `multiEntry` metadata is read on demand
/// from the backend so it never drifts from the schema.
#[derive(Debug)]
pub(crate) struct IdbIndexState {
    pub(crate) db_name: String,
    pub(crate) store_name: String,
    pub(crate) index_name: String,
    /// The `IDBObjectStore` handle that vended this index (`index.objectStore`
    /// + the source store for cursors opened on the index).
    pub(crate) object_store: ObjectId,
}

/// Per-`IDBCursor` / `IDBCursorWithValue` state, keyed in
/// [`super::super::VmInner::idb_cursor_states`] (§4.9).
pub(crate) struct IdbCursorState {
    /// Backend iteration mechanics (position + direction + range) — the
    /// single source of truth for "where" the cursor is.
    pub(crate) backend: elidex_indexeddb::cursor::IdbCursorState,
    /// `cursor.source` — the `IDBObjectStore` or `IDBIndex` handle.
    pub(crate) source: ObjectId,
    /// `cursor.request` — the iteration request, re-fired by
    /// `continue` / `advance` / `continuePrimaryKey` (plan §3 DR-1).
    pub(crate) request: ObjectId,
    /// `openKeyCursor` (key-only) vs `openCursor` (with value).  Gates the
    /// `IDBCursor` vs `IDBCursorWithValue` prototype + whether `value` is
    /// snapshotted.
    pub(crate) key_only: bool,
    /// W3C IDB §4.9 "got value flag" — true once an iteration has delivered a
    /// record, false initially / after `continue`-before-delivery / on
    /// exhaustion.  The §4.9 InvalidStateError gate for
    /// `continue`/`advance`/`update`/`delete` (and the double-`continue`
    /// guard — plan §3 DR-2).  NOT the backend's `got_deleted`.
    pub(crate) got_value: bool,
    /// `cursor.key` — §4.9 attribute snapshot committed at delivery (DR-2).
    pub(crate) key: JsValue,
    /// `cursor.primaryKey` — snapshot committed at delivery.
    pub(crate) primary_key: JsValue,
    /// `cursor.value` (with-value cursors only) — snapshot committed at
    /// delivery; held across `delete()`/`update()` until the next iteration.
    pub(crate) value: JsValue,
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

    /// W3C IndexedDB §2.7.1 "cleanup Indexed Database transactions": DEACTIVATE
    /// every still-`Active` transaction (set `state = inactive`), and commit the
    /// ones whose request list is empty.  Run at the END of every microtask
    /// checkpoint (`drain_microtasks`), the exact HTML "perform a microtask
    /// checkpoint" step 5 seam — i.e. once the task that created (or last
    /// activated) the transaction has completed, BEFORE any later task can
    /// observe it.
    ///
    /// Both arms matter:
    /// * Empty active txns commit (the common case also commits earlier via §5.9
    ///   step 8.3 after a request's success event; this is the fallback for a
    ///   zero-request transaction that never fired an event).
    /// * NON-empty active txns are deactivated (→ `inactive`) so that a later
    ///   task in the same drain cannot issue new requests against them
    ///   (`require_active` must then throw `TransactionInactiveError`); a request
    ///   event later reactivates the txn via `request::reactivate_if_inactive`
    ///   (§5.9 / §5.10 step 6).  Committing them here would be wrong — their
    ///   outstanding requests must still deliver.
    ///
    /// Ids are collected first so `commit_transaction` (which mutates the entry
    /// in place and queues a task, but never inserts or removes map entries)
    /// cannot invalidate the iteration.  De-dup with §5.9 step 8.3 and a prior
    /// checkpoint's cleanup: a txn already committed/deactivated is
    /// `Committing` / `Finished` / `Inactive`, so the `Active` filter skips it.
    pub(crate) fn idb_cleanup_transactions(&mut self) {
        // §2.7.1 cleanup applies only to transactions with a "cleanup event
        // loop" — those created by a script call to `transaction()`.  A
        // versionchange (upgrade) transaction (`upgrade_request.is_some()`) has
        // no cleanup event loop: it is created by the `open` algorithm in one
        // task but ACTIVATED by a later `IdbUpgrade` task, so it must NOT be
        // touched at the checkpoint between those two tasks.  Its lifecycle runs
        // via `run_post_dispatch` after `upgradeneeded` instead.
        let active: Vec<ObjectId> = self
            .idb_transaction_states
            .iter()
            .filter(|(_, st)| st.state == IdbTxnState::Active && st.upgrade_request.is_none())
            .map(|(id, _)| *id)
            .collect();
        for id in active {
            let empty = self
                .idb_transaction_states
                .get(&id)
                .is_some_and(|st| st.request_list.is_empty());
            if empty {
                // Active → Committing (commit_transaction accepts non-Committing
                // /-Finished); deactivation is subsumed by the terminal state.
                txn::commit_transaction(self, id);
            } else if let Some(st) = self.idb_transaction_states.get_mut(&id) {
                st.state = IdbTxnState::Inactive;
            }
        }
    }
}

// IDB UA-fire seam (W3C IDB §5.9/§5.10/§4.2) lives in `dispatch`; the
// EventTarget listener model + §2.9 dispatch are now the shared core
// (`#11-eventtarget-dispatch-core`).  Re-exported so callers keep using
// `super::*`.
pub(crate) mod dispatch;
pub(crate) use dispatch::{fire_idb_event, fire_version_change_event, FireResult};
