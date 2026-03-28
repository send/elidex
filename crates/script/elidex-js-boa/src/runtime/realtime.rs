//! WebSocket / SSE event dispatch for the content thread frame loop.

use boa_engine::{Context, JsObject, JsValue};

use elidex_net::sse::SseEvent;
use elidex_net::ws::WsEvent;
use elidex_plugin::{CloseEventInit, EventPayload};

use crate::bridge::HostBridge;
use crate::globals::events::create_standalone_event;

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

/// Invoke the `onXXX` callback (if set) and all `addEventListener` listeners
/// for the given event object.
///
/// `this_value` is the object used as `this` when calling the callback (e.g. the
/// `WebSocket` or `EventSource` JS object), matching browser behavior where
/// `this === ws` inside `ws.onmessage`.
fn invoke_callback_and_listeners(
    callback: Option<JsObject>,
    listeners: &[JsObject],
    event_obj: &JsValue,
    this_value: &JsValue,
    ctx: &mut Context,
) {
    if let Some(func) = callback {
        if let Err(err) = func.call(this_value, std::slice::from_ref(event_obj), ctx) {
            eprintln!("[JS Event Error] {err}");
        }
        // Microtask checkpoint after each handler (HTML §8.1.7.3).
        if let Err(err) = ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }
    for listener in listeners {
        if let Err(err) = listener.call(this_value, std::slice::from_ref(event_obj), ctx) {
            eprintln!("[JS Event Error] {err}");
        }
        if let Err(err) = ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }
}

fn dispatch_ws_event(id: u64, event: WsEvent, bridge: &HostBridge, ctx: &mut Context) {
    match event {
        WsEvent::Connected {
            protocol,
            extensions,
        } => dispatch_ws_connected(id, protocol, extensions, bridge, ctx),
        WsEvent::TextMessage(data) => dispatch_ws_text(id, &data, bridge, ctx),
        WsEvent::BinaryMessage(data) => dispatch_ws_binary(id, &data, bridge, ctx),
        WsEvent::BytesSent(n) => {
            bridge.with_ws_callbacks(id, |cb| {
                cb.buffered_amount
                    .set(cb.buffered_amount.get().saturating_sub(n));
            });
        }
        WsEvent::Error(msg) => dispatch_ws_error(id, &msg, bridge, ctx),
        WsEvent::Closed {
            code,
            reason,
            was_clean,
        } => dispatch_ws_closed(id, code, &reason, was_clean, bridge, ctx),
    }
}

fn dispatch_ws_connected(
    id: u64,
    protocol: String,
    extensions: String,
    bridge: &HostBridge,
    ctx: &mut Context,
) {
    let Some((callback, listeners, js_object)) = bridge
        .with_ws_callbacks(id, |cb| {
            // Guard: CONNECTING(0) → OPEN(1) is the only valid transition here.
            if cb.ready_state.get() != 0 {
                return None;
            }
            cb.ready_state.set(1); // OPEN
            *cb.protocol.borrow_mut() = protocol;
            *cb.extensions.borrow_mut() = extensions;
            Some((
                cb.onopen.clone(),
                cb.listener_registry
                    .get("open")
                    .cloned()
                    .unwrap_or_default(),
                cb.js_object.clone(),
            ))
        })
        .flatten()
    else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj =
        create_standalone_event("open", &EventPayload::None, false, Some(&this_val), ctx);
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
}

fn dispatch_ws_text(id: u64, data: &str, bridge: &HostBridge, ctx: &mut Context) {
    let Some((callback, origin, listeners, js_object)) = bridge.with_ws_callbacks(id, |cb| {
        (
            cb.onmessage.clone(),
            cb.origin.clone(),
            cb.listener_registry
                .get("message")
                .cloned()
                .unwrap_or_default(),
            cb.js_object.clone(),
        )
    }) else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj = create_standalone_event(
        "message",
        &EventPayload::Message {
            data: data.to_string(),
            origin: origin.clone(),
            last_event_id: String::new(),
        },
        false,
        Some(&this_val),
        ctx,
    );
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
}

fn dispatch_ws_binary(id: u64, data: &[u8], bridge: &HostBridge, ctx: &mut Context) {
    // Binary data encoded as base64 string.
    // Full ArrayBuffer support deferred to M4-9 (TypedArray infrastructure).
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let Some((callback, origin, listeners, js_object)) = bridge.with_ws_callbacks(id, |cb| {
        (
            cb.onmessage.clone(),
            cb.origin.clone(),
            cb.listener_registry
                .get("message")
                .cloned()
                .unwrap_or_default(),
            cb.js_object.clone(),
        )
    }) else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj = create_standalone_event(
        "message",
        &EventPayload::Message {
            data: encoded,
            origin: origin.clone(),
            last_event_id: String::new(),
        },
        false,
        Some(&this_val),
        ctx,
    );
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
}

