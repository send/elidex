//! IDBIndex (W3C IndexedDB §4.6 / §6.3).
//!
//! An index is a secondary access path into an object store.  Its retrieval
//! operations (`get` / `getKey` / `getAll` / `getAllKeys` / `count`) and
//! cursor openers mirror the `IDBObjectStore` one-shot pattern (brand-check →
//! `require_active` → range/key marshal → synchronous backend call →
//! `request::async_execute`), differing only in routing through the backend
//! `index::*` / `cursor::open_index_cursor` algorithms.
//!
//! Handle identity (§4.5 `index()` NOTE — `store.index("x") ===
//! store.index("x")`, and `=== createIndex("x",…)`) is provided by the
//! per-store-instance cache `IdbObjectStoreState::index_handles`, threaded
//! through [`get_or_create_index_handle`].

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::value::{self, Query};
use super::{cursor, object_store, request, DeferredOutcome, IdbIndexState};

/// Allocate a bare `IDBIndex` wrapper + side-store state (no caching).  Used
/// directly by the `createIndex` async-abort path (DR-4): the index was rolled
/// back, so the returned handle must NOT enter the same-instance cache.
pub(super) fn alloc_index_handle(
    vm: &mut VmInner,
    store_id: ObjectId,
    db_name: &str,
    store_name: &str,
    index_name: &str,
) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbIndex,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_index_prototype,
        extensible: true,
    });
    vm.idb_index_states.insert(
        id,
        IdbIndexState {
            db_name: db_name.to_string(),
            store_name: store_name.to_string(),
            index_name: index_name.to_string(),
            object_store: store_id,
        },
    );
    id
}

/// Return the cached `IDBIndex` handle for `(store, index_name)`, creating +
/// caching one if absent (§4.5 same-instance identity).  The caller has
/// already established that the index exists.
pub(super) fn get_or_create_index_handle(
    vm: &mut VmInner,
    store_id: ObjectId,
    db_name: &str,
    store_name: &str,
    index_name: &str,
) -> ObjectId {
    if let Some(&existing) = vm
        .idb_object_store_states
        .get(&store_id)
        .and_then(|s| s.index_handles.get(index_name))
    {
        return existing;
    }
    let id = alloc_index_handle(vm, store_id, db_name, store_name, index_name);
    if let Some(s) = vm.idb_object_store_states.get_mut(&store_id) {
        s.index_handles.insert(index_name.to_string(), id);
    }
    id
}

/// Brand-check that `this` is an `IDBIndex`.
fn require_index_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbIndex) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBIndex.prototype.{member} called on non-IDBIndex"
    )))
}

/// `(db_name, store_name, index_name, transaction)` for an index handle — the
/// resolved context for a retrieval / cursor operation.
fn index_ctx(
    ctx: &NativeContext<'_>,
    index_id: ObjectId,
) -> Result<(String, String, String, ObjectId), VmError> {
    let (db, store, index, store_id) = {
        let st = ctx
            .vm
            .idb_index_states
            .get(&index_id)
            .ok_or_else(|| VmError::type_error("IDBIndex state missing"))?;
        (
            st.db_name.clone(),
            st.store_name.clone(),
            st.index_name.clone(),
            st.object_store,
        )
    };
    let txn = ctx
        .vm
        .idb_object_store_states
        .get(&store_id)
        .and_then(|s| s.transaction)
        .ok_or_else(|| VmError::type_error("IDBIndex object store has no transaction"))?;
    Ok((db, store, index, txn))
}

/// Resolve `(db, store, index, txn)` for a retrieval / cursor operation and run
/// its shared §4.6 guards: the transaction must be active
/// (`TransactionInactiveError`), and the index (and its store) must not have
/// been deleted (`InvalidStateError`) — else a backend op would hit a dropped
/// table and surface a raw `UnknownError`.
fn op_ctx(
    ctx: &mut NativeContext<'_>,
    index_id: ObjectId,
    method: &str,
) -> Result<(String, String, String, ObjectId), VmError> {
    let (db, store, index, txn) = index_ctx(ctx, index_id)?;
    object_store::require_active(ctx, txn, method)?;
    let backend = ctx.vm.require_idb_backend()?;
    // A `NotFoundError` from either probe is the store/index-deleted case
    // (→ InvalidStateError); any other backend error is a real failure that
    // must surface as itself rather than be masked as a deletion.
    let deleted_msg = format!("IDBIndex.{method}: the index or its object store has been deleted");
    if let Err(e) = backend.get_store_meta(&db, &store) {
        return Err(value::deleted_or_throw(ctx, &e, &deleted_msg));
    }
    if let Err(e) = elidex_indexeddb::index::get_index_meta(&backend, &db, &store, &index) {
        return Err(value::deleted_or_throw(ctx, &e, &deleted_msg));
    }
    Ok((db, store, index, txn))
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.6).  Metadata read on demand from the backend.
// ---------------------------------------------------------------------------

