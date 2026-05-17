// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `EventSource` interface (WHATWG HTML §9.2) — VM thin binding to
//! the renderer-side `NetworkHandle` broker.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! constructor + URL parse glue, close emission, and minimal
//! `addEventListener` / `removeEventListener` shim required by
//! WHATWG §9.2 for named-event delivery (CRIT-3 fold per plan v1.1).
//! The broker handles SSE framing, reconnection, and lastEventId
//! sticky semantics.
//!
//! ## Storage layout
//!
//! - [`ObjectKind::EventSource`][] is payload-free.
//! - Per-instance state ([`super::super::host_data::EventSourceState`]):
//!   readyState + url + withCredentials + sticky lastEventId +
//!   broker `conn_id` + 3 `on*` handler `ObjectId`s + per-instance
//!   `addEventListener` registry (`HashMap<String, Vec<ObjectId>>`).
//!   Lives in [`super::super::host_data::HostData::event_source_states`]
//!   keyed by the instance `ObjectId`.
//! - Reverse lookup
//!   [`super::super::host_data::HostData::sse_conn_to_object`] maps
//!   broker `conn_id` → instance `ObjectId` for routing incoming
//!   `SseEvent`s back to the right wrapper from the extended
//!   `tick_network` drain.
//!
//! ## addEventListener scope (CRIT-3 fold)
//!
//! WHATWG HTML §9.2 lets servers emit named events
//! (`event: notification\ndata: ...`) that JS code receives via
//! `evtsrc.addEventListener("notification", cb)`.  Silent-dropping
//! these would be a spec-violating user-data loss, so this PR
//! ships the minimal `addEventListener(type, listener)` /
//! `removeEventListener(type, listener)` pair.  Full options
//! surface (capture / once / passive / signal) is deferred to
//! `#11-realtime-event-listeners`.
//!
//! ## Lifecycle
//!
//! - Constructor opens a broker SSE connection eagerly.
//! - GC sweep emits `EventSourceClose` per swept-instance `conn_id`
//!   (see `vm/gc/collect.rs` sweep tail).
//! - `Vm::unbind` snapshots and closes every active conn (shared
//!   CRIT-A teardown with WebSocket).
//!
//! ## Phase 0b scaffolding
//!
//! The constructor body is a Phase 1 stub (`TypeError`); prototype +
//! brand check + register-globals wiring + constants are in place.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::VmInner;
use super::events::install_ctor;

impl VmInner {
    /// Allocate `EventSource.prototype` chained to `Object.prototype`,
    /// install the readyState / url / withCredentials accessors,
    /// close / addEventListener / removeEventListener methods, 3
    /// `on*` handler getter/setter pairs, the CONNECTING / OPEN /
    /// CLOSED IDL constants, and expose the user-callable
    /// `EventSource` constructor on `globalThis`.
    ///
    /// SSE lacks a `CLOSING` constant per WHATWG HTML §9.2 (3-state
    /// machine, not 4) — `CLOSING` is WebSocket-only.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean the
    /// call-order invariant from `register_globals()` was violated.
    pub(in crate::vm) fn register_event_source_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_event_source_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.event_source_prototype = Some(proto_id);

        // Phase 3 will install accessors / methods on `proto_id`.

        let global_sid = self.well_known.event_source_global;
        install_ctor(
            self,
            proto_id,
            "EventSource",
            native_event_source_constructor,
            global_sid,
        );

        install_sse_readystate_constants(self, proto_id, global_sid);
    }
}

/// Install the 3 readyState constants (CONNECTING=0 / OPEN=1 /
/// CLOSED=2) on both the EventSource constructor object and
/// `EventSource.prototype` per WHATWG HTML §9.2 IDL.  Note SSE
/// has no `CLOSING` — the spec models a 3-state machine where
/// transient errors loop back to CONNECTING during reconnect.
fn install_sse_readystate_constants(vm: &mut VmInner, proto_id: ObjectId, global_sid: StringId) {
    let Some(JsValue::Object(ctor_id)) = vm.globals.get(&global_sid).copied() else {
        return;
    };
    let constants: [(StringId, u8); 3] = [
        (vm.well_known.ws_connecting_const, 0),
        (vm.well_known.ws_open_const, 1),
        // CLOSED is value 2 on SSE (3-state machine), value 3 on
        // WS — same well-known string, different per-interface
        // numeric value.  Spec-mandated per WHATWG HTML §9.2 IDL
        // (CONNECTING=0, OPEN=1, CLOSED=2).
        (vm.well_known.ws_closed_const, 2),
    ];
    for (name_sid, value) in constants {
        let key = PropertyKey::String(name_sid);
        vm.define_shaped_property(
            ctor_id,
            key,
            PropertyValue::Data(JsValue::Number(f64::from(value))),
            PropertyAttrs::BUILTIN,
        );
        vm.define_shaped_property(
            proto_id,
            key,
            PropertyValue::Data(JsValue::Number(f64::from(value))),
            PropertyAttrs::BUILTIN,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check (D-13 simplify lesson: private fn)
// ---------------------------------------------------------------------------

/// Return the receiver's `ObjectId` if `this` brands as an
/// `EventSource`, else a `TypeError`.  Mirror of
/// `require_websocket_this` in the sibling module.
#[allow(dead_code)]
pub(super) fn require_event_source_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "EventSource.prototype.{method} called on non-EventSource"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::EventSource) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "EventSource.prototype.{method} called on non-EventSource"
        )))
    }
}

// ---------------------------------------------------------------------------
// Constructor (Phase 0b stub — Phase 3 replaces body)
// ---------------------------------------------------------------------------

/// `new EventSource(url, init?)` — placeholder for Phase 3.
///
/// User-callable per WHATWG HTML §9.2.1.  Throws `TypeError` until
/// Phase 3 wires URL parse + dict-member read + broker open +
/// reverse-map insert.
#[allow(clippy::needless_pass_by_value)]
fn native_event_source_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'EventSource': Please use the 'new' operator",
        ));
    }
    Err(VmError::type_error(
        "EventSource constructor not yet implemented (Phase 3)",
    ))
}
