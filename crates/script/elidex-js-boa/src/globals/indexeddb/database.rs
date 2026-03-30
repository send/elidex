//! `IDBDatabase` JS object builder.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::transaction::build_transaction_object;

/// Check that the database is in upgrade mode (via bridge state, tamper-proof).
fn require_upgrade(bridge: &HostBridge, db_name: &str, op: &str) -> JsResult<()> {
    if !bridge.is_idb_upgrading(db_name) {
        return Err(JsNativeError::typ()
            .with_message(format!(
                "InvalidStateError: {op} only allowed during upgrade"
            ))
            .into());
    }
    Ok(())
}

/// Build a full `IDBDatabase` JS object.
pub(crate) fn build_database_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    name: &str,
    version: u64,
) -> JsObject {
    let store_names = bridge
        .with_idb(|backend| backend.list_store_names(name).unwrap_or_default())
        .unwrap_or_default();

    let names_array = boa_engine::object::builtins::JsArray::new(ctx);
    for sn in &store_names {
        let _ = names_array.push(JsValue::from(js_string!(sn.as_str())), ctx);
    }

    let db_name = name.to_owned();

    #[allow(clippy::cast_precision_loss)]
    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(
            js_string!("name"),
            js_string!(db_name.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("version"),
            JsValue::from(version as f64),
            Attribute::all(),
        )
        .property(
            js_string!("objectStoreNames"),
            JsValue::from(names_array),
            Attribute::all(),
        )
        .property(
            js_string!("onversionchange"),
            JsValue::null(),
            Attribute::all(),
        )
        .property(js_string!("onclose"), JsValue::null(), Attribute::all())
        .property(js_string!("onabort"), JsValue::null(), Attribute::all())
        .property(
            js_string!("__elidex_idb_name__"),
            js_string!(db_name.as_str()),
            Attribute::empty(),
        )
        .build();

    // close() — unregister from open connections (B12)
    let close_bridge = bridge.clone();
    let close_name = db_name.clone();
    let close_obj_ref = obj.clone();
    let close_fn = NativeFunction::from_copy_closure_with_captures(
        |_, _, (bridge, db_name, db_obj), _ctx| {
            bridge.unregister_idb_connection(db_name, db_obj);
            Ok(JsValue::undefined())
        },
        (close_bridge, close_name, close_obj_ref),
    );
    let _ = obj.set(
        js_string!("close"),
        JsValue::from(close_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // createObjectStore(name, options?)
    register_create_object_store(&obj, ctx, bridge, &db_name);

    // deleteObjectStore(name)
    register_delete_object_store(&obj, ctx, bridge, &db_name);

    // transaction(storeNames, mode?)
    register_transaction(&obj, ctx, bridge, &db_name);

    obj
}

fn register_create_object_store(
    obj: &JsObject,
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
) {
    let b = bridge.clone();
    let name = db_name.to_owned();
    let db_ref = obj.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, _db_obj), ctx| {
            require_upgrade(bridge, db_name, "createObjectStore")?;

            let store_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("createObjectStore requires a name")
                })?
                .to_std_string_escaped();

            let mut key_path: Option<String> = None;
            let mut auto_increment = false;

            if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
                if let Ok(kp) = opts.get(js_string!("keyPath"), ctx) {
                    if let Some(s) = kp.as_string() {
                        key_path = Some(s.to_std_string_escaped());
                    }
                }
                if let Ok(ai) = opts.get(js_string!("autoIncrement"), ctx) {
                    auto_increment = ai.to_boolean();
                }
            }

            // Validate constraints (D9)
            if auto_increment {
                if let Some(ref kp) = key_path {
                    if kp.is_empty() {
                        return Err(JsNativeError::typ()
                            .with_message("InvalidAccessError: autoIncrement with empty keyPath")
                            .into());
                    }
                    // Array keyPath not supported in M4-6
                }
            }

            let result = bridge.with_idb(|backend| {
                backend.create_object_store(
                    db_name,
                    &store_name,
                    key_path.as_deref(),
                    auto_increment,
                )
            });

            match result {
                Some(Ok(())) => {
                    let store_obj = super::object_store::build_object_store(
                        ctx,
                        bridge,
                        db_name,
                        &store_name,
                        JsValue::null(),
                    );
                    Ok(JsValue::from(store_obj))
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        (b, name, db_ref),
    );
    let _ = obj.set(
        js_string!("createObjectStore"),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

fn register_delete_object_store(
    obj: &JsObject,
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
) {
    let b = bridge.clone();
    let name = db_name.to_owned();
    let db_ref = obj.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, _db_obj), _ctx| {
            require_upgrade(bridge, db_name, "deleteObjectStore")?;

            let store_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("deleteObjectStore requires a name")
                })?
                .to_std_string_escaped();

            let result =
                bridge.with_idb(|backend| backend.delete_object_store(db_name, &store_name));

            match result {
                Some(Ok(())) => Ok(JsValue::undefined()),
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        (b, name, db_ref),
    );
    let _ = obj.set(
        js_string!("deleteObjectStore"),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

fn register_transaction(obj: &JsObject, ctx: &mut Context, bridge: &HostBridge, db_name: &str) {
    let b = bridge.clone();
    let name = db_name.to_owned();
    let db_ref = obj.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, _db_obj), ctx| {
            // Clear upgrade mode via bridge (tamper-proof)
            bridge.set_idb_upgrading(None);

            let store_names =
                extract_store_names(args.first().unwrap_or(&JsValue::undefined()), ctx)?;

            let mode_str = args
                .get(1)
                .and_then(JsValue::as_string)
                .map_or_else(|| "readonly".to_owned(), |s| s.to_std_string_escaped());

            let mode = match mode_str.as_str() {
                "readonly" => elidex_indexeddb::IdbTransactionMode::ReadOnly,
                "readwrite" => elidex_indexeddb::IdbTransactionMode::ReadWrite,
                _ => {
                    return Err(JsNativeError::typ()
                        .with_message(format!("TypeError: invalid transaction mode '{mode_str}'"))
                        .into());
                }
            };

            // Validate store names exist
            let existing = bridge
                .with_idb(|backend| backend.list_store_names(db_name).unwrap_or_default())
                .unwrap_or_default();
            for sn in &store_names {
                if !existing.contains(sn) {
                    return Err(JsNativeError::typ()
                        .with_message(format!("NotFoundError: object store '{sn}' does not exist"))
                        .into());
                }
            }

            // Begin SQLite transaction
            let tx_result = bridge.with_idb(|backend| {
                elidex_indexeddb::IdbTransaction::begin(
                    backend.conn(),
                    db_name,
                    store_names.clone(),
                    mode,
                )
            });

            match tx_result {
                Some(Ok(_tx)) => {
                    // NOTE: IdbTransaction handle is dropped here — commit/abort
                    // operate via raw SQL on the connection. The Rust state machine
                    // is not consulted. M4-10 will store the handle in the bridge
                    // and enforce mode/scope checks through it.
                    let tx_obj =
                        build_transaction_object(ctx, bridge, db_name, &store_names, &mode_str);
                    Ok(JsValue::from(tx_obj))
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        (b, name, db_ref),
    );
    let _ = obj.set(
        js_string!("transaction"),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

/// Extract store names from a string or array of strings argument.
fn extract_store_names(val: &JsValue, ctx: &mut Context) -> JsResult<Vec<String>> {
    if let Some(s) = val.as_string() {
        return Ok(vec![s.to_std_string_escaped()]);
    }
    if let Some(obj) = val.as_object() {
        if obj.is_array() {
            let len = obj.get(js_string!("length"), ctx)?.to_u32(ctx)?;
            let mut names = Vec::with_capacity(len as usize);
            for i in 0..len {
                let elem = obj.get(i, ctx)?;
                let name = elem
                    .as_string()
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("store name must be a string")
                    })?
                    .to_std_string_escaped();
                names.push(name);
            }
            return Ok(names);
        }
    }
    Err(JsNativeError::typ()
        .with_message("transaction() requires a store name or array of store names")
        .into())
}
