//! IDBCursor / IDBCursorWithValue (W3C IndexedDB §4.9 / §6.7).
//!
//! A cursor iterates an object store or index in key order.  The genuinely
//! new control-flow vs the one-shot object-store operations (plan §3) is the
//! **iteration re-fire**: `openCursor` returns a request whose result is the
//! cursor (or `null`); `continue` / `advance` / `continuePrimaryKey`
//! re-deliver the cursor's EXISTING request (§5.6 request-given variant) after
//! advancing the backend position.  `update` / `delete` are by contrast
//! ordinary one-shot **new** requests sourced at the cursor.
//!
//! The §4.9 "got value flag" + the per-iteration `key` / `primaryKey` /
//! `value` attribute snapshots are VM-side state (the backend has only a live
//! position + a separate `got_deleted` flag).  They are committed at DELIVERY
//! ([`commit_iteration`]), never at the synchronous call site, so a
//! same-handler double-`continue()` correctly throws (the second sees
//! `got_value == false`) — plan §3 DR-2.
//!
//! Layering: all iteration ordering / SQL lives in the `elidex-indexeddb`
//! backend (`cursor.rs`).  The only host "algorithm" here is the
//! got-value/snapshot/re-fire orchestration, which is VM-event-loop-bound and
//! correctly engine-side (plan §9).

#![cfg(feature = "engine")]

use elidex_indexeddb::cursor::{CursorDirection, CursorSource};

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::{object_store, request, value, DeferredOutcome, IdbCursorState, IdbReadyState};

/// `CursorDirection` → its JS string form (§4.9 `direction`).
fn direction_str(d: CursorDirection) -> &'static str {
    match d {
        CursorDirection::Next => "next",
        CursorDirection::NextUnique => "nextunique",
        CursorDirection::Prev => "prev",
        CursorDirection::PrevUnique => "prevunique",
    }
}

