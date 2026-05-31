//! IDBTransaction commit / abort lifecycle (W3C IndexedDB §5.4 / §5.5).
//!
//! Two-phase by design: the synchronous phase (`state = committing` +
//! durable backend write, or rollback) runs in `commit_transaction` /
//! `abort_transaction`; the observable event (`complete` / `abort`) fires
//! from a deferred [`PendingTask`] so the auto-commit sweep can iterate the
//! transaction map without user JS mutating it mid-iteration (plan §4.3).

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::super::pending_tasks::PendingTask;
use super::{fire_idb_event, object_store, value, IdbReadyState, IdbTransactionState, IdbTxnState};

/// W3C IDB §5.4 "commit a transaction" synchronous phase.  Sets
/// `state = committing` eagerly (the de-dup predicate the auto-commit
/// sweep reads), writes the durable backend transaction, then queues the
/// deferred [`PendingTask::IdbCommitDone`] for the `complete` event.
/// No-op if already committing / finished (idempotent under the two
/// commit triggers — §5.9 step 8.3 and the sweep).
pub(crate) fn commit_transaction(vm: &mut VmInner, txn_id: ObjectId) {
    let backend_txn = {
        let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) else {
            return;
        };
        if matches!(st.state, IdbTxnState::Committing | IdbTxnState::Finished) {
            return;
        }
        st.state = IdbTxnState::Committing;
        st.backend_txn.take()
    };
    // §5.4 step 2.3: write outstanding changes.  Backend is synchronous,
    // so the "in parallel" wait collapses to an immediate commit here.
    if let Some(mut bt) = backend_txn {
        if let Some(backend) = vm.idb_backend.clone() {
            if let Err(e) = bt.commit(backend.conn()) {
                // §5.4 step 2.4: write failed → abort with the error.
                let exc = value::backend_error_to_dom_exception(vm, &e);
                abort_transaction(vm, txn_id, Some(exc));
                return;
            }
        }
    }
    // §5.4 step 2.5: queue the deferred finish + `complete` event.
    vm.queue_task(PendingTask::IdbCommitDone { txn_id });
}

/// W3C IDB §5.5 "abort a transaction" synchronous phase: roll back the
/// backend transaction, set `state = finished`, mark every still-pending
/// request done with the abort `error`, then queue the deferred
/// [`PendingTask::IdbAbortDone`] for the `abort` event.
pub(crate) fn abort_transaction(vm: &mut VmInner, txn_id: ObjectId, error: Option<ObjectId>) {
    let (backend_txn, requests, upgrade_handle, upgrade_old_version, upgrade_request, db) = {
        let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) else {
            return;
        };
        if st.state == IdbTxnState::Finished {
            return;
        }
        st.state = IdbTxnState::Finished;
        // §4.10 `error`: record the abort cause so `transaction.error` reports
        // it (overrides any earlier value; the last abort cause wins).
        if error.is_some() {
            st.error = error;
        }
        (
            st.backend_txn.take(),
            std::mem::take(&mut st.request_list),
            st.upgrade_handle.take(),
            st.upgrade_old_version,
            st.upgrade_request,
            st.db,
        )
    };
    if let Some(mut bt) = backend_txn {
        if let Some(backend) = vm.idb_backend.clone() {
            let _ = bt.abort(backend.conn());
        }
    }
    // §5.8 abort an upgrade transaction: reset the version + clear the db
    // back-ref so a re-open re-runs the upgrade.
    if let Some(handle) = upgrade_handle {
        if let Some(backend) = vm.idb_backend.clone() {
            let _ =
                elidex_indexeddb::database::abort_upgrade(&backend, &handle, upgrade_old_version);
        }
        if let Some(dbid) = db {
            if let Some(dbs) = vm.idb_database_states.get_mut(&dbid) {
                if dbs.upgrade_txn == Some(txn_id) {
                    dbs.upgrade_txn = None;
                }
            }
        }
    }
    // §5.5: abort each pending request — set error + done, drop any
    // staged result so its `IdbDeliver` task (if still queued) no-ops.
    for req in requests {
        if let Some(rs) = vm.idb_request_states.get_mut(&req) {
            rs.ready_state = IdbReadyState::Done;
            rs.result = JsValue::Undefined;
            rs.error = error;
            rs.deferred = None;
        }
    }
    // An aborted upgrade transaction's open request fails with the error
    // (its `error` event fires via the IdbDeliver path is N/A here — set
    // directly + leave the open request's own error fire to the abort
    // event the caller observes; the open request error is surfaced by the
    // request `error` accessor).
    if let Some(req) = upgrade_request {
        if let Some(rs) = vm.idb_request_states.get_mut(&req) {
            rs.ready_state = IdbReadyState::Done;
            rs.error = error;
            rs.transaction = None;
        }
    }
    vm.queue_task(PendingTask::IdbAbortDone { txn_id });
}

