//! `EventSource` constructor and prototype for boa.
//!
//! Implements the WHATWG `EventSource` API (HTML §9.2) as a global constructor.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the SSE connection ID.
const SSE_ID_KEY: &str = "__elidex_sse_id__";

/// Extract the connection ID from an `EventSource` JS object.
fn extract_sse_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("EventSource: not an EventSource object")
    })?;
    let id_val = obj.get(js_string!(SSE_ID_KEY), ctx)?;
    let id = id_val
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("EventSource: missing connection ID"))?;
    Ok(id as u64)
}

/// Which `onXXX` handler to get/set.
#[derive(Clone, Copy)]
enum SseHandler {
    Open,
    Message,
    Error,
}

impl_empty_trace!(SseHandler);

/// Captures for SSE event handler getters/setters.
#[derive(Clone)]
struct SseHandlerCaptures {
    bridge: HostBridge,
    which: SseHandler,
}

impl_empty_trace!(SseHandlerCaptures);

/// Register the `EventSource` constructor as a global.
pub fn register_event_source(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> { sse_constructor(args, bridge, ctx) },
        b,
    );

    ctx.register_global_builtin_callable(js_string!("EventSource"), 1, constructor)
        .expect("failed to register EventSource");

    // Set static constants on the EventSource constructor.
    let global = ctx.global_object();
    let es_val = global
        .get(js_string!("EventSource"), ctx)
        .expect("EventSource must exist");
    if let Some(es_obj) = es_val.as_object() {
        es_obj
            .set(js_string!("CONNECTING"), JsValue::from(0), false, ctx)
            .expect("set CONNECTING");
        es_obj
            .set(js_string!("OPEN"), JsValue::from(1), false, ctx)
            .expect("set OPEN");
        es_obj
            .set(js_string!("CLOSED"), JsValue::from(2), false, ctx)
            .expect("set CLOSED");
    }
}

/// `new EventSource(url, options?)` implementation.
#[allow(clippy::too_many_lines)]
fn sse_constructor(args: &[JsValue], bridge: &HostBridge, ctx: &mut Context) -> JsResult<JsValue> {
    // 1. Parse URL.
    let url_str = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| {
            JsNativeError::typ().with_message("EventSource: URL argument is required")
        })?;

    let url = url::Url::parse(&url_str).map_err(|e| {
        JsNativeError::syntax().with_message(format!("EventSource: invalid URL: {e}"))
    })?;

    // 2. Validate scheme (http or https only).
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(JsNativeError::syntax()
                .with_message(format!("EventSource: unsupported scheme: {scheme}"))
                .into());
        }
    }

    // 3. Parse options.
    let with_credentials = if let Some(opts) = args.get(1).and_then(JsValue::as_object) {
        let wc = opts.get(js_string!("withCredentials"), ctx)?;
        wc.to_boolean()
    } else {
        false
    };

    // 4. Build the JS object with properties.
    // Clone realm before ObjectInitializer to avoid borrow conflict with ctx.
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);

    // url (readonly)
    init.property(
        js_string!("url"),
        js_string!(url.as_str()),
        Attribute::READONLY,
    );

    // withCredentials (readonly)
    init.property(
        js_string!("withCredentials"),
        JsValue::from(with_credentials),
        Attribute::READONLY,
    );

    // readyState getter — reads from SseCallbacks via bridge.
    let b_rs = bridge.clone();
    init.accessor(
        js_string!("readyState"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let id = extract_sse_id(this, ctx)?;
                    let inner = bridge.inner.borrow();
                    let state = inner
                        .realtime
                        .sse_callbacks(id)
                        .map_or(2, |cb| cb.ready_state.get());
                    Ok(JsValue::from(f64::from(state)))
                },
                b_rs,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // close()
    let b_close = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_sse_id(this, ctx)?;

                // Set readyState to CLOSED (2).
                {
                    let inner = bridge.inner.borrow();
                    if let Some(cb) = inner.realtime.sse_callbacks(id) {
                        cb.ready_state.set(2);
                    }
                }

                bridge.inner.borrow().realtime.sse_close(id);
                Ok(JsValue::undefined())
            },
            b_close,
        ),
        js_string!("close"),
        0,
    );

    // Event handler accessors: onopen, onmessage, onerror
    register_sse_event_accessor(&mut init, bridge, &realm, "onopen", SseHandler::Open);
    register_sse_event_accessor(&mut init, bridge, &realm, "onmessage", SseHandler::Message);
    register_sse_event_accessor(&mut init, bridge, &realm, "onerror", SseHandler::Error);

    // Static constants on instance.
    init.property(
        js_string!("CONNECTING"),
        JsValue::from(0),
        Attribute::READONLY,
    );
    init.property(js_string!("OPEN"), JsValue::from(1), Attribute::READONLY);
    init.property(js_string!("CLOSED"), JsValue::from(2), Attribute::READONLY);

    let es_obj = init.build();

    // 5. Store hidden connection ID (placeholder).
    es_obj
        .set(js_string!(SSE_ID_KEY), JsValue::from(0.0_f64), false, ctx)
        .expect("set sse id");

    // 6. Open the SSE connection via the bridge.
    // cookie_jar: None for now (cross-origin credential handling is deferred).
    let id = bridge.inner.borrow_mut().realtime.open_event_source(
        url,
        with_credentials,
        None,
        es_obj.clone(),
    );

    // Update the hidden ID property with the real value.
    es_obj
        .set(js_string!(SSE_ID_KEY), JsValue::from(id as f64), false, ctx)
        .expect("set sse id");

    Ok(es_obj.into())
}

