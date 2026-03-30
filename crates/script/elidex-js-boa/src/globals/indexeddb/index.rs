//! `IDBIndex` JS object builder.

use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::factory::{idb_key_to_js_value, js_value_to_idb_key};
use super::object_store::{self, parse_json_to_js};
use super::request;

/// Shared captures: `(bridge, db_name, store_name, index_name)`.
type IndexCaptures = (HostBridge, String, String, String);

/// Build a full `IDBIndex` JS object.
pub(crate) fn build_index_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    db_name: &str,
    store_name: &str,
    index_name: &str,
) -> JsObject {
    let meta = bridge
        .with_idb(|backend| {
            elidex_indexeddb::index::get_index_meta(backend, db_name, store_name, index_name).ok()
        })
        .flatten();

    let (key_path, unique, multi_entry) = meta
        .map(|m| (m.key_path, m.unique, m.multi_entry))
        .unwrap_or_default();

    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("name"), js_string!(index_name), Attribute::all())
        .property(
            js_string!("keyPath"),
            js_string!(key_path.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("unique"),
            JsValue::from(unique),
            Attribute::all(),
        )
        .property(
            js_string!("multiEntry"),
            JsValue::from(multi_entry),
            Attribute::all(),
        )
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
        .property(
            js_string!("__elidex_index_name__"),
            js_string!(index_name),
            Attribute::empty(),
        )
        .build();

    let caps: IndexCaptures = (
        bridge.clone(),
        db_name.to_owned(),
        store_name.to_owned(),
        index_name.to_owned(),
    );

    register_idx_method(&obj, ctx, "get", &caps, idx_get);
    register_idx_method(&obj, ctx, "getKey", &caps, idx_get_key);
    register_idx_method(&obj, ctx, "getAll", &caps, idx_get_all);
    register_idx_method(&obj, ctx, "getAllKeys", &caps, idx_get_all_keys);
    register_idx_method(&obj, ctx, "count", &caps, idx_count);
    register_idx_method(&obj, ctx, "openCursor", &caps, idx_open_cursor);
    register_idx_method(&obj, ctx, "openKeyCursor", &caps, idx_open_key_cursor);

    // objectStore back-reference (lightweight — just the name, not a full store object)
    let store_ref = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("name"), js_string!(store_name), Attribute::all())
        .build();
    let _ = obj.set(
        js_string!("objectStore"),
        JsValue::from(store_ref),
        false,
        ctx,
    );

    obj
}

fn register_idx_method(
    obj: &JsObject,
    ctx: &mut Context,
    name: &str,
    caps: &IndexCaptures,
    handler: fn(&[JsValue], &IndexCaptures, &mut Context) -> boa_engine::JsResult<JsValue>,
) {
    let c = caps.clone();
    let f = NativeFunction::from_copy_closure_with_captures(
        move |_, args, captures, ctx| handler(args, captures, ctx),
        c,
    );
    let _ = obj.set(
        js_string!(name),
        JsValue::from(f.to_js_function(ctx.realm())),
        false,
        ctx,
    );
}

fn idx_get(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
    let req = request::build_request(ctx);

    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::index::index_get(backend, db_name, store_name, index_name, &key)
    });
    match result {
        Some(Ok(Some(val))) => request::resolve_request(&req, parse_json_to_js(&val, ctx), ctx),
        Some(Ok(None)) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn idx_get_key(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let key = js_value_to_idb_key(args.first().unwrap_or(&JsValue::undefined()), ctx)?;
    let req = request::build_request(ctx);

    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::index::index_get_key(backend, db_name, store_name, index_name, &key)
    });
    match result {
        Some(Ok(Some(pk))) => request::resolve_request(&req, idb_key_to_js_value(&pk, ctx), ctx),
        Some(Ok(None)) => request::resolve_request(&req, JsValue::undefined(), ctx),
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn idx_get_all(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let range = object_store::extract_range(args.first(), ctx)?;
    let count = object_store::extract_count(args.get(1), ctx)?;
    let req = request::build_request(ctx);

    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::index::index_get_all(
            backend,
            db_name,
            store_name,
            index_name,
            range.as_ref(),
            count,
        )
    });
    match result {
        Some(Ok(rows)) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for (_pk, val) in &rows {
                let _ = arr.push(parse_json_to_js(val, ctx), ctx);
            }
            request::resolve_request(&req, JsValue::from(arr), ctx);
        }
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn idx_get_all_keys(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let range = object_store::extract_range(args.first(), ctx)?;
    let count = object_store::extract_count(args.get(1), ctx)?;
    let req = request::build_request(ctx);

    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::index::index_get_all_keys(
            backend,
            db_name,
            store_name,
            index_name,
            range.as_ref(),
            count,
        )
    });
    match result {
        Some(Ok(keys)) => {
            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for k in &keys {
                let _ = arr.push(idb_key_to_js_value(k, ctx), ctx);
            }
            request::resolve_request(&req, JsValue::from(arr), ctx);
        }
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn idx_count(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let range = object_store::extract_range(args.first(), ctx)?;
    let req = request::build_request(ctx);

    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::index::index_count(
            backend,
            db_name,
            store_name,
            index_name,
            range.as_ref(),
        )
    });
    match result {
        #[allow(clippy::cast_precision_loss)]
        Some(Ok(n)) => request::resolve_request(&req, JsValue::from(n as f64), ctx),
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}

fn idx_open_cursor(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    idx_open_cursor_impl(args, caps, ctx, false)
}

fn idx_open_key_cursor(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    idx_open_cursor_impl(args, caps, ctx, true)
}

fn idx_open_cursor_impl(
    args: &[JsValue],
    caps: &IndexCaptures,
    ctx: &mut Context,
    key_only: bool,
) -> boa_engine::JsResult<JsValue> {
    let (bridge, db_name, store_name, index_name) = caps;
    let range = object_store::extract_range(args.first(), ctx)?;
    let direction = args
        .get(1)
        .and_then(JsValue::as_string)
        .map_or_else(|| "next".to_owned(), |s| s.to_std_string_escaped());
    let dir = elidex_indexeddb::cursor::CursorDirection::parse(&direction).ok_or_else(|| {
        JsNativeError::typ().with_message(format!("invalid cursor direction '{direction}'"))
    })?;

    let req = request::build_request(ctx);
    let result = bridge.with_idb(|backend| {
        elidex_indexeddb::cursor::open_index_cursor(
            backend, db_name, store_name, index_name, range, dir, key_only,
        )
    });

    match result {
        Some(Ok(cursor_state)) => {
            if cursor_state.current().is_none() {
                request::resolve_request(&req, JsValue::null(), ctx);
            } else {
                let cursor_id = bridge.store_idb_cursor(cursor_state);
                let cursor_obj = super::cursor::build_cursor_object(
                    ctx, bridge, cursor_id, key_only, &direction, &req,
                );
                request::resolve_request(&req, JsValue::from(cursor_obj), ctx);
            }
        }
        Some(Err(ref e)) => request::reject_request_backend(&req, e, ctx),
        None => request::reject_request(&req, "IndexedDB backend not available", ctx),
    }
    Ok(JsValue::from(req))
}
