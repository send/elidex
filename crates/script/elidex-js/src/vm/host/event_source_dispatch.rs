// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Realtime event delivery for `EventSource` instances — UA-fire
//! helpers consumed by [`super::fetch_tick::VmInner::tick_network`]
//! through `dispatch_realtime_event`.
//!
//! Split out of [`super::event_source`] (which holds the JS-API
//! surface — ctor, accessors, methods, on* setters) so the
//! event-allocation + dispatch paths don't bloat the user-facing
//! module.  Mirrors the [`super::websocket_dispatch`] split.
//!
//! ## Dispatch through the shared §2.9 VmObject core
//!
//! Every event fires through the shared seam
//! ([`super::event_target_dispatch_vm::dispatch_vm_simple_event`] for
//! plain `open` / `error`, [`super::event_target_dispatch_vm::fire_vm_message_event`]
//! for `message` / named events).  The seam walks the unified
//! `vm_event_listeners` home, so `on*` handlers + every
//! `addEventListener` listener (with capture / once / passive /
//! `{signal}`) fire off one event object, and the GC-root / dispatch-
//! flag discipline lives there — not here.
//!
//! ## §9.2.6 "message vs named" fan-out is emergent
//!
//! `onmessage` is an `EventHandler`-kind listener of type `"message"`;
//! `addEventListener("foo", cb)` is a `Normal`-kind listener of type
//! `"foo"`.  Dispatching a `MessageEvent` of the parsed `event_type`
//! at the shared core therefore delivers a default (`message`) event to
//! `onmessage` + `addEventListener("message")`, and a named event only
//! to `addEventListener(name)` — exactly WHATWG HTML §9.2.6 without a
//! hand-coded fan-out.
//!
//! ## Side-table mutation ordering
//!
//! State transitions (`transition_to(Open)` / `transition_to(Closed)`
//! / `transition_to(Connecting)`) and `last_event_id` / `origin_sid`
//! updates run BEFORE the event-fire so handlers observing
//! `es.readyState === EventSource.OPEN` / reading `e.lastEventId` from
//! inside the `open` / `message` callback see the post-mutation values
//! (matches Chrome / Firefox semantics).

#![cfg(feature = "engine")]

use elidex_api_ws::SseReadyState;

use super::super::value::{JsValue, NativeContext, ObjectId};
use super::event_target_dispatch_vm::{dispatch_vm_simple_event, fire_vm_message_event};

/// Handle `SseEvent::Connected { final_url }`.
///
/// Order per WHATWG HTML §9.2.3 step "announce the connection":
/// transition state → refresh `MessageEvent.origin` from the final
/// URL per §9.2.6 "Dispatch the event" → fire `Event("open")` through
/// the shared core.  Per-state matrix: `Connecting→Open` is the
/// first-handshake announce, `Open→Open` is the legitimate idempotent
/// path after the broker's auto-reconnect cycle; only the terminal
/// `Closed` state short-circuits (JS-side `close()` raced ahead).
///
/// The origin refresh runs on every `Connected` so auto-reconnect
/// cycles that land on a different final URL observe the new origin.
pub(super) fn dispatch_sse_connected(
    ctx: &mut NativeContext<'_>,
    instance: ObjectId,
    final_url: &url::Url,
) {
    let new_origin_sid = ctx
        .vm
        .strings
        .intern(&final_url.origin().ascii_serialization());
    {
        let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        let _ = state.transition_to(SseReadyState::Open);
        state.origin_sid = new_origin_sid;
    }
    let type_sid = ctx.vm.well_known.ws_open_event_type;
    let _ = dispatch_vm_simple_event(ctx, instance, type_sid, false, false);
}

