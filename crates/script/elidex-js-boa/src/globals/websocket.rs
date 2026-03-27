//! `WebSocket` constructor and prototype for boa.
//!
//! Implements the WHATWG WebSocket API (HTML §9.3) as a global constructor.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the WebSocket connection ID.
const WS_ID_KEY: &str = "__elidex_ws_id__";

/// Extract the connection ID from a WebSocket JS object.
fn extract_ws_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("WebSocket: not a WebSocket object"))?;
    let id_val = obj.get(js_string!(WS_ID_KEY), ctx)?;
    let id = id_val
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("WebSocket: missing connection ID"))?;
    Ok(id as u64)
}

/// Which `onXXX` handler to get/set.
#[derive(Clone, Copy)]
enum WsHandler {
    Open,
    Message,
    Error,
    Close,
}

impl_empty_trace!(WsHandler);

/// Captures for WS event handler getters/setters.
#[derive(Clone)]
struct WsHandlerCaptures {
    bridge: HostBridge,
    which: WsHandler,
}

impl_empty_trace!(WsHandlerCaptures);

/// Register the `WebSocket` constructor as a global.
pub fn register_websocket(ctx: &mut Context, bridge: &HostBridge) {
    let b = bridge.clone();
    let constructor = NativeFunction::from_copy_closure_with_captures(
        |_this, args, bridge, ctx| -> JsResult<JsValue> { ws_constructor(args, bridge, ctx) },
        b,
    );

    ctx.register_global_builtin_callable(js_string!("WebSocket"), 1, constructor)
        .expect("failed to register WebSocket");

    // Set static constants on the WebSocket constructor.
    let global = ctx.global_object();
    let ws_val = global
        .get(js_string!("WebSocket"), ctx)
        .expect("WebSocket must exist");
    if let Some(ws_obj) = ws_val.as_object() {
        ws_obj
            .set(js_string!("CONNECTING"), JsValue::from(0), false, ctx)
            .expect("set CONNECTING");
        ws_obj
            .set(js_string!("OPEN"), JsValue::from(1), false, ctx)
            .expect("set OPEN");
        ws_obj
            .set(js_string!("CLOSING"), JsValue::from(2), false, ctx)
            .expect("set CLOSING");
        ws_obj
            .set(js_string!("CLOSED"), JsValue::from(3), false, ctx)
            .expect("set CLOSED");
    }
}

