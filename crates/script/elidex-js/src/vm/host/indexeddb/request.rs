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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
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

/// Stage a final outcome on an already-created request and queue its
/// delivery task.  Used by the factory `open` / `deleteDatabase` flow,
/// whose request is created up-front (to return it synchronously) before
/// the backend result is known.
pub(crate) fn stage_and_queue(vm: &mut VmInner, request_id: ObjectId, outcome: DeferredOutcome) {
    if let Some(st) = vm.idb_request_states.get_mut(&request_id) {
        st.deferred = Some(outcome);
    }
    vm.queue_task(PendingTask::IdbDeliver { request_id });
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
    error_obj: Option<ObjectId>,
) {
    let type_sid = ctx.vm.well_known.error;
    let handler_sid = ctx.vm.well_known.onerror;
    if let Some(tid) = txn_id {
        reactivate_if_inactive(ctx.vm, tid);
    }
    // §5.10 step 7: dispatch (error bubbles + is cancelable).
    let res = fire_idb_event(ctx, request_id, type_sid, handler_sid, true, true);
    if let Some(tid) = txn_id {
        run_post_dispatch(ctx, tid, &res, error_obj);
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
/// aborts the transaction.  Also reused by the §5.7 upgrade flow after the
/// `upgradeneeded` event (the upgrade transaction auto-commits the same
/// way once its request list empties).
pub(super) fn run_post_dispatch(
    ctx: &mut NativeContext<'_>,
    txn_id: ObjectId,
    res: &FireResult,
    error_to_abort: Option<ObjectId>,
) {
    // Only an Active or (explicit-commit) Committing transaction runs the
    // post-dispatch lifecycle.  A Committing txn is one whose `commit()` was
    // called while requests were still outstanding (§5.4 step 2.1 wait): its
    // deferred write runs here once the list empties.
    let (active, committing) = match ctx.vm.idb_transaction_states.get(&txn_id).map(|s| s.state) {
        Some(IdbTxnState::Active) => (true, false),
        Some(IdbTxnState::Committing) => (false, true),
        _ => return,
    };
    // step 8.1: a still-active txn goes inactive after dispatch (a committing
    // txn stays committing — it accepts no new requests).
    if active {
        if let Some(st) = ctx.vm.idb_transaction_states.get_mut(&txn_id) {
            st.state = IdbTxnState::Inactive;
        }
    }
    // step 8.2: a listener threw → abort with "AbortError".
    if res.threw {
        let abort_err = build_abort_error(ctx.vm);
        txn::abort_transaction(ctx.vm, txn_id, Some(abort_err));
        return;
    }
    // §5.10 step 8.3: error not canceled (preventDefault not called by the
    // request OR any bubbled-to transaction/database listener) → abort the
    // transaction with the request's error.
    if let Some(err) = error_to_abort {
        if !res.canceled {
            txn::abort_transaction(ctx.vm, txn_id, Some(err));
            return;
        }
    }
    // §5.9 step 8.3 / §5.4 step 2.1: when the request list empties, commit.
    let empty = ctx
        .vm
        .idb_transaction_states
        .get(&txn_id)
        .is_some_and(|s| s.request_list.is_empty());
    if empty {
        if committing {
            // explicit commit() requested earlier; the last outstanding
            // delivery just drained the list — do the deferred durable write.
            txn::finalize_commit(ctx.vm, txn_id);
        } else {
            // normal auto-commit: flip Inactive→Committing + write.
            txn::commit_transaction(ctx.vm, txn_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.1)
// ---------------------------------------------------------------------------

/// Brand-check that `this` is an `IDBRequest` / `IDBOpenDBRequest`.
fn require_request_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbRequest) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBRequest.prototype.{member} called on non-IDBRequest"
    )))
}

/// `request.readyState` → `"pending"` / `"done"` (§4.1).
pub(crate) fn native_req_get_ready_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "readyState")?;
    let state = ctx
        .vm
        .idb_request_states
        .get(&id)
        .map_or(IdbReadyState::Pending, |s| s.ready_state);
    let sid = ctx.vm.strings.intern(state.as_str());
    Ok(JsValue::String(sid))
}

/// `request.result` (§4.1) — `InvalidStateError` while still pending.
pub(crate) fn native_req_get_result(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "result")?;
    match ctx.vm.idb_request_states.get(&id) {
        // §4.1 step 2: return the result, or `undefined` if the request
        // resulted in an error (a done-with-error request must NOT expose a
        // stale result — e.g. an aborted upgrade whose open request still
        // holds the connection it was given before the abort).
        Some(s) if s.ready_state == IdbReadyState::Done => {
            if s.error.is_some() {
                Ok(JsValue::Undefined)
            } else {
                Ok(s.result)
            }
        }
        _ => Err(super::value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBRequest.result: the request has not completed",
        )),
    }
}

/// `request.error` (§4.1) — the `DOMException` on failure, else `null`.
/// `InvalidStateError` while still pending (symmetric with `result`: the
/// §4.1 getter throws when the request's done flag is false).
pub(crate) fn native_req_get_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "error")?;
    match ctx.vm.idb_request_states.get(&id) {
        Some(s) if s.ready_state == IdbReadyState::Done => {
            Ok(s.error.map_or(JsValue::Null, JsValue::Object))
        }
        _ => Err(super::value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBRequest.error: the request has not completed",
        )),
    }
}

/// `request.source` (§4.1) — the object store / index / cursor, else `null`.
pub(crate) fn native_req_get_source(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "source")?;
    let src = ctx.vm.idb_request_states.get(&id).and_then(|s| s.source);
    Ok(src.map_or(JsValue::Null, JsValue::Object))
}

/// `request.transaction` (§4.1) — the owning transaction, else `null`.
pub(crate) fn native_req_get_transaction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_request_this(ctx, this, "transaction")?;
    let txn = ctx
        .vm
        .idb_request_states
        .get(&id)
        .and_then(|s| s.transaction);
    Ok(txn.map_or(JsValue::Null, JsValue::Object))
}

/// Build an `"AbortError"` `DOMException` (WHATWG DOM) for §5.9 step 8.2.
fn build_abort_error(vm: &mut VmInner) -> ObjectId {
    let name = vm.strings.intern("AbortError");
    match vm.build_dom_exception(name, "transaction aborted: an event handler threw") {
        JsValue::Object(id) => id,
        _ => unreachable!("build_dom_exception returned a non-object"),
    }
}