/// Deferred phase of §5.4 (`PendingTask::IdbCommitDone`): set
/// `state = finished` (step 2.5.2), finalize an upgrade backend-side, fire
/// `complete` (step 2.5.3), and for an upgrade transaction clear the open
/// request's `transaction` + fire its `success` (step 2.5.4).
pub(crate) fn dispatch_commit_done(vm: &mut VmInner, txn_id: ObjectId) {
    let (upgrade_request, upgrade_handle, db) = {
        let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) else {
            return;
        };
        st.state = IdbTxnState::Finished;
        (st.upgrade_request, st.upgrade_handle.take(), st.db)
    };
    if let Some(handle) = upgrade_handle {
        if let Some(backend) = vm.idb_backend.clone() {
            let _ = elidex_indexeddb::database::finish_upgrade(&backend, &handle);
        }
        // The version-change transaction is over — clear the db back-ref.
        if let Some(dbid) = db {
            if let Some(dbs) = vm.idb_database_states.get_mut(&dbid) {
                if dbs.upgrade_txn == Some(txn_id) {
                    dbs.upgrade_txn = None;
                }
            }
        }
    }
    let complete_sid = vm.well_known.complete;
    let oncomplete_sid = vm.well_known.oncomplete;
    let mut ctx = NativeContext::new_call(vm);
    // step 2.5.3: fire `complete` (non-bubbling, non-cancelable).
    fire_idb_event(&mut ctx, txn_id, complete_sid, oncomplete_sid, false, false);
    // step 2.5.4: an upgrade transaction's open request now resolves —
    // clear its `transaction`, mark it done, and fire `success` (its
    // `result` was set to the IDBDatabase by the factory `open` flow).
    if let Some(req) = upgrade_request {
        if let Some(rs) = ctx.vm.idb_request_states.get_mut(&req) {
            rs.transaction = None;
            rs.ready_state = IdbReadyState::Done;
        }
        let success_sid = ctx.vm.well_known.success;
        let onsuccess_sid = ctx.vm.well_known.onsuccess;
        fire_idb_event(&mut ctx, req, success_sid, onsuccess_sid, false, false);
    }
}

/// Deferred phase of §5.5 (`PendingTask::IdbAbortDone`): fire `abort` at
/// the transaction (bubbling, non-cancelable).  When the aborted transaction
/// is an upgrade transaction, the open request also fails — fire `error` at it
/// (§5.1 step 10.8: a failed upgrade returns an `"AbortError"` and the open
/// request's `error` event fires) so an `openReq.onerror` handler runs, not
/// just the transaction's `onabort`.
pub(crate) fn dispatch_abort_done(vm: &mut VmInner, txn_id: ObjectId) {
    let upgrade_request = vm
        .idb_transaction_states
        .get(&txn_id)
        .and_then(|s| s.upgrade_request);
    let abort_sid = vm.well_known.abort;
    let onabort_sid = vm.well_known.onabort;
    let error_sid = vm.well_known.error;
    let onerror_sid = vm.well_known.onerror;
    let mut ctx = NativeContext::new_call(vm);
    fire_idb_event(&mut ctx, txn_id, abort_sid, onabort_sid, false, true);
    if let Some(req) = upgrade_request {
        // `abort_transaction` already set the open request's `error` +
        // `readyState = "done"` and cleared its `transaction`; just deliver
        // the event (error bubbles + is cancelable, like any request error).
        fire_idb_event(&mut ctx, req, error_sid, onerror_sid, true, true);
    }
}

// ---------------------------------------------------------------------------
// IDBTransaction wrapper + JS-facing methods (§4.10)
// ---------------------------------------------------------------------------

/// Allocate a normal (non-upgrade) `IDBTransaction` wrapper (§3.1.1).
/// Created `Active`; auto-commits when control returns to the event loop.
pub(crate) fn create_transaction(
    vm: &mut VmInner,
    db: ObjectId,
    db_name: &str,
    scope: Vec<String>,
    mode: elidex_indexeddb::IdbTransactionMode,
    backend_txn: elidex_indexeddb::IdbTransaction,
) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbTransaction,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_transaction_prototype,
        extensible: true,
    });
    vm.idb_transaction_states.insert(
        id,
        IdbTransactionState::new_active(mode, db, db_name, scope, backend_txn),
    );
    id
}

fn require_txn_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbTransaction) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBTransaction.prototype.{method} called on non-IDBTransaction"
    )))
}

