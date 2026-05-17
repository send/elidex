// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Realtime event delivery for `WebSocket` instances — UA-fire helpers
//! consumed by `vm/host/fetch_tick.rs::dispatch_realtime_event`.
//!
//! Split out of [`super::websocket`] (which holds the JS-API surface —
//! ctor, accessors, methods, on* setters) so the event-allocation +
//! handler-call paths don't bloat the user-facing module.  All helpers
//! take the receiver `instance` `ObjectId` already resolved by the
//! caller (via the `ws_conn_to_object` reverse map) — borrow discipline
//! lives there, not here.
//!
//! ## GC root invariant
//!
//! Every helper that allocates an Event object pushes it onto the
//! VM temp-root stack with [`super::super::VmInner::push_temp_root`]
//! BEFORE invoking the user handler.  Without this, a GC triggered
//! inside the handler call (or by any allocation it transitively
//! performs) could reclaim the event object, leaving the user-visible
//! `e.target` / `e.code` / etc. dangling.
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
use super::super::shape::{self, ShapeId};
use super::super::value::{
    JsValue, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue, StringId,
};
use super::super::VmInner;
use super::events::{set_event_slot_raw, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_TARGET};

/// Handle `WsEvent::Connected { protocol, extensions }`.
///
/// Order per WHATWG §9.3 "the WebSocket connection is established"
/// (step 3 onwards): transition state → populate negotiated fields →
/// fire `Event("open")` via cached `onopen`.  Handlers observe the
/// post-transition state when they re-enter via `this.readyState`
/// / `this.protocol` / `this.extensions`.
pub(super) fn dispatch_ws_connected(
    vm: &mut VmInner,
    instance: ObjectId,
    protocol: String,
    extensions: String,
) {
    // ---- State mutation: transition + populate ----
    let handler = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.websocket_states.get_mut(&instance) else {
            return;
        };
        // Tolerate races: if a JS-side `close()` already moved the
        // state to CLOSING/CLOSED before the broker's `Connected`
        // arrived, drop the open dispatch (the natural `Closed`
        // event still fires after).  `transition_to` short-circuits
        // illegal moves rather than panicking, so the assertion
        // wouldn't fire — but the open event shouldn't dispatch
        // either.
        if !matches!(state.ready_state, elidex_api_ws::WsReadyState::Connecting) {
            return;
        }
        let _ = state.transition_to(elidex_api_ws::WsReadyState::Open);
        state.protocol = protocol;
        state.extensions = extensions;
        state.onopen
    };

    let Some(handler_id) = handler else {
        // No `onopen` handler registered — Phase 1 has no
        // `addEventListener` surface for WebSocket (deferred per
        // `#11-realtime-event-listeners`), so silent-drop is the
        // correct behaviour.  The state mutation above still ran so
        // synchronous `readyState` polls observe the new state.
        return;
    };

    fire_plain_event(vm, instance, handler_id, vm.well_known.ws_open_event_type);
}