/// Get the `onXXX` callback reference from `SseCallbacks`.
fn get_sse_handler(
    cb: &crate::bridge::realtime::SseCallbacks,
    which: SseHandler,
) -> Option<&boa_engine::JsObject> {
    match which {
        SseHandler::Open => cb.onopen.as_ref(),
        SseHandler::Message => cb.onmessage.as_ref(),
        SseHandler::Error => cb.onerror.as_ref(),
    }
}

/// Set the `onXXX` callback on `SseCallbacks`.
fn set_sse_handler(
    cb: &mut crate::bridge::realtime::SseCallbacks,
    which: SseHandler,
    handler: Option<boa_engine::JsObject>,
) {
    match which {
        SseHandler::Open => cb.onopen = handler,
        SseHandler::Message => cb.onmessage = handler,
        SseHandler::Error => cb.onerror = handler,
    }
}

/// Register an `onXXX` event handler accessor (getter/setter) on the `EventSource` object.
fn register_sse_event_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    name: &str,
    which: SseHandler,
) {
    let getter_cap = SseHandlerCaptures {
        bridge: bridge.clone(),
        which,
    };
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, cap, ctx| -> JsResult<JsValue> {
            let id = extract_sse_id(this, ctx)?;
            let inner = cap.bridge.inner.borrow();
            let handler = inner
                .realtime
                .sse_callbacks(id)
                .and_then(|cb| get_sse_handler(cb, cap.which));
            Ok(handler.map_or(JsValue::null(), |h| JsValue::from(h.clone())))
        },
        getter_cap,
    )
    .to_js_function(realm);

    let setter_cap = SseHandlerCaptures {
        bridge: bridge.clone(),
        which,
    };
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, cap, ctx| -> JsResult<JsValue> {
            let id = extract_sse_id(this, ctx)?;
            let val = args.first().cloned().unwrap_or(JsValue::null());
            let handler = if val.is_null() || val.is_undefined() {
                None
            } else {
                Some(
                    val.as_object()
                        .ok_or_else(|| {
                            JsNativeError::typ()
                                .with_message("event handler must be a function or null")
                        })?
                        .clone(),
                )
            };
            let mut inner = cap.bridge.inner.borrow_mut();
            if let Some(cb) = inner.realtime.sse_callbacks_mut(id) {
                set_sse_handler(cb, cap.which, handler);
            }
            Ok(JsValue::undefined())
        },
        setter_cap,
    )
    .to_js_function(realm);

    init.accessor(
        js_string!(name),
        Some(getter),
        Some(setter),
        Attribute::CONFIGURABLE,
    );
}