/// Parse the optional `direction` argument (§4.9 default `"next"`).  An
/// unrecognized string is a WebIDL enum conversion failure → `TypeError`.
pub(super) fn parse_direction(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<CursorDirection, VmError> {
    match arg {
        None | Some(JsValue::Undefined) => Ok(CursorDirection::Next),
        Some(v) => {
            let sid = ctx.to_string_val(v)?;
            let s = ctx.get_utf8(sid);
            CursorDirection::parse(&s).ok_or_else(|| {
                VmError::type_error(format!(
                    "IDBCursorDirection: '{s}' is not a valid enum value"
                ))
            })
        }
    }
}

/// Allocate an `IDBCursor` (key-only) / `IDBCursorWithValue` wrapper over a
/// freshly-opened backend cursor, stage its iteration outcome, and return the
/// owning `IDBRequest` (plan §3 — the cursor + request are wired so
/// `cursor.source = request.source = source`, `cursor.request = request`, and
/// `request.result = cursor` at delivery).  Shared by `IDBObjectStore` /
/// `IDBIndex` `openCursor` / `openKeyCursor`.
pub(super) fn create_cursor(
    ctx: &mut NativeContext<'_>,
    source: ObjectId,
    txn: ObjectId,
    backend_state: elidex_indexeddb::cursor::IdbCursorState,
    key_only: bool,
) -> JsValue {
    let proto = if key_only {
        ctx.vm.idb_cursor_prototype
    } else {
        ctx.vm.idb_cursor_with_value_prototype
    };
    let cursor_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::IdbCursor,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Issue the iteration request FIRST (its CursorIteration outcome roots the
    // cursor across delivery, DR-3); native calls are atomic w.r.t. GC, so the
    // cursor's own side-store entry can be inserted immediately after with the
    // request id wired in.  `request.source = source`, `request.transaction =
    // txn` (so it lands in the txn's request list → auto-commit waits for the
    // first iteration).
    let req = request::async_execute(
        ctx.vm,
        Some(source),
        Some(txn),
        DeferredOutcome::CursorIteration { cursor_id },
        None,
    );
    ctx.vm.idb_cursor_states.insert(
        cursor_id,
        IdbCursorState {
            backend: backend_state,
            source,
            request: req,
            key_only,
            got_value: false,
            key: JsValue::Undefined,
            primary_key: JsValue::Undefined,
            value: JsValue::Undefined,
        },
    );
    JsValue::Object(req)
}

/// W3C IDB §6.7 "iterate a cursor" — the delivery step (called from
/// `request::dispatch_idb_deliver` for a staged `CursorIteration` outcome).
/// Reads the cursor's backend position: a record present ⇒ set the got-value
/// flag, commit the `key` / `primaryKey` / `value` snapshots, and
/// `request.result = cursor`; exhausted ⇒ got-value stays false, attributes
/// `undefined`, `request.result = null` (plan §3 DR-2).
pub(super) fn commit_iteration(vm: &mut VmInner, request_id: ObjectId, cursor_id: ObjectId) {
    // Snapshot the backend entry out (cloned) so the cursor-state borrow is
    // released before the marshalling re-borrows `vm`.
    let (entry, key_only) = match vm.idb_cursor_states.get(&cursor_id) {
        Some(c) => (
            c.backend
                .current()
                .map(|e| (e.key.clone(), e.primary_key.clone(), e.value.clone())),
            c.key_only,
        ),
        None => return,
    };
    if let Some((key, primary_key, value_json)) = entry {
        let key_js = value::idb_key_to_js(vm, &key);
        let pk_js = value::idb_key_to_js(vm, &primary_key);
        // §4.9: `value` is snapshotted only for a with-value cursor; a key-only
        // cursor has no `value` attribute.
        let value_js = if key_only {
            JsValue::Undefined
        } else {
            value_json
                .as_deref()
                .map_or(JsValue::Undefined, |j| value::json_to_js(vm, j))
        };
        if let Some(c) = vm.idb_cursor_states.get_mut(&cursor_id) {
            c.got_value = true;
            c.key = key_js;
            c.primary_key = pk_js;
            c.value = value_js;
        }
        if let Some(st) = vm.idb_request_states.get_mut(&request_id) {
            st.ready_state = IdbReadyState::Done;
            st.result = JsValue::Object(cursor_id);
            st.error = None;
        }
    } else {
        // Exhausted (or the current record was deleted): result is null and the
        // cursor's attributes clear; got-value stays false so a further
        // `continue()` throws InvalidStateError.
        if let Some(c) = vm.idb_cursor_states.get_mut(&cursor_id) {
            c.got_value = false;
            c.key = JsValue::Undefined;
            c.primary_key = JsValue::Undefined;
            c.value = JsValue::Undefined;
        }
        if let Some(st) = vm.idb_request_states.get_mut(&request_id) {
            st.ready_state = IdbReadyState::Done;
            st.result = JsValue::Null;
            st.error = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Shared accessors / guards
// ---------------------------------------------------------------------------

/// Brand-check that `this` is an `IDBCursor` / `IDBCursorWithValue`.
fn require_cursor_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbCursor) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBCursor.prototype.{member} called on non-IDBCursor"
    )))
}

/// The cursor's owning transaction (via its iteration request, §4.9).
fn cursor_txn(ctx: &NativeContext<'_>, cursor_id: ObjectId) -> Result<ObjectId, VmError> {
    let request = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .map(|c| c.request)
        .ok_or_else(|| VmError::type_error("IDBCursor state missing"))?;
    ctx.vm
        .idb_request_states
        .get(&request)
        .and_then(|s| s.transaction)
        .ok_or_else(|| VmError::type_error("IDBCursor has no transaction"))
}

/// §4.9: whether the cursor's source / effective object store still exists
/// (`false` ⇒ deleted mid-transaction → InvalidStateError).
fn cursor_source_live(ctx: &mut NativeContext<'_>, cursor_id: ObjectId) -> bool {
    let Some(backend) = ctx.vm.ensure_idb_backend() else {
        return false;
    };
    let Some(st) = ctx.vm.idb_cursor_states.get(&cursor_id) else {
        return false;
    };
    match st.backend.source() {
        CursorSource::ObjectStore {
            db_name,
            store_name,
        } => backend.get_store_meta(db_name, store_name).is_ok(),
        CursorSource::Index {
            db_name,
            store_name,
            index_name,
        } => {
            backend.get_store_meta(db_name, store_name).is_ok()
                && elidex_indexeddb::index::get_index_meta(
                    &backend, db_name, store_name, index_name,
                )
                .is_ok()
        }
    }
}