/// `index.name` (§4.6) — the stored handle name (survives deletion).
pub(crate) fn native_index_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_index_this(ctx, this, "name")?;
    let name = ctx
        .vm
        .idb_index_states
        .get(&id)
        .map(|s| s.index_name.clone())
        .unwrap_or_default();
    Ok(JsValue::String(ctx.vm.strings.intern(&name)))
}

/// Read this index's backend metadata (`None` if it has been deleted).
fn index_meta(
    ctx: &mut NativeContext<'_>,
    index_id: ObjectId,
) -> Option<elidex_indexeddb::IndexMeta> {
    let (db, store, index, _) = index_ctx(ctx, index_id).ok()?;
    let backend = ctx.vm.ensure_idb_backend()?;
    elidex_indexeddb::index::get_index_meta(&backend, &db, &store, &index).ok()
}

/// `index.keyPath` (§4.6) — the key path string (array key paths are tracked
/// by `#11-idb-compound-index-keypath`, so always a string here).
pub(crate) fn native_index_get_key_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_index_this(ctx, this, "keyPath")?;
    Ok(index_meta(ctx, id).map_or(JsValue::Null, |m| {
        JsValue::String(ctx.vm.strings.intern(&m.key_path))
    }))
}

/// `index.unique` (§4.6).
pub(crate) fn native_index_get_unique(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_index_this(ctx, this, "unique")?;
    Ok(JsValue::Boolean(
        index_meta(ctx, id).is_some_and(|m| m.unique),
    ))
}

/// `index.multiEntry` (§4.6).
pub(crate) fn native_index_get_multi_entry(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_index_this(ctx, this, "multiEntry")?;
    Ok(JsValue::Boolean(
        index_meta(ctx, id).is_some_and(|m| m.multi_entry),
    ))
}

/// `index.objectStore` → the owning `IDBObjectStore` (§4.6).
pub(crate) fn native_index_get_object_store(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_index_this(ctx, this, "objectStore")?;
    Ok(ctx
        .vm
        .idb_index_states
        .get(&id)
        .map_or(JsValue::Null, |s| JsValue::Object(s.object_store)))
}

// ---------------------------------------------------------------------------
// Retrieval operations (W3C IDB §4.6 / §6.3)
// ---------------------------------------------------------------------------