/// `new WebSocket(url, protocols?)` implementation.
#[allow(clippy::too_many_lines)]
fn ws_constructor(args: &[JsValue], bridge: &HostBridge, ctx: &mut Context) -> JsResult<JsValue> {
    // 1. Parse URL.
    let url_str = args
        .first()
        .map(|v| v.to_string(ctx))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| JsNativeError::typ().with_message("WebSocket: URL argument is required"))?;

    let url = url::Url::parse(&url_str).map_err(|e| {
        JsNativeError::syntax().with_message(format!("WebSocket: invalid URL: {e}"))
    })?;

    // 2. Validate scheme.
    match url.scheme() {
        "ws" | "wss" => {}
        scheme => {
            return Err(JsNativeError::syntax()
                .with_message(format!("WebSocket: unsupported scheme: {scheme}"))
                .into());
        }
    }

    // 3. Reject fragments.
    if url.fragment().is_some() {
        return Err(JsNativeError::syntax()
            .with_message("WebSocket: URL must not contain a fragment")
            .into());
    }

    // 4. Mixed content check: https origin + ws:// URL.
    let origin = bridge.origin();
    if let elidex_plugin::SecurityOrigin::Tuple { ref scheme, .. } = origin {
        if scheme == "https" && url.scheme() == "ws" {
            return Err(JsNativeError::error()
                .with_message(
                    "WebSocket: mixed content blocked (secure page cannot connect via ws://)",
                )
                .into());
        }
    }

    // 5. Parse protocols.
    let protocols = parse_protocols(args.get(1), ctx)?;

    // 6. Build the JS object with properties.
    // Clone realm before ObjectInitializer to avoid borrow conflict with ctx.
    let realm = ctx.realm().clone();
    let mut init = ObjectInitializer::new(ctx);

    // url (readonly)
    init.property(
        js_string!("url"),
        js_string!(url.as_str()),
        Attribute::READONLY,
    );

    // readyState getter — reads from WsCallbacks via bridge.
    let b_rs = bridge.clone();
    init.accessor(
        js_string!("readyState"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let id = extract_ws_id(this, ctx)?;
                    let inner = bridge.inner.borrow();
                    let state = inner
                        .realtime
                        .ws_callbacks(id)
                        .map_or(3, |cb| cb.ready_state.get());
                    Ok(JsValue::from(f64::from(state)))
                },
                b_rs,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // protocol getter
    let b_proto = bridge.clone();
    init.accessor(
        js_string!("protocol"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let id = extract_ws_id(this, ctx)?;
                    let inner = bridge.inner.borrow();
                    let val = inner
                        .realtime
                        .ws_callbacks(id)
                        .map(|cb| cb.protocol.borrow().clone())
                        .unwrap_or_default();
                    Ok(JsValue::from(js_string!(val)))
                },
                b_proto,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // extensions getter
    let b_ext = bridge.clone();
    init.accessor(
        js_string!("extensions"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let id = extract_ws_id(this, ctx)?;
                    let inner = bridge.inner.borrow();
                    let val = inner
                        .realtime
                        .ws_callbacks(id)
                        .map(|cb| cb.extensions.borrow().clone())
                        .unwrap_or_default();
                    Ok(JsValue::from(js_string!(val)))
                },
                b_ext,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // bufferedAmount getter
    let b_ba = bridge.clone();
    init.accessor(
        js_string!("bufferedAmount"),
        Some(
            NativeFunction::from_copy_closure_with_captures(
                |this, _args, bridge, ctx| -> JsResult<JsValue> {
                    let id = extract_ws_id(this, ctx)?;
                    let inner = bridge.inner.borrow();
                    let val = inner
                        .realtime
                        .ws_callbacks(id)
                        .map_or(0, |cb| cb.buffered_amount.get());
                    Ok(JsValue::from(val as f64))
                },
                b_ba,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // binaryType getter/setter (stored on the object itself)
    init.property(
        js_string!("binaryType"),
        js_string!("blob"),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // send(data)
    let b_send = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_ws_id(this, ctx)?;
                // Check readyState == OPEN (1)
                let state = {
                    let inner = bridge.inner.borrow();
                    inner
                        .realtime
                        .ws_callbacks(id)
                        .map(|cb| cb.ready_state.get())
                };
                match state {
                    Some(1) => {} // OPEN
                    _ => {
                        return Err(JsNativeError::error()
                            .with_message("WebSocket: send() called when readyState is not OPEN")
                            .into());
                    }
                }
                let data = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let _ = bridge.inner.borrow().realtime.ws_send_text(id, data);
                Ok(JsValue::undefined())
            },
            b_send,
        ),
        js_string!("send"),
        1,
    );

    // close(code?, reason?)
    let b_close = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_ws_id(this, ctx)?;

                // Parse and validate code.
                let code = if let Some(v) = args.first() {
                    if v.is_undefined() {
                        1000u16
                    } else {
                        let n = v.to_number(ctx)? as u16;
                        if n != 1000 && !(3000..=4999).contains(&n) {
                            return Err(JsNativeError::error()
                                .with_message(format!(
                                    "WebSocket: invalid close code {n} (must be 1000 or 3000-4999)"
                                ))
                                .into());
                        }
                        n
                    }
                } else {
                    1000
                };

                // Parse and validate reason.
                let reason = if let Some(v) = args.get(1) {
                    if v.is_undefined() {
                        String::new()
                    } else {
                        let s = v.to_string(ctx)?.to_std_string_escaped();
                        if s.len() > 123 {
                            return Err(JsNativeError::syntax()
                                .with_message("WebSocket: close reason must be <= 123 bytes UTF-8")
                                .into());
                        }
                        s
                    }
                } else {
                    String::new()
                };

                // Set readyState to CLOSING (2).
                {
                    let inner = bridge.inner.borrow();
                    if let Some(cb) = inner.realtime.ws_callbacks(id) {
                        cb.ready_state.set(2);
                    }
                }

                bridge.inner.borrow().realtime.ws_close(id, code, reason);
                Ok(JsValue::undefined())
            },
            b_close,
        ),
        js_string!("close"),
        0,
    );

    // Event handler accessors: onopen, onmessage, onerror, onclose
    register_ws_event_accessor(&mut init, bridge, &realm, "onopen", WsHandler::Open);
    register_ws_event_accessor(&mut init, bridge, &realm, "onmessage", WsHandler::Message);
    register_ws_event_accessor(&mut init, bridge, &realm, "onerror", WsHandler::Error);
    register_ws_event_accessor(&mut init, bridge, &realm, "onclose", WsHandler::Close);

    // Static constants on instance as well (per spec).
    init.property(
        js_string!("CONNECTING"),
        JsValue::from(0),
        Attribute::READONLY,
    );
    init.property(js_string!("OPEN"), JsValue::from(1), Attribute::READONLY);
    init.property(js_string!("CLOSING"), JsValue::from(2), Attribute::READONLY);
    init.property(js_string!("CLOSED"), JsValue::from(3), Attribute::READONLY);

    let ws_obj = init.build();

    // 7. Store hidden connection ID (placeholder).
    ws_obj
        .set(js_string!(WS_ID_KEY), JsValue::from(0.0_f64), false, ctx)
        .expect("set ws id");

    // 8. Open the WebSocket connection via the bridge.
    let origin_str = origin.serialize();
    let id = bridge.inner.borrow_mut().realtime.open_websocket(
        url,
        protocols,
        origin_str,
        ws_obj.clone(),
    );

    // Update the hidden ID property with the real value.
    ws_obj
        .set(js_string!(WS_ID_KEY), JsValue::from(id as f64), false, ctx)
        .expect("set ws id");

    Ok(ws_obj.into())
}

