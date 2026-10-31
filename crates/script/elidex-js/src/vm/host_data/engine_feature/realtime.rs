//! WebSocket / EventSource per-instance state (D-12 `#11-net-ws-sse`).
//!
//! [`BinaryType`], [`WebSocketState`], [`EventSourceState`] and their
//! `readyState` state machines, plus the `HostData` connection-ID
//! allocators and the teardown drain of the realtime side-tables.
//! Carved out of `host_data/mod.rs` as a cohesion seam (pure move).

use super::super::super::value::StringId;
use super::HostData;

/// `WebSocket.binaryType` enum (WHATWG WebSockets §9.3).
///
/// Default is `Blob` per spec.  WebIDL `enum BinaryType {
/// "blob", "arraybuffer" }` — assignment to any other string
/// throws TypeError (handled in the setter, not here — Phase 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinaryType {
    /// Default: incoming binary frames are delivered as `Blob`
    /// instances on the next `MessageEvent.data`.
    #[default]
    Blob,
    /// Incoming binary frames are delivered as `ArrayBuffer`
    /// instances.  Set via `ws.binaryType = "arraybuffer"`.
    ArrayBuffer,
}

/// Per-`WebSocket` instance out-of-band state.  Held on
/// `HostData` in the `websocket_states` map keyed by the
/// instance `ObjectId`.
///
/// Lives on HostData (not VmInner) because the broker handle's
/// per-conn_id state dies on `Vm::unbind`; JS-visible state has
/// to die with it.  `transition_to` enforces WHATWG §9.3 state
/// monotonicity (CONNECTING → OPEN → CLOSING → CLOSED, no
/// backward moves, no CONNECTING → CLOSED direct without an
/// intermediate CLOSING — same-state is a no-op).
#[derive(Debug)]
pub struct WebSocketState {
    /// Current readyState bucket (matches the JS-visible
    /// `WebSocket.readyState` integer).
    pub ready_state: elidex_api_ws::WsReadyState,
    /// Constructor URL after normalization + validation (so
    /// `ws.url` echoes back the post-promotion `ws://` /
    /// `wss://` form per WHATWG §9.3.4 step 2 "URL serializer").
    pub url: String,
    /// Pre-interned WebSocket-URL origin (per WHATWG §9.3.7 the
    /// `MessageEvent.origin` for incoming frames is the server's
    /// origin, NOT the page origin).  Cached once at
    /// constructor time so per-message dispatch reads a
    /// `StringId` from the side-table rather than re-parsing
    /// `url` + serialising the origin on every event.  Opaque
    /// origins serialise to the literal `"null"`.
    pub origin_sid: StringId,
    /// Negotiated sub-protocol — `""` until `WsEvent::Connected`
    /// supplies the value, then frozen.
    pub protocol: String,
    /// Negotiated extensions — `""` until Connected, then frozen.
    pub extensions: String,
    /// Bytes queued for transmission but not yet flushed.  JS
    /// `send()` increments via `saturating_add`; broker
    /// `WsEvent::BytesSent(n)` decrements via `saturating_sub`.
    /// `u64` to match the spec's `unsigned long long`.
    pub buffered_amount: u64,
    /// Current binaryType — see [`BinaryType`].
    pub binary_type: BinaryType,
    /// Broker connection ID — paired with HostData's
    /// `ws_conn_to_object` reverse map for event routing and
    /// emitted on `WsCommand` / `WebSocketClose` messages.
    pub conn_id: u64,
    // `onopen` / `onmessage` / `onerror` / `onclose` are NOT stored
    // here: since `#11-realtime-event-listeners` they live (with all
    // `addEventListener` listeners) in the unified
    // `VmInner::vm_event_listeners` home, dispatched through the
    // shared §2.9 VmObject core.
}

impl WebSocketState {
    /// Apply a `readyState` transition.  Per WHATWG §9.3 the
    /// state machine is strictly monotonic — once OPEN the
    /// only legal next states are CLOSING / CLOSED, and once
    /// CLOSING the only legal next state is CLOSED.  Same-state
    /// transitions are accepted as no-ops (idempotent
    /// `close()` calls land here).
    ///
    /// Returns `Err(&'static str)` on an illegal transition
    /// AND fires a `debug_assert!` — the wrong direction is
    /// always a code bug, never user-driven.  Production
    /// release builds receive the Result so the caller can
    /// short-circuit gracefully (e.g. drop a stale broker
    /// event after `Vm::unbind` snapshotted the state).
    pub fn transition_to(&mut self, new: elidex_api_ws::WsReadyState) -> Result<(), &'static str> {
        use elidex_api_ws::WsReadyState::{Closed, Closing, Connecting, Open};
        // Legal transitions per WHATWG §9.3 (CONNECTING →
        // anything; OPEN → forward-or-same; CLOSING →
        // forward-or-same; CLOSED → CLOSED only).  Same-state
        // pairs are idempotent no-ops folded into each arm.
        // Wildcard `_` returns `false` for backward /
        // terminal-exit pairs (Open→Connecting, Closing→
        // Connecting / Open, Closed→Connecting / Open /
        // Closing).
        let ok = matches!(
            (self.ready_state, new),
            (Connecting, _)
                | (Open, Open | Closing | Closed)
                | (Closing, Closing | Closed)
                | (Closed, Closed)
        );
        if ok {
            self.ready_state = new;
            Ok(())
        } else {
            debug_assert!(
                false,
                "illegal WebSocket readyState transition {:?} → {:?}",
                self.ready_state, new
            );
            Err("illegal WebSocket readyState transition")
        }
    }
}

