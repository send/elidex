//! IDBObjectStore record operations (W3C IndexedDB §4.5 / §6.1–§6.6).
//!
//! Each method brand-checks the receiver, validates the owning
//! transaction's state (§2.7.1 `TransactionInactiveError` /
//! `ReadOnlyError`), marshals the key / value, runs the synchronous
//! backend operation, and hands the outcome to `request::async_execute`
//! (§5.6) — the result is delivered via a database task, never inline.
//!
//! `createIndex` / `deleteIndex` / `index` + `openCursor` / `openKeyCursor`
//! ship with `IDBIndex` / `IDBCursor` in D-20b.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::value::{self, Query};
use super::{request, DeferredOutcome, IdbTxnState};

/// Allocate an `IDBObjectStore` wrapper bound to `txn` (§4.5).
pub(crate) fn create_object_store_wrapper(
    vm: &mut VmInner,
    db_name: &str,
    store_name: &str,
    txn: ObjectId,
) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbObjectStore,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_object_store_prototype,
        extensible: true,
    });
    vm.idb_object_store_states.insert(
        id,
        super::IdbObjectStoreState {
            db_name: db_name.to_string(),
            store_name: store_name.to_string(),
            transaction: Some(txn),
        },
    );
    id
}

/// Brand-check that `this` is an `IDBObjectStore`.
fn require_store_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbObjectStore) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBObjectStore.prototype.{method} called on non-IDBObjectStore"
    )))
}

/// `(db_name, store_name, transaction_id)` for a store wrapper.
fn store_ctx(
    ctx: &NativeContext<'_>,
    store_id: ObjectId,
) -> Result<(String, String, ObjectId), VmError> {
    let st = ctx
        .vm
        .idb_object_store_states
        .get(&store_id)
        .ok_or_else(|| VmError::type_error("IDBObjectStore state missing"))?;
    let txn = st
        .transaction
        .ok_or_else(|| VmError::type_error("IDBObjectStore has no transaction"))?;
    Ok((st.db_name.clone(), st.store_name.clone(), txn))
}

/// §2.7.1: requests may only be issued while the transaction is `Active`.
fn require_active(ctx: &mut NativeContext<'_>, txn: ObjectId, method: &str) -> Result<(), VmError> {
    let active = matches!(
        ctx.vm.idb_transaction_states.get(&txn).map(|s| s.state),
        Some(IdbTxnState::Active)
    );
    if active {
        Ok(())
    } else {
        Err(value::dom_exc(
            ctx,
            "TransactionInactiveError",
            format!("IDBObjectStore.{method}: the transaction is not active"),
        ))
    }
}

/// §4.5: write operations require a `readwrite` / `versionchange` mode.
fn require_writable(
    ctx: &mut NativeContext<'_>,
    txn: ObjectId,
    method: &str,
) -> Result<(), VmError> {
    let writable = matches!(
        ctx.vm.idb_transaction_states.get(&txn).map(|s| s.mode),
        Some(
            elidex_indexeddb::IdbTransactionMode::ReadWrite
                | elidex_indexeddb::IdbTransactionMode::VersionChange
        )
    );
    if writable {
        Ok(())
    } else {
        Err(value::dom_exc(
            ctx,
            "ReadOnlyError",
            format!("IDBObjectStore.{method}: the transaction is read-only"),
        ))
    }
}

