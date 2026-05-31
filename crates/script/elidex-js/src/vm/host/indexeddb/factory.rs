//! IDBFactory — the `indexedDB` global (W3C IndexedDB §4.3) + the
//! database open / upgrade orchestration (§5.1 / §5.7).
//!
//! `open` / `deleteDatabase` return an `IDBOpenDBRequest` immediately and
//! deliver the result via a database task (§5.6); `databases` returns a
//! Promise; `cmp` (Stage 4, needs full key marshalling) compares two keys.
//! All record/key algorithm lives in the `elidex-indexeddb` backend; this
//! module marshals arguments + drives the request / upgrade lifecycle.

#![cfg(feature = "engine")]

use super::super::super::natives_promise::{create_promise, settle_promise};
use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::VmInner;
use super::super::pending_tasks::PendingTask;
use super::{
    database, fire_version_change_event, request, value, DeferredOutcome, IdbTransactionState,
};

/// Brand-check that `this` is the `IDBFactory` singleton.
fn require_idb_factory_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbFactory) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "IDBFactory.prototype.{method} called on non-IDBFactory"
    )))
}

/// Coerce an optional name argument to a Rust `String` (ECMAScript
/// ToString, §4.3 `open` / `deleteDatabase` first argument is a `DOMString`).
fn arg_name(ctx: &mut NativeContext<'_>, arg: Option<JsValue>) -> Result<String, VmError> {
    let sid = ctx.to_string_val(arg.unwrap_or(JsValue::Undefined))?;
    Ok(ctx.get_utf8(sid))
}

/// `indexedDB.open(name, version?)` → `IDBOpenDBRequest` (W3C IDB §4.3 /
/// §5.1).  Synchronous backend probe; result (or upgrade) delivered async.
#[allow(clippy::too_many_lines)] // the §5.1 Success/UpgradeNeeded/Error branch set is one coherent algorithm
pub(crate) fn native_idb_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_idb_factory_this(ctx, this, "open")?;
    let name = arg_name(ctx, args.first().copied())?;
    // §3.3.1: when supplied, version must be an integer ≥ 1 (else TypeError).
    let version = match args.get(1).copied() {
        None | Some(JsValue::Undefined) => None,
        Some(v) => {
            let n = ctx.to_number(v)?;
            if !n.is_finite() || n < 1.0 || n.fract() != 0.0 {
                return Err(VmError::type_error(
                    "IDBFactory.open: version must be an integer >= 1",
                ));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(n as u64)
        }
    };
    let backend = ctx.vm.require_idb_backend()?;
    let req = request::create_request(ctx.vm, None, None, true);
    match elidex_indexeddb::database::open_database(&backend, &name, version) {
        Ok(elidex_indexeddb::IdbOpenResult::Success(handle)) => {
            let db = database::create_database_wrapper(ctx.vm, handle.name(), handle.version());
            request::stage_and_queue(ctx.vm, req, DeferredOutcome::Success(JsValue::Object(db)));
        }
        Ok(elidex_indexeddb::IdbOpenResult::UpgradeNeeded {
            handle,
            old_version,
            new_version,
        }) => {
            let db = database::create_database_wrapper(ctx.vm, handle.name(), handle.version());
            match elidex_indexeddb::IdbTransaction::begin(
                backend.conn(),
                &name,
                Vec::new(),
                elidex_indexeddb::IdbTransactionMode::VersionChange,
            ) {
                Ok(vtxn) => {
                    let txn_id = create_upgrade_transaction(
                        ctx.vm,
                        db,
                        &name,
                        vtxn,
                        req,
                        handle,
                        old_version,
                    );
                    if let Some(rs) = ctx.vm.idb_request_states.get_mut(&req) {
                        rs.result = JsValue::Object(db);
                        rs.transaction = Some(txn_id);
                    }
                    ctx.vm.queue_task(PendingTask::IdbUpgrade {
                        request_id: req,
                        old_version,
                        new_version,
                    });
                }
                Err(e) => {
                    let exc = value::backend_error_to_dom_exception(ctx.vm, &e);
                    request::stage_and_queue(ctx.vm, req, DeferredOutcome::Error(exc));
                }
            }
        }
        Err(e) => {
            let exc = value::backend_error_to_dom_exception(ctx.vm, &e);
            request::stage_and_queue(ctx.vm, req, DeferredOutcome::Error(exc));
        }
    }
    Ok(JsValue::Object(req))
}

