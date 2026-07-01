// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Realtime event delivery for `WebSocket` instances — UA-fire helpers
//! consumed by `vm/host/fetch_tick.rs::dispatch_realtime_event`.
//!
//! Split out of [`super::websocket`] (which holds the JS-API surface —
//! ctor, accessors, methods) so the dispatch paths don't bloat the
//! user-facing module.  All helpers take the receiver `instance`
//! `ObjectId` already resolved by the caller (via the
//! `ws_conn_to_object` reverse map) — borrow discipline lives there.
//!
//! ## Dispatch through the shared §2.9 VmObject core
//!
//! Since `#11-realtime-event-listeners` every event fires through the
//! shared seam ([`super::event_target_dispatch_vm::dispatch_vm_simple_event`]
//! for plain `open` / `error`, [`super::event_target_dispatch_vm::fire_vm_message_event`]
//! / [`super::event_target_dispatch_vm::fire_vm_close_event`] for
//! `message` / `close`).  The seam walks the unified
//! `vm_event_listeners` home (on* handlers + every `addEventListener`
//! listener) off one event object, with the GC-root / dispatch-flag /
//! `vm_path_has_listener` lazy-alloc discipline living there — not here.
//!
//! ## Side-table mutation ordering
//!
//! State transitions (`transition_to(Open)` / `transition_to(Closed)`)
//! and per-state field populates (`protocol`, `extensions`) run
//! BEFORE the event-fire so handlers observing
//! `ws.readyState === WebSocket.OPEN` from inside the `open` callback
//! see the post-transition value (matches Chrome / Firefox semantics).

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::host_data::BinaryType;
use super::super::value::{JsValue, NativeContext, ObjectId};
use super::event_target_dispatch_vm::{
    dispatch_vm_simple_event, fire_vm_close_event, fire_vm_message_event,
};

/// Handle `WsEvent::Connected { protocol, extensions }`.
///
/// Order per WebSockets Standard §4 (Feedback from the protocol),
/// "When the WebSocket connection is established"
/// (step 3 onwards): transition state → populate negotiated fields →
/// fire `Event("open")` via cached `onopen`.  Handlers observe the
/// post-transition state when they re-enter via `this.readyState`
/// / `this.protocol` / `this.extensions`.
pub(super) fn dispatch_ws_connected(
    ctx: &mut NativeContext<'_>,
    instance: ObjectId,
    protocol: String,
    extensions: String,
) {
    // ---- State mutation: transition + populate ----
    {
        let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.websocket_states.get_mut(&instance) else {
            return;
        };
        // Tolerate races: if a JS-side `close()` already moved the
        // state to CLOSING/CLOSED before the broker's `Connected`
        // arrived, drop the open dispatch (the natural `Closed`
        // event still fires after).
        if !matches!(state.ready_state, elidex_api_ws::WsReadyState::Connecting) {
            return;
        }
        let _ = state.transition_to(elidex_api_ws::WsReadyState::Open);
        state.protocol = protocol;
        state.extensions = extensions;
    }

    // Fire a plain `Event("open")` through the shared §2.9 VmObject core
    // — every `addEventListener("open")` listener + the `onopen`
    // handler.  The `vm_path_has_listener` gate inside makes an
    // unobserved fire allocation-free, so the old "no handler → return"
    // short-circuit is no longer needed.
    let open_sid = ctx.vm.well_known.ws_open_event_type;
    let _ = dispatch_vm_simple_event(ctx, instance, open_sid, false, false);
}

/// Handle `WsEvent::Closed { code, reason, was_clean }`.
///
/// Order per WebSockets Standard §4 (Feedback from the protocol),
/// "When the WebSocket connection is closed": transition state → fire
/// `CloseEvent("close", {code, reason, wasClean})` via cached
/// `onclose`.  Idempotent if the side-table already moved past
/// CLOSED (e.g. unbind / GC sweep raced).
pub(super) fn dispatch_ws_closed(
    ctx: &mut NativeContext<'_>,
    instance: ObjectId,
    code: u16,
    reason: &str,
    was_clean: bool,
) {
    {
        let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.websocket_states.get_mut(&instance) else {
            return;
        };
        if matches!(state.ready_state, elidex_api_ws::WsReadyState::Closed) {
            // Idempotent — already terminal.  No second close
            // event fires.
            return;
        }
        let _ = state.transition_to(elidex_api_ws::WsReadyState::Closed);
    }

    let close_sid = ctx.vm.well_known.ws_close_event_type;
    // `reason` passes as `&str` and is interned past the gate inside
    // `fire_vm_close_event` — an unobserved close never interns it.
    let _ = fire_vm_close_event(ctx, instance, close_sid, code, reason, was_clean);
}

