// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `WebSocket` interface (WHATWG WebSockets §9.3) — VM thin binding
//! to the renderer-side `NetworkHandle` broker.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! constructor + URL validation glue, send/close JS-side argument
//! coercion, and broker `RendererToNetwork::WebSocket*` emission.
//! The URL scheme normalization + validation algorithm lives in
//! engine-independent [`elidex_api_ws`] (`normalize_ws_url` +
//! `validate_ws_url` + `is_mixed_content`); the broker handles all
//! framing / handshake / I/O.
//!
//! ## Storage layout
//!
//! - [`ObjectKind::WebSocket`][] is payload-free.
//! - Per-instance state ([`super::super::host_data::WebSocketState`]):
//!   readyState + url + protocol + extensions + bufferedAmount +
//!   binaryType + broker `conn_id` + 4 `on*` handler `ObjectId`s.
//!   Lives in [`super::super::host_data::HostData::websocket_states`]
//!   keyed by the instance `ObjectId`.
//! - Reverse lookup
//!   [`super::super::host_data::HostData::ws_conn_to_object`] maps
//!   broker `conn_id` → instance `ObjectId` for routing incoming
//!   `WsEvent`s back to the right wrapper from the extended
//!   `tick_network` drain.
//!
//! ## Lifecycle
//!
//! - Constructor opens a broker connection eagerly and stores the
//!   allocated `conn_id` in the side-table.
//! - GC sweep emits a `WebSocketClose` per swept-instance `conn_id`
//!   (see `vm/gc/collect.rs` sweep tail) so the broker I/O thread
//!   terminates rather than leaking.
//! - `Vm::unbind` drains every `(conn_id, _)` from the WS+SSE
//!   side-tables (CRIT-A: snapshot BEFORE clearing), sends the
//!   matching `WebSocketClose` to the outgoing handle, then clears
//!   the side-tables and resets the per-VM counter.  Mirror of
//!   `reject_pending_fetches_with_error` shape at
//!   `vm/host/fetch_tick.rs:82-131`.
//!
//! ## Phase 0b scaffolding
//!
//! The constructor body currently stubs as a `TypeError` —
//! prototype install + brand check + register-globals wiring +
//! constructor function object are in place but the actual URL
//! parse / mixed-content gate / protocols token validation /
//! broker open land in Phase 1.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::VmInner;
use super::events::install_ctor;

impl VmInner {
    /// Allocate `WebSocket.prototype` chained to `Object.prototype`,
    /// install the readyState / url / protocol / extensions /
    /// bufferedAmount / binaryType accessors, send / close methods,
    /// 4 `on*` handler getter/setter pairs, the CONNECTING / OPEN /
    /// CLOSING / CLOSED IDL constants, and expose the user-callable
    /// `WebSocket` constructor on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean the
    /// call-order invariant from `register_globals()` was violated.
    pub(in crate::vm) fn register_websocket_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_websocket_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.websocket_prototype = Some(proto_id);

        // Phase 1+ will install accessors / methods on `proto_id`
        // here.  Constants are installed below on BOTH the ctor and
        // the prototype per WebIDL `const unsigned short` rules.

        let global_sid = self.well_known.websocket_global;
        install_ctor(
            self,
            proto_id,
            "WebSocket",
            native_websocket_constructor,
            global_sid,
        );

        install_ws_readystate_constants(self, proto_id, global_sid);
    }
}

/// Install the 4 readyState constants (CONNECTING=0 / OPEN=1 /
/// CLOSING=2 / CLOSED=3) on both the WebSocket constructor object
/// and `WebSocket.prototype` per WHATWG WebSockets §9.3.4 IDL.
///
/// Per MIN-3 (plan v1.1): instances do NOT receive own-property
/// copies — they inherit through the proto chain.  Two install
/// sites only.
fn install_ws_readystate_constants(vm: &mut VmInner, proto_id: ObjectId, global_sid: StringId) {
    let Some(JsValue::Object(ctor_id)) = vm.globals.get(&global_sid).copied() else {
        return;
    };
    let constants: [(StringId, u8); 4] = [
        (vm.well_known.ws_connecting_const, 0),
        (vm.well_known.ws_open_const, 1),
        (vm.well_known.ws_closing_const, 2),
        (vm.well_known.ws_closed_const, 3),
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
// Brand check (D-13 simplify lesson: private fn, not a method on VmInner)
// ---------------------------------------------------------------------------

/// Return the receiver's `ObjectId` if `this` brands as a
/// `WebSocket`, else a `TypeError`.  Mirror of FileReader's
/// `require_file_reader_this` shape — kept private to this module
/// per D-13 simplify lesson (brand checks have no cross-module
/// callers, so a free function avoids `pub(crate)` API surface).
#[allow(dead_code)]
pub(super) fn require_websocket_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "WebSocket.prototype.{method} called on non-WebSocket"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::WebSocket) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "WebSocket.prototype.{method} called on non-WebSocket"
        )))
    }
}

// ---------------------------------------------------------------------------
// Constructor (Phase 0b stub — Phase 1 replaces body)
// ---------------------------------------------------------------------------

/// `new WebSocket(url, protocols?)` — placeholder for Phase 1.
///
/// User-callable per WHATWG WebSockets §9.3.1: returns
/// `TypeError` for now so any test that instantiates fails
/// loudly until Phase 1 wires URL parse / scheme promotion /
/// mixed-content / protocols token validation / broker open.
///
/// `is_construct()` gate matches the spec's `[Constructor]`
/// attribute: bare `WebSocket(...)` call (without `new`) throws.
#[allow(clippy::needless_pass_by_value)]
fn native_websocket_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'WebSocket': Please use the 'new' operator",
        ));
    }
    Err(VmError::type_error(
        "WebSocket constructor not yet implemented (Phase 1)",
    ))
}