/// Throw an `InvalidStateError` `DOMException` (§4.9 cursor-state guards).
fn invalid_state(ctx: &mut NativeContext<'_>, message: &str) -> VmError {
    value::dom_exc(ctx, "InvalidStateError", message.to_string())
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.9)
// ---------------------------------------------------------------------------

/// `cursor.direction` → `"next"` / `"nextunique"` / `"prev"` / `"prevunique"`.
pub(crate) fn native_cursor_get_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "direction")?;
    let dir = ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(CursorDirection::Next, |c| c.backend.direction());
    Ok(JsValue::String(ctx.vm.strings.intern(direction_str(dir))))
}

/// `cursor.key` — the per-iteration key snapshot (§4.9).
pub(crate) fn native_cursor_get_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "key")?;
    Ok(ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(JsValue::Undefined, |c| c.key))
}

/// `cursor.primaryKey` — the per-iteration primary-key snapshot (§4.9).
pub(crate) fn native_cursor_get_primary_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "primaryKey")?;
    Ok(ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(JsValue::Undefined, |c| c.primary_key))
}

/// `cursor.source` → the `IDBObjectStore` / `IDBIndex` (§4.9).
pub(crate) fn native_cursor_get_source(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "source")?;
    Ok(ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(JsValue::Null, |c| JsValue::Object(c.source)))
}

/// `cursor.request` → the iteration `IDBRequest` (§4.9).
pub(crate) fn native_cursor_get_request(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "request")?;
    Ok(ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(JsValue::Null, |c| JsValue::Object(c.request)))
}

/// `cursor.value` — the per-iteration value snapshot (IDBCursorWithValue,
/// §4.9).  Installed only on the with-value prototype.
pub(crate) fn native_cursor_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_cursor_this(ctx, this, "value")?;
    Ok(ctx
        .vm
        .idb_cursor_states
        .get(&id)
        .map_or(JsValue::Undefined, |c| c.value))
}

// ---------------------------------------------------------------------------
// Iteration (re-fire) — continue / advance / continuePrimaryKey (§4.9 / §6.7)
// ---------------------------------------------------------------------------

/// Shared §4.9 pre-iteration guards for `continue` / `advance` /
/// `continuePrimaryKey`: the transaction must be active, the source must not
/// be deleted, and the got-value flag must be set (else the cursor is between
/// iterations / exhausted — the double-`continue` guard, DR-2).  Returns the
/// cursor's `(request, transaction)`.
fn require_iterable(
    ctx: &mut NativeContext<'_>,
    cursor_id: ObjectId,
    method: &str,
) -> Result<(ObjectId, ObjectId), VmError> {
    let txn = cursor_txn(ctx, cursor_id)?;
    object_store::require_active(ctx, txn, method)?;
    if !cursor_source_live(ctx, cursor_id) {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor's source or effective object store has been deleted",
        ));
    }
    let got_value = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .is_some_and(|c| c.got_value);
    if !got_value {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor is not iterable in its current state",
        ));
    }
    let request = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .map(|c| c.request)
        .expect("cursor state present (checked above)");
    Ok((request, txn))
}

/// Re-fire the cursor's request after a successful backend advance: clear the
/// got-value flag (committed true again only at the next delivery) and re-stage
/// `CursorIteration` on the SAME request (§5.6 request-given variant, DR-1).
fn refire(ctx: &mut NativeContext<'_>, cursor_id: ObjectId, request: ObjectId) -> JsValue {
    if let Some(c) = ctx.vm.idb_cursor_states.get_mut(&cursor_id) {
        c.got_value = false;
    }
    request::async_execute(
        ctx.vm,
        None,
        None,
        DeferredOutcome::CursorIteration { cursor_id },
        Some(request),
    );
    JsValue::Undefined
}

/// `cursor.continue(key?)` (§4.9).
pub(crate) fn native_cursor_continue(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let cursor_id = require_cursor_this(ctx, this, "continue")?;
    let (request, _txn) = require_iterable(ctx, cursor_id, "continue")?;
    // Optional target key (an invalid / wrong-direction key is a synchronous
    // DataError — checked by the backend BEFORE the position is mutated, so the
    // got-value flag is left intact and the handler may retry).
    let target = match args.first().copied() {
        None | Some(JsValue::Undefined) => None,
        Some(k) => Some(value::js_to_idb_key(ctx, k)?),
    };
    let backend = ctx.vm.require_idb_backend()?;
    let res = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get_mut(&cursor_id)
            .expect("cursor state present");
        elidex_indexeddb::cursor::continue_cursor(&backend, &mut st.backend, target.as_ref())
    };
    match res {
        Ok(()) => Ok(refire(ctx, cursor_id, request)),
        Err(e) => Err(value::backend_error_as_throw(ctx, &e)),
    }
}

