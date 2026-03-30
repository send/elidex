//! `IDBTransaction` JS object builder.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::request;

/// Build an `IDBTransaction` JS object.
pub(crate) fn build_transaction_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
    store_names: &[String],
    mode: &str,
) -> JsObject {
    let names_array = boa_engine::object::builtins::JsArray::new(ctx);
    for sn in store_names {
        let _ = names_array.push(JsValue::from(js_string!(sn.as_str())), ctx);
    }

    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("mode"), js_string!(mode), Attribute::all())
        .property(
            js_string!("durability"),
            js_string!("default"),
            Attribute::all(),
        )
        .property(
            js_string!("objectStoreNames"),
            JsValue::from(names_array),
            Attribute::all(),
        )
        .property(js_string!("error"), JsValue::null(), Attribute::all())
        .property(js_string!("oncomplete"), JsValue::null(), Attribute::all())
        .property(js_string!("onerror"), JsValue::null(), Attribute::all())
        .property(js_string!("onabort"), JsValue::null(), Attribute::all())
        .property(
            js_string!("__elidex_idb_name__"),
            js_string!(db_name),
            Attribute::empty(),
        )
        .property(
            js_string!("__elidex_idb_active__"),
            JsValue::from(true),
            Attribute::WRITABLE,
        )
        .build();

    // db back-reference (set by caller if needed)
    let _ = obj.set(js_string!("db"), JsValue::null(), false, ctx);

    // objectStore(name)
    register_object_store_accessor(&obj, ctx, bridge, db_name);

    // commit() / abort()
    register_finalize(&obj, ctx, bridge, "commit", "COMMIT", "oncomplete");
    register_finalize(&obj, ctx, bridge, "abort", "ROLLBACK", "onabort");

    obj
}

fn register_object_store_accessor(
    obj: &JsObject,
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
) {
    let b = bridge.clone();
    let name = db_name.to_owned();
    let tx_obj = obj.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, db_name, tx), ctx| {
            let store_name = args
                .first()
                .and_then(JsValue::as_string)
                .ok_or_else(|| JsNativeError::typ().with_message("objectStore() requires a name"))?
                .to_std_string_escaped();

            // Verify store exists
            let exists = bridge
                .with_idb(|backend| {
                    backend
                        .list_store_names(db_name)
                        .unwrap_or_default()
                        .contains(&store_name)
                })
                .unwrap_or(false);

            if !exists {
                return Err(JsNativeError::typ()
                    .with_message(format!(
                        "NotFoundError: object store '{store_name}' not found"
                    ))
                    .into());
            }

            let store_obj = super::object_store::build_object_store(
                ctx,
                bridge,
                db_name,
                &store_name,
                JsValue::from(tx.clone()),
            );
            Ok(JsValue::from(store_obj))
        },
        (b, name, tx_obj),
    );
    let _ = obj.set(
        js_string!("objectStore"),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

fn register_finalize(
    obj: &JsObject,
    ctx: &mut Context,
    bridge: &HostBridge,
    js_name: &str,
    sql: &'static str,
    handler: &'static str,
) {
    let b = bridge.clone();
    let tx_ref = obj.clone();
    let fn_obj = NativeFunction::from_copy_closure_with_captures(
        |_, _, (bridge, sql_cmd, handler_name, tx), ctx| {
            let active = tx
                .get(js_string!("__elidex_idb_active__"), ctx)?
                .to_boolean();
            if !active {
                return Err(JsNativeError::typ()
                    .with_message("InvalidStateError: transaction is not active")
                    .into());
            }

            let result = bridge.with_idb(|backend| backend.conn().execute_batch(sql_cmd));

            match result {
                Some(Ok(())) => {
                    let _ = tx.set(
                        js_string!("__elidex_idb_active__"),
                        JsValue::from(false),
                        false,
                        ctx,
                    );
                    request::fire_handler(tx, handler_name, ctx);
                    Ok(JsValue::undefined())
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ()
                    .with_message("IndexedDB backend not available")
                    .into()),
            }
        },
        (b, sql, handler, tx_ref),
    );
    let _ = obj.set(
        js_string!(js_name),
        JsValue::from(fn_obj.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}