/// Handle `WsEvent::Closed { code, reason, was_clean }`.
///
/// Order per WHATWG §9.3 close algorithm: transition state → fire
/// `CloseEvent("close", {code, reason, wasClean})` via cached
/// `onclose`.  Idempotent if the side-table already moved past
/// CLOSED (e.g. unbind / GC sweep raced).
pub(super) fn dispatch_ws_closed(
    vm: &mut VmInner,
    instance: ObjectId,
    code: u16,
    reason: &str,
    was_clean: bool,
) {
    let handler = {
        let Some(hd) = vm.host_data.as_deref_mut() else {
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
        state.onclose
    };

    let Some(handler_id) = handler else {
        return;
    };

    fire_close_event(vm, instance, handler_id, code, reason, was_clean);
}

/// Handle `WsEvent::TextMessage(s)`.
///
/// Fires `MessageEvent("message", { data: s, origin, lastEventId:
/// "", source: null, ports: [] })` through the cached `onmessage`
/// handler.  `origin` is the **WebSocket URL's** origin (per WHATWG
/// §9.3.7), NOT the page origin — read from the side-table's
/// pre-interned `origin_sid` (computed once at ctor time, no
/// per-dispatch URL parse).  No state transition.
pub(super) fn dispatch_ws_text_message(vm: &mut VmInner, instance: ObjectId, s: &str) {
    let Some((handler_id, origin_sid)) = snapshot_message_dispatch(vm, instance, |st| st.onmessage)
    else {
        return;
    };
    let data_sid = vm.strings.intern(s);
    let message_sid = vm.well_known.message;
    let empty_sid = vm.well_known.empty;
    fire_message_event(
        vm,
        instance,
        handler_id,
        message_sid,
        JsValue::String(data_sid),
        origin_sid,
        empty_sid,
    );
}

/// Handle `WsEvent::BinaryMessage(bytes)`.
///
/// `data` is allocated as a `Blob` (with empty MIME per WHATWG
/// §9.3.7 "if type is Binary and binaryType is 'blob', … type
/// attribute set to the empty string") when `binaryType === "blob"`,
/// or as a fresh `ArrayBuffer` when `binaryType === "arraybuffer"`.
/// The allocation happens BEFORE `fire_message_event` is called;
/// its first action is `push_temp_root(data)` so the data Object is
/// rooted across every subsequent allocation (event, ports Array).
/// No state transition.
pub(super) fn dispatch_ws_binary_message(vm: &mut VmInner, instance: ObjectId, bytes: Vec<u8>) {
    let (handler_id, origin_sid, binary_type) = {
        let Some(hd) = vm.host_data.as_deref() else {
            return;
        };
        let Some(state) = hd.websocket_states.get(&instance) else {
            return;
        };
        let Some(handler_id) = state.onmessage else {
            return;
        };
        (handler_id, state.origin_sid, state.binary_type)
    };

    let data_obj = match binary_type {
        BinaryType::Blob => {
            let empty = vm.well_known.empty;
            super::blob::create_blob_from_bytes(vm, Arc::from(bytes), empty)
        }
        BinaryType::ArrayBuffer => super::array_buffer::create_array_buffer_from_bytes(vm, bytes),
    };
    let message_sid = vm.well_known.message;
    let empty_sid = vm.well_known.empty;
    fire_message_event(
        vm,
        instance,
        handler_id,
        message_sid,
        JsValue::Object(data_obj),
        origin_sid,
        empty_sid,
    );
}

/// Handle `WsEvent::Error(_)`.
///
/// Fires a plain `Event("error")` through the cached `onerror`
/// handler.  Per WHATWG §9.3.7 the WebSocket `"error"` event is a
/// plain `Event`, NOT an `ErrorEvent` (no message / filename /
/// lineno).  Discarding the broker's error string here is correct
/// per spec — the script-visible surface is intentionally opaque to
/// avoid leaking server-internals.  No state transition (the
/// matching `WsEvent::Closed` follows and drives the
/// `transition_to(Closed)`).
pub(super) fn dispatch_ws_error(vm: &mut VmInner, instance: ObjectId) {
    let handler = vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&instance))
        .and_then(|s| s.onerror);
    let Some(handler_id) = handler else {
        return;
    };
    fire_plain_event(vm, instance, handler_id, vm.well_known.error);
}

/// Handle `WsEvent::BytesSent(n)`.
///
/// Pure side-table mutation: decrement `bufferedAmount` by the
/// broker's reported bytes-flushed count.  Saturating arithmetic
/// guards against the broker over-reporting (e.g. due to internal
/// re-fragmentation accounting that diverges from the JS-visible
/// `saturating_add` in `send()`).  No event fires per WHATWG §9.3
/// — `bufferedAmount` is a pull-only observable.
pub(super) fn dispatch_ws_bytes_sent(vm: &mut VmInner, instance: ObjectId, n: u64) {
    let Some(hd) = vm.host_data.as_deref_mut() else {
        return;
    };
    if let Some(state) = hd.websocket_states.get_mut(&instance) {
        state.buffered_amount = state.buffered_amount.saturating_sub(n);
    }
}

/// Snapshot the handler `ObjectId` selected by `pick` together with
/// the pre-interned `origin_sid`, dropping the `host_data` borrow
/// before the caller continues.  Returns `None` (silent-drop) when
/// the VM is unbound, the side-table entry is gone (GC sweep race),
/// or `pick` returns `None` (no handler registered) — short-
/// circuiting before any downstream allocation work.
fn snapshot_message_dispatch(
    vm: &VmInner,
    instance: ObjectId,
    pick: impl FnOnce(&super::super::host_data::WebSocketState) -> Option<ObjectId>,
) -> Option<(ObjectId, StringId)> {
    let hd = vm.host_data.as_deref()?;
    let state = hd.websocket_states.get(&instance)?;
    let handler_id = pick(state)?;
    Some((handler_id, state.origin_sid))
}

