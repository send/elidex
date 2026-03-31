//! `IDBFactory` — `window.indexedDB` global.
//!
//! Methods: `open(name, version?)`, `deleteDatabase(name)`, `databases()`, `cmp(a, b)`.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::events;
use super::request;

/// Register `window.indexedDB` as an `IDBFactory` object.
pub fn register_idb_factory(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let factory = build_factory(ctx, &b);
    ctx.register_global_property(js_string!("indexedDB"), factory, Attribute::all())
        .expect("failed to register indexedDB");
}

fn build_factory(ctx: &mut Context, bridge: &HostBridge) -> JsValue {
    let b = bridge.clone();
    let open_fn = build_open_fn(ctx, &b);

    let b = bridge.clone();
    let delete_fn = build_delete_fn(ctx, &b);

    let b = bridge.clone();
    let databases_fn = build_databases_fn(ctx, &b);

    let cmp_fn = build_cmp_fn(ctx);

    let factory = ObjectInitializer::new(ctx)
        .function(open_fn, js_string!("open"), 2)
        .function(delete_fn, js_string!("deleteDatabase"), 1)
        .function(databases_fn, js_string!("databases"), 0)
        .function(cmp_fn, js_string!("cmp"), 2)
        .build();

    JsValue::from(factory)
}

/// `indexedDB.open(name, version?)`
///
/// Returns an `IDBOpenDBRequest`. Synchronously executes the open protocol
/// and fires `onsuccess` or `onupgradeneeded` handlers synchronously (inline
/// in the current call stack).
#[allow(clippy::too_many_lines)]
fn build_open_fn(_ctx: &mut Context, bridge: &HostBridge) -> NativeFunction {
    let b = bridge.clone();
    NativeFunction::from_copy_closure_with_captures(
        |_, args, bridge, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("open() requires a name"))?
                .to_std_string_escaped();

            let version: Option<u64> = if args.len() > 1 && !args[1].is_undefined() {
                let v = args[1].to_number(ctx)?;
                if v.is_nan() || v < 1.0 || v != v.floor() {
                    return Err(JsNativeError::typ()
                        .with_message("version must be a positive integer")
                        .into());
                }
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                Some(v as u64)
            } else {
                None
            };

            bridge
                .ensure_idb_backend()
                .map_err(|e| JsNativeError::typ().with_message(format!("{e}")))?;

            let open_request = request::build_open_request(ctx);

            // Fire versionchange on existing connections (same-tab) and queue
            // cross-tab notification for the browser thread.
            if let Some(ver) = version {
                let current = bridge
                    .with_idb(|backend| backend.get_version(&name).ok().flatten())
                    .flatten();
                if let Some(cur) = current {
                    if ver > cur {
                        bridge.fire_idb_versionchange(&name, cur, Some(ver), ctx);
                        // Queue cross-tab versionchange for browser thread broadcast.
                        bridge.queue_idb_versionchange_request(&name, cur, Some(ver));
                    }
                }
            }

            let result = bridge.with_idb(|backend| {
                elidex_indexeddb::database::open_database(backend, &name, version)
            });

            match result {
                Some(Ok(elidex_indexeddb::IdbOpenResult::Success(handle))) => {
                    let db_obj = super::super::indexeddb::build_database_object(
                        ctx,
                        bridge,
                        handle.name(),
                        handle.version(),
                    );
                    bridge.register_idb_connection(&name, db_obj.clone());
                    request::resolve_request(&open_request, JsValue::from(db_obj), ctx);
                    // open() fires onsuccess inline (special case — CRUD ops don't)
                    request::fire_handler(&open_request, "onsuccess", ctx);
                }
                Some(Ok(elidex_indexeddb::IdbOpenResult::UpgradeNeeded {
                    handle,
                    old_version,
                    new_version,
                })) => {
                    let db_obj = super::super::indexeddb::build_database_object(
                        ctx,
                        bridge,
                        handle.name(),
                        handle.version(),
                    );
                    bridge.register_idb_connection(&name, db_obj.clone());

                    // Set result to db before firing upgradeneeded
                    let _ = open_request.set(
                        js_string!("result"),
                        JsValue::from(db_obj.clone()),
                        false,
                        ctx,
                    );

                    // Mark db as in upgrade mode via bridge (tamper-proof)
                    bridge.set_idb_upgrading(Some(&name));

                    // Begin savepoint so schema changes can be rolled back on abort
                    let _ = bridge
                        .with_idb(|backend| backend.conn().execute_batch("SAVEPOINT idb_upgrade"));

                    // Fire onupgradeneeded with IDBVersionChangeEvent
                    let event = events::build_version_change_event(
                        "upgradeneeded",
                        old_version,
                        Some(new_version),
                        &open_request,
                        ctx,
                    );

                    let handler = open_request
                        .get(js_string!("onupgradeneeded"), ctx)
                        .unwrap_or(JsValue::null());
                    let upgrade_ok = if let Some(func) = handler.as_callable() {
                        func.call(
                            &JsValue::from(open_request.clone()),
                            &[JsValue::from(event)],
                            ctx,
                        )
                        .is_ok()
                    } else {
                        true
                    };

                    if !upgrade_ok {
                        // onupgradeneeded threw — rollback schema changes + abort
                        bridge.set_idb_upgrading(None);
                        let _ = bridge.with_idb(|backend| {
                            let _ = backend.conn().execute_batch("ROLLBACK TO idb_upgrade");
                            let _ = backend.conn().execute_batch("RELEASE idb_upgrade");
                            elidex_indexeddb::database::abort_upgrade(backend, &handle, old_version)
                        });
                        request::reject_request(
                            &open_request,
                            "AbortError: upgrade callback threw",
                            ctx,
                        );
                        return Ok(JsValue::from(open_request));
                    }

                    // Upgrade succeeded — release savepoint
                    let _ = bridge
                        .with_idb(|backend| backend.conn().execute_batch("RELEASE idb_upgrade"));

                    // After upgradeneeded callback, fire onsuccess
                    let _ = open_request.set(
                        js_string!("readyState"),
                        js_string!(request::READY_STATE_DONE),
                        false,
                        ctx,
                    );
                    request::fire_handler(&open_request, "onsuccess", ctx);
                    // Upgrade flag stays active — cleared by first transaction() call
                }
                Some(Err(e)) => {
                    bridge.set_idb_upgrading(None);
                    request::reject_request(&open_request, &e.to_string(), ctx);
                }
                None => {
                    request::reject_request(&open_request, "IndexedDB backend not available", ctx);
                }
            }

            Ok(JsValue::from(open_request))
        },
        b,
    )
}