/// Per-`EventSource` instance out-of-band state.  Held on
/// `HostData` in the `event_source_states` map keyed by the
/// instance `ObjectId`.
///
/// SSE has a 3-state machine (no CLOSING — error / fatal /
/// close all land directly).  The transient `Error` event
/// moves OPEN → CONNECTING during automatic reconnect, then
/// `Connected` snaps back to OPEN.  `FatalError` and JS
/// `close()` are terminal.
#[derive(Debug)]
pub struct EventSourceState {
    pub ready_state: elidex_api_ws::SseReadyState,
    /// Constructor URL after parse + relative-resolution.  No
    /// scheme promotion (SSE is HTTP-only).
    pub url: String,
    /// Pre-interned origin used as `MessageEvent.origin` on
    /// every dispatched server event.  Seeded at constructor
    /// time from the ctor URL's origin as a defensive default
    /// for the (unreachable-in-practice) pre-Connected window,
    /// then refreshed at every `Connected` dispatch
    /// (`vm::host::event_source_dispatch::dispatch_sse_connected`)
    /// from the post-redirect final URL's origin per WHATWG
    /// HTML §9.2 "Dispatch the event".  Opaque-origin
    /// serialisation returns the literal `"null"`, but SSE's
    /// HTTP(S) scheme gate (`connect.rs`) makes that path
    /// unreachable in practice.
    pub origin_sid: StringId,
    /// `init.withCredentials` echo.
    pub with_credentials: bool,
    /// Sticky lastEventId — broker emits cumulative value per
    /// HTML §9.2 (IMP-5).  Tracked here so the JS surface
    /// `lastEventId` accessor (not in this PR — handler-only)
    /// + reconnect's `Last-Event-ID` header are kept in sync.
    pub last_event_id: String,
    /// Broker connection ID — paired with HostData's
    /// `sse_conn_to_object` reverse map.
    pub conn_id: u64,
    // `onopen` / `onmessage` / `onerror` and the bespoke per-type
    // `event_listeners` registry are NOT stored here: since
    // `#11-realtime-event-listeners` every listener (on* handlers +
    // named-event `addEventListener` registrations) lives in the
    // unified `VmInner::vm_event_listeners` home, dispatched through
    // the shared §2.9 VmObject core.  Named-event delivery + the
    // §9.2.6 "message vs named" fan-out are emergent from it.
}

impl EventSourceState {
    /// Apply an SSE `readyState` transition.  Legal moves:
    /// - `Connecting → Open` (handshake completed)
    /// - `Open → Connecting` (transient `SseEvent::Error`,
    ///   auto-reconnect IMP-3)
    /// - `Connecting → Closed` / `Open → Closed` (JS
    ///   `close()` or `SseEvent::FatalError`)
    /// - Same-state = idempotent
    ///
    /// Illegal moves trip `debug_assert!` and return Err.
    pub fn transition_to(&mut self, new: elidex_api_ws::SseReadyState) -> Result<(), &'static str> {
        use elidex_api_ws::SseReadyState::{Closed, Connecting, Open};
        // Legal transitions per WHATWG HTML §9.2 — CONNECTING
        // and OPEN can flow to any state (OPEN→CONNECTING is
        // the legitimate auto-reconnect path on transient
        // `SseEvent::Error` per IMP-3), CLOSED is terminal.
        let ok = matches!(
            (self.ready_state, new),
            (Connecting | Open, _) | (Closed, Closed)
        );
        if ok {
            self.ready_state = new;
            Ok(())
        } else {
            debug_assert!(
                false,
                "illegal EventSource readyState transition {:?} → {:?}",
                self.ready_state, new
            );
            Err("illegal EventSource readyState transition")
        }
    }
}

impl HostData {
    // -------------------------------------------------------------
    // D-12 #11-net-ws-sse: ID allocation + side-table accessors
    // -------------------------------------------------------------

    /// Allocate a fresh per-VM WebSocket connection ID.  Counter
    /// resets on `Vm::unbind` so connection IDs are scoped to
    /// the current bind cycle, matching the broker handle's
    /// own lifetime.
    pub(crate) fn alloc_ws_conn_id(&mut self) -> u64 {
        let id = self.ws_next_conn_id;
        self.ws_next_conn_id = self.ws_next_conn_id.wrapping_add(1);
        id
    }

    /// Allocate a fresh per-VM SSE connection ID.  Same scope
    /// contract as [`Self::alloc_ws_conn_id`].
    pub(crate) fn alloc_sse_conn_id(&mut self) -> u64 {
        let id = self.sse_next_conn_id;
        self.sse_next_conn_id = self.sse_next_conn_id.wrapping_add(1);
        id
    }

    /// Drain the WebSocket / EventSource side-tables and
    /// produce the broker `conn_id` lists that
    /// `Vm::teardown_document` must close BEFORE clearing state, per
    /// CRIT-A.  Returns `(ws_conn_ids, sse_conn_ids)`; the
    /// caller emits `WebSocketClose` / `EventSourceClose` per
    /// id and then this method's *post-drain* state is the
    /// cleared baseline for the next `bind`.
    ///
    /// Counter fields (`ws_next_conn_id` / `sse_next_conn_id`)
    /// also reset so the next bind starts fresh — matches the
    /// broker handle's lifetime contract.
    pub(crate) fn drain_realtime_for_unbind(&mut self) -> (Vec<u64>, Vec<u64>) {
        let ws_conns: Vec<u64> = self.ws_conn_to_object.keys().copied().collect();
        let sse_conns: Vec<u64> = self.sse_conn_to_object.keys().copied().collect();
        self.websocket_states.clear();
        self.ws_conn_to_object.clear();
        self.ws_next_conn_id = 0;
        self.event_source_states.clear();
        self.sse_conn_to_object.clear();
        self.sse_next_conn_id = 0;
        (ws_conns, sse_conns)
    }
}