/// §5.11 clone with the transaction-inactive guard: the transaction is set
/// inactive for the duration of the structured clone so a getter side
/// effect cannot issue a request against it, then restored.
fn clone_value_guarded(
    ctx: &mut NativeContext<'_>,
    txn: ObjectId,
    value: JsValue,
) -> Result<String, VmError> {
    let prev = ctx.vm.idb_transaction_states.get(&txn).map(|s| s.state);
    if let Some(s) = ctx.vm.idb_transaction_states.get_mut(&txn) {
        s.state = IdbTxnState::Inactive;
    }
    let result = value::value_to_json(ctx, value);
    // Restore the pre-clone state ONLY if the clone left the txn inactive
    // (i.e. untouched).  `value_to_json` runs user JS (toJSON / property
    // getters via JSON.stringify); a getter that called `txn.abort()` (legal
    // while inactive) already transitioned the txn to Finished and rolled
    // back its backend handle — blindly restoring `prev` would resurrect a
    // dead transaction.  The caller re-checks `require_active` after this.
    if let Some(p) = prev {
        if let Some(s) = ctx.vm.idb_transaction_states.get_mut(&txn) {
            if s.state == IdbTxnState::Inactive {
                s.state = p;
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.5).  Metadata is read on demand from the
// backend so it never drifts from the schema (plan §2.3).
// ---------------------------------------------------------------------------

/// `(db_name, store_name)` for a store wrapper accessor.
fn store_meta_ctx(
    ctx: &NativeContext<'_>,
    store_id: ObjectId,
) -> Result<(String, String), VmError> {
    let st = ctx
        .vm
        .idb_object_store_states
        .get(&store_id)
        .ok_or_else(|| VmError::type_error("IDBObjectStore state missing"))?;
    Ok((st.db_name.clone(), st.store_name.clone()))
}

/// `store.name` (§4.5).
pub(crate) fn native_os_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_store_this(ctx, this, "name")?;
    let (_, store) = store_meta_ctx(ctx, id)?;
    Ok(JsValue::String(ctx.vm.strings.intern(&store)))
}

/// `store.keyPath` (§4.5) — the key path string, or `null` for an
/// out-of-line-key store.  (Array key paths are not yet supported, plan §1.)
pub(crate) fn native_os_get_key_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_store_this(ctx, this, "keyPath")?;
    let (db, store) = store_meta_ctx(ctx, id)?;
    let backend = ctx.vm.require_idb_backend()?;
    let (key_path, _) = backend
        .get_store_meta(&db, &store)
        .map_err(|e| value::backend_error_as_throw(ctx, &e))?;
    Ok(key_path.map_or(JsValue::Null, |kp| {
        JsValue::String(ctx.vm.strings.intern(&kp))
    }))
}

/// `store.autoIncrement` (§4.5).
pub(crate) fn native_os_get_auto_increment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_store_this(ctx, this, "autoIncrement")?;
    let (db, store) = store_meta_ctx(ctx, id)?;
    let backend = ctx.vm.require_idb_backend()?;
    let (_, auto_increment) = backend
        .get_store_meta(&db, &store)
        .map_err(|e| value::backend_error_as_throw(ctx, &e))?;
    Ok(JsValue::Boolean(auto_increment))
}

/// `store.indexNames` (§4.5).  Sorted `Array<string>` (no `DOMStringList`).
pub(crate) fn native_os_get_index_names(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_store_this(ctx, this, "indexNames")?;
    let (db, store) = store_meta_ctx(ctx, id)?;
    let backend = ctx.vm.require_idb_backend()?;
    let mut names = backend.list_index_names(&db, &store).unwrap_or_default();
    names.sort();
    let elems: Vec<JsValue> = names
        .iter()
        .map(|n| JsValue::String(ctx.vm.strings.intern(n)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elems)))
}

/// `store.transaction` → the owning `IDBTransaction` (§4.5).
pub(crate) fn native_os_get_transaction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_store_this(ctx, this, "transaction")?;
    let txn = ctx
        .vm
        .idb_object_store_states
        .get(&id)
        .and_then(|s| s.transaction);
    Ok(txn.map_or(JsValue::Null, JsValue::Object))
}