fn dispatch_ws_error(id: u64, msg: &str, bridge: &HostBridge, ctx: &mut Context) {
    let Some((callback, listeners, js_object)) = bridge.with_ws_callbacks(id, |cb| {
        (
            cb.onerror.clone(),
            cb.listener_registry
                .get("error")
                .cloned()
                .unwrap_or_default(),
            cb.js_object.clone(),
        )
    }) else {
        eprintln!("[WebSocket] error: {msg}");
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj =
        create_standalone_event("error", &EventPayload::None, false, Some(&this_val), ctx);
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
    eprintln!("[WebSocket] error: {msg}");
}

fn dispatch_ws_closed(
    id: u64,
    code: u16,
    reason: &str,
    was_clean: bool,
    bridge: &HostBridge,
    ctx: &mut Context,
) {
    let Some((callback, listeners, js_object)) = bridge
        .with_ws_callbacks(id, |cb| {
            // Guard: only OPEN(1) or CLOSING(2) can transition to CLOSED(3).
            let state = cb.ready_state.get();
            if state == 3 {
                return None; // Already CLOSED.
            }
            cb.ready_state.set(3); // CLOSED
            Some((
                cb.onclose.clone(),
                cb.listener_registry
                    .get("close")
                    .cloned()
                    .unwrap_or_default(),
                cb.js_object.clone(),
            ))
        })
        .flatten()
    else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj = create_standalone_event(
        "close",
        &EventPayload::CloseEvent(CloseEventInit {
            code,
            reason: reason.to_string(),
            was_clean,
        }),
        false,
        Some(&this_val),
        ctx,
    );
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
    // Remove from registry.
    bridge.remove_ws(id);
}

fn dispatch_sse_event(id: u64, event: SseEvent, bridge: &HostBridge, ctx: &mut Context) {
    match event {
        SseEvent::Connected => dispatch_sse_connected(id, bridge, ctx),
        SseEvent::Event {
            event_type,
            data,
            last_event_id,
        } => dispatch_sse_message(id, &event_type, &data, &last_event_id, bridge, ctx),
        SseEvent::Error(msg) => dispatch_sse_error(id, &msg, bridge, ctx),
        SseEvent::FatalError(msg) => dispatch_sse_fatal(id, &msg, bridge, ctx),
    }
}

fn dispatch_sse_connected(id: u64, bridge: &HostBridge, ctx: &mut Context) {
    let Some((callback, listeners, js_object)) = bridge
        .with_sse_callbacks(id, |cb| {
            // Guard: CONNECTING(0) → OPEN(1).
            if cb.ready_state.get() != 0 {
                return None;
            }
            cb.ready_state.set(1); // OPEN
            Some((
                cb.onopen.clone(),
                cb.listener_registry
                    .get("open")
                    .cloned()
                    .unwrap_or_default(),
                cb.js_object.clone(),
            ))
        })
        .flatten()
    else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj =
        create_standalone_event("open", &EventPayload::None, false, Some(&this_val), ctx);
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
}

fn dispatch_sse_message(
    id: u64,
    event_type: &str,
    data: &str,
    last_event_id: &str,
    bridge: &HostBridge,
    ctx: &mut Context,
) {
    let Some((callback, origin, listeners, js_object)) = bridge.with_sse_callbacks(id, |cb| {
        // For "message" events: dispatch to onmessage + "message" listeners.
        // For custom event types: dispatch only to addEventListener listeners
        // for that type (per WHATWG HTML §9.2 — onmessage is NOT called for
        // custom event types).
        let cb_fn = if event_type == "message" {
            cb.onmessage.clone()
        } else {
            None
        };
        let type_listeners = cb
            .listener_registry
            .get(event_type)
            .cloned()
            .unwrap_or_default();
        (
            cb_fn,
            cb.origin.clone(),
            type_listeners,
            cb.js_object.clone(),
        )
    }) else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj = create_standalone_event(
        event_type,
        &EventPayload::Message {
            data: data.to_string(),
            origin: origin.clone(),
            last_event_id: last_event_id.to_string(),
        },
        false,
        Some(&this_val),
        ctx,
    );
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
}

fn dispatch_sse_error(id: u64, msg: &str, bridge: &HostBridge, ctx: &mut Context) {
    // Recoverable error — readyState back to CONNECTING (auto-reconnect).
    let Some((callback, listeners, js_object)) = bridge.with_sse_callbacks(id, |cb| {
        cb.ready_state.set(0); // CONNECTING
        (
            cb.onerror.clone(),
            cb.listener_registry
                .get("error")
                .cloned()
                .unwrap_or_default(),
            cb.js_object.clone(),
        )
    }) else {
        eprintln!("[EventSource] recoverable error: {msg}");
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj =
        create_standalone_event("error", &EventPayload::None, false, Some(&this_val), ctx);
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
    eprintln!("[EventSource] recoverable error: {msg}");
}

fn dispatch_sse_fatal(id: u64, msg: &str, bridge: &HostBridge, ctx: &mut Context) {
    // Fatal error — readyState to CLOSED, no reconnect.
    let Some((callback, listeners, js_object)) = bridge
        .with_sse_callbacks(id, |cb| {
            // Guard: don't transition if already CLOSED.
            if cb.ready_state.get() == 2 {
                return None;
            }
            cb.ready_state.set(2); // CLOSED
            Some((
                cb.onerror.clone(),
                cb.listener_registry
                    .get("error")
                    .cloned()
                    .unwrap_or_default(),
                cb.js_object.clone(),
            ))
        })
        .flatten()
    else {
        return;
    };
    let this_val = JsValue::from(js_object);
    let event_obj =
        create_standalone_event("error", &EventPayload::None, false, Some(&this_val), ctx);
    invoke_callback_and_listeners(callback, &listeners, &event_obj, &this_val, ctx);
    // Close and remove from registry.
    bridge.sse_close(id);
    eprintln!("[EventSource] fatal error: {msg}");
}
