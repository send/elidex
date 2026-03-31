//! `Worker` constructor registration (WHATWG HTML §10.1).
//!
//! Registers `new Worker(scriptURL, options?)` on the global object. The
//! constructor fetches the script, spawns a worker thread, and returns a
//! JS object with `postMessage`, `terminate`, and event handler properties.

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// JavaScript MIME types accepted for worker scripts (WHATWG HTML §10.1.3).
pub(crate) const JS_MIME_TYPES: &[&str] = &[
    "text/javascript",
    "application/javascript",
    "application/x-javascript",
];

/// Validate a worker script HTTP response: check MIME type and HTTP status.
/// Returns `Ok(script_source)` on success, `Err(error_message)` on failure.
pub(crate) fn validate_worker_script_response(
    response: &elidex_net::Response,
    url: &url::Url,
) -> Result<String, String> {
    // Check MIME type
    let content_type = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str());
    if let Some(ct) = content_type {
        let mime = ct
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !mime.is_empty() && !JS_MIME_TYPES.contains(&mime.as_str()) {
            return Err(format!("Invalid MIME type for worker script: {mime}"));
        }
    }
    // Check HTTP status
    if response.status < 200 || response.status >= 300 {
        return Err(format!(
            "Worker script fetch failed with status {} for {url}",
            response.status
        ));
    }
    Ok(String::from_utf8_lossy(&response.body).to_string())
}

/// Register the `Worker` constructor on the global object.
pub fn register_worker_constructor(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let ctor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| construct_worker(args, bridge, ctx),
        b,
    );
    ctx.register_global_callable(js_string!("Worker"), 1, ctor)
        .expect("failed to register Worker");
}

/// Implementation of `new Worker(scriptURL, options?)`.
#[allow(clippy::too_many_lines)]
fn construct_worker(
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    // 1. Get script URL string.
    let url_str = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    if url_str.is_empty() {
        return Err(JsNativeError::typ()
            .with_message("Worker: script URL is required")
            .into());
    }

    // 2. Parse options.
    let options = args.get(1);
    let name = options
        .and_then(JsValue::as_object)
        .map(|opts| opts.get(js_string!("name"), ctx))
        .transpose()?
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    // Check type option — only "classic" supported.
    if let Some(opts) = options.and_then(JsValue::as_object) {
        let type_val = opts.get(js_string!("type"), ctx)?;
        if !type_val.is_undefined() {
            let type_str = type_val.to_string(ctx)?.to_std_string_escaped();
            if type_str == "module" {
                return Err(JsNativeError::typ()
                    .with_message("Worker: type 'module' is not supported")
                    .into());
            }
        }
    }

    // Parse credentials option (WHATWG HTML §10.1.3).
    // Valid values: "same-origin" (default per spec), "omit", "include".
    // Affects cookie inclusion on script fetch. Since each worker gets an
    // independent FetchHandle with no shared cookie jar, this currently has
    // no practical effect, but we validate per spec.
    let credentials = options
        .and_then(JsValue::as_object)
        .map(|opts| opts.get(js_string!("credentials"), ctx))
        .transpose()?
        .filter(|v| !v.is_undefined())
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map_or_else(|| "same-origin".to_string(), |s| s.to_std_string_escaped());

    if !["omit", "same-origin", "include"].contains(&credentials.as_str()) {
        return Err(JsNativeError::typ()
            .with_message(format!("Worker: invalid credentials value: {credentials}"))
            .into());
    }

    // 3. Resolve URL relative to current document URL.
    let base_url = bridge
        .current_url()
        .unwrap_or_else(|| url::Url::parse("about:blank").expect("about:blank is valid"));

    let resolved = base_url
        .join(&url_str)
        .map_err(|e| JsNativeError::typ().with_message(format!("Worker: invalid URL: {e}")))?;

    // 4. Same-origin check (WHATWG HTML §10.1.3).
    let mut is_same_origin = resolved.scheme() == "data" || base_url.origin() == resolved.origin();

    // For blob: URLs, the origin is embedded in the URL: blob:https://example.com/uuid
    // Parse it and compare with base_url's origin.
    if resolved.scheme() == "blob" {
        let blob_origin = resolved.path();
        if let Ok(inner_url) = url::Url::parse(blob_origin) {
            is_same_origin = inner_url.origin() == base_url.origin();
        }
    }

    if !is_same_origin {
        return Err(JsNativeError::typ()
            .with_message(format!(
                "SecurityError: Worker script URL {resolved} is not same-origin with {base_url}"
            ))
            .into());
    }

    // 5. Build the Worker JS object (returned immediately per WHATWG HTML §10.1.3).
    let worker_obj = build_worker_js_object(ctx, bridge);

    // 6. Create channel pair and spawn worker thread.
    // Per WHATWG spec, script fetching happens asynchronously in the worker
    // thread — the constructor returns the Worker object immediately.
    let (parent_channel, worker_channel) = elidex_plugin::channel_pair();

    let script_url_clone = resolved.clone();
    let name_clone = name.clone();
    // Create a sibling NetworkHandle for the worker, sharing the parent's broker.
    let Some(worker_nh) = bridge.create_sibling_network_handle() else {
        return Err(JsNativeError::typ()
            .with_message("network unavailable for worker")
            .into());
    };
    let thread_handle = std::thread::spawn(move || {
        crate::worker_thread::worker_thread_main(
            script_url_clone,
            name_clone,
            worker_channel,
            worker_nh,
        );
    });

    // 7. Create WorkerHandle and register in bridge.
    let handle =
        elidex_api_workers::WorkerHandle::new(name, resolved, parent_channel, thread_handle);

    let worker_id = bridge.register_worker(handle, worker_obj.clone());

    // Store worker ID on the JS object for O(1) lookup.
    #[allow(clippy::cast_precision_loss)]
    worker_obj
        .set(
            js_string!("__elidex_worker_id__"),
            JsValue::from(worker_id as f64),
            false,
            ctx,
        )
        .expect("failed to set worker ID");

    Ok(JsValue::from(worker_obj))
}

