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

use super::super::shape::{self, ShapeId};
use super::super::value::{JsValue, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue};
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

/// Allocate a plain `Event(type)` (no payload slots beyond core-9),
/// pin it as a GC root, and synchronously invoke `handler_id` with
/// `this = instance` and `args = [event]`.
///
/// Used for `WsEvent::Connected` → `"open"`.  Phase 2 reuses this
/// for `WsEvent::Error` → `"error"`.
fn fire_plain_event(
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