/// Allocate a `MessageEvent(type, { data, origin, lastEventId,
/// source: null, ports: [] })` and fire it via `handler_id`.
///
/// Delegates event allocation + slot install + handler dispatch to
/// [`alloc_ua_event`] + [`fire_handler`] (the same pair used by
/// [`fire_plain_event`] / [`fire_close_event`]).  The payload tail
/// `[data, origin, lastEventId, source, ports]` matches the slot
/// order built at `event_shapes::build_precomputed_event_shapes::
/// message`; drift between the two writers puts user-visible slot
/// values in the wrong properties.
///
/// `type_sid` is `well_known.message` for the WebSocket text /
/// binary callers (the only spec-defined MessageEvent type on WS)
/// and the broker-supplied `event_type` interned at the EventSource
/// dispatch site for SSE named events (`event: notification\ndata:
/// ...`).  `last_event_id_sid` is `well_known.empty` for WebSocket
/// (the `MessageEvent.lastEventId` attribute is SSE-only — WS spec
/// §9.3.7 leaves it as the IDL default empty string) and the
/// sticky cumulative value from `EventSourceState::last_event_id`
/// for SSE.
///
/// GC discipline (mirrors `host::pending_tasks::dispatch_post_message`):
/// `data` is rooted first because the binary path passes a freshly
/// allocated Blob / ArrayBuffer with no other live root; `ports_arr`
/// is then created AND pushed onto a nested temp root before
/// `alloc_ua_event` runs, because `vm.alloc_object` inside
/// `alloc_ua_event` is itself a GC trigger point (per
/// `inner.rs::alloc_object`'s "callers must ensure that any
/// ObjectIds reachable only through `obj`'s fields are already
/// rooted" contract) and `ports_arr` is otherwise reachable only
/// through the `payload_slots: Vec<PropertyValue>` Rust local that
/// the GC cannot see.  The freshly-allocated event is rooted by
/// `fire_handler`'s temp-root push, which keeps both `event` and
/// the `ports_arr` slot live across the user-callable invocation;
/// after that returns, both temp roots drop together.
///
/// `pub(super)` so the sibling [`super::event_source_dispatch`]
/// module can reuse the helper for SSE named-event delivery
/// without duplicating the 5-slot payload + GC-root dance.
pub(super) fn fire_message_event(
    vm: &mut VmInner,
    instance: ObjectId,
    handler_id: ObjectId,
    type_sid: StringId,
    data: JsValue,
    origin_sid: StringId,
    last_event_id_sid: StringId,
) {
    let mut g_data = vm.push_temp_root(data);
    let event_proto = g_data.message_event_prototype.or(g_data.event_prototype);
    let shape_id = g_data
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .message;
    let ports_arr = g_data.create_array_object(Vec::new());
    let mut g_ports = g_data.push_temp_root(JsValue::Object(ports_arr));

    let payload_slots = vec![
        PropertyValue::Data(data),
        PropertyValue::Data(JsValue::String(origin_sid)),
        PropertyValue::Data(JsValue::String(last_event_id_sid)),
        PropertyValue::Data(JsValue::Null),
        PropertyValue::Data(JsValue::Object(ports_arr)),
    ];
    let event_id = alloc_ua_event(&mut g_ports, type_sid, shape_id, payload_slots, event_proto);
    fire_handler(&mut g_ports, instance, handler_id, event_id);
    drop(g_ports);
    drop(g_data);
}

/// Allocate a plain `Event(type)` (no payload slots beyond core-9),
/// pin it as a GC root, and synchronously invoke `handler_id` with
/// `this = instance` and `args = [event]`.
///
/// Used for `WsEvent::Connected` → `"open"`, `WsEvent::Error` →
/// `"error"`, and the SSE Connected / Error / FatalError dispatch
/// helpers.  `pub(super)` so the sibling
/// [`super::event_source_dispatch`] module reuses the helper
/// instead of duplicating the alloc + fire dance.
pub(super) fn fire_plain_event(
    vm: &mut VmInner,
    instance: ObjectId,
    handler_id: ObjectId,
    type_sid: super::super::value::StringId,
) {
    // Plain `Event` shape — `EventPayload::None` / non-payload events
    // land at the `core` terminal (core-9 only, no payload slots).
    let shape_id = vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;
    let event_id = alloc_ua_event(vm, type_sid, shape_id, Vec::new(), vm.event_prototype);
    fire_handler(vm, instance, handler_id, event_id);
}

