//! IDBDatabase connection (W3C IndexedDB §4.4): wrapper allocation +
//! `createObjectStore` / `deleteObjectStore` (§5.7 schema ops, valid only
//! inside an upgrade transaction) + `transaction` (§4.4) + `close`.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::{object_store, txn, value, IdbDatabaseState, IdbTxnState};

/// Allocate an `IDBDatabase` connection wrapper + its side-store state.
pub(crate) fn create_database_wrapper(vm: &mut VmInner, db_name: &str, version: u64) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbDatabase,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_database_prototype,
        extensible: true,
    });
    vm.idb_database_states.insert(
        id,
        IdbDatabaseState {
            db_name: db_name.to_string(),
            version,
            closed: false,
            ..Default::default()
        },
    );
    id
}

fn require_db_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbDatabase) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBDatabase.prototype.{method} called on non-IDBDatabase"
    )))
}

/// Resolve the database's active version-change transaction for a schema
/// op (§5.7): `InvalidStateError` if there is none, `TransactionInactiveError`
/// if it is not active.
fn upgrade_txn_for(
    ctx: &mut NativeContext<'_>,
    db_id: ObjectId,
    method: &str,
) -> Result<ObjectId, VmError> {
    let txn = ctx
        .vm
        .idb_database_states
        .get(&db_id)
        .and_then(|s| s.upgrade_txn);
    let Some(txn) = txn else {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            format!("IDBDatabase.{method}: not inside a version change transaction"),
        ));
    };
    let active = matches!(
        ctx.vm.idb_transaction_states.get(&txn).map(|s| s.state),
        Some(IdbTxnState::Active)
    );
    if active {
        Ok(txn)
    } else {
        Err(value::dom_exc(
            ctx,
            "TransactionInactiveError",
            format!("IDBDatabase.{method}: the version change transaction is not active"),
        ))
    }
}

fn db_name_of(ctx: &NativeContext<'_>, db_id: ObjectId) -> String {
    ctx.vm
        .idb_database_states
        .get(&db_id)
        .map(|s| s.db_name.clone())
        .unwrap_or_default()
}

