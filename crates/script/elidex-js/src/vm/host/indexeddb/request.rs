//! IDBRequest lifecycle (W3C IndexedDB §4.1 / §5.6 / §5.9 / §5.10).
//!
//! A request is created by an object-store / index / cursor / factory
//! operation, its backend result computed synchronously and staged in
//! `IdbRequestState.deferred`, then delivered via the
//! [`PendingTask::IdbDeliver`] database task (§5.6 step 5.6) — the event
//! fires after control returns to the event loop, never inline.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage,
};
use super::super::super::VmInner;
use super::super::pending_tasks::PendingTask;
use super::{
    fire_idb_event, txn, DeferredOutcome, FireResult, IdbReadyState, IdbRequestState, IdbTxnState,
};

/// Allocate a fresh `IDBRequest` (or `IDBOpenDBRequest` when `is_open`)
/// wrapper + its side-store state.  Prototype is chosen so
/// `instanceof IDBOpenDBRequest` / `IDBRequest` holds.
pub(crate) fn create_request(
    vm: &mut VmInner,
    source: Option<ObjectId>,
    transaction: Option<ObjectId>,
    is_open: bool,
) -> ObjectId {
    let proto = if is_open {
        vm.idb_open_db_request_prototype
    } else {
        vm.idb_request_prototype
    };
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbRequest,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.idb_request_states.insert(
        id,
        IdbRequestState {
            source,
            transaction,
            is_open,
            ..Default::default()
        },
    );
    id
}

/// W3C IDB §5.6 "asynchronously execute a request": create the request,
/// append it to the transaction's request list (step 4), stage the
/// already-computed backend `outcome`, and queue the delivery task
/// (step 5.6).  The backend operation itself runs synchronously at the
/// call site (no real parallelism), so `outcome` is final here.
pub(crate) fn async_execute(
    vm: &mut VmInner,
    source: Option<ObjectId>,
    transaction: Option<ObjectId>,
    outcome: DeferredOutcome,
) -> ObjectId {
    let req = create_request(vm, source, transaction, false);
    if let Some(st) = vm.idb_request_states.get_mut(&req) {
        st.deferred = Some(outcome);
    }
    if let Some(tid) = transaction {
        if let Some(tx) = vm.idb_transaction_states.get_mut(&tid) {
            tx.request_list.push(req);
        }
    }
    vm.queue_task(PendingTask::IdbDeliver { request_id: req });
    req
}

/// Drain step for [`PendingTask::IdbDeliver`] (§5.6 step 5.6).  Removes the
/// request from its transaction's request list **before** firing (so §5.9
/// step 8.3 reads the post-removal list), sets `readyState = "done"` +
/// result / error, then fires the `success` / `error` event.
pub(crate) fn dispatch_idb_deliver(vm: &mut VmInner, request_id: ObjectId) {
    let (outcome, txn_id) = {
        let Some(st) = vm.idb_request_states.get_mut(&request_id) else {
            return;
        };
        let Some(outcome) = st.deferred.take() else {
            return;
        };
        (outcome, st.transaction)
    };
    // §5.6 step 5.6.1: remove request from the transaction's request list.
    // MUST precede firing — fire_success/§5.9 step 8.3 reads the list to
    // decide auto-commit (plan §4.2 load-bearing invariant).
    if let Some(tid) = txn_id {
        if let Some(tx) = vm.idb_transaction_states.get_mut(&tid) {
            tx.request_list.retain(|&r| r != request_id);
        }
    }
    let is_error = {
        let st = vm
            .idb_request_states
            .get_mut(&request_id)
            .expect("request state present (just read above)");
        st.ready_state = IdbReadyState::Done;
        match outcome {
            DeferredOutcome::Success(v) => {
                st.result = v;
                st.error = None;
                false
            }
            DeferredOutcome::Error(e) => {
                st.result = JsValue::Undefined;
                st.error = Some(e);
                true
            }
        }
    };
    let error_id = if is_error {
        vm.idb_request_states.get(&request_id).and_then(|s| s.error)
    } else {
        None
    };
    let mut ctx = NativeContext::new_call(vm);
    if is_error {
        fire_error(&mut ctx, request_id, txn_id, error_id);
    } else {
        fire_success(&mut ctx, request_id, txn_id);
    }
}