/// `cursor.advance(count)` (§4.9).  `count` is `[EnforceRange] unsigned long`;
/// `count == 0` is a `TypeError` thrown FIRST (before the transaction checks).
pub(crate) fn native_cursor_advance(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let cursor_id = require_cursor_this(ctx, this, "advance")?;
    let count_arg = value::require_arg(args, 0, "IDBCursor", "advance", 1)?;
    let count = enforce_range_unsigned_long(ctx, count_arg)?;
    if count == 0 {
        return Err(VmError::type_error(
            "IDBCursor.advance: count must not be 0",
        ));
    }
    let (request, _txn) = require_iterable(ctx, cursor_id, "advance")?;
    let backend = ctx.vm.require_idb_backend()?;
    let res = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get_mut(&cursor_id)
            .expect("cursor state present");
        elidex_indexeddb::cursor::advance(&backend, &mut st.backend, count)
    };
    match res {
        Ok(()) => Ok(refire(ctx, cursor_id, request)),
        Err(e) => Err(value::backend_error_as_throw(ctx, &e)),
    }
}

/// `cursor.continuePrimaryKey(key, primaryKey)` (§4.9).  Index cursors only,
/// not `*Unique` direction (both InvalidAccessError, checked before the
/// got-value guard per spec order).
pub(crate) fn native_cursor_continue_primary_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let cursor_id = require_cursor_this(ctx, this, "continuePrimaryKey")?;
    let txn = cursor_txn(ctx, cursor_id)?;
    object_store::require_active(ctx, txn, "continuePrimaryKey")?;
    // §4.9 steps 2-3: InvalidAccessError precedes the source-deleted /
    // got-value InvalidStateErrors.  Both are cheaply decidable host-side
    // (source brand + backend direction), so check them here in spec order.
    let (source, direction) = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get(&cursor_id)
            .ok_or_else(|| VmError::type_error("IDBCursor state missing"))?;
        (st.source, st.backend.direction())
    };
    if !matches!(ctx.vm.get_object(source).kind, ObjectKind::IdbIndex) {
        return Err(value::dom_exc(
            ctx,
            "InvalidAccessError",
            "IDBCursor.continuePrimaryKey: only valid on index cursors",
        ));
    }
    if matches!(
        direction,
        CursorDirection::NextUnique | CursorDirection::PrevUnique
    ) {
        return Err(value::dom_exc(
            ctx,
            "InvalidAccessError",
            "IDBCursor.continuePrimaryKey: not valid with a unique direction",
        ));
    }
    if !cursor_source_live(ctx, cursor_id) {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor's source or effective object store has been deleted",
        ));
    }
    let got_value = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .is_some_and(|c| c.got_value);
    if !got_value {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor is not iterable in its current state",
        ));
    }
    let request = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .map(|c| c.request)
        .expect("cursor state present");
    let key_arg = value::require_arg(args, 0, "IDBCursor", "continuePrimaryKey", 2)?;
    let pk_arg = value::require_arg(args, 1, "IDBCursor", "continuePrimaryKey", 2)?;
    let key = value::js_to_idb_key(ctx, key_arg)?;
    let primary_key = value::js_to_idb_key(ctx, pk_arg)?;
    let backend = ctx.vm.require_idb_backend()?;
    let res = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get_mut(&cursor_id)
            .expect("cursor state present");
        elidex_indexeddb::cursor::continue_primary_key(
            &backend,
            &mut st.backend,
            &key,
            &primary_key,
        )
    };
    match res {
        Ok(()) => Ok(refire(ctx, cursor_id, request)),
        Err(e) => Err(value::backend_error_as_throw(ctx, &e)),
    }
}

// ---------------------------------------------------------------------------
// Mutation — update / delete (§4.9) — ordinary NEW requests sourced at cursor
// ---------------------------------------------------------------------------