/// `index.get(query)` (§6.3): first record value matching the key / range.
pub(crate) fn native_index_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let index_id = require_index_this(ctx, this, "get")?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, "get")?;
    let arg = value::require_arg(args, 0, "IDBIndex", "get", 1)?;
    let query = value::js_to_query(ctx, arg)?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match query {
        Query::Key(k) => {
            match elidex_indexeddb::index::index_get(&backend, &db, &store, &index, &k) {
                Ok(Some(json)) => DeferredOutcome::Success(value::json_to_js(ctx.vm, &json)),
                Ok(None) => DeferredOutcome::Success(JsValue::Undefined),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
        Query::Range(r) => {
            match elidex_indexeddb::index::index_get_all(
                &backend,
                &db,
                &store,
                &index,
                Some(&r),
                Some(1),
            ) {
                Ok(rows) => DeferredOutcome::Success(
                    rows.first().map_or(JsValue::Undefined, |(_, json)| {
                        value::json_to_js(ctx.vm, json)
                    }),
                ),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
    };
    issue(ctx, index_id, txn, outcome)
}

/// `index.getKey(query)` (§6.3): primary key of the first matching record.
pub(crate) fn native_index_get_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let index_id = require_index_this(ctx, this, "getKey")?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, "getKey")?;
    let arg = value::require_arg(args, 0, "IDBIndex", "getKey", 1)?;
    let query = value::js_to_query(ctx, arg)?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match query {
        Query::Key(k) => {
            match elidex_indexeddb::index::index_get_key(&backend, &db, &store, &index, &k) {
                Ok(Some(pk)) => DeferredOutcome::Success(value::idb_key_to_js(ctx.vm, &pk)),
                Ok(None) => DeferredOutcome::Success(JsValue::Undefined),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
        Query::Range(r) => {
            match elidex_indexeddb::index::index_get_all_keys(
                &backend,
                &db,
                &store,
                &index,
                Some(&r),
                Some(1),
            ) {
                Ok(keys) => DeferredOutcome::Success(
                    keys.first()
                        .map_or(JsValue::Undefined, |k| value::idb_key_to_js(ctx.vm, k)),
                ),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
    };
    issue(ctx, index_id, txn, outcome)
}

/// `index.getAll(query?, count?)` (§6.3).
pub(crate) fn native_index_get_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let index_id = require_index_this(ctx, this, "getAll")?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, "getAll")?;
    let range = object_store::optional_range(ctx, args.first().copied())?;
    let count = object_store::optional_count(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::index::index_get_all(
        &backend,
        &db,
        &store,
        &index,
        range.as_ref(),
        count,
    ) {
        Ok(rows) => {
            let vals: Vec<JsValue> = rows
                .iter()
                .map(|(_, json)| value::json_to_js(ctx.vm, json))
                .collect();
            DeferredOutcome::Success(JsValue::Object(ctx.vm.create_array_object(vals)))
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    issue(ctx, index_id, txn, outcome)
}

/// `index.getAllKeys(query?, count?)` (§6.3).
pub(crate) fn native_index_get_all_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let index_id = require_index_this(ctx, this, "getAllKeys")?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, "getAllKeys")?;
    let range = object_store::optional_range(ctx, args.first().copied())?;
    let count = object_store::optional_count(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::index::index_get_all_keys(
        &backend,
        &db,
        &store,
        &index,
        range.as_ref(),
        count,
    ) {
        Ok(keys) => {
            let vals: Vec<JsValue> = keys
                .iter()
                .map(|k| value::idb_key_to_js(ctx.vm, k))
                .collect();
            DeferredOutcome::Success(JsValue::Object(ctx.vm.create_array_object(vals)))
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    issue(ctx, index_id, txn, outcome)
}

/// `index.count(query?)` (§6.3).
pub(crate) fn native_index_count(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let index_id = require_index_this(ctx, this, "count")?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, "count")?;
    let range = object_store::optional_range(ctx, args.first().copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome =
        match elidex_indexeddb::index::index_count(&backend, &db, &store, &index, range.as_ref()) {
            #[allow(clippy::cast_precision_loss)]
            Ok(n) => DeferredOutcome::Success(JsValue::Number(n as f64)),
            Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
        };
    issue(ctx, index_id, txn, outcome)
}

// ---------------------------------------------------------------------------
// Cursor openers (W3C IDB §4.6)
// ---------------------------------------------------------------------------

/// Shared `openCursor` / `openKeyCursor` over an index (§4.6).
fn open_cursor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    key_only: bool,
) -> Result<JsValue, VmError> {
    let method = if key_only {
        "openKeyCursor"
    } else {
        "openCursor"
    };
    let index_id = require_index_this(ctx, this, method)?;
    let (db, store, index, txn) = op_ctx(ctx, index_id, method)?;
    let range = object_store::optional_range(ctx, args.first().copied())?;
    let direction = cursor::parse_direction(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    match elidex_indexeddb::cursor::open_index_cursor(
        &backend, &db, &store, &index, range, direction, key_only,
    ) {
        Ok(state) => Ok(cursor::create_cursor(ctx, index_id, txn, state, key_only)),
        Err(e) => {
            let exc = value::backend_error_to_dom_exception(ctx.vm, &e);
            issue(ctx, index_id, txn, DeferredOutcome::Error(exc))
        }
    }
}

/// `index.openCursor(query?, direction?)` (§4.6).
pub(crate) fn native_index_open_cursor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    open_cursor(ctx, this, args, false)
}

/// `index.openKeyCursor(query?, direction?)` (§4.6).
pub(crate) fn native_index_open_key_cursor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    open_cursor(ctx, this, args, true)
}

/// Issue a one-shot index request sourced at `index_id` (§5.6).
fn issue(
    ctx: &mut NativeContext<'_>,
    index_id: ObjectId,
    txn: ObjectId,
    outcome: DeferredOutcome,
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(index_id),
        Some(txn),
        outcome,
        None,
    )))
}
