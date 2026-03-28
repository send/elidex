//! `EventSource` constructor and prototype for boa.
//!
//! Implements the WHATWG `EventSource` API (HTML §9.2) as a global constructor.

use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::{Attribute, PropertyDescriptorBuilder};
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use elidex_net::FetchHandle;

use crate::bridge::HostBridge;

use super::{extract_connection_id, set_readystate_constants};

/// Hidden property key storing the SSE connection ID.
const SSE_ID_KEY: &str = "__elidex_sse_id__";

/// `EventSource` readyState constants (WHATWG HTML §9.2).
const SSE_READYSTATE_CONSTANTS: [(&str, i32); 3] = [("CONNECTING", 0), ("OPEN", 1), ("CLOSED", 2)];

/// Extract the connection ID from an `EventSource` JS object.
fn extract_sse_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    extract_connection_id(this, SSE_ID_KEY, "EventSource", ctx)
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

/// Captures for the `EventSource` constructor closure.
#[derive(Clone)]
struct SseConstructorCaptures {
    bridge: HostBridge,
    fetch_handle: Option<Rc<FetchHandle>>,
}

impl_empty_trace!(SseConstructorCaptures);

/// Register the `EventSource` constructor as a global.
pub fn register_event_source(
    ctx: &mut Context,
    bridge: &HostBridge,
    fetch_handle: Option<Rc<FetchHandle>>,
) {
    let cap = SseConstructorCaptures {
        bridge: bridge.clone(),
        fetch_handle,
    };
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, cap, ctx| -> JsResult<JsValue> {
            sse_constructor(args, &cap.bridge, cap.fetch_handle.as_ref(), ctx)
        },
        cap,
    );

    ctx.register_global_builtin_callable(js_string!("EventSource"), 1, constructor)
        .expect("failed to register EventSource");

    // Set static constants on the EventSource constructor.
    let global = ctx.global_object();
    let es_val = global
        .get(js_string!("EventSource"), ctx)
        .expect("EventSource must exist");
    if let Some(es_obj) = es_val.as_object() {
        set_readystate_constants(&es_obj, &SSE_READYSTATE_CONSTANTS, ctx);
    }
}

/// `new EventSource(url, options?)` implementation.
#[allow(clippy::too_many_lines)]
fn sse_constructor(
    args: &[JsValue],
    bridge: &HostBridge,
    fetch_handle: Option<&Rc<FetchHandle>>,
    ctx: &mut Context,
) -> JsResult<JsValue> {
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

    // 2b. SSRF check.
    if let Err(e) = elidex_plugin::url_security::validate_url(&url) {
        return Err(JsNativeError::typ()
            .with_message(format!("EventSource URL blocked: {e}"))
            .into());
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

    // addEventListener(type, listener)
    let b_add = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_sse_id(this, ctx)?;
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let listener = args.get(1).and_then(JsValue::as_object).ok_or_else(|| {
                    JsNativeError::typ()
                        .with_message("addEventListener: listener must be a function")
                })?;
                let mut inner = bridge.inner.borrow_mut();
                if let Some(cb) = inner.realtime.sse_callbacks_mut(id) {
                    let listeners = cb.listener_registry.entry(event_type).or_default();
                    // Deduplicate by JsObject pointer equality.
                    let already = listeners.iter().any(|l| JsObject::equals(l, &listener));
                    if !already {
                        listeners.push(listener);
                    }
                }
                Ok(JsValue::undefined())
            },
            b_add,
        ),
        js_string!("addEventListener"),
        2,
    );

    // removeEventListener(type, listener)
    let b_remove = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_sse_id(this, ctx)?;
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let listener = args.get(1).and_then(JsValue::as_object);
                if let Some(ref listener) = listener {
                    let mut inner = bridge.inner.borrow_mut();
                    if let Some(cb) = inner.realtime.sse_callbacks_mut(id) {
                        if let Some(listeners) = cb.listener_registry.get_mut(&event_type) {
                            listeners.retain(|l| !JsObject::equals(l, listener));
                        }
                    }
                }
                Ok(JsValue::undefined())
            },
            b_remove,
        ),
        js_string!("removeEventListener"),
        2,
    );

    let es_obj = init.build();

    // Static constants on instance.
    set_readystate_constants(&es_obj, &SSE_READYSTATE_CONSTANTS, ctx);

    // 5. Ensure the shared cookie jar is set for withCredentials support.
    if with_credentials {
        if let Some(fh) = fetch_handle {
            bridge
                .inner
                .borrow_mut()
                .realtime
                .set_cookie_jar(Some(fh.cookie_jar_arc()));
        }
    }

    // 6. Open the SSE connection via the bridge.
    // Pass the document origin for CORS validation on the SSE response.
    let doc_origin = Some(bridge.origin().serialize());
    let id = bridge
        .inner
        .borrow_mut()
        .realtime
        .open_event_source(url, with_credentials, doc_origin, es_obj.clone())
        .map_err(|e| JsNativeError::typ().with_message(e))?;

    // Store hidden connection ID as non-configurable, non-writable (readonly).
    let _ = es_obj.define_property_or_throw(
        js_string!(SSE_ID_KEY),
        PropertyDescriptorBuilder::new()
            .value(JsValue::from(id as f64))
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build(),
        ctx,
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_readystate_constants_correct() {
        // Verify the constant array matches WHATWG HTML §9.2 values.
        assert_eq!(SSE_READYSTATE_CONSTANTS.len(), 3);
        assert_eq!(SSE_READYSTATE_CONSTANTS[0], ("CONNECTING", 0));
        assert_eq!(SSE_READYSTATE_CONSTANTS[1], ("OPEN", 1));
        assert_eq!(SSE_READYSTATE_CONSTANTS[2], ("CLOSED", 2));
    }

    #[test]
    fn sse_url_scheme_validation() {
        // http and https are valid EventSource schemes; ws/ftp are not.
        let http = url::Url::parse("http://example.com/stream").unwrap();
        assert!(http.scheme() == "http" || http.scheme() == "https");

        let ws = url::Url::parse("ws://example.com/stream").unwrap();
        assert!(ws.scheme() != "http" && ws.scheme() != "https");
    }
}