/// `transaction.objectStore(name)` → `IDBObjectStore` (§4.10).
pub(crate) fn native_txn_object_store(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let txn_id = require_txn_this(ctx, this, "objectStore")?;
    let (db_name, scope, state, mode) = {
        let s = ctx
            .vm
            .idb_transaction_states
            .get(&txn_id)
            .ok_or_else(|| VmError::type_error("IDBTransaction state missing"))?;
        (s.db_name.clone(), s.scope.clone(), s.state, s.mode)
    };
    if state == IdbTxnState::Finished {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBTransaction.objectStore: the transaction has finished",
        ));
    }
    let name_sid = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let name = ctx.get_utf8(name_sid);
    // A versionchange transaction spans every store, so its (empty) scope
    // is not consulted; a normal transaction must name an in-scope store.
    if mode != elidex_indexeddb::IdbTransactionMode::VersionChange && !scope.contains(&name) {
        return Err(value::dom_exc(
            ctx,
            "NotFoundError",
            format!("IDBTransaction.objectStore: '{name}' is not in the transaction's scope"),
        ));
    }
    Ok(JsValue::Object(object_store::create_object_store_wrapper(
        ctx.vm, &db_name, &name, txn_id,
    )))
}

/// `transaction.commit()` (§3.1.1) — request an explicit commit.
pub(crate) fn native_txn_commit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let txn_id = require_txn_this(ctx, this, "commit")?;
    let state = ctx.vm.idb_transaction_states.get(&txn_id).map(|s| s.state);
    if matches!(state, Some(IdbTxnState::Committing | IdbTxnState::Finished)) {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBTransaction.commit: the transaction has already committed or aborted",
        ));
    }
    commit_transaction(ctx.vm, txn_id);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.10)
// ---------------------------------------------------------------------------

/// `transaction.mode` → `"readonly"` / `"readwrite"` / `"versionchange"`.
pub(crate) fn native_txn_get_mode(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_txn_this(ctx, this, "mode")?;
    let mode = ctx
        .vm
        .idb_transaction_states
        .get(&id)
        .map_or(elidex_indexeddb::IdbTransactionMode::ReadOnly, |s| s.mode);
    let s = match mode {
        elidex_indexeddb::IdbTransactionMode::ReadOnly => "readonly",
        elidex_indexeddb::IdbTransactionMode::ReadWrite => "readwrite",
        elidex_indexeddb::IdbTransactionMode::VersionChange => "versionchange",
    };
    Ok(JsValue::String(ctx.vm.strings.intern(s)))
}

/// `transaction.durability` (§4.10).  The backend enforces only `"default"`
/// durability (the hint is not yet honored — plan §1), so report `"default"`.
pub(crate) fn native_txn_get_durability(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_txn_this(ctx, this, "durability")?;
    Ok(JsValue::String(ctx.vm.strings.intern("default")))
}

/// `transaction.db` → the owning `IDBDatabase` (§4.10).
pub(crate) fn native_txn_get_db(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_txn_this(ctx, this, "db")?;
    let db = ctx.vm.idb_transaction_states.get(&id).and_then(|s| s.db);
    Ok(db.map_or(JsValue::Null, JsValue::Object))
}

/// `transaction.error` → the abort `DOMException`, else `null` (§4.10).
pub(crate) fn native_txn_get_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_txn_this(ctx, this, "error")?;
    let err = ctx.vm.idb_transaction_states.get(&id).and_then(|s| s.error);
    Ok(err.map_or(JsValue::Null, JsValue::Object))
}

/// `transaction.objectStoreNames` (§4.10).  Sorted `Array<string>` (no
/// `DOMStringList` in this VM).  A versionchange transaction spans every
/// store, so it lists the database's stores; a normal transaction lists its
/// scope.
pub(crate) fn native_txn_get_object_store_names(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_txn_this(ctx, this, "objectStoreNames")?;
    let (db_name, scope, mode) = {
        let Some(s) = ctx.vm.idb_transaction_states.get(&id) else {
            return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
        };
        (s.db_name.clone(), s.scope.clone(), s.mode)
    };
    let mut names = if mode == elidex_indexeddb::IdbTransactionMode::VersionChange {
        let backend = ctx.vm.require_idb_backend()?;
        backend.list_store_names(&db_name).unwrap_or_default()
    } else {
        scope
    };
    names.sort();
    let elems: Vec<JsValue> = names
        .iter()
        .map(|n| JsValue::String(ctx.vm.strings.intern(n)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elems)))
}

/// `transaction.abort()` (§3.1.1).
pub(crate) fn native_txn_abort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let txn_id = require_txn_this(ctx, this, "abort")?;
    let state = ctx.vm.idb_transaction_states.get(&txn_id).map(|s| s.state);
    if matches!(state, Some(IdbTxnState::Finished)) {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBTransaction.abort: the transaction has already committed or aborted",
        ));
    }
    abort_transaction(ctx.vm, txn_id, None);
    Ok(JsValue::Undefined)
}
