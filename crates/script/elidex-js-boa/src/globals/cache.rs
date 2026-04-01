//! Cache API JS bindings (WHATWG Cache API).
//!
//! Registers `caches` global on both window and worker scopes.

use std::rc::Rc;

use boa_engine::{
    js_string, object::ObjectInitializer, Context, JsNativeError, JsObject, JsResult, JsValue,
    NativeFunction,
};

use crate::bridge::HostBridge;

/// Register `caches` global (CacheStorage).
pub fn register_caches(ctx: &mut Context, bridge: &HostBridge) {
    let caches = build_cache_storage(ctx, bridge);
    let global = ctx.global_object();
    global
        .set(js_string!("caches"), caches, false, ctx)
        .expect("failed to register caches");
}

#[allow(clippy::too_many_lines)]
fn build_cache_storage(ctx: &mut Context, bridge: &HostBridge) -> JsValue {
    let b = bridge.clone();
    let open_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("caches.open requires a string name")
                })?;

            bridge
                .ensure_cache_backend()
                .map_err(|e| JsNativeError::typ().with_message(e))?;
            bridge
                .with_cache(|conn| {
                    elidex_cache_api::storage::open(conn, &name).map_err(|e| format!("{e}"))
                })
                .unwrap_or(Err("cache backend not initialized".into()))
                .map_err(|e| JsNativeError::typ().with_message(e))?;

            let cache_obj = build_cache_object(ctx, bridge, &name)?;
            let val: JsValue = cache_obj.into();
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        b,
    );

    let b = bridge.clone();
    let has_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("caches.has requires a string name")
                })?;

            bridge
                .ensure_cache_backend()
                .map_err(|e| JsNativeError::typ().with_message(e))?;
            let result = bridge
                .with_cache(|conn| elidex_cache_api::storage::has(conn, &name).unwrap_or(false))
                .unwrap_or(false);

            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::from(result), ctx);
            Ok(promise.into())
        },
        b,
    );

    let b = bridge.clone();
    let delete_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let name = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("caches.delete requires a string name")
                })?;

            bridge
                .ensure_cache_backend()
                .map_err(|e| JsNativeError::typ().with_message(e))?;
            let result = bridge
                .with_cache(|conn| elidex_cache_api::storage::delete(conn, &name).unwrap_or(false))
                .unwrap_or(false);

            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::from(result), ctx);
            Ok(promise.into())
        },
        b,
    );

    let b = bridge.clone();
    let keys_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, ctx| {
            bridge
                .ensure_cache_backend()
                .map_err(|e| JsNativeError::typ().with_message(e))?;
            let names = bridge
                .with_cache(|conn| elidex_cache_api::storage::keys(conn).unwrap_or_default())
                .unwrap_or_default();

            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for name in names {
                arr.push(JsValue::from(js_string!(name)), ctx)?;
            }
            let val: JsValue = arr.into();
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        b,
    );

    let b = bridge.clone();
    let match_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| JsNativeError::typ().with_message("caches.match requires a URL"))?;

            bridge
                .ensure_cache_backend()
                .map_err(|e| JsNativeError::typ().with_message(e))?;

            let result = bridge
                .with_cache(|conn| {
                    let cache_names = elidex_cache_api::storage::keys(conn).unwrap_or_default();
                    for name in cache_names {
                        if let Ok(Some(entry)) = elidex_cache_api::store::match_request(
                            conn,
                            &name,
                            &url,
                            "GET",
                            &[],
                            &elidex_cache_api::MatchOptions::default(),
                        ) {
                            return Some(entry);
                        }
                    }
                    None
                })
                .flatten();

            // Phase 2: returns body string. M4-8.5 will return full Response object.
            let val = match result {
                Some(entry) => JsValue::from(js_string!(String::from_utf8_lossy(
                    &entry.response_body
                )
                .to_string())),
                None => JsValue::undefined(),
            };
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        b,
    );

    ObjectInitializer::new(ctx)
        .function(open_fn, js_string!("open"), 1)
        .function(has_fn, js_string!("has"), 1)
        .function(delete_fn, js_string!("delete"), 1)
        .function(keys_fn, js_string!("keys"), 0)
        .function(match_fn, js_string!("match"), 1)
        .build()
        .into()
}

/// Captures for per-cache closures: (bridge, cache_name).
type CacheCaptures = (HostBridge, Rc<str>);

