//! `IDBObjectStore` JS object builder.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use elidex_indexeddb::{IdbKey, IdbKeyRange};

use crate::bridge::HostBridge;

use super::factory::{idb_key_to_js_value, js_value_to_idb_key};
use super::request;

/// Shared captures for object store closures: `(bridge, db_name, store_name)`.
type StoreCaptures = (HostBridge, String, String);

/// Build an `IDBObjectStore` JS object.
pub(crate) fn build_object_store(
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
    store_name: &str,
    transaction: JsValue,
) -> JsObject {
    let (key_path, auto_increment) = bridge
        .with_idb(|backend| backend.get_store_meta(db_name, store_name).ok())
        .flatten()
        .unwrap_or((None, false));

    let index_names = bridge
        .with_idb(|backend| {
            backend
                .list_index_names(db_name, store_name)
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let idx_array = boa_engine::object::builtins::JsArray::new(ctx);
    for n in &index_names {
        let _ = idx_array.push(JsValue::from(js_string!(n.as_str())), ctx);
    }

    let kp_val = key_path
        .as_deref()
        .map_or(JsValue::null(), |kp| JsValue::from(js_string!(kp)));

    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("name"), js_string!(store_name), Attribute::all())
        .property(js_string!("keyPath"), kp_val, Attribute::all())
        .property(
            js_string!("autoIncrement"),
            JsValue::from(auto_increment),
            Attribute::all(),
        )
        .property(
            js_string!("indexNames"),
            JsValue::from(idx_array),
            Attribute::all(),
        )
        .property(js_string!("transaction"), transaction, Attribute::all())
        .property(
            js_string!("__elidex_idb_name__"),
            js_string!(db_name),
            Attribute::empty(),
        )
        .property(
            js_string!("__elidex_store_name__"),
            js_string!(store_name),
            Attribute::empty(),
        )
        .build();

    let caps: StoreCaptures = (bridge.clone(), db_name.to_owned(), store_name.to_owned());

    register_method(&obj, ctx, "put", &caps, op_put);
    register_method(&obj, ctx, "add", &caps, op_add);
    register_method(&obj, ctx, "get", &caps, op_get);
    register_method(&obj, ctx, "getKey", &caps, op_get_key);
    register_method(&obj, ctx, "getAll", &caps, op_get_all);
    register_method(&obj, ctx, "getAllKeys", &caps, op_get_all_keys);
    register_method(&obj, ctx, "delete", &caps, op_delete);
    register_method(&obj, ctx, "clear", &caps, op_clear);
    register_method(&obj, ctx, "count", &caps, op_count);
    register_method(&obj, ctx, "openCursor", &caps, op_open_cursor);
    register_method(&obj, ctx, "openKeyCursor", &caps, op_open_key_cursor);
    register_method(&obj, ctx, "getAllRecords", &caps, op_get_all_records);

    // createIndex / deleteIndex / index
    register_index_methods(&obj, ctx, bridge, db_name, store_name);

    obj
}

fn register_method(
    obj: &JsObject,
    ctx: &mut Context,
    name: &str,
    caps: &StoreCaptures,
    handler: fn(&[JsValue], &StoreCaptures, &mut Context) -> JsResult<JsValue>,
) {
    let c = caps.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        move |_, args, captures, ctx| handler(args, captures, ctx),
        c,
    );
    let _ = obj.set(
        js_string!(name),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

// --- CRUD operations ---

/// Function type for `ops::put` / `ops::add`.
type InsertFn = fn(
    &elidex_indexeddb::IdbBackend,
    &str,
    &str,
    Option<elidex_indexeddb::IdbKey>,
    &str,
) -> Result<elidex_indexeddb::IdbKey, elidex_indexeddb::BackendError>;

/// Insert a record with atomic index maintenance (SAVEPOINT for rollback on index failure).
fn insert_with_indexes(
    backend: &elidex_indexeddb::IdbBackend,
    db_name: &str,
    store_name: &str,
    key: Option<elidex_indexeddb::IdbKey>,
    value: &str,
    insert_fn: InsertFn,
) -> Result<elidex_indexeddb::IdbKey, elidex_indexeddb::BackendError> {
    backend
        .conn()
        .execute_batch("SAVEPOINT idb_insert")
        .map_err(elidex_indexeddb::BackendError::from)?;
    let inserted = match insert_fn(backend, db_name, store_name, key, value) {
        Ok(k) => k,
        Err(e) => {
            let _ = backend.conn().execute_batch("ROLLBACK TO idb_insert");
            let _ = backend.conn().execute_batch("RELEASE idb_insert");
            return Err(e);
        }
    };
    if let Err(e) = elidex_indexeddb::index::update_indexes_for_put(
        backend, db_name, store_name, &inserted, value,
    ) {
        let _ = backend.conn().execute_batch("ROLLBACK TO idb_insert");
        let _ = backend.conn().execute_batch("RELEASE idb_insert");
        return Err(e);
    }
    backend
        .conn()
        .execute_batch("RELEASE idb_insert")
        .map_err(elidex_indexeddb::BackendError::from)?;
    Ok(inserted)
}

fn op_put(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let value = serialize_value(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
    let key = if args.len() > 1 && !args[1].is_undefined() {
        Some(js_value_to_idb_key(&args[1], ctx)?)
    } else {
        None
    };

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        insert_with_indexes(
            backend,
            db_name,
            store_name,
            key,
            &value,
            elidex_indexeddb::ops::put,
        )
    });
    resolve_op_result(&req, result, ctx);
    Ok(JsValue::from(req))
}

fn op_add(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let value = serialize_value(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
    let key = if args.len() > 1 && !args[1].is_undefined() {
        Some(js_value_to_idb_key(&args[1], ctx)?)
    } else {
        None
    };

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        insert_with_indexes(
            backend,
            db_name,
            store_name,
            key,
            &value,
            elidex_indexeddb::ops::add,
        )
    });
    resolve_op_result(&req, result, ctx);
    Ok(JsValue::from(req))
}