/// Build the Worker JS object with `postMessage`, `terminate`, and event
/// handler properties.
#[allow(clippy::too_many_lines)]
fn build_worker_js_object(ctx: &mut Context, bridge: &HostBridge) -> JsObject {
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);

    // postMessage(data)
    let b_post = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let data = args.first().cloned().unwrap_or(JsValue::undefined());

                let Ok(Some(json_str)) =
                    crate::globals::worker_scope::js_json_stringify(&data, ctx)
                else {
                    return Err(JsNativeError::typ()
                        .with_message("DataCloneError: Failed to serialize message")
                        .into());
                };

                let worker_id = find_worker_id_from_this(this, ctx);
                if let Some(id) = worker_id {
                    let origin = bridge
                        .current_url()
                        .map_or_else(|| "null".to_string(), |u| u.origin().ascii_serialization());
                    bridge.with_worker_registry(|reg| {
                        if let Some(entry) = reg.get_entry_mut(id) {
                            entry.handle.post_message(json_str, origin);
                        }
                    });
                }
                Ok(JsValue::undefined())
            },
            b_post,
        ),
        js_string!("postMessage"),
        1,
    );

    // terminate()
    let b_term = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let worker_id = find_worker_id_from_this(this, ctx);
                if let Some(id) = worker_id {
                    bridge.with_worker_registry(|reg| {
                        reg.terminate_worker(id);
                    });
                }
                Ok(JsValue::undefined())
            },
            b_term,
        ),
        js_string!("terminate"),
        0,
    );

    // addEventListener(type, callback)
    let b_al = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                if let Some(cb) = args.get(1).and_then(JsValue::as_object) {
                    if let Some(id) = find_worker_id_from_this(this, ctx) {
                        bridge.with_worker_registry(|reg| {
                            reg.add_event_listener(id, event_type, cb.clone());
                        });
                    }
                }
                Ok(JsValue::undefined())
            },
            b_al,
        ),
        js_string!("addEventListener"),
        2,
    );

    // removeEventListener(type, callback)
    let b_remove = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                if let Some(cb) = args.get(1).and_then(JsValue::as_object) {
                    if let Some(id) = find_worker_id_from_this(this, ctx) {
                        bridge.with_worker_registry(|reg| {
                            reg.remove_event_listener(id, &event_type, &cb);
                        });
                    }
                }
                Ok(JsValue::undefined())
            },
            b_remove,
        ),
        js_string!("removeEventListener"),
        2,
    );

    // dispatchEvent(event)
    let b_dispatch = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let event = args.first().cloned().unwrap_or(JsValue::undefined());
                let event_obj = event.as_object().ok_or_else(|| {
                    JsNativeError::typ().with_message("dispatchEvent: argument is not an object")
                })?;
                let event_type = event_obj
                    .get(js_string!("type"), ctx)?
                    .to_string(ctx)?
                    .to_std_string_escaped();
                if let Some(id) = find_worker_id_from_this(this, ctx) {
                    let callbacks =
                        bridge.with_worker_registry(|reg| reg.get_callbacks(id, &event_type));
                    for cb in callbacks {
                        let _ = cb.call(this, std::slice::from_ref(&event), ctx);
                    }
                }
                Ok(JsValue::from(true))
            },
            b_dispatch,
        ),
        js_string!("dispatchEvent"),
        1,
    );

    let obj = init.build();

    // Hidden __elidex_worker_obj__ marker for identity matching.
    // Worker ID will be stored after registration.
    obj.set(
        js_string!("__elidex_worker_marker__"),
        JsValue::from(true),
        false,
        ctx,
    )
    .expect("failed to set marker");

    // onmessage / onerror / onmessageerror as accessor properties.
    register_worker_event_handler_prop(&obj, &realm, bridge, ctx, "onmessage", "message");
    register_worker_event_handler_prop(&obj, &realm, bridge, ctx, "onerror", "error");
    register_worker_event_handler_prop(&obj, &realm, bridge, ctx, "onmessageerror", "messageerror");

    obj
}