/// Parse `protocols` argument: string or array of strings.
/// Checks for duplicates (throws `SyntaxError`).
fn parse_protocols(arg: Option<&JsValue>, ctx: &mut Context) -> JsResult<Vec<String>> {
    let Some(val) = arg else {
        return Ok(Vec::new());
    };
    if val.is_undefined() || val.is_null() {
        return Ok(Vec::new());
    }

    // Check if it's an array.
    if let Some(obj) = val.as_object() {
        let is_array = obj.is_array();
        if is_array {
            let arr = JsArray::from_object(obj.clone())
                .map_err(|_| JsNativeError::typ().with_message("WebSocket: invalid protocols"))?;
            let len = arr.length(ctx)?;
            let mut protocols = Vec::with_capacity(len as usize);
            for i in 0..len {
                let elem = arr.get(i, ctx)?;
                let s = elem.to_string(ctx)?.to_std_string_escaped();
                protocols.push(s);
            }
            // Check duplicates.
            let mut seen = std::collections::HashSet::new();
            for p in &protocols {
                if !seen.insert(p.as_str()) {
                    return Err(JsNativeError::syntax()
                        .with_message(format!("WebSocket: duplicate sub-protocol: {p}"))
                        .into());
                }
            }
            return Ok(protocols);
        }
    }

    // Single string protocol.
    let s = val.to_string(ctx)?.to_std_string_escaped();
    Ok(vec![s])
}

/// Get the `onXXX` callback reference from `WsCallbacks`.
fn get_ws_handler(
    cb: &crate::bridge::realtime::WsCallbacks,
    which: WsHandler,
) -> Option<&boa_engine::JsObject> {
    match which {
        WsHandler::Open => cb.onopen.as_ref(),
        WsHandler::Message => cb.onmessage.as_ref(),
        WsHandler::Error => cb.onerror.as_ref(),
        WsHandler::Close => cb.onclose.as_ref(),
    }
}

/// Set the `onXXX` callback on `WsCallbacks`.
fn set_ws_handler(
    cb: &mut crate::bridge::realtime::WsCallbacks,
    which: WsHandler,
    handler: Option<boa_engine::JsObject>,
) {
    match which {
        WsHandler::Open => cb.onopen = handler,
        WsHandler::Message => cb.onmessage = handler,
        WsHandler::Error => cb.onerror = handler,
        WsHandler::Close => cb.onclose = handler,
    }
}

/// Register an `onXXX` event handler accessor (getter/setter) on the WebSocket object.
fn register_ws_event_accessor(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    name: &str,
    which: WsHandler,
) {
    let getter_cap = WsHandlerCaptures {
        bridge: bridge.clone(),
        which,
    };
    let getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, cap, ctx| -> JsResult<JsValue> {
            let id = extract_ws_id(this, ctx)?;
            let inner = cap.bridge.inner.borrow();
            let handler = inner
                .realtime
                .ws_callbacks(id)
                .and_then(|cb| get_ws_handler(cb, cap.which));
            Ok(handler.map_or(JsValue::null(), |h| JsValue::from(h.clone())))
        },
        getter_cap,
    )
    .to_js_function(realm);

    let setter_cap = WsHandlerCaptures {
        bridge: bridge.clone(),
        which,
    };
    let setter = NativeFunction::from_copy_closure_with_captures(
        |this, args, cap, ctx| -> JsResult<JsValue> {
            let id = extract_ws_id(this, ctx)?;
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
            if let Some(cb) = inner.realtime.ws_callbacks_mut(id) {
                set_ws_handler(cb, cap.which, handler);
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
    fn parse_protocols_empty() {
        let mut ctx = Context::default();
        let result = parse_protocols(None, &mut ctx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_protocols_single_string() {
        let mut ctx = Context::default();
        let val = JsValue::from(js_string!("chat"));
        let result = parse_protocols(Some(&val), &mut ctx).unwrap();
        assert_eq!(result, vec!["chat"]);
    }

    #[test]
    fn parse_protocols_array() {
        let mut ctx = Context::default();
        let arr = JsArray::new(&mut ctx);
        arr.push(js_string!("chat"), &mut ctx).unwrap();
        arr.push(js_string!("superchat"), &mut ctx).unwrap();
        let val = JsValue::from(arr);
        let result = parse_protocols(Some(&val), &mut ctx).unwrap();
        assert_eq!(result, vec!["chat", "superchat"]);
    }

    #[test]
    fn parse_protocols_duplicate_rejected() {
        let mut ctx = Context::default();
        let arr = JsArray::new(&mut ctx);
        arr.push(js_string!("chat"), &mut ctx).unwrap();
        arr.push(js_string!("chat"), &mut ctx).unwrap();
        let val = JsValue::from(arr);
        let result = parse_protocols(Some(&val), &mut ctx);
        assert!(result.is_err());
    }
}
