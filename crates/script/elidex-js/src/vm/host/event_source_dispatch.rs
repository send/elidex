// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Realtime event delivery for `EventSource` instances — UA-fire
//! helpers consumed by [`super::fetch_tick::VmInner::tick_network`]
//! through `dispatch_realtime_event`.
//!
//! Split out of [`super::event_source`] (which holds the JS-API
//! surface — ctor, accessors, methods, on* / addEventListener
//! setters) so the event-allocation + handler-call paths don't bloat
//! the user-facing module.  Mirrors the [`super::websocket_dispatch`]
//! split for the same reason.
//!
//! ## GC root invariant
//!
//! Each helper that allocates an Event object delegates to the
//! shared [`super::websocket_dispatch::fire_plain_event`] /
//! [`super::websocket_dispatch::fire_message_event_for_type`]
//! helpers, which `push_temp_root` the event before invoking the
//! user handler.  Per-listener fan-out (named events fired through
//! `addEventListener`) wraps each iteration in an explicit
//! `push_temp_root(handler_id)` so a GC triggered by a sibling
//! handler cannot reclaim a pending callback whose only retention
//! is the per-instance `event_listeners` registry.
//!
//! ## Side-table mutation ordering
//!
//! State transitions (`transition_to(Open)` / `transition_to(Closed)`
//! / `transition_to(Connecting)`) and `last_event_id` updates run
//! BEFORE the event-fire so handlers observing
//! `es.readyState === EventSource.OPEN` / reading
//! `e.lastEventId` from inside the `open` / `message` callback see
//! the post-mutation values (matches Chrome / Firefox semantics).
//!
//! ## Listener registry snapshot-then-iterate
//!
//! Per WHATWG DOM §2.7 EventTarget, the registered listener list
//! is snapshotted at dispatch time — listeners added during the
//! handler call do NOT receive this dispatch, and listeners removed
//! during the call DO still receive it if they were registered when
//! the snapshot was taken.  Implemented by cloning the
//! `Vec<ObjectId>` inside the borrow scope and dropping the borrow
//! before the fan-out loop, so the user handler may freely
//! `addEventListener` / `removeEventListener` without panicking on
//! a re-entrant `BorrowMutError`.

#![cfg(feature = "engine")]

use elidex_api_ws::SseReadyState;

use super::super::value::{JsValue, ObjectId, StringId};
use super::super::VmInner;
use super::websocket_dispatch::{fire_message_event, fire_plain_event};

/// Handle `SseEvent::Connected`.
///
/// Order per WHATWG HTML §9.2.5 step "announce the connection":
/// transition state → fire `Event("open")` via cached `onopen` and
/// any `addEventListener("open", ...)` listeners.  Per-state
/// matrix: `Connecting→Open` is the first-handshake announce,
/// `Open→Open` is the legitimate idempotent path after the
/// broker's auto-reconnect cycle (`SseEvent::Error` →
/// `Connecting`, then a fresh `SseEvent::Connected` snaps it
/// back to `Open`); only the terminal `Closed` state short-
/// circuits (JS-side `close()` raced ahead of the broker's
/// `Connected` reply).
pub(super) fn dispatch_sse_connected(vm: &mut VmInner, instance: ObjectId) {
    let (handler, listeners) = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        // Tolerate races: only Connecting / Open are valid source
        // states for the open event (Open → Open is idempotent
        // following an auto-reconnect that the VM observed; Closed
        // means the user already terminated).
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        let _ = state.transition_to(SseReadyState::Open);
        (state.onopen, listener_snapshot(state, "open"))
    };
    let type_sid = vm.well_known.ws_open_event_type;
    fan_out_plain_event(vm, instance, type_sid, handler, listeners);
}

/// Handle `SseEvent::Event { event_type, data, last_event_id }`.
///
/// Per WHATWG HTML §9.2.4 "dispatch the event": update sticky
/// `last_event_id` from the broker's cumulative value
/// ([`elidex_net::sse::SseParserState::take_event`] always emits the
/// current buffer per §9.2.6), then build a `MessageEvent` named
/// `event_type` with `data` / `origin` (ctor URL origin cached on
/// the side-table) / `lastEventId` (the sticky cumulative value).
/// Fan-out rule (§9.2.4):
/// - `event_type == "message"` → fire `onmessage` PLUS every
///   `addEventListener("message", ...)` listener.
/// - `event_type != "message"` → fire only the
///   `addEventListener(event_type, ...)` listeners.  `onmessage`
///   does NOT receive named events (it is the
///   `addEventListener("message", ...)` entry per WHATWG
///   EventHandler IDL §8.1.7.2).
pub(super) fn dispatch_sse_event(
    vm: &mut VmInner,
    instance: ObjectId,
    event_type: &str,
    data: &str,
    last_event_id: String,
) {
    // Common case (no `id:` line in the server stream) → skip the
    // intern round-trip + WTF-16 conversion for the empty value.
    let last_event_id_sid = if last_event_id.is_empty() {
        vm.well_known.empty
    } else {
        vm.strings.intern(&last_event_id)
    };
    // Pre-intern the event type so the `is this the default
    // "message" type?` branch is a `StringId` equality check
    // (cheap integer compare) rather than a UTF-8 byte walk.
    let type_sid = vm.strings.intern(event_type);
    let (onmessage, listeners, origin_sid) = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        // Broker emits the cumulative sticky value per WHATWG
        // §9.2.6 step 11 "Set the last event ID buffer ..." —
        // unconditional propagation matches both the server's
        // `id:\n` (empty value resets) and the multi-event
        // accumulator semantics.
        state.last_event_id = last_event_id;
        let target_onmessage = if type_sid == vm.well_known.message {
            state.onmessage
        } else {
            None
        };
        (
            target_onmessage,
            listener_snapshot(state, event_type),
            state.origin_sid,
        )
    };
    let data_sid = vm.strings.intern(data);
    let data_val = JsValue::String(data_sid);

    // Fan-out: onmessage first (when applicable), then every
    // addEventListener registration in insertion order.
    if let Some(handler_id) = onmessage {
        fire_message_event(
            vm,
            instance,
            handler_id,
            type_sid,
            data_val,
            origin_sid,
            last_event_id_sid,
        );
    }
    for handler_id in listeners {
        // Push a temp root on the listener id in case GC was
        // triggered by the previous handler invocation reclaiming
        // every other reference path to this callback (the
        // per-instance `event_listeners` registry is the only
        // retention for addEventListener'd handlers).
        let mut g = vm.push_temp_root(JsValue::Object(handler_id));
        fire_message_event(
            &mut g,
            instance,
            handler_id,
            type_sid,
            data_val,
            origin_sid,
            last_event_id_sid,
        );
        drop(g);
    }
}