/// Allocate a `CloseEvent("close", {code, reason, wasClean})` and
/// fire it via `handler_id`.  Slot layout mirrors
/// `event_shapes.rs::build_precomputed_event_shapes::close_event`:
/// core-9 then `code`, `reason`, `wasClean`.
fn fire_close_event(
    vm: &mut VmInner,
    instance: ObjectId,
    handler_id: ObjectId,
    code: u16,
    reason: &str,
    was_clean: bool,
) {
    let reason_sid = vm.strings.intern(reason);
    let payload_slots = vec![
        PropertyValue::Data(JsValue::Number(f64::from(code))),
        PropertyValue::Data(JsValue::String(reason_sid)),
        PropertyValue::Data(JsValue::Boolean(was_clean)),
    ];
    let shape_id = vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .close_event;
    let event_id = alloc_ua_event(
        vm,
        vm.well_known.ws_close_event_type,
        shape_id,
        payload_slots,
        // Prefer CloseEvent.prototype so `e instanceof CloseEvent`
        // holds — falls back to Event.prototype if CloseEvent is
        // somehow not yet registered (defensive: register_globals
        // ordering guarantees it is, but the fallback matches the
        // pattern at `pending_tasks::dispatch_post_message`).
        vm.close_event_prototype.or(vm.event_prototype),
    );
    fire_handler(vm, instance, handler_id, event_id);
}

/// Allocate an Event-kind object with the specified prototype, core-9
/// slot defaults, and trailing payload slots — UA-fired (isTrusted=true,
/// bubbles=false, cancelable=false, composed=false).  Mirrors the
/// pattern at `pending_tasks::dispatch_post_message` lines 256-333.
///
/// The caller is responsible for `set_event_slot_raw` of target /
/// currentTarget after this returns (see [`fire_handler`]).
fn alloc_ua_event(
    vm: &mut VmInner,
    type_sid: super::super::value::StringId,
    shape_id: ShapeId,
    payload_slots: Vec<PropertyValue>,
    prototype: Option<ObjectId>,
) -> ObjectId {
    let event_id = vm.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: false,
            passive: false,
            type_sid,
            bubbles: false,
            composed: false,
            composed_path: None,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype,
        extensible: true,
    });

    let timestamp_ms = vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    let mut slots: Vec<PropertyValue> = Vec::with_capacity(9 + payload_slots.len());
    slots.push(PropertyValue::Data(JsValue::String(type_sid)));
    slots.push(PropertyValue::Data(JsValue::Boolean(false))); // bubbles
    slots.push(PropertyValue::Data(JsValue::Boolean(false))); // cancelable
    slots.push(PropertyValue::Data(JsValue::Number(0.0))); // eventPhase
    slots.push(PropertyValue::Data(JsValue::Null)); // target (set below)
    slots.push(PropertyValue::Data(JsValue::Null)); // currentTarget (set below)
    slots.push(PropertyValue::Data(JsValue::Number(timestamp_ms)));
    slots.push(PropertyValue::Data(JsValue::Boolean(false))); // composed
    slots.push(PropertyValue::Data(JsValue::Boolean(true))); // isTrusted
    slots.extend(payload_slots);
    vm.define_with_precomputed_shape(event_id, shape_id, slots);
    event_id
}

/// Pin the event with a temp root, set `target` / `currentTarget` to
/// `instance`, then synchronously invoke `handler_id(this=instance,
/// args=[event])`.  Errors are swallowed per WHATWG IDL
/// EventHandler attribute semantics (`onclose = ...` style handlers
/// log uncaught exceptions to console rather than propagating).
fn fire_handler(vm: &mut VmInner, instance: ObjectId, handler_id: ObjectId, event_id: ObjectId) {
    set_event_slot_raw(vm, event_id, EVENT_SLOT_TARGET, JsValue::Object(instance));
    set_event_slot_raw(
        vm,
        event_id,
        EVENT_SLOT_CURRENT_TARGET,
        JsValue::Object(instance),
    );

    // Root the event across the user-callable invocation.  `call` may
    // re-enter the interpreter (`gc_enabled = true` at native call
    // boundaries) and any alloc inside the handler body that triggers
    // a GC would otherwise reclaim the event object the user is
    // currently inspecting via `e.code` / `e.target` / etc.
    let mut g = vm.push_temp_root(JsValue::Object(event_id));
    // Errors swallowed per WHATWG IDL EventHandler attribute
    // semantics: uncaught exceptions inside an `on*` callback are
    // reported to the global error handler, not propagated to the
    // dispatch site.
    let _ = g.call(
        handler_id,
        JsValue::Object(instance),
        &[JsValue::Object(event_id)],
    );
    drop(g);
}