fn op_get(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;

    let req = request::build_request(ctx);
    let result =
        bridge.with_idb(|backend| elidex_indexeddb::ops::get(backend, db_name, store_name, &key));

    match result {
        Some(Ok(Some(val))) => {
            let parsed = parse_json_to_js(&val, ctx);
            request::resolve_request(&req, parsed, ctx);
        }
        Some(Ok(None)) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_get_key(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;

    let req = request::build_request(ctx);
    let result = bridge
        .with_idb(|backend| elidex_indexeddb::ops::get_key(backend, db_name, store_name, &key));

    match result {
        Some(Ok(Some(k))) => request::resolve_request(&req, idb_key_to_js_value(&k, ctx), ctx),
        Some(Ok(None)) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_get_all(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let range = extract_range(args.first(), ctx)?;
    let count = extract_count(args.get(1), ctx)?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::ops::get_all(backend, db_name, store_name, range.as_ref(), count)
    });

    match result {
        Some(Ok(rows)) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for (_key, val) in &rows {
                let _ = arr.push(parse_json_to_js(val, ctx), ctx);
            }
            request::resolve_request(&req, JsValue::from(arr), ctx);
        }
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_get_all_keys(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let range = extract_range(args.first(), ctx)?;
    let count = extract_count(args.get(1), ctx)?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::ops::get_all_keys(backend, db_name, store_name, range.as_ref(), count)
    });

    match result {
        Some(Ok(keys)) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for k in &keys {
                let _ = arr.push(idb_key_to_js_value(k, ctx), ctx);
            }
            request::resolve_request(&req, JsValue::from(arr), ctx);
        }
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_delete(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;

    let req = request::build_request(ctx);

    // Try as key range first, then single key
    let target = if let Some(range) = extract_range(args.first(), ctx)? {
        elidex_indexeddb::DeleteTarget::Range(range)
    } else {
        let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
        elidex_indexeddb::DeleteTarget::Key(key)
    };

    let result = bridge.with_idb(|backend| {
        // Clean up index entries before deleting records
        match &target {
            elidex_indexeddb::DeleteTarget::Key(key) => {
                elidex_indexeddb::index::remove_indexes_for_delete(
                    backend, db_name, store_name, key,
                )?;
            }
            elidex_indexeddb::DeleteTarget::Range(range) => {
                elidex_indexeddb::index::remove_indexes_for_range(
                    backend, db_name, store_name, range,
                )?;
            }
        }
        elidex_indexeddb::ops::delete(backend, db_name, store_name, &target)
    });

    match result {
        Some(Ok(())) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

#[allow(clippy::unnecessary_wraps)] // consistent handler signature
fn op_clear(_args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::ops::clear(backend, db_name, store_name)?;
        elidex_indexeddb::index::clear_indexes(backend, db_name, store_name)?;
        Ok::<_, elidex_indexeddb::BackendError>(())
    });

    match result {
        Some(Ok(())) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_count(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let range = extract_range(args.first(), ctx)?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::ops::count(backend, db_name, store_name, range.as_ref())
    });

    match result {
        #[allow(clippy::cast_precision_loss)]
        Some(Ok(n)) => request::resolve_request(&req, JsValue::from(n as f64), ctx),
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

/// `getAllRecords()` — IDB 3.0 §4.5. Returns `[{key, primaryKey, value}]`.
fn op_get_all_records(
    args: &[JsValue],
    caps: &StoreCaptures,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let range = extract_range(args.first(), ctx)?;
    let count = extract_count(args.get(1), ctx)?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::ops::get_all(backend, db_name, store_name, range.as_ref(), count)
    });

    match result {
        Some(Ok(rows)) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for (key, val) in &rows {
                let key_js = idb_key_to_js_value(key, ctx);
                let pk_js = idb_key_to_js_value(key, ctx);
                let val_js = parse_json_to_js(val, ctx);
                let record = boa_engine::object::ObjectInitializer::new(ctx)
                    .property(
                        js_string!("key"),
                        key_js,
                        boa_engine::property::Attribute::all(),
                    )
                    .property(
                        js_string!("primaryKey"),
                        pk_js,
                        boa_engine::property::Attribute::all(),
                    )
                    .property(
                        js_string!("value"),
                        val_js,
                        boa_engine::property::Attribute::all(),
                    )
                    .build();
                let _ = arr.push(JsValue::from(record), ctx);
            }
            request::resolve_request(&req, JsValue::from(arr), ctx);
        }
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn op_open_cursor(args: &[JsValue], caps: &StoreCaptures, ctx: &mut Context) -> JsResult<JsValue> {
    open_cursor_impl(args, caps, ctx, false)
}

fn op_open_key_cursor(
    args: &[JsValue],
    caps: &StoreCaptures,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    open_cursor_impl(args, caps, ctx, true)
}

fn open_cursor_impl(
    args: &[JsValue],
    caps: &StoreCaptures,
    ctx: &mut Context,
    key_only: bool,
) -> JsResult<JsValue> {
    let (bridge, db_name, store_name) = caps;
    let range = extract_range(args.first(), ctx)?;
    let direction = args
        .get(1)
        .and_then(JsValue::as_string)
        .map_or_else(|| "next".to_owned(), |s| s.to_std_string_escaped());
    let dir = elidex_indexeddb::cursor::CursorDirection::parse(&direction).ok_or_else(|| {
        JsNativeError::typ().with_message(format!("TypeError: invalid direction '{direction}'"))
    })?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::cursor::open_store_cursor(
            backend, db_name, store_name, range, dir, key_only,
        )
    });

    match result {
        Some(Ok(cursor_state)) => {
            if cursor_state.current().is_none() {
                // Empty result set
                request::resolve_request(&req, JsValue::null(), ctx);
            } else {
                let cursor_id = bridge.store_idb_cursor(cursor_state);
                let cursor_obj = super::cursor::build_cursor_object(
                    ctx, bridge, cursor_id, key_only, &direction, &req,
                );
                request::resolve_request(&req, JsValue::from(cursor_obj), ctx);
            }
        }
        Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

// --- Index methods ---

#[allow(clippy::too_many_lines)]
fn register_index_methods(
    obj: &JsObject,
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
    store_name: &str,
) {
    let caps: StoreCaptures = (bridge.clone(), db_name.to_owned(), store_name.to_owned());

    // createIndex(name, keyPath, options?)
    let c = caps.clone();
    let create_idx = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, store_name), ctx| {
            let idx_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("createIndex requires a name"))?
                .to_std_string_escaped();
            let key_path = args
                .get(1)
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("createIndex requires a keyPath"))?
                .to_std_string_escaped();

            let mut unique = false;
            let mut multi_entry = false;
            if let Some(opts) = args.get(2).and_then(JsValue::as_object) {
                if let Ok(u) = opts.get(js_string!("unique"), ctx) {
                    unique = u.to_boolean();
                }
                if let Ok(m) = opts.get(js_string!("multiEntry"), ctx) {
                    multi_entry = m.to_boolean();
                }
            }

            let result = bridge.with_idb(|backend| {
                elidex_indexeddb::index::create_index(
                    backend,
                    db_name,
                    store_name,
                    &idx_name,
                    &key_path,
                    unique,
                    multi_entry,
                )
            });

            match result {
                Some(Ok(())) => {
                    let idx_obj = super::index::build_index_object(
                        ctx, bridge, db_name, store_name, &idx_name,
                    );
                    Ok(JsValue::from(idx_obj))
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        c,
    );
    let _ = obj.set(
        js_string!("createIndex"),
        JsValue::from(create_idx.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // deleteIndex(name)
    let c = caps.clone();
    let delete_idx = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, store_name), _ctx| {
            let idx_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("deleteIndex requires a name"))?
                .to_std_string_escaped();

            let result = bridge.with_idb(|backend| {
                elidex_indexeddb::index::delete_index(backend, db_name, store_name, &idx_name)
            });
            match result {
                Some(Ok(())) => Ok(JsValue::undefined()),
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        c,
    );
    let _ = obj.set(
        js_string!("deleteIndex"),
        JsValue::from(delete_idx.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // index(name)
    let c = caps;
    let index_fn = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, store_name), ctx| {
            let idx_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("index() requires a name"))?
                .to_std_string_escaped();

            // Verify exists
            let meta = bridge.with_idb(|backend| {
                elidex_indexeddb::index::get_index_meta(backend, db_name, store_name, &idx_name)
            });
            match meta {
                Some(Ok(_)) => {
                    let idx_obj = super::index::build_index_object(
                        ctx, bridge, db_name, store_name, &idx_name,
                    );
                    Ok(JsValue::from(idx_obj))
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        c,
    );
    let _ = obj.set(
        js_string!("index"),
        JsValue::from(index_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

// --- Helpers ---

pub(super) fn serialize_value(val: &JsValue, ctx: &mut Context) -> JsResult<String> {
    // Use JSON.stringify via boa
    let json_obj = ctx.global_object().get(js_string!("JSON"), ctx)?;
    if let Some(json) = json_obj.as_object() {
        let stringify = json.get(js_string!("stringify"), ctx)?;
        if let Some(func) = stringify.as_callable() {
            let result = func
                .call(&json_obj, std::slice::from_ref(val), ctx)
                .map_err(|_| {
                    JsNativeError::typ().with_message("DataCloneError: value is not serializable")
                })?;
            if let Some(s) = result.as_string() {
                return Ok(s.to_std_string_escaped());
            }
            // JSON.stringify returns undefined for non-serializable values
            // (Symbol, Function, undefined). Reject per W3C spec (DataCloneError).
            if result.is_undefined() {
                return Err(JsNativeError::typ()
                    .with_message("DataCloneError: value is not serializable")
                    .into());
            }
        }
    }
    // Fallback for primitives (numbers, booleans, null)
    Ok(val.to_string(ctx)?.to_std_string_escaped())
}

pub(super) fn parse_json_to_js(json: &str, ctx: &mut Context) -> JsValue {
    let Ok(json_val) = ctx.global_object().get(js_string!("JSON"), ctx) else {
        return JsValue::from(js_string!(json));
    };
    let Some(json_ns) = json_val.as_object() else {
        return JsValue::from(js_string!(json));
    };
    let Ok(parse_fn) = json_ns.get(js_string!("parse"), ctx) else {
        return JsValue::from(js_string!(json));
    };
    let Some(func) = parse_fn.as_callable() else {
        return JsValue::from(js_string!(json));
    };
    func.call(&json_val, &[JsValue::from(js_string!(json))], ctx)
        .unwrap_or_else(|_| JsValue::from(js_string!(json)))
}

pub(super) fn extract_range(
    val: Option<&JsValue>,
    ctx: &mut Context,
) -> JsResult<Option<IdbKeyRange>> {
    let Some(v) = val else { return Ok(None) };
    if v.is_undefined() || v.is_null() {
        return Ok(None);
    }

    // Check if it's an IDBKeyRange object (has __elidex_idb_range__ marker)
    if let Some(ref obj) = v.as_object() {
        if let Ok(marker) = obj.get(js_string!("__elidex_idb_range__"), ctx) {
            if marker.to_boolean() {
                return extract_range_from_obj(obj, ctx);
            }
        }
    }

    // Otherwise treat as a key → IdbKeyRange::only
    if let Ok(key) = js_value_to_idb_key(v, ctx) {
        return Ok(Some(IdbKeyRange::only(key)));
    }

    Ok(None)
}

fn extract_range_from_obj(obj: &JsObject, ctx: &mut Context) -> JsResult<Option<IdbKeyRange>> {
    let lower = obj.get(js_string!("lower"), ctx).ok();
    let upper = obj.get(js_string!("upper"), ctx).ok();
    let lower_open = obj
        .get(js_string!("lowerOpen"), ctx)
        .map(|v| v.to_boolean())
        .unwrap_or(false);
    let upper_open = obj
        .get(js_string!("upperOpen"), ctx)
        .map(|v| v.to_boolean())
        .unwrap_or(false);

    let lower_key = lower
        .as_ref()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .map(|v| js_value_to_idb_key(v, ctx))
        .transpose()?;
    let upper_key = upper
        .as_ref()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .map(|v| js_value_to_idb_key(v, ctx))
        .transpose()?;

    Ok(Some(IdbKeyRange {
        lower: lower_key,
        upper: upper_key,
        lower_open,
        upper_open,
    }))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn extract_count(val: Option<&JsValue>, ctx: &mut Context) -> JsResult<Option<u32>> {
    let Some(v) = val else { return Ok(None) };
    if v.is_undefined() {
        return Ok(None);
    }
    let n = v.to_number(ctx)?;
    if n.is_nan() || n < 0.0 {
        return Ok(None);
    }
    // W3C §4.5: count=0 means "no limit" (treat as None)
    if n == 0.0 {
        return Ok(None);
    }
    Ok(Some(n as u32))
}

fn resolve_op_result(
    req: &JsObject,
    result: Option<Result<IdbKey, elidex_indexeddb::BackendError>>,
    ctx: &mut Context,
) {
    match result {
        Some(Ok(key)) => request::resolve_request(req, idb_key_to_js_value(&key, ctx), ctx),
        Some(Err(e)) => request::reject_request(req, &e.to_string(), ctx),
        None => request::reject_request(req, "IndexedDB backend not available", ctx),
    }
}
