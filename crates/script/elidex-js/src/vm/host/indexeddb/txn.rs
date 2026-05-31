//! IDBTransaction commit / abort lifecycle (W3C IndexedDB §5.4 / §5.5).
//!
//! Two-phase by design: the synchronous phase (`state = committing` +
//! durable backend write, or rollback) runs in `commit_transaction` /
//! `abort_transaction`; the observable event (`complete` / `abort`) fires
//! from a deferred [`PendingTask`] so the auto-commit sweep can iterate the
//! transaction map without user JS mutating it mid-iteration (plan §4.3).

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, NativeContext, ObjectId};
use super::super::super::VmInner;
use super::super::pending_tasks::PendingTask;
use super::{fire_idb_event, value, IdbReadyState, IdbTxnState};

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
    let (backend_txn, requests) = {
        let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) else {
            return;
        };
        if st.state == IdbTxnState::Finished {
            return;
        }
        st.state = IdbTxnState::Finished;
        (st.backend_txn.take(), std::mem::take(&mut st.request_list))
    };
    if let Some(mut bt) = backend_txn {
        if let Some(backend) = vm.idb_backend.clone() {
            let _ = bt.abort(backend.conn());
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
    vm.queue_task(PendingTask::IdbAbortDone { txn_id });
}

/// Deferred phase of §5.4 (`PendingTask::IdbCommitDone`): set
/// `state = finished` (step 2.5.2), finalize an upgrade backend-side, fire
/// `complete` (step 2.5.3), and for an upgrade transaction clear the open
/// request's `transaction` + fire its `success` (step 2.5.4).
pub(crate) fn dispatch_commit_done(vm: &mut VmInner, txn_id: ObjectId) {
    let (upgrade_request, upgrade_handle) = {
        let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) else {
            return;
        };
        st.state = IdbTxnState::Finished;
        (st.upgrade_request, st.upgrade_handle.take())
    };
    if let Some(handle) = upgrade_handle {
        if let Some(backend) = vm.idb_backend.clone() {
            let _ = elidex_indexeddb::database::finish_upgrade(&backend, &handle);
        }
    }
    let complete_sid = vm.well_known.complete;
    let oncomplete_sid = vm.well_known.oncomplete;
    let mut ctx = NativeContext::new_call(vm);
    // step 2.5.3: fire `complete` (non-bubbling, non-cancelable).
    fire_idb_event(&mut ctx, txn_id, complete_sid, oncomplete_sid, false, false);
    // step 2.5.4: an upgrade transaction's open request now resolves.
    if let Some(req) = upgrade_request {
        if let Some(rs) = ctx.vm.idb_request_states.get_mut(&req) {
            rs.transaction = None;
        }
        let success_sid = ctx.vm.well_known.success;
        let onsuccess_sid = ctx.vm.well_known.onsuccess;
        fire_idb_event(&mut ctx, req, success_sid, onsuccess_sid, false, false);
    }
}

/// Deferred phase of §5.5 (`PendingTask::IdbAbortDone`): fire `abort` at
/// the transaction (bubbling, non-cancelable).
pub(crate) fn dispatch_abort_done(vm: &mut VmInner, txn_id: ObjectId) {
    let abort_sid = vm.well_known.abort;
    let onabort_sid = vm.well_known.onabort;
    let mut ctx = NativeContext::new_call(vm);
    fire_idb_event(&mut ctx, txn_id, abort_sid, onabort_sid, false, true);
}
