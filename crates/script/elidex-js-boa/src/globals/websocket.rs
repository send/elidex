//! `WebSocket` constructor and prototype for boa.
//!
//! Implements the WHATWG WebSocket API (HTML §9.3) as a global constructor.

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

use super::{define_connection_id, extract_connection_id, set_readystate_constants};

/// Hidden property key storing the WebSocket connection ID.
const WS_ID_KEY: &str = "__elidex_ws_id__";

/// WebSocket readyState constants (WHATWG HTML §9.3).
const WS_READYSTATE_CONSTANTS: [(&str, i32); 4] = [
    ("CONNECTING", 0),
    ("OPEN", 1),
    ("CLOSING", 2),
    ("CLOSED", 3),
];

/// Extract the connection ID from a WebSocket JS object.
fn extract_ws_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    extract_connection_id(this, WS_ID_KEY, "WebSocket", ctx)
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
        set_readystate_constants(&ws_obj, &WS_READYSTATE_CONSTANTS, ctx);
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

    // 2b. SSRF check: convert ws/wss to http/https for validate_url.
    {
        let http_scheme = if url.scheme() == "wss" {
            "https"
        } else {
            "http"
        };
        let mut check_url = url.clone();
        check_url.set_scheme(http_scheme).ok();
        if let Err(e) = elidex_plugin::url_security::validate_url(&check_url) {
            return Err(JsNativeError::typ()
                .with_message(format!("WebSocket URL blocked: {e}"))
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
                    let state = bridge
                        .with_ws_callbacks(id, |cb| cb.ready_state.get())
                        .unwrap_or(3);
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
                    let val = bridge
                        .with_ws_callbacks(id, |cb| cb.protocol.borrow().clone())
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
                    let val = bridge
                        .with_ws_callbacks(id, |cb| cb.extensions.borrow().clone())
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
                    let val = bridge
                        .with_ws_callbacks(id, |cb| cb.buffered_amount.get())
                        .unwrap_or(0);
                    Ok(JsValue::from(val as f64))
                },
                b_ba,
            )
            .to_js_function(&realm),
        ),
        None,
        Attribute::CONFIGURABLE,
    );

    // binaryType getter/setter with validation per WHATWG §9.3.1.
    // Store the actual value in a hidden property.
    init.property(
        js_string!("__elidex_binary_type__"),
        js_string!("blob"),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );
    init.accessor(
        js_string!("binaryType"),
        Some(
            NativeFunction::from_copy_closure(|this, _args, ctx| -> JsResult<JsValue> {
                let obj = this
                    .as_object()
                    .ok_or_else(|| JsNativeError::typ().with_message("not a WebSocket object"))?;
                obj.get(js_string!("__elidex_binary_type__"), ctx)
            })
            .to_js_function(&realm),
        ),
        Some(
            NativeFunction::from_copy_closure(|this, args, ctx| -> JsResult<JsValue> {
                let value = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                // Only accept "blob" or "arraybuffer" per WHATWG §9.3.1
                if value != "blob" && value != "arraybuffer" {
                    return Ok(JsValue::undefined());
                }
                let obj = this
                    .as_object()
                    .ok_or_else(|| JsNativeError::typ().with_message("not a WebSocket object"))?;
                obj.set(
                    js_string!("__elidex_binary_type__"),
                    JsValue::from(js_string!(value)),
                    false,
                    ctx,
                )?;
                Ok(JsValue::undefined())
            })
            .to_js_function(&realm),
        ),
        Attribute::CONFIGURABLE,
    );

    // send(data)
    let b_send = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_ws_id(this, ctx)?;
                // Check readyState == OPEN (1)
                let state = bridge.with_ws_callbacks(id, |cb| cb.ready_state.get());
                match state {
                    Some(1) => {} // OPEN — send normally
                    Some(0) => {
                        // CONNECTING — throw InvalidStateError per WHATWG §9.3.1
                        return Err(JsNativeError::typ()
                            .with_message("InvalidStateError: WebSocket is in CONNECTING state")
                            .into());
                    }
                    _ => {
                        // CLOSING (2) or CLOSED (3) — silently return per WHATWG §9.3.1
                        return Ok(JsValue::undefined());
                    }
                }
                let data = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                // Calculate byte length before sending for bufferedAmount update.
                let byte_len = data.len() as u64;
                let _ = bridge.ws_send_text(id, data);
                // Synchronously increment bufferedAmount per WHATWG §9.3.1.
                // The I/O thread will send BufferedAmountUpdate to decrement
                // after transmission.
                bridge.with_ws_callbacks(id, |cb| {
                    let current = cb.buffered_amount.get();
                    cb.buffered_amount.set(current + byte_len);
                });
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

                // Early return if already CLOSING or CLOSED per WHATWG §9.3.1.
                let current_state = bridge
                    .with_ws_callbacks(id, |cb| cb.ready_state.get())
                    .unwrap_or(3);
                if current_state == 2 || current_state == 3 {
                    return Ok(JsValue::undefined());
                }

                // Parse and validate code.
                let code = if let Some(v) = args.first() {
                    if v.is_undefined() {
                        1000u16
                    } else {
                        let code_f64 = v.to_number(ctx)?;
                        if code_f64.fract() != 0.0 || !code_f64.is_finite() {
                            return Err(JsNativeError::typ()
                                .with_message(
                                    "InvalidAccessError: close code must be an integer",
                                )
                                .into());
                        }
                        let n = code_f64 as u16;
                        if n != 1000 && !(3000..=4999).contains(&n) {
                            return Err(JsNativeError::typ()
                                .with_message(format!(
                                    "InvalidAccessError: close code must be 1000 or in range 3000-4999, got {n}"
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
                bridge.with_ws_callbacks(id, |cb| {
                    cb.ready_state.set(2);
                });

                bridge.ws_close(id, code, reason);
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

    // addEventListener(type, listener)
    let b_add = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| -> JsResult<JsValue> {
                let id = extract_ws_id(this, ctx)?;
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
                bridge.with_ws_callbacks_mut(id, |cb| {
                    let listeners = cb.listener_registry.entry(event_type).or_default();
                    // Deduplicate by JsObject pointer equality.
                    let already = listeners.iter().any(|l| JsObject::equals(l, &listener));
                    if !already {
                        listeners.push(listener);
                    }
                });
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
                let id = extract_ws_id(this, ctx)?;
                let event_type = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let listener = args.get(1).and_then(JsValue::as_object);
                if let Some(ref listener) = listener {
                    bridge.with_ws_callbacks_mut(id, |cb| {
                        if let Some(listeners) = cb.listener_registry.get_mut(&event_type) {
                            listeners.retain(|l| !JsObject::equals(l, listener));
                        }
                    });
                }
                Ok(JsValue::undefined())
            },
            b_remove,
        ),
        js_string!("removeEventListener"),
        2,
    );

    let ws_obj = init.build();

    // Static constants on instance as well (per spec).
    set_readystate_constants(&ws_obj, &WS_READYSTATE_CONSTANTS, ctx);

    // 7. Open the WebSocket connection via the bridge.
    let origin_str = origin.serialize();
    let id = bridge
        .open_websocket(url, protocols, origin_str, ws_obj.clone())
        .map_err(|e| JsNativeError::typ().with_message(e))?;

    define_connection_id(&ws_obj, WS_ID_KEY, id, ctx);

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
                validate_protocol_string(&s)?;
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
    validate_protocol_string(&s)?;
    Ok(vec![s])
}

/// Validate a WebSocket sub-protocol string per RFC 6455 §4.1.
fn validate_protocol_string(s: &str) -> JsResult<()> {
    if s.is_empty() {
        return Err(JsNativeError::syntax()
            .with_message("WebSocket protocol must not be empty")
            .into());
    }
    if !s.chars().all(is_valid_protocol_char) {
        return Err(JsNativeError::syntax()
            .with_message("WebSocket protocol contains invalid characters")
            .into());
    }
    Ok(())
}

/// Check if a character is valid in a WebSocket sub-protocol name.
/// RFC 6455 §4.1: token characters (RFC 2616 §2.2) — ASCII printable
/// (0x21-0x7E) excluding separators.
fn is_valid_protocol_char(c: char) -> bool {
    let b = c as u32;
    (0x21..=0x7E).contains(&b)
        && !matches!(
            c,
            '"' | '('
                | ')'
                | ','
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '?'
                | '@'
                | '['
                | '\\'
                | ']'
                | '{'
                | '}'
        )
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
            let handler = cap
                .bridge
                .with_ws_callbacks(id, |cb| get_ws_handler(cb, cap.which).cloned());
            Ok(handler.flatten().map_or(JsValue::null(), JsValue::from))
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
            cap.bridge.with_ws_callbacks_mut(id, |cb| {
                set_ws_handler(cb, cap.which, handler);
            });
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

    #[test]
    fn parse_protocols_null_returns_empty() {
        let mut ctx = Context::default();
        let result = parse_protocols(Some(&JsValue::null()), &mut ctx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_protocols_undefined_returns_empty() {
        let mut ctx = Context::default();
        let result = parse_protocols(Some(&JsValue::undefined()), &mut ctx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_protocols_multiple_distinct() {
        let mut ctx = Context::default();
        let arr = JsArray::new(&mut ctx);
        arr.push(js_string!("graphql-ws"), &mut ctx).unwrap();
        arr.push(js_string!("graphql-transport-ws"), &mut ctx)
            .unwrap();
        arr.push(js_string!("mqtt"), &mut ctx).unwrap();
        let val = JsValue::from(arr);
        let result = parse_protocols(Some(&val), &mut ctx).unwrap();
        assert_eq!(result, vec!["graphql-ws", "graphql-transport-ws", "mqtt"]);
    }

    #[test]
    fn parse_protocols_empty_array() {
        let mut ctx = Context::default();
        let arr = JsArray::new(&mut ctx);
        let val = JsValue::from(arr);
        let result = parse_protocols(Some(&val), &mut ctx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_protocols_empty_string_rejected() {
        let mut ctx = Context::default();
        let val = JsValue::from(js_string!(""));
        let result = parse_protocols(Some(&val), &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn parse_protocols_invalid_chars_rejected() {
        let mut ctx = Context::default();
        // Space is not a valid token character
        let val = JsValue::from(js_string!("bad protocol"));
        let result = parse_protocols(Some(&val), &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn parse_protocols_separator_chars_rejected() {
        let mut ctx = Context::default();
        // Comma is a separator
        let val = JsValue::from(js_string!("a,b"));
        let result = parse_protocols(Some(&val), &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn parse_protocols_valid_chars_accepted() {
        let mut ctx = Context::default();
        let val = JsValue::from(js_string!("graphql-ws.v2"));
        let result = parse_protocols(Some(&val), &mut ctx).unwrap();
        assert_eq!(result, vec!["graphql-ws.v2"]);
    }
}