/// Shared §4.9 write guards for `update` / `delete`: active + writable +
/// source-live + got-value + not-key-only.  Returns the owning transaction.
fn require_writable_cursor(
    ctx: &mut NativeContext<'_>,
    cursor_id: ObjectId,
    method: &str,
) -> Result<ObjectId, VmError> {
    let txn = cursor_txn(ctx, cursor_id)?;
    object_store::require_active(ctx, txn, method)?;
    object_store::require_writable(ctx, txn, method)?;
    if !cursor_source_live(ctx, cursor_id) {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor's source or effective object store has been deleted",
        ));
    }
    let (got_value, key_only) = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .map_or((false, true), |c| (c.got_value, c.key_only));
    if !got_value {
        return Err(invalid_state(
            ctx,
            "IDBCursor: the cursor is not in a valid state for this operation",
        ));
    }
    if key_only {
        return Err(invalid_state(
            ctx,
            "IDBCursor: cannot modify records through a key-only cursor",
        ));
    }
    Ok(txn)
}

/// `cursor.update(value)` (§4.9) → `IDBRequest` (a new request whose result is
/// the updated record's primary key).
pub(crate) fn native_cursor_update(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let cursor_id = require_cursor_this(ctx, this, "update")?;
    let value_arg = value::require_arg(args, 0, "IDBCursor", "update", 1)?;
    let txn = require_writable_cursor(ctx, cursor_id, "update")?;
    // §4.9 step: clone the value (txn inactive during the clone, §5.11), then
    // re-check active (a getter side effect could have aborted the txn).
    let json = object_store::clone_value_guarded(ctx, txn, value_arg)?;
    object_store::require_active(ctx, txn, "update")?;
    let primary_key = ctx
        .vm
        .idb_cursor_states
        .get(&cursor_id)
        .map_or(JsValue::Undefined, |c| c.primary_key);
    let backend = ctx.vm.require_idb_backend()?;
    let res = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get(&cursor_id)
            .expect("cursor state present");
        elidex_indexeddb::cursor::update_current(&backend, &st.backend, &json)
    };
    let outcome = match res {
        // The write ran synchronously against the backend; the request just
        // delivers the record's primary key (§4.9 update result).
        Ok(()) => DeferredOutcome::Success(primary_key),
        // §4.9: an in-line-key mismatch (DataError) or a deleted current record
        // (InvalidStateError) is a synchronous throw, not a request error.
        Err(
            e @ (elidex_indexeddb::BackendError::DataError(_)
            | elidex_indexeddb::BackendError::InvalidStateError(_)),
        ) => {
            return Err(value::backend_error_as_throw(ctx, &e));
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(cursor_id),
        Some(txn),
        outcome,
        None,
    )))
}

/// `cursor.delete()` (§4.9) → `IDBRequest` (result `undefined`).
pub(crate) fn native_cursor_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let cursor_id = require_cursor_this(ctx, this, "delete")?;
    let txn = require_writable_cursor(ctx, cursor_id, "delete")?;
    let backend = ctx.vm.require_idb_backend()?;
    let res = {
        let st = ctx
            .vm
            .idb_cursor_states
            .get_mut(&cursor_id)
            .expect("cursor state present");
        elidex_indexeddb::cursor::delete_current(&backend, &mut st.backend)
    };
    let outcome = match res {
        Ok(()) => DeferredOutcome::Success(JsValue::Undefined),
        // A double-delete is a synchronous InvalidStateError.
        Err(e @ elidex_indexeddb::BackendError::InvalidStateError(_)) => {
            return Err(value::backend_error_as_throw(ctx, &e));
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(cursor_id),
        Some(txn),
        outcome,
        None,
    )))
}

/// WebIDL `[EnforceRange] unsigned long` (§3.2.4.x): `ToNumber`, then reject
/// non-finite / out-of-`[0, 2^32-1]` with `TypeError` (no silent wrap).
fn enforce_range_unsigned_long(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<u32, VmError> {
    let n = ctx.to_number(val)?;
    if !n.is_finite() {
        return Err(VmError::type_error(
            "IDBCursor.advance: count is not a finite number",
        ));
    }
    let truncated = n.trunc();
    if !(0.0..=4_294_967_295.0).contains(&truncated) {
        return Err(VmError::type_error(
            "IDBCursor.advance: count is outside the range of unsigned long",
        ));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = truncated as u32;
    Ok(result)
}