/// `indexedDB.deleteDatabase(name)`
fn build_delete_fn(_ctx: &mut Context, bridge: &HostBridge) -> NativeFunction {
    let b = bridge.clone();
    NativeFunction::from_copy_closure_with_captures(
        |_, args, bridge, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("deleteDatabase() requires a name")
                })?
                .to_std_string_escaped();

            bridge
                .ensure_idb_backend()
                .map_err(|e| JsNativeError::typ().with_message(format!("{e}")))?;

            let open_request = request::build_open_request(ctx);

            // Fire versionchange on open connections (A7: newVersion=null for delete)
            let old_ver = bridge
                .with_idb(|backend| backend.get_version(&name).ok().flatten())
                .flatten()
                .unwrap_or(0);
            bridge.fire_idb_versionchange(&name, old_ver, None, ctx);
            // Queue cross-tab versionchange for browser thread broadcast.
            bridge.queue_idb_versionchange_request(&name, old_ver, None);

            let result = bridge
                .with_idb(|backend| elidex_indexeddb::database::delete_database(backend, &name));

            match result {
                Some(Ok(_old_version)) => {
                    request::resolve_request(&open_request, JsValue::undefined(), ctx);
                    request::fire_handler(&open_request, "onsuccess", ctx);
                }
                Some(Err(e)) => {
                    request::reject_request(&open_request, &e.to_string(), ctx);
                }
                None => {
                    request::reject_request(&open_request, "IndexedDB backend not available", ctx);
                }
            }

            Ok(JsValue::from(open_request))
        },
        b,
    )
}

/// `indexedDB.databases()` — returns a Promise resolving to an array of `{name, version}`.
fn build_databases_fn(_ctx: &mut Context, bridge: &HostBridge) -> NativeFunction {
    let b = bridge.clone();
    NativeFunction::from_copy_closure_with_captures(
        |_, _, bridge, ctx| {
            bridge
                .ensure_idb_backend()
                .map_err(|e| JsNativeError::typ().with_message(format!("{e}")))?;

            let dbs = bridge
                .with_idb(elidex_indexeddb::database::list_databases)
                .transpose()
                .map_err(|e| JsNativeError::typ().with_message(format!("{e}")))?
                .unwrap_or_default();

            let array = boa_engine::object::builtins::JsArray::new(ctx);
            for (name, version) in dbs {
                let info = ObjectInitializer::new(ctx)
                    .property(
                        js_string!("name"),
                        js_string!(name.as_str()),
                        Attribute::all(),
                    )
                    .property(
                        js_string!("version"),
                        #[allow(clippy::cast_precision_loss)]
                        JsValue::from(version as f64),
                        Attribute::all(),
                    )
                    .build();
                array.push(JsValue::from(info), ctx)?;
            }

            // Wrap in a resolved promise
            let promise: boa_engine::object::builtins::JsPromise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::from(array), ctx);
            Ok(promise.into())
        },
        b,
    )
}