/// Handle `SseEvent::Error(_)`.
///
/// Per WHATWG HTML §9.2.5 (handshake-time failure) and §9.2.6
/// (mid-stream "reestablish the connection" path) — a transient
/// network error transitions the readyState BACKWARDS to
/// `CONNECTING` because the broker is auto-reconnecting; the next
/// successful handshake will fire `SseEvent::Connected` and snap
/// it back to `Open` via [`dispatch_sse_connected`].  This is
/// intentional and spec-mandated; do NOT mistake it for a forward-
/// only state machine.  `FatalError` lives in the sibling helper
/// below.
///
/// The error string is discarded at the
/// [`super::fetch_tick`] arm (server-internals opacity, mirror of
/// WS's `WsEvent::Error(_)` discard); the event interface here is
/// a plain `Event("error")` per §9.2.5 (NOT an `ErrorEvent`).
pub(super) fn dispatch_sse_error(vm: &mut VmInner, instance: ObjectId) {
    let (handler, listeners) = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        // Terminal: if user JS already closed the EventSource,
        // suppress the error fire (matches CloseEvent dispatch
        // suppression on `dispatch_sse_connected` for the same
        // reason).
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        let _ = state.transition_to(SseReadyState::Connecting);
        (state.onerror, listener_snapshot(state, "error"))
    };
    let type_sid = vm.well_known.error;
    fan_out_plain_event(vm, instance, type_sid, handler, listeners);
}

/// Handle `SseEvent::FatalError(_)`.
///
/// Per WHATWG HTML §9.2.5 "fail the connection" — terminal: the
/// broker will NOT auto-reconnect (HTTP non-200, wrong Content-
/// Type, or `connect_sse_stream` returns
/// `SseConnectError::Fatal`).  Transitions readyState to `Closed`
/// and fires a plain `Event("error")`.  No CloseEvent is dispatched
/// per spec (SSE has no equivalent of WebSocket's CloseEvent —
/// `close` is purely a JS-initiated terminator).
pub(super) fn dispatch_sse_fatal_error(vm: &mut VmInner, instance: ObjectId) {
    let (handler, listeners) = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        let _ = state.transition_to(SseReadyState::Closed);
        (state.onerror, listener_snapshot(state, "error"))
    };
    let type_sid = vm.well_known.error;
    fan_out_plain_event(vm, instance, type_sid, handler, listeners);
}

/// Snapshot the `addEventListener`-registered listener IDs for
/// `event_type` from the per-instance registry.  Returns
/// `Vec::new()` when nothing is registered.  Caller MUST hold the
/// `host_data` borrow that owns `state`; the returned `Vec` is
/// independent of the registry and survives the borrow drop.
fn listener_snapshot(
    state: &super::super::host_data::EventSourceState,
    event_type: &str,
) -> Vec<ObjectId> {
    state
        .event_listeners
        .get(event_type)
        .cloned()
        .unwrap_or_default()
}

/// Fan-out a plain `Event(type_sid)` to the cached `on*` handler
/// (when set) and every snapshotted addEventListener listener.  GC
/// roots each listener-vec entry across its dispatch because the
/// per-instance `event_listeners` registry is the listener's only
/// retention path.
fn fan_out_plain_event(
    vm: &mut VmInner,
    instance: ObjectId,
    type_sid: StringId,
    on_handler: Option<ObjectId>,
    listeners: Vec<ObjectId>,
) {
    if let Some(handler_id) = on_handler {
        fire_plain_event(vm, instance, handler_id, type_sid);
    }
    for handler_id in listeners {
        let mut g = vm.push_temp_root(JsValue::Object(handler_id));
        fire_plain_event(&mut g, instance, handler_id, type_sid);
        drop(g);
    }
}