/// Handle `WsEvent::TextMessage(s)`.
///
/// Fires `MessageEvent("message", { data: s, origin, lastEventId:
/// "", source: null, ports: [] })` through the cached `onmessage`
/// handler.  `origin` is the **WebSocket URL's** origin (per WebSockets
/// Standard §4, Feedback from the protocol), NOT the page origin — read from the side-table's
/// pre-interned `origin_sid` (computed once at ctor time, no
/// per-dispatch URL parse).  No state transition.
pub(super) fn dispatch_ws_text_message(ctx: &mut NativeContext<'_>, instance: ObjectId, s: &str) {
    let Some(origin_sid) = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&instance))
        .map(|st| st.origin_sid)
    else {
        return;
    };
    // `s` is interned inside the `build_data` thunk; the type (`"message"`)
    // and empty `lastEventId` pass as `&str` and intern past the gate, so an
    // unobserved socket interns nothing.
    let _ = fire_vm_message_event(
        ctx,
        instance,
        "message",
        |ctx| JsValue::String(ctx.vm.strings.intern(s)),
        origin_sid,
        "",
    );
}

/// Handle `WsEvent::BinaryMessage(bytes)`.
///
/// `data` is allocated as a `Blob` (with empty MIME per WebSockets
/// Standard §4, Feedback from the protocol: "type indicates that the
/// data is Binary and binary type is 'blob' … a new Blob object")
/// when `binaryType === "blob"`,
/// or as a fresh `ArrayBuffer` when `binaryType === "arraybuffer"`.  The
/// allocation is deferred into `fire_vm_message_event`'s `build_data`
/// thunk, which runs it only past the lazy-alloc gate — so a socket with
/// no `message` listener never builds the (potentially large) payload.
/// No state transition.
pub(super) fn dispatch_ws_binary_message(
    ctx: &mut NativeContext<'_>,
    instance: ObjectId,
    bytes: Vec<u8>,
) {
    let Some((origin_sid, binary_type)) = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&instance))
        .map(|st| (st.origin_sid, st.binary_type))
    else {
        return;
    };

    // The Blob / ArrayBuffer is built inside the `build_data` thunk, which
    // `fire_vm_message_event` runs only after its lazy-alloc gate passes — an
    // unread socket never pays for the (potentially large) payload build.
    let _ = fire_vm_message_event(
        ctx,
        instance,
        "message",
        |ctx| match binary_type {
            BinaryType::Blob => {
                let empty = ctx.vm.well_known.empty;
                JsValue::Object(super::blob::create_blob_from_bytes(
                    ctx.vm,
                    Arc::from(bytes),
                    empty,
                ))
            }
            BinaryType::ArrayBuffer => JsValue::Object(
                super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes),
            ),
        },
        origin_sid,
        "",
    );
}

/// Handle `WsEvent::Error(_)`.
///
/// Fires a plain `Event("error")` through the cached `onerror`
/// handler.  Per WebSockets Standard §4 (Feedback from the protocol)
/// the WebSocket `"error"` event is a
/// plain `Event`, NOT an `ErrorEvent` (no message / filename /
/// lineno).  Discarding the broker's error string here is correct
/// per spec — the script-visible surface is intentionally opaque to
/// avoid leaking server-internals.  No state transition (the
/// matching `WsEvent::Closed` follows and drives the
/// `transition_to(Closed)`).
pub(super) fn dispatch_ws_error(ctx: &mut NativeContext<'_>, instance: ObjectId) {
    let error_sid = ctx.vm.well_known.error;
    let _ = dispatch_vm_simple_event(ctx, instance, error_sid, false, false);
}

/// Handle `WsEvent::BytesSent(n)`.
///
/// Pure side-table mutation: decrement `bufferedAmount` by the
/// broker's reported bytes-flushed count.  Saturating arithmetic
/// guards against the broker over-reporting (e.g. due to internal
/// re-fragmentation accounting that diverges from the JS-visible
/// `saturating_add` in `send()`).  No event fires per WebSockets
/// Standard §3 (The WebSocket interface)
/// — `bufferedAmount` is a pull-only observable.
pub(super) fn dispatch_ws_bytes_sent(ctx: &mut NativeContext<'_>, instance: ObjectId, n: u64) {
    let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
        return;
    };
    if let Some(state) = hd.websocket_states.get_mut(&instance) {
        state.buffered_amount = state.buffered_amount.saturating_sub(n);
    }
}

// The bespoke `fire_message_event` / `fire_plain_event` /
// `alloc_ua_event` / `fire_handler` helpers (single-on*-handler direct
// fire) were removed by `#11-realtime-event-listeners`: WebSocket and
// EventSource now UA-fire through the shared §2.9 VmObject seam
// (`event_target_dispatch_vm::{dispatch_vm_simple_event,
// fire_vm_message_event, fire_vm_close_event}`), which walks the unified
// `vm_event_listeners` home (on* handlers + every addEventListener
// listener) off one event object.