/// Shared `add` / `put` (§6.1).  `is_add` rejects duplicate keys with
/// `ConstraintError`; `put` overwrites.
fn add_or_put(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    is_add: bool,
) -> Result<JsValue, VmError> {
    let method = if is_add { "add" } else { "put" };
    let store_id = require_store_this(ctx, this, method)?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, method)?;
    require_writable(ctx, txn, method)?;
    // WebIDL: the value argument is required (the key, arg 1, is optional) —
    // a missing value is a TypeError, not an `undefined` value to clone.
    value::require_arg(args, 0, "IDBObjectStore", method, 1)?;
    // §4.5 step 8: convert the explicit key BEFORE cloning the value — an
    // invalid key is a DataError that must precede the clone (whose failure
    // is DataCloneError), and key conversion must run before the clone's
    // user-getter side effects.
    let key = match args.get(1).copied() {
        None | Some(JsValue::Undefined) => None,
        Some(k) => Some(value::js_to_idb_key(ctx, k)?),
    };
    // §10.2.4 steps 5-6: the deterministic key / key-path `DataError`s that do
    // NOT need the value are thrown BEFORE the clone, so a rejected add()/put()
    // never runs the value's `toJSON` / getters (the clone's observable side
    // effects).  The value-dependent in-line key-path extraction failure stays
    // in the backend op (it inherently needs the cloned value).
    let backend = ctx.vm.require_idb_backend()?;
    let (key_path, auto_increment) = backend
        .get_store_meta(&db, &store)
        .map_err(|e| value::backend_error_as_throw(ctx, &e))?;
    if key.is_some() && key_path.is_some() {
        return Err(value::dom_exc(
            ctx,
            "DataError",
            "an explicit key cannot be supplied to an object store using in-line keys",
        ));
    }
    if key.is_none() && key_path.is_none() && !auto_increment {
        return Err(value::dom_exc(
            ctx,
            "DataError",
            "a key is required for an object store using out-of-line keys without a key generator",
        ));
    }
    // §4.5 step 10: clone the value (txn inactive during the clone, §5.11).
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let json = clone_value_guarded(ctx, txn, value)?;
    // The clone may have run user JS that aborted / finished the txn; the
    // transaction must still be active to take the write.
    require_active(ctx, txn, method)?;
    let result = if is_add {
        elidex_indexeddb::ops::add(&backend, &db, &store, key, &json)
    } else {
        elidex_indexeddb::ops::put(&backend, &db, &store, key, &json)
    };
    let outcome = match result {
        Ok(k) => DeferredOutcome::Success(value::idb_key_to_js(ctx.vm, &k)),
        // §10.2.4: a deterministic key-validation failure (`DataError` — e.g.
        // an explicit key on an inline-key store, or a value the key path
        // cannot extract a key from) is thrown SYNCHRONOUSLY, before the
        // request is queued.  Only operational failures (`ConstraintError`
        // duplicate key, backend errors) are delivered through the request.
        Err(e @ elidex_indexeddb::BackendError::DataError(_)) => {
            return Err(value::backend_error_as_throw(ctx, &e));
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

pub(crate) fn native_os_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    add_or_put(ctx, this, args, true)
}

pub(crate) fn native_os_put(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    add_or_put(ctx, this, args, false)
}

/// `get(query)` (§6.2): single-key → first matching record; range → first
/// record in range.
pub(crate) fn native_os_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "get")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "get")?;
    let arg = value::require_arg(args, 0, "IDBObjectStore", "get", 1)?;
    let query = value::js_to_query(ctx, arg)?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match query {
        Query::Key(k) => match elidex_indexeddb::ops::get(&backend, &db, &store, &k) {
            Ok(Some(json)) => DeferredOutcome::Success(value::json_to_js(ctx.vm, &json)),
            Ok(None) => DeferredOutcome::Success(JsValue::Undefined),
            Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
        },
        Query::Range(r) => {
            match elidex_indexeddb::ops::get_all(&backend, &db, &store, Some(&r), Some(1)) {
                Ok(rows) => DeferredOutcome::Success(
                    rows.first().map_or(JsValue::Undefined, |(_, json)| {
                        value::json_to_js(ctx.vm, json)
                    }),
                ),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

/// `getKey(query)` (§6.2): the primary key of the first matching record.
pub(crate) fn native_os_get_key(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "getKey")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "getKey")?;
    let arg = value::require_arg(args, 0, "IDBObjectStore", "getKey", 1)?;
    let query = value::js_to_query(ctx, arg)?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match query {
        Query::Key(k) => match elidex_indexeddb::ops::get_key(&backend, &db, &store, &k) {
            Ok(Some(found)) => DeferredOutcome::Success(value::idb_key_to_js(ctx.vm, &found)),
            Ok(None) => DeferredOutcome::Success(JsValue::Undefined),
            Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
        },
        Query::Range(r) => {
            match elidex_indexeddb::ops::get_all_keys(&backend, &db, &store, Some(&r), Some(1)) {
                Ok(keys) => DeferredOutcome::Success(
                    keys.first()
                        .map_or(JsValue::Undefined, |k| value::idb_key_to_js(ctx.vm, k)),
                ),
                Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
            }
        }
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

/// Optional range argument for `getAll` / `getAllKeys` / `count`.
fn optional_range(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<Option<elidex_indexeddb::IdbKeyRange>, VmError> {
    match arg {
        // §4.5: the `query` arg is optional — OMITTED or an explicit `undefined`
        // means "no query" (all records).  A supplied `null` is a value, not an
        // omission: it goes through key conversion and fails with `DataError`
        // (so `store.getAll(null)` throws, not returns everything).
        None | Some(JsValue::Undefined) => Ok(None),
        Some(v) => match value::js_to_query(ctx, v)? {
            Query::Range(r) => Ok(Some(r)),
            Query::Key(k) => Ok(Some(elidex_indexeddb::IdbKeyRange::only(k))),
        },
    }
}

/// Optional `count` argument for `getAll` / `getAllKeys` (§4.5).  The WebIDL
/// type is `unsigned long` (ECMAScript ToUint32 — so a negative argument wraps
/// rather than meaning "none"), and §6.2 step 1 maps a count of `0` (or an
/// absent argument) to infinity, i.e. "all records" → no backend `LIMIT`.
fn optional_count(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<Option<u32>, VmError> {
    match arg {
        None | Some(JsValue::Undefined) => Ok(None),
        Some(v) => {
            let n = super::super::super::coerce::to_uint32(ctx.vm, v)?;
            Ok(if n == 0 { None } else { Some(n) })
        }
    }
}

pub(crate) fn native_os_get_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "getAll")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "getAll")?;
    let range = optional_range(ctx, args.first().copied())?;
    let count = optional_count(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::ops::get_all(&backend, &db, &store, range.as_ref(), count)
    {
        Ok(rows) => {
            let vals: Vec<JsValue> = rows
                .iter()
                .map(|(_, json)| value::json_to_js(ctx.vm, json))
                .collect();
            DeferredOutcome::Success(JsValue::Object(ctx.vm.create_array_object(vals)))
        }
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

pub(crate) fn native_os_get_all_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "getAllKeys")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "getAllKeys")?;
    let range = optional_range(ctx, args.first().copied())?;
    let count = optional_count(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome =
        match elidex_indexeddb::ops::get_all_keys(&backend, &db, &store, range.as_ref(), count) {
            Ok(keys) => {
                let vals: Vec<JsValue> = keys
                    .iter()
                    .map(|k| value::idb_key_to_js(ctx.vm, k))
                    .collect();
                DeferredOutcome::Success(JsValue::Object(ctx.vm.create_array_object(vals)))
            }
            Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
        };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

/// `delete(query)` (§6.4): key or range.
pub(crate) fn native_os_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "delete")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "delete")?;
    require_writable(ctx, txn, "delete")?;
    let arg = value::require_arg(args, 0, "IDBObjectStore", "delete", 1)?;
    let target = match value::js_to_query(ctx, arg)? {
        Query::Key(k) => elidex_indexeddb::DeleteTarget::Key(k),
        Query::Range(r) => elidex_indexeddb::DeleteTarget::Range(r),
    };
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::ops::delete(&backend, &db, &store, &target) {
        Ok(()) => DeferredOutcome::Success(JsValue::Undefined),
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

/// `clear()` (§6.6).
pub(crate) fn native_os_clear(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "clear")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "clear")?;
    require_writable(ctx, txn, "clear")?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::ops::clear(&backend, &db, &store) {
        Ok(()) => DeferredOutcome::Success(JsValue::Undefined),
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}

/// `count(query?)` (§6.5).
pub(crate) fn native_os_count(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let store_id = require_store_this(ctx, this, "count")?;
    let (db, store, txn) = store_ctx(ctx, store_id)?;
    require_active(ctx, txn, "count")?;
    let range = optional_range(ctx, args.first().copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let outcome = match elidex_indexeddb::ops::count(&backend, &db, &store, range.as_ref()) {
        #[allow(clippy::cast_precision_loss)]
        Ok(n) => DeferredOutcome::Success(JsValue::Number(n as f64)),
        Err(e) => DeferredOutcome::Error(value::backend_error_to_dom_exception(ctx.vm, &e)),
    };
    Ok(JsValue::Object(request::async_execute(
        ctx.vm,
        Some(store_id),
        Some(txn),
        outcome,
    )))
}