#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn build_cache_object(
    ctx: &mut Context,
    bridge: &HostBridge,
    cache_name: &str,
) -> JsResult<JsObject> {
    let name: Rc<str> = cache_name.into();

    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let match_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| JsNativeError::typ().with_message("cache.match requires a URL"))?;

            let result = bridge
                .with_cache(|conn| {
                    elidex_cache_api::store::match_request(
                        conn,
                        name,
                        &url,
                        "GET",
                        &[],
                        &elidex_cache_api::MatchOptions::default(),
                    )
                    .ok()
                    .flatten()
                })
                .flatten();

            let val = match result {
                Some(entry) => JsValue::from(js_string!(String::from_utf8_lossy(
                    &entry.response_body
                )
                .to_string())),
                None => JsValue::undefined(),
            };
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        captures,
    );

    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let put_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("cache.put requires a request URL")
                })?;

            let body = args
                .get(1)
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let entry = elidex_cache_api::CachedEntry {
                request_url: url,
                request_method: "GET".into(),
                response_status: 200,
                response_status_text: "OK".into(),
                response_headers: vec![],
                response_body: body.into_bytes(),
                vary_headers: vec![],
                is_opaque: false,
            };

            bridge
                .with_cache(|conn| {
                    elidex_cache_api::store::put(conn, name, &entry).map_err(|e| format!("{e}"))
                })
                .unwrap_or(Err("cache backend not initialized".into()))
                .map_err(|e| JsNativeError::typ().with_message(e))?;

            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
            Ok(promise.into())
        },
        captures,
    );

    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let delete_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| JsNativeError::typ().with_message("cache.delete requires a URL"))?;

            let result = bridge
                .with_cache(|conn| {
                    elidex_cache_api::store::delete(
                        conn,
                        name,
                        &url,
                        "GET",
                        &[],
                        &elidex_cache_api::MatchOptions::default(),
                    )
                    .unwrap_or(false)
                })
                .unwrap_or(false);

            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::from(result), ctx);
            Ok(promise.into())
        },
        captures,
    );

    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let keys_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, (bridge, name), ctx| {
            let entries = bridge
                .with_cache(|conn| elidex_cache_api::store::keys(conn, name).unwrap_or_default())
                .unwrap_or_default();

            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for entry in entries {
                arr.push(JsValue::from(js_string!(entry.request_url)), ctx)?;
            }
            let val: JsValue = arr.into();
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        captures,
    );

    // matchAll(request?, options?) — returns array of matching responses
    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let match_all_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let entries = bridge
                .with_cache(|conn| {
                    if url.is_empty() {
                        elidex_cache_api::store::keys(conn, name).unwrap_or_default()
                    } else {
                        elidex_cache_api::store::match_all(
                            conn,
                            name,
                            &url,
                            "GET",
                            &[],
                            &elidex_cache_api::MatchOptions::default(),
                        )
                        .unwrap_or_default()
                    }
                })
                .unwrap_or_default();

            let arr = boa_engine::object::builtins::JsArray::new(ctx);
            for entry in entries {
                arr.push(
                    JsValue::from(js_string!(
                        String::from_utf8_lossy(&entry.response_body).to_string()
                    )),
                    ctx,
                )?;
            }
            let val: JsValue = arr.into();
            let promise = boa_engine::object::builtins::JsPromise::resolve(val, ctx);
            Ok(promise.into())
        },
        captures,
    );

    // add(request) — fetch + put (simplified: accepts URL string)
    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let add_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let url = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .ok_or_else(|| JsNativeError::typ().with_message("cache.add requires a URL"))?;

            // Phase 2: synchronous stub — stores empty response for the URL.
            // Real implementation requires fetch() integration.
            let entry = elidex_cache_api::CachedEntry {
                request_url: url,
                request_method: "GET".into(),
                response_status: 200,
                response_status_text: "OK".into(),
                response_headers: vec![],
                response_body: Vec::new(),
                vary_headers: vec![],
                is_opaque: false,
            };

            bridge
                .with_cache(|conn| {
                    elidex_cache_api::store::put(conn, name, &entry).map_err(|e| format!("{e}"))
                })
                .unwrap_or(Err("cache backend not initialized".into()))
                .map_err(|e| JsNativeError::typ().with_message(e))?;

            let promise =
                boa_engine::object::builtins::JsPromise::resolve(JsValue::undefined(), ctx);
            Ok(promise.into())
        },
        captures,
    );

    // addAll(requests) — batch fetch + put (atomic, all-or-nothing)
    let captures: CacheCaptures = (bridge.clone(), Rc::clone(&name));
    let add_all_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, (bridge, name), ctx| {
            let arr = args.first().cloned().unwrap_or(JsValue::undefined());
            let arr_obj = arr.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("cache.addAll requires an array")
            })?;

            let raw_length = arr_obj.get(js_string!("length"), ctx)?.to_number(ctx)?;
            if !raw_length.is_finite() || !(0.0..=10_000.0).contains(&raw_length) {
                return Err(JsNativeError::typ()
                    .with_message("cache.addAll: invalid array length")
                    .into());
            }
            let length = raw_length as u64;

            let mut entries = Vec::with_capacity(length as usize);
            for i in 0..length {
                let item = arr_obj.get(i, ctx)?;
                let url = item
                    .as_string()
                    .map(|s| s.to_std_string_escaped())
                    .ok_or_else(|| {
                        JsNativeError::typ().with_message("cache.addAll: array items must be URLs")
                    })?;

                entries.push(elidex_cache_api::CachedEntry {
                    request_url: url,
                    request_method: "GET".into(),
                    response_status: 200,
                    response_status_text: "OK".into(),
                    response_headers: vec![],
                    response_body: Vec::new(),
                    vary_headers: vec![],
                    is_opaque: false,
                });
            }

            bridge
                .with_cache(|conn| {
                    elidex_cache_api::store::add_all(conn, name, &entries)
                        .map_err(|e| format!("{e}"))
                })
                .unwrap_or(Err("cache backend not initialized".into()))
                .map_err(|e| JsNativeError::typ().with_message(e))?;

            Ok(JsValue::undefined())
        },
        captures,
    );

    let obj = ObjectInitializer::new(ctx)
        .function(match_fn, js_string!("match"), 1)
        .function(match_all_fn, js_string!("matchAll"), 1)
        .function(add_fn, js_string!("add"), 1)
        .function(add_all_fn, js_string!("addAll"), 1)
        .function(put_fn, js_string!("put"), 2)
        .function(delete_fn, js_string!("delete"), 1)
        .function(keys_fn, js_string!("keys"), 0)
        .build();

    Ok(obj)
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_module_compiles() {
        // Compilation test — actual JS integration tested via JsRuntime
    }
}