/// `indexedDB.cmp(first, second)` → `-1` / `0` / `1` (§4.3).  Compares two
/// keys by the W3C key ordering (delegated to the backend `IdbKey: Ord`);
/// an invalid key throws `DataError`.
pub(crate) fn native_idb_cmp(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_idb_factory_this(ctx, this, "cmp")?;
    let a = value::js_to_idb_key(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let b = value::js_to_idb_key(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let n = match a.cmp(&b) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    };
    Ok(JsValue::Number(n))
}

/// `indexedDB.deleteDatabase(name)` → `IDBOpenDBRequest` (§4.3 / §5.3).
/// Cross-connection `versionchange` / `blocked` fan-out is deferred to
/// `#11-idb-connection-queue` (single-VM scope = no other connections).
pub(crate) fn native_idb_delete_database(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_idb_factory_this(ctx, this, "deleteDatabase")?;
    let name = arg_name(ctx, args.first().copied())?;
    let backend = ctx.vm.require_idb_backend()?;
    let req = request::create_request(ctx.vm, None, None, true);
    match elidex_indexeddb::database::delete_database(&backend, &name) {
        Ok(_old_version) => {
            request::stage_and_queue(ctx.vm, req, DeferredOutcome::Success(JsValue::Undefined));
        }
        Err(e) => {
            let exc = value::backend_error_to_dom_exception(ctx.vm, &e);
            request::stage_and_queue(ctx.vm, req, DeferredOutcome::Error(exc));
        }
    }
    Ok(JsValue::Object(req))
}

/// `indexedDB.databases()` → `Promise<sequence<IDBDatabaseInfo>>` (§4.3).
/// Resolves synchronously (the SQLite listing is immediate) with an array
/// of `{ name, version }` records.
pub(crate) fn native_idb_databases(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_idb_factory_this(ctx, this, "databases")?;
    let promise = create_promise(ctx.vm);
    let Some(backend) = ctx.vm.ensure_idb_backend() else {
        let arr = ctx.vm.create_array_object(Vec::new());
        let _ = settle_promise(ctx.vm, promise, false, JsValue::Object(arr));
        return Ok(JsValue::Object(promise));
    };
    match elidex_indexeddb::database::list_databases(&backend) {
        Ok(list) => {
            let infos: Vec<JsValue> = list
                .iter()
                .map(|(name, version)| build_db_info(ctx.vm, name, *version))
                .collect();
            let arr = ctx.vm.create_array_object(infos);
            let _ = settle_promise(ctx.vm, promise, false, JsValue::Object(arr));
        }
        Err(e) => {
            let exc = value::backend_error_to_dom_exception(ctx.vm, &e);
            let _ = settle_promise(ctx.vm, promise, true, JsValue::Object(exc));
        }
    }
    Ok(JsValue::Object(promise))
}

/// Build an `IDBDatabaseInfo` (`{ name, version }`) plain object.
fn build_db_info(vm: &mut VmInner, name: &str, version: u64) -> JsValue {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    let name_sid = vm.strings.intern(name);
    let name_key = vm.well_known.name;
    vm.define_shaped_property(
        id,
        PropertyKey::String(name_key),
        PropertyValue::Data(JsValue::String(name_sid)),
        PropertyAttrs::DATA,
    );
    let version_key = vm.strings.intern("version");
    #[allow(clippy::cast_precision_loss)]
    vm.define_shaped_property(
        id,
        PropertyKey::String(version_key),
        PropertyValue::Data(JsValue::Number(version as f64)),
        PropertyAttrs::DATA,
    );
    JsValue::Object(id)
}

/// Allocate the upgrade `IDBTransaction` (mode `versionchange`, §5.7).
/// Active from creation; auto-commits when the `upgradeneeded` handler
/// turn ends + its request list is empty.
fn create_upgrade_transaction(
    vm: &mut VmInner,
    db: ObjectId,
    db_name: &str,
    vtxn: elidex_indexeddb::IdbTransaction,
    open_req: ObjectId,
    handle: elidex_indexeddb::IdbDatabaseHandle,
    old_version: u64,
) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbTransaction,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_transaction_prototype,
        extensible: true,
    });
    vm.idb_transaction_states.insert(
        id,
        IdbTransactionState {
            upgrade_request: Some(open_req),
            upgrade_handle: Some(handle),
            upgrade_old_version: old_version,
            ..IdbTransactionState::new_active(
                elidex_indexeddb::IdbTransactionMode::VersionChange,
                db,
                db_name,
                Vec::new(),
                vtxn,
            )
        },
    );
    // Back-ref so `createObjectStore` / `deleteObjectStore` on the
    // IDBDatabase can reach the active upgrade transaction (§5.7).
    if let Some(dbs) = vm.idb_database_states.get_mut(&db) {
        dbs.upgrade_txn = Some(id);
    }
    id
}

/// Drain step for [`PendingTask::IdbUpgrade`] (§5.7): fire `upgradeneeded`
/// at the open request, then run the upgrade transaction's post-dispatch
/// lifecycle (set inactive → commit if its request list is empty).  The
/// commit's deferred task finalizes the version bump + fires the open
/// request's `success`.
pub(crate) fn dispatch_idb_upgrade(
    vm: &mut VmInner,
    request_id: ObjectId,
    old_version: u64,
    new_version: u64,
) {
    let txn_id = vm
        .idb_request_states
        .get(&request_id)
        .and_then(|s| s.transaction);
    // §5.7 step 10.3: set the open request's done flag before firing
    // `upgradeneeded`, so `event.target.result` (the connection) is readable
    // inside the handler (else the §4.1 `result` getter throws
    // InvalidStateError while the request is still pending).
    if let Some(rs) = vm.idb_request_states.get_mut(&request_id) {
        rs.ready_state = super::IdbReadyState::Done;
    }
    let upgradeneeded_sid = vm.well_known.upgradeneeded;
    let onupgradeneeded_sid = vm.well_known.onupgradeneeded;
    let mut ctx = NativeContext::new_call(vm);
    let res = fire_version_change_event(
        &mut ctx,
        request_id,
        upgradeneeded_sid,
        onupgradeneeded_sid,
        old_version,
        Some(new_version),
    );
    if let Some(tid) = txn_id {
        request::run_post_dispatch(&mut ctx, tid, &res, None);
    }
}
