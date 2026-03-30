//! `IDBCursor` / `IDBCursorWithValue` JS object builder.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::factory::{idb_key_to_js_value, js_value_to_idb_key};
use super::object_store::parse_json_to_js;
use super::request;

/// Build an `IDBCursor` or `IDBCursorWithValue` JS object with full methods.
#[allow(clippy::too_many_lines)]
pub(crate) fn build_cursor_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    cursor_id: u64,
    key_only: bool,
    direction: &str,
    req: &JsObject,
) -> JsObject {
    #[allow(clippy::cast_precision_loss)]
    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(
            js_string!("direction"),
            js_string!(direction),
            Attribute::all(),
        )
        .property(
            js_string!("request"),
            JsValue::from(req.clone()),
            Attribute::all(),
        )
        .property(js_string!("source"), JsValue::null(), Attribute::all())
        .property(
            js_string!("__elidex_cursor_id__"),
            JsValue::from(cursor_id as f64),
            Attribute::empty(),
        )
        .property(
            js_string!("__elidex_key_only__"),
            JsValue::from(key_only),
            Attribute::empty(),
        )
        .build();

    // Populate key/primaryKey/value from current state
    populate_cursor_props(&obj, bridge, cursor_id, key_only, ctx);

    // advance(count)
    let b = bridge.clone();
    let cur_obj = obj.clone();
    let advance_fn = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, cursor), ctx| {
            let cursor_id = get_cursor_id(cursor, ctx)?;
            let count = args
                .first()
                .map(|v| v.to_u32(ctx))
                .transpose()?
                .unwrap_or(1);
            // W3C §4.9: advance(0) throws TypeError (not DOMException)
            if count == 0 {
                return Err(JsNativeError::typ()
                    .with_message("advance count must be a positive number")
                    .into());
            }

            let result = bridge.with_idb_cursor(cursor_id, |backend, state| {
                elidex_indexeddb::cursor::advance(backend, state, count)
            });

            match result {
                Some(Ok(())) => {
                    let key_only = cursor
                        .get(js_string!("__elidex_key_only__"), ctx)?
                        .to_boolean();
                    populate_cursor_props(cursor, bridge, cursor_id, key_only, ctx);
                    Ok(JsValue::undefined())
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ().with_message("cursor not found").into()),
            }
        },
        (b, cur_obj),
    );
    let _ = obj.set(
        js_string!("advance"),
        JsValue::from(advance_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // continue(key?)
    let b = bridge.clone();
    let cur_obj = obj.clone();
    let continue_fn = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, cursor), ctx| {
            let cursor_id = get_cursor_id(cursor, ctx)?;
            let target_key = if !args.is_empty() && !args[0].is_undefined() {
                Some(js_value_to_idb_key(&args[0], ctx)?)
            } else {
                None
            };

            let result = bridge.with_idb_cursor(cursor_id, |backend, state| {
                elidex_indexeddb::cursor::continue_cursor(backend, state, target_key.as_ref())
            });

            match result {
                Some(Ok(())) => {
                    let key_only = cursor
                        .get(js_string!("__elidex_key_only__"), ctx)?
                        .to_boolean();
                    populate_cursor_props(cursor, bridge, cursor_id, key_only, ctx);
                    Ok(JsValue::undefined())
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ().with_message("cursor not found").into()),
            }
        },
        (b, cur_obj),
    );
    let _ = obj.set(
        js_string!("continue"),
        JsValue::from(continue_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // continuePrimaryKey(key, primaryKey) — B1
    let b = bridge.clone();
    let cur_obj = obj.clone();
    let continue_pk_fn = NativeFunction::from_copy_closure_with_captures(
        |_, args, (bridge, cursor), ctx| {
            let cursor_id = get_cursor_id(cursor, ctx)?;
            let key = super::factory::js_value_to_idb_key(
                args.first().unwrap_or(&JsValue::undefined()),
                ctx,
            )?;
            let pk = super::factory::js_value_to_idb_key(
                args.get(1).unwrap_or(&JsValue::undefined()),
                ctx,
            )?;

            let result = bridge.with_idb_cursor(cursor_id, |backend, state| {
                elidex_indexeddb::cursor::continue_primary_key(backend, state, &key, &pk)
            });

            match result {
                Some(Ok(())) => {
                    let key_only = cursor
                        .get(js_string!("__elidex_key_only__"), ctx)?
                        .to_boolean();
                    populate_cursor_props(cursor, bridge, cursor_id, key_only, ctx);
                    Ok(JsValue::undefined())
                }
                Some(Err(e)) => Err(JsNativeError::typ().with_message(format!("{e}")).into()),
                None => Err(JsNativeError::typ().with_message("cursor not found").into()),
            }
        },
        (b, cur_obj),
    );
    let _ = obj.set(
        js_string!("continuePrimaryKey"),
        JsValue::from(continue_pk_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    // update(value) — only for IDBCursorWithValue (not key-only)
    if !key_only {
        let b = bridge.clone();
        let cur_obj = obj.clone();
        let update_fn = NativeFunction::from_copy_closure_with_captures(
            |_, args, (bridge, cursor), ctx| {
                let cursor_id = get_cursor_id(cursor, ctx)?;
                let value = super::object_store::serialize_value(
                    args.first().unwrap_or(&JsValue::undefined()),
                    ctx,
                )?;

                let result = bridge.with_idb_cursor(cursor_id, |backend, state| {
                    elidex_indexeddb::cursor::update_current(backend, state, &value)
                });

                let req = request::build_request(ctx);
                match result {
                    Some(Ok(())) => request::resolve_request(&req, JsValue::undefined(), ctx),
                    Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
                    None => request::reject_request(&req, "cursor not found", ctx),
                }
                Ok(JsValue::from(req))
            },
            (b, cur_obj),
        );
        let _ = obj.set(
            js_string!("update"),
            JsValue::from(update_fn.to_js_function(ctx.realm())),
            false,
            ctx,
        );
    }

    // delete()
    let b = bridge.clone();
    let cur_obj = obj.clone();
    let delete_fn = NativeFunction::from_copy_closure_with_captures(
        |_, _, (bridge, cursor), ctx| {
            let cursor_id = get_cursor_id(cursor, ctx)?;

            let result = bridge.with_idb_cursor(cursor_id, |backend, state| {
                elidex_indexeddb::cursor::delete_current(backend, state)
            });

            let req = request::build_request(ctx);
            match result {
                Some(Ok(())) => request::resolve_request(&req, JsValue::undefined(), ctx),
                Some(Err(e)) => request::reject_request(&req, &e.to_string(), ctx),
                None => request::reject_request(&req, "cursor not found", ctx),
            }
            Ok(JsValue::from(req))
        },
        (b, cur_obj),
    );
    let _ = obj.set(
        js_string!("delete"),
        JsValue::from(delete_fn.to_js_function(ctx.realm())),
        false,
        ctx,
    );

    obj
}

/// Update the cursor JS object's key/primaryKey/value properties from the current state.
fn populate_cursor_props(
    obj: &JsObject,
    bridge: &HostBridge,
    cursor_id: u64,
    key_only: bool,
    ctx: &mut Context,
) {
    let entry = bridge.with_idb_cursor(cursor_id, |_, state| {
        state
            .current()
            .map(|e| (e.key.clone(), e.primary_key.clone(), e.value.clone()))
    });

    if let Some(Some((key, pk, value))) = entry {
        let _ = obj.set(
            js_string!("key"),
            idb_key_to_js_value(&key, ctx),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("primaryKey"),
            idb_key_to_js_value(&pk, ctx),
            false,
            ctx,
        );
        if !key_only {
            if let Some(val) = value {
                let _ = obj.set(js_string!("value"), parse_json_to_js(&val, ctx), false, ctx);
            } else {
                let _ = obj.set(js_string!("value"), JsValue::undefined(), false, ctx);
            }
        }
    } else {
        // Cursor exhausted — clean up backend state (F7: prevent memory leak)
        bridge.remove_idb_cursor(cursor_id);
        let _ = obj.set(js_string!("key"), JsValue::null(), false, ctx);
        let _ = obj.set(js_string!("primaryKey"), JsValue::null(), false, ctx);
        if !key_only {
            let _ = obj.set(js_string!("value"), JsValue::undefined(), false, ctx);
        }
    }
}

/// Extract `cursor_id` from the hidden property.
fn get_cursor_id(obj: &JsObject, ctx: &mut Context) -> boa_engine::JsResult<u64> {
    let val = obj.get(js_string!("__elidex_cursor_id__"), ctx)?;
    let n = val.to_number(ctx)?;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    Ok(n as u64)
}