/// Parse `createObjectStore` options: `{ keyPath?, autoIncrement? }`.
fn parse_create_store_options(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<(Option<String>, bool), VmError> {
    let Some(JsValue::Object(opts)) = arg else {
        return Ok((None, false));
    };
    // WebIDL `keyPath` is `(DOMString or sequence<DOMString>)?`: `null` /
    // `undefined` → out-of-line keys; anything else coerces to a `DOMString`
    // (so `{ keyPath: 1 }` is the path `"1"`).  An array is a valid *compound*
    // (sequence) key path — the backend does not support those yet, so reject
    // it rather than silently create an out-of-line store with different key
    // semantics (deferred: array/compound key paths).
    let kp_key = PropertyKey::String(ctx.vm.strings.intern("keyPath"));
    let key_path = match ctx.get_property_value(opts, kp_key)? {
        JsValue::Null | JsValue::Undefined => None,
        JsValue::Object(id) if matches!(ctx.get_object(id).kind, ObjectKind::Array { .. }) => {
            return Err(value::dom_exc(
                ctx,
                "NotSupportedError",
                "IDBDatabase.createObjectStore: array (compound) key paths are not supported",
            ));
        }
        other => {
            let sid = ctx.to_string_val(other)?;
            Some(ctx.get_utf8(sid))
        }
    };
    let ai_key = PropertyKey::String(ctx.vm.strings.intern("autoIncrement"));
    let ai_val = ctx.get_property_value(opts, ai_key)?;
    let auto_increment = ctx.to_boolean(ai_val);
    // §4.4 createObjectStore: an empty in-line key path with a key generator
    // is contradictory (nowhere to inject the generated key) → InvalidAccessError.
    if auto_increment && key_path.as_deref() == Some("") {
        return Err(value::dom_exc(
            ctx,
            "InvalidAccessError",
            "IDBDatabase.createObjectStore: autoIncrement with an empty key path",
        ));
    }
    Ok((key_path, auto_increment))
}

/// `db.createObjectStore(name, options?)` → `IDBObjectStore` (§5.7).
pub(crate) fn native_db_create_object_store(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let db_id = require_db_this(ctx, this, "createObjectStore")?;
    let txn = upgrade_txn_for(ctx, db_id, "createObjectStore")?;
    let db_name = db_name_of(ctx, db_id);
    let name_sid = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let name = ctx.get_utf8(name_sid);
    let (key_path, auto_increment) = parse_create_store_options(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    match backend.create_object_store(&db_name, &name, key_path.as_deref(), auto_increment) {
        Ok(()) => Ok(JsValue::Object(object_store::create_object_store_wrapper(
            ctx.vm, &db_name, &name, txn,
        ))),
        Err(e) => Err(value::backend_error_as_throw(ctx, &e)),
    }
}

/// `db.deleteObjectStore(name)` (§5.7).
pub(crate) fn native_db_delete_object_store(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let db_id = require_db_this(ctx, this, "deleteObjectStore")?;
    let _txn = upgrade_txn_for(ctx, db_id, "deleteObjectStore")?;
    let db_name = db_name_of(ctx, db_id);
    let name_sid = ctx.to_string_val(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let name = ctx.get_utf8(name_sid);
    let backend = ctx.vm.require_idb_backend()?;
    match backend.delete_object_store(&db_name, &name) {
        Ok(()) => Ok(JsValue::Undefined),
        Err(e) => Err(value::backend_error_as_throw(ctx, &e)),
    }
}

/// Parse the `storeNames` argument (a `DOMString` or sequence) into a
/// scope list.
fn parse_store_names(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<Vec<String>, VmError> {
    let v = arg.unwrap_or(JsValue::Undefined);
    if let JsValue::Object(id) = v {
        let elements = match &ctx.get_object(id).kind {
            ObjectKind::Array { elements } => Some(elements.clone()),
            _ => None,
        };
        if let Some(elems) = elements {
            let mut names = Vec::with_capacity(elems.len());
            for e in elems {
                let sid = ctx.to_string_val(e)?;
                names.push(ctx.get_utf8(sid));
            }
            return Ok(names);
        }
    }
    let sid = ctx.to_string_val(v)?;
    Ok(vec![ctx.get_utf8(sid)])
}

/// Parse the transaction `mode` argument (default `"readonly"`;
/// `"versionchange"` is not constructible via `transaction()` → TypeError).
fn parse_mode(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<elidex_indexeddb::IdbTransactionMode, VmError> {
    match arg {
        None | Some(JsValue::Undefined) => Ok(elidex_indexeddb::IdbTransactionMode::ReadOnly),
        Some(v) => {
            let sid = ctx.to_string_val(v)?;
            match ctx.get_utf8(sid).as_str() {
                "readonly" => Ok(elidex_indexeddb::IdbTransactionMode::ReadOnly),
                "readwrite" => Ok(elidex_indexeddb::IdbTransactionMode::ReadWrite),
                other => Err(VmError::type_error(format!(
                    "IDBDatabase.transaction: invalid mode '{other}'"
                ))),
            }
        }
    }
}

/// `db.transaction(storeNames, mode?)` → `IDBTransaction` (§4.4).
pub(crate) fn native_db_transaction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let db_id = require_db_this(ctx, this, "transaction")?;
    let (db_name, closed, upgrading) = {
        let s = ctx
            .vm
            .idb_database_states
            .get(&db_id)
            .ok_or_else(|| VmError::type_error("IDBDatabase state missing"))?;
        (s.db_name.clone(), s.closed, s.upgrade_txn.is_some())
    };
    if closed {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBDatabase.transaction: the connection is closed",
        ));
    }
    if upgrading {
        return Err(value::dom_exc(
            ctx,
            "InvalidStateError",
            "IDBDatabase.transaction: a version change transaction is running",
        ));
    }
    let names = parse_store_names(ctx, args.first().copied())?;
    if names.is_empty() {
        return Err(value::dom_exc(
            ctx,
            "InvalidAccessError",
            "IDBDatabase.transaction: the store names list is empty",
        ));
    }
    let mode = parse_mode(ctx, args.get(1).copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let existing = backend
        .list_store_names(&db_name)
        .map_err(|e| value::backend_error_as_throw(ctx, &e))?;
    for n in &names {
        if !existing.contains(n) {
            return Err(value::dom_exc(
                ctx,
                "NotFoundError",
                format!("IDBDatabase.transaction: no object store named '{n}'"),
            ));
        }
    }
    // §3.1.1: transaction creation eagerly opens the backend `BEGIN`.  On the
    // single shared SQLite connection this means a second overlapping
    // transaction created in the same task surfaces the backend's
    // nested-BEGIN rejection instead of being queued/serialized behind the
    // first (§5.4 "transaction scheduling": overlapping-scope write
    // transactions run in creation order, others may run concurrently).
    // Proper scheduling needs a connection pool / transaction queue and is
    // deferred to `#11-idb-connection-queue` (backend-gated); single-VM
    // single-task code paths — the common case — are unaffected.
    let backend_txn =
        elidex_indexeddb::IdbTransaction::begin(backend.conn(), &db_name, names.clone(), mode)
            .map_err(|e| value::backend_error_as_throw(ctx, &e))?;
    let txn_id = txn::create_transaction(ctx.vm, db_id, &db_name, names, mode, backend_txn);
    Ok(JsValue::Object(txn_id))
}

// ---------------------------------------------------------------------------
// Readonly accessors (W3C IDB §4.4)
// ---------------------------------------------------------------------------

/// `db.name` (§4.4).
pub(crate) fn native_db_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_db_this(ctx, this, "name")?;
    let name = db_name_of(ctx, id);
    Ok(JsValue::String(ctx.vm.strings.intern(&name)))
}

/// `db.version` (§4.4).
pub(crate) fn native_db_get_version(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_db_this(ctx, this, "version")?;
    let version = ctx.vm.idb_database_states.get(&id).map_or(0, |s| s.version);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(version as f64))
}

/// `db.objectStoreNames` (§4.4).  A DOMStringList in the spec; this VM has no
/// `DOMStringList`, so the bridge returns a sorted `Array<string>` (v1).
pub(crate) fn native_db_get_object_store_names(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_db_this(ctx, this, "objectStoreNames")?;
    let db_name = db_name_of(ctx, id);
    let backend = ctx.vm.require_idb_backend()?;
    let mut names = backend.list_store_names(&db_name).unwrap_or_default();
    names.sort();
    let elems: Vec<JsValue> = names
        .iter()
        .map(|n| JsValue::String(ctx.vm.strings.intern(n)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elems)))
}

/// `db.close()` (§4.4).
pub(crate) fn native_db_close(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let db_id = require_db_this(ctx, this, "close")?;
    if let Some(s) = ctx.vm.idb_database_states.get_mut(&db_id) {
        s.closed = true;
    }
    Ok(JsValue::Undefined)
}