/// Handle `SseEvent::Event { event_type, data, last_event_id }`.
///
/// Per WHATWG HTML §9.2.6 "Dispatch the event": update sticky
/// `last_event_id` from the broker's cumulative value
/// ([`elidex_net::sse::SseParserState::take_event`] always emits the
/// current buffer per §9.2.6), then dispatch a `MessageEvent` named
/// `event_type` with `data` / `origin` (post-redirect URL origin,
/// refreshed at each `Connected`) / `lastEventId` (sticky cumulative)
/// at the shared core.  The §9.2.6 "message vs named" fan-out falls
/// out of the dispatch (see module doc) — no hand-coding here.
pub(super) fn dispatch_sse_event(
    ctx: &mut NativeContext<'_>,
    instance: ObjectId,
    event_type: &str,
    data: &str,
    last_event_id: &str,
) {
    let origin_sid = {
        let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        // A message must NOT fire on a CLOSED source (§9.2.6 / §9.2.9) — mirrors
        // the `dispatch_sse_connected` / `fire_sse_error` CLOSED guards (Codex
        // R2b-B). Without this a buffered event still in the NetworkHandle at the
        // time the user `close()`d (or a fatal error CLOSED the source) would fire
        // on the closed EventSource. Pairs with `es_keepalive` never rooting a
        // CLOSED source on `has_queued_task` (R2b-A): the queued task is dropped
        // here, so the wrapper is safely collectible.
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        // Broker emits the cumulative sticky value per WHATWG §9.2.6 step 11 —
        // unconditional propagation (independent of any listener) matches both
        // `id:\n` (empty resets) and the multi-event accumulator.  Updated in
        // place (reuse the buffer) so a high-volume stream avoids a per-event
        // String realloc; `fire_vm_message_event` also reads it (`&str`) below.
        state.last_event_id.clear();
        state.last_event_id.push_str(last_event_id);
        state.origin_sid
    };
    // `event_type` / `last_event_id` / `data` are all interned past
    // `fire_vm_message_event`'s gate (the `&str` args + the `data` thunk), so
    // an unobserved EventSource — e.g. a named keepalive stream the page never
    // `addEventListener`s — interns none of these server-controlled strings.
    let _ = fire_vm_message_event(
        ctx,
        instance,
        event_type,
        |ctx| JsValue::String(ctx.vm.strings.intern(data)),
        origin_sid,
        last_event_id,
    );
}

/// Handle `SseEvent::Error(_)`.
///
/// Per WHATWG HTML §9.2.3 "Processing model" (both the handshake-time
/// failure and the mid-stream "reestablish the connection" path) — a
/// transient network error transitions the readyState BACKWARDS to
/// `CONNECTING` because the broker is auto-reconnecting; the next
/// successful handshake fires `SseEvent::Connected` and snaps it back to
/// `Open`.  This is spec-mandated; do NOT mistake it for a forward-only
/// state machine.  The error string is discarded at the
/// [`super::fetch_tick`] arm (server-internals opacity); the interface is
/// a plain `Event("error")` per §9.2.3 (NOT an `ErrorEvent`).
pub(super) fn dispatch_sse_error(ctx: &mut NativeContext<'_>, instance: ObjectId) {
    fire_sse_error(ctx, instance, SseReadyState::Connecting);
}

/// Handle `SseEvent::FatalError(_)`.
///
/// Per WHATWG HTML §9.2.3 "fail the connection" — terminal: the broker
/// will NOT auto-reconnect.  Transitions readyState to `Closed` and
/// fires a plain `Event("error")`.  No CloseEvent is dispatched per
/// spec (SSE has no equivalent of WebSocket's CloseEvent).
pub(super) fn dispatch_sse_fatal_error(ctx: &mut NativeContext<'_>, instance: ObjectId) {
    fire_sse_error(ctx, instance, SseReadyState::Closed);
}

/// Shared body for the two SSE error paths (`dispatch_sse_error` →
/// transient `Connecting`, `dispatch_sse_fatal_error` → terminal
/// `Closed`): guard the VM / side-table, suppress when the user already
/// `close()`d (state is `Closed`), transition to `new_state`, then fire a
/// plain `Event("error")` through the shared §2.9 core (never an
/// `ErrorEvent`, per WHATWG HTML §9.2.3).
fn fire_sse_error(ctx: &mut NativeContext<'_>, instance: ObjectId, new_state: SseReadyState) {
    {
        let Some(hd) = ctx.vm.host_data.as_deref_mut() else {
            return;
        };
        let Some(state) = hd.event_source_states.get_mut(&instance) else {
            return;
        };
        if matches!(state.ready_state, SseReadyState::Closed) {
            return;
        }
        let _ = state.transition_to(new_state);
    }
    let type_sid = ctx.vm.well_known.error;
    let _ = dispatch_vm_simple_event(ctx, instance, type_sid, false, false);
}