/// Fire a `success` event at a request (W3C IDB §5.9).
pub(crate) fn fire_success(
    ctx: &mut NativeContext<'_>,
    request_id: ObjectId,
    txn_id: Option<ObjectId>,
) {
    let success_sid = ctx.vm.well_known.success;
    let onsuccess_sid = ctx.vm.well_known.onsuccess;
    if let Some(tid) = txn_id {
        reactivate_if_inactive(ctx.vm, tid);
    }
    // §5.9 step 7: dispatch (success is non-bubbling, non-cancelable).
    let res = fire_idb_event(ctx, request_id, success_sid, onsuccess_sid, false, false);
    if let Some(tid) = txn_id {
        run_post_dispatch(ctx, tid, &res, None);
    }
}

/// Fire an `error` event at a request (W3C IDB §5.10).  Bubbling +
/// cancelable; when not canceled the error aborts the transaction.
pub(crate) fn fire_error(
    ctx: &mut NativeContext<'_>,
    request_id: ObjectId,
    txn_id: Option<ObjectId>,
    error_id: Option<ObjectId>,
) {
    let error_sid = ctx.vm.well_known.error;
    let onerror_sid = ctx.vm.well_known.onerror;
    if let Some(tid) = txn_id {
        reactivate_if_inactive(ctx.vm, tid);
    }
    // §5.10 step 7: dispatch (error bubbles + is cancelable).
    let res = fire_idb_event(ctx, request_id, error_sid, onerror_sid, true, true);
    if let Some(tid) = txn_id {
        run_post_dispatch(ctx, tid, &res, error_id);
    }
}

/// §5.9 step 6 / §5.10 step 6: reactivate an inactive transaction for the
/// duration of event dispatch so a handler may issue new requests.
fn reactivate_if_inactive(vm: &mut VmInner, txn_id: ObjectId) {
    if let Some(st) = vm.idb_transaction_states.get_mut(&txn_id) {
        if st.state == IdbTxnState::Inactive {
            st.state = IdbTxnState::Active;
        }
    }
}

/// §5.9 step 8 / §5.10 step 8: post-dispatch transaction lifecycle.  Runs
/// only if the transaction is still `Active`.  `error_to_abort` is `Some`
/// for the error path (§5.10): when the event was not canceled the error
/// aborts the transaction.
fn run_post_dispatch(
    ctx: &mut NativeContext<'_>,
    txn_id: ObjectId,
    res: &FireResult,
    error_to_abort: Option<ObjectId>,
) {
    let active = matches!(
        ctx.vm.idb_transaction_states.get(&txn_id).map(|s| s.state),
        Some(IdbTxnState::Active)
    );
    if !active {
        return;
    }
    // step 8.1: set inactive.
    if let Some(st) = ctx.vm.idb_transaction_states.get_mut(&txn_id) {
        st.state = IdbTxnState::Inactive;
    }
    // step 8.2: a listener threw → abort with "AbortError".
    if res.threw {
        let abort_err = build_abort_error(ctx.vm);
        txn::abort_transaction(ctx.vm, txn_id, Some(abort_err));
        return;
    }
    // §5.10 step 8.3: error not canceled (preventDefault not called) →
    // abort the transaction with the request's error.
    if let Some(err) = error_to_abort {
        if !res.canceled {
            txn::abort_transaction(ctx.vm, txn_id, Some(err));
            return;
        }
    }
    // §5.9 step 8.3 / §5.10 step 8.4: request list empty → commit.
    let empty = ctx
        .vm
        .idb_transaction_states
        .get(&txn_id)
        .is_some_and(|s| s.request_list.is_empty());
    if empty {
        txn::commit_transaction(ctx.vm, txn_id);
    }
}

/// Build an `"AbortError"` `DOMException` (WHATWG DOM) for §5.9 step 8.2.
fn build_abort_error(vm: &mut VmInner) -> ObjectId {
    let name = vm.strings.intern("AbortError");
    match vm.build_dom_exception(name, "transaction aborted: an event handler threw") {
        JsValue::Object(id) => id,
        _ => unreachable!("build_dom_exception returned a non-object"),
    }
}