/// Register an IDL event handler attribute on the Worker JS object.
fn register_worker_event_handler_prop(
    obj: &JsObject,
    realm: &boa_engine::realm::Realm,
    bridge: &HostBridge,
    ctx: &mut Context,
    prop_name: &str,
    event_type: &str,
) {
    let b_set = bridge.clone();
    let et_set = event_type.to_string();
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, (bridge, event_type): &(HostBridge, String), ctx| {
            let callback = args.first().and_then(JsValue::as_object);
            if let Some(id) = find_worker_id_from_this(this, ctx) {
                match event_type.as_str() {
                    "message" => bridge.with_worker_registry(|reg| reg.set_onmessage(id, callback)),
                    "error" => bridge.with_worker_registry(|reg| reg.set_onerror(id, callback)),
                    "messageerror" => {
                        bridge.with_worker_registry(|reg| reg.set_onmessageerror(id, callback));
                    }
                    _ => {}
                }
            }
            Ok(JsValue::undefined())
        },
        (b_set, et_set),
    );

    let b_get = bridge.clone();
    let et_get = event_type.to_string();
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, (bridge, event_type): &(HostBridge, String), ctx| {
            let handler = find_worker_id_from_this(this, ctx).and_then(|id| {
                bridge.with_worker_registry(|reg| reg.get_event_handler(id, event_type))
            });
            Ok(handler.map_or(JsValue::null(), JsValue::from))
        },
        (b_get, et_get),
    );

    let desc = boa_engine::property::PropertyDescriptor::builder()
        .set(setter.to_js_function(realm))
        .get(getter.to_js_function(realm))
        .enumerable(true)
        .configurable(true)
        .build();
    obj.define_property_or_throw(js_string!(prop_name), desc, ctx)
        .expect("failed to define worker event handler");
}

/// Find the worker registry ID from a `this` [`JsValue`].
///
/// Reads the hidden `__elidex_worker_id__` property stored on the Worker JS
/// object at construction time for O(1) lookup.
fn find_worker_id_from_this(this: &JsValue, ctx: &mut Context) -> Option<u64> {
    let this_obj = this.as_object()?;
    let id_val = this_obj.get(js_string!("__elidex_worker_id__"), ctx).ok()?;
    if id_val.is_undefined() {
        return None;
    }
    id_val.to_number(ctx).ok().and_then(|n| {
        if !n.is_finite() || n < 1.0 || n.fract() != 0.0 {
            return None;
        }
        Some(n as u64)
    })
}