/// `indexedDB.cmp(a, b)` — compare two IDB keys.
fn build_cmp_fn(_ctx: &mut Context) -> NativeFunction {
    NativeFunction::from_copy_closure(|_, args, ctx| {
        let a = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
        let b = js_value_to_idb_key(args.get(1).unwrap_or(&JsValue::undefined()), ctx)?;

        let result = match a.cmp(&b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        Ok(JsValue::from(result))
    })
}

/// Maximum nesting depth for array keys from JS (matches backend `MAX_KEY_DEPTH`).
const MAX_JS_KEY_DEPTH: usize = 64;

/// Convert a JS value to an `IdbKey`.
///
/// Valid JS key types: number, string, Date (as number), Array of keys.
pub(crate) fn js_value_to_idb_key(
    val: &JsValue,
    ctx: &mut Context,
) -> JsResult<elidex_indexeddb::IdbKey> {
    js_value_to_idb_key_depth(val, ctx, 0)
}

fn js_value_to_idb_key_depth(
    val: &JsValue,
    ctx: &mut Context,
    depth: usize,
) -> JsResult<elidex_indexeddb::IdbKey> {
    if let Some(n) = val.as_number() {
        if n.is_nan() || n.is_infinite() {
            return Err(JsNativeError::typ()
                .with_message("DataError: NaN/Infinity not valid IDB keys")
                .into());
        }
        return Ok(elidex_indexeddb::IdbKey::Number(n));
    }

    if let Some(s) = val.as_string() {
        return Ok(elidex_indexeddb::IdbKey::String(s.to_std_string_escaped()));
    }

    if let Some(obj) = val.as_object() {
        // Check if it's an array (before Date, since arrays may have getTime)
        if obj.is_array() {
            if depth >= MAX_JS_KEY_DEPTH {
                return Err(JsNativeError::typ()
                    .with_message("DataError: array key nesting too deep")
                    .into());
            }
            let len = obj.get(js_string!("length"), ctx)?.to_u32(ctx)?;
            let mut keys = Vec::with_capacity(len as usize);
            for i in 0..len {
                let elem = obj.get(i, ctx)?;
                keys.push(js_value_to_idb_key_depth(&elem, ctx, depth + 1)?);
            }
            return Ok(elidex_indexeddb::IdbKey::Array(keys));
        }

        // C2: Check if it's a Date object — convert to IdbKey::Date
        if let Ok(time_val) = obj.get(js_string!("getTime"), ctx) {
            if let Some(get_time) = time_val.as_callable() {
                if let Ok(ms) = get_time.call(&JsValue::from(obj.clone()), &[], ctx) {
                    if let Some(t) = ms.as_number() {
                        if t.is_finite() {
                            return Ok(elidex_indexeddb::IdbKey::Date(t));
                        }
                    }
                }
            }
        }
    }

    Err(JsNativeError::typ()
        .with_message("DataError: value is not a valid IDB key")
        .into())
}

/// Convert an `IdbKey` to a JS value.
#[allow(dead_code)] // Used in Step 10+ (IDBObjectStore, IDBCursor)
pub(crate) fn idb_key_to_js_value(key: &elidex_indexeddb::IdbKey, ctx: &mut Context) -> JsValue {
    match key {
        elidex_indexeddb::IdbKey::Number(n) => JsValue::from(*n),
        elidex_indexeddb::IdbKey::Date(n) => {
            // Construct JS Date object: new Date(ms)
            let date_ctor = ctx.global_object().get(js_string!("Date"), ctx).ok();
            if let Some(ctor) = date_ctor.and_then(|v| v.as_object()) {
                if let Ok(date) = ctor.construct(&[JsValue::from(*n)], Some(&ctor), ctx) {
                    return date.into();
                }
            }
            // Fallback: return as number if Date constructor unavailable
            JsValue::from(*n)
        }
        elidex_indexeddb::IdbKey::String(s) => JsValue::from(js_string!(s.as_str())),
        elidex_indexeddb::IdbKey::Array(items) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for item in items {
                let _ = arr.push(idb_key_to_js_value(item, ctx), ctx);
            }
            JsValue::from(arr)
        }
    }
}
