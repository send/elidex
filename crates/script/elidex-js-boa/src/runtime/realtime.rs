//! WebSocket / SSE event dispatch for the content thread frame loop.

use boa_engine::{js_string, Context, JsValue};

use elidex_net::sse::SseEvent;
use elidex_net::ws::WsEvent;

use crate::bridge::HostBridge;

/// Dispatch all pending WebSocket and SSE events.
///
/// Called from the content thread frame loop after draining events from
/// `bridge.drain_realtime_events()`. Invokes JS callbacks (`onopen`,
/// `onmessage`, `onerror`, `onclose`) and updates `readyState`.
pub(crate) fn dispatch_realtime_events(
    ws_events: Vec<(u64, WsEvent)>,
    sse_events: Vec<(u64, SseEvent)>,
    bridge: &HostBridge,
    ctx: &mut Context,
) {
    for (id, event) in ws_events {
        dispatch_ws_event(id, event, bridge, ctx);
    }
    for (id, event) in sse_events {
        dispatch_sse_event(id, event, bridge, ctx);
    }
}

fn dispatch_ws_event(id: u64, event: WsEvent, bridge: &HostBridge, ctx: &mut Context) {
    match event {
        WsEvent::Connected {
            protocol,
            extensions,
        } => {
            // Update readyState to OPEN and negotiated protocol/extensions.
            let callback = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.ws_callbacks(id) else {
                    return;
                };
                cb.ready_state.set(1); // OPEN
                *cb.protocol.borrow_mut() = protocol;
                *cb.extensions.borrow_mut() = extensions;
                cb.onopen.clone()
            };
            if let Some(func) = callback {
                let event_obj = create_simple_event("open", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
        }
        WsEvent::TextMessage(data) => {
            let (callback, origin) = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.ws_callbacks(id) else {
                    return;
                };
                (cb.onmessage.clone(), cb.origin.clone())
            };
            if let Some(func) = callback {
                let event_obj = create_message_event(&data, &origin, "", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
        }
        WsEvent::BinaryMessage(data) => {
            // Binary data represented as lossy UTF-8 string.
            // Full ArrayBuffer support deferred to M4-9 (TypedArray infrastructure).
            let text = String::from_utf8_lossy(&data);
            let (callback, origin) = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.ws_callbacks(id) else {
                    return;
                };
                (cb.onmessage.clone(), cb.origin.clone())
            };
            if let Some(func) = callback {
                let event_obj = create_message_event(&text, &origin, "", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
        }
        WsEvent::BufferedAmountUpdate(n) => {
            let inner = bridge.inner.borrow();
            if let Some(cb) = inner.realtime.ws_callbacks(id) {
                cb.buffered_amount.set(n);
            }
        }
        WsEvent::Error(msg) => {
            let callback = {
                let inner = bridge.inner.borrow();
                inner
                    .realtime
                    .ws_callbacks(id)
                    .and_then(|cb| cb.onerror.clone())
            };
            if let Some(func) = callback {
                let event_obj = create_simple_event("error", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
            eprintln!("[WebSocket] error: {msg}");
        }
        WsEvent::Closed {
            code,
            reason,
            was_clean,
        } => {
            // Update readyState to CLOSED and invoke onclose.
            let callback = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.ws_callbacks(id) else {
                    return;
                };
                cb.ready_state.set(3); // CLOSED
                cb.onclose.clone()
            };
            if let Some(func) = callback {
                let event_obj = create_close_event(code, &reason, was_clean, ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
            // Remove from registry.
            bridge.inner.borrow_mut().realtime.remove_ws(id);
        }
    }
}

fn dispatch_sse_event(id: u64, event: SseEvent, bridge: &HostBridge, ctx: &mut Context) {
    match event {
        SseEvent::Connected => {
            let callback = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.sse_callbacks(id) else {
                    return;
                };
                cb.ready_state.set(1); // OPEN
                cb.onopen.clone()
            };
            if let Some(func) = callback {
                let event_obj = create_simple_event("open", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
        }
        SseEvent::Event {
            event_type,
            data,
            last_event_id,
        } => {
            let (callback, origin) = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.sse_callbacks(id) else {
                    return;
                };
                let cb_fn = if event_type == "message" {
                    cb.onmessage.clone()
                } else {
                    // TODO: custom event type dispatch via addEventListener
                    // For now, fall back to onmessage for all event types.
                    cb.onmessage.clone()
                };
                (cb_fn, cb.origin.clone())
            };
            if let Some(func) = callback {
                let event_obj = create_message_event(&data, &origin, &last_event_id, ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
        }
        SseEvent::Error(msg) => {
            // Recoverable error — readyState back to CONNECTING (auto-reconnect).
            let callback = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.sse_callbacks(id) else {
                    return;
                };
                cb.ready_state.set(0); // CONNECTING
                cb.onerror.clone()
            };
            if let Some(func) = callback {
                let event_obj = create_simple_event("error", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
            eprintln!("[EventSource] recoverable error: {msg}");
        }
        SseEvent::FatalError(msg) => {
            // Fatal error — readyState to CLOSED, no reconnect.
            let callback = {
                let inner = bridge.inner.borrow();
                let Some(cb) = inner.realtime.sse_callbacks(id) else {
                    return;
                };
                cb.ready_state.set(2); // CLOSED
                cb.onerror.clone()
            };
            if let Some(func) = callback {
                let event_obj = create_simple_event("error", ctx);
                let _ = func.call(&JsValue::undefined(), &[event_obj], ctx);
                let _ = ctx.run_jobs();
            }
            // Remove from registry.
            bridge.inner.borrow_mut().realtime.remove_sse(id);
            eprintln!("[EventSource] fatal error: {msg}");
        }
    }
}

// ---------------------------------------------------------------------------
// Event object helpers
// ---------------------------------------------------------------------------

/// Create a simple Event object with just a `type` property.
fn create_simple_event(event_type: &str, ctx: &mut Context) -> JsValue {
    use boa_engine::object::ObjectInitializer;
    use boa_engine::property::Attribute;
    let ro = Attribute::READONLY | Attribute::ENUMERABLE;
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("type"),
            JsValue::from(js_string!(event_type)),
            ro,
        )
        .build();
    obj.into()
}

/// Create a `MessageEvent` object with `type`, `data`, `origin`, `lastEventId`, `source`, `ports`.
fn create_message_event(
    data: &str,
    origin: &str,
    last_event_id: &str,
    ctx: &mut Context,
) -> JsValue {
    use boa_engine::object::builtins::JsArray;
    use boa_engine::object::ObjectInitializer;
    use boa_engine::property::Attribute;
    let ro = Attribute::READONLY | Attribute::ENUMERABLE;
    let ports = JsArray::new(ctx);
    let obj = ObjectInitializer::new(ctx)
        .property(js_string!("type"), JsValue::from(js_string!("message")), ro)
        .property(js_string!("data"), JsValue::from(js_string!(data)), ro)
        .property(js_string!("origin"), JsValue::from(js_string!(origin)), ro)
        .property(
            js_string!("lastEventId"),
            JsValue::from(js_string!(last_event_id)),
            ro,
        )
        .property(js_string!("source"), JsValue::null(), ro)
        .property(js_string!("ports"), JsValue::from(ports), ro)
        .build();
    obj.into()
}

/// Create a `CloseEvent` object with `type`, `code`, `reason`, `wasClean`.
fn create_close_event(code: u16, reason: &str, was_clean: bool, ctx: &mut Context) -> JsValue {
    use boa_engine::object::ObjectInitializer;
    use boa_engine::property::Attribute;
    let ro = Attribute::READONLY | Attribute::ENUMERABLE;
    let obj = ObjectInitializer::new(ctx)
        .property(js_string!("type"), JsValue::from(js_string!("close")), ro)
        .property(js_string!("code"), JsValue::from(i32::from(code)), ro)
        .property(js_string!("reason"), JsValue::from(js_string!(reason)), ro)
        .property(js_string!("wasClean"), JsValue::from(was_clean), ro)
        .build();
    obj.into()
}
