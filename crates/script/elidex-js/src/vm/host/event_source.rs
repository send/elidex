//! `EventSource` interface (WHATWG HTML §9.2) — VM thin binding to
//! the renderer-side `NetworkHandle` broker.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! constructor + URL parse glue, close emission, and minimal
//! `addEventListener` / `removeEventListener` shim required by
//! WHATWG §9.2 for named-event delivery (per the in-PR fold of
//! the originally-spec-violating silent-drop alternative).
//! The broker handles SSE framing, reconnection, and lastEventId
//! sticky semantics.
//!
//! ## Storage layout
//!
//! - [`ObjectKind::EventSource`][] is payload-free.
//! - Per-instance state ([`super::super::host_data::EventSourceState`]):
//!   readyState + url + pre-interned origin + withCredentials +
//!   sticky lastEventId + broker `conn_id` + 3 `on*` handler
//!   `ObjectId`s + per-instance `addEventListener` registry
//!   (`HashMap<String, Vec<ObjectId>>`).  Lives in
//!   [`super::super::host_data::HostData::event_source_states`]
//!   keyed by the instance `ObjectId`.
//! - Reverse lookup
//!   [`super::super::host_data::HostData::sse_conn_to_object`] maps
//!   broker `conn_id` → instance `ObjectId` for routing incoming
//!   `SseEvent`s back to the right wrapper from the extended
//!   `tick_network` drain.
//!
//! ## addEventListener scope
//!
//! WHATWG HTML §9.2 lets servers emit named events
//! (`event: notification\ndata: ...`) that JS code receives via
//! `evtsrc.addEventListener("notification", cb)`.  Silent-dropping
//! these would be a spec-violating user-data loss, so this PR
//! ships the minimal `addEventListener(type, listener)` /
//! `removeEventListener(type, listener)` pair.  Full options
//! surface (capture / once / passive / signal) is deferred to
//! `#11-realtime-event-listeners`.  Per WHATWG DOM §2.7.2 the
//! `(type, callback, capture)` triple is de-duplicated on
//! registration — the minimal shim collapses `capture` to `false`
//! so the dedup check reduces to `(type, callback ObjectId)`.
//!
//! ## Lifecycle
//!
//! - Constructor opens a broker SSE connection eagerly.
//! - GC sweep emits `EventSourceClose` per swept-instance `conn_id`
//!   (see `vm/gc/collect.rs` sweep tail).
//! - `Vm::unbind` snapshots and closes every active conn (shared
//!   CRIT-A teardown with WebSocket).
//!
//! ### `MessageEvent.origin` semantics
//!
//! `MessageEvent.origin` tracks the post-redirect URL per WHATWG
//! HTML §9.2.6 "Dispatch the event" — see
//! [`super::super::host_data::EventSourceState::origin_sid`] for
//! the refresh path (re-derived from
//! [`elidex_net::sse::SseEvent::Connected`]'s `final_url` payload
//! at every handshake / auto-reconnect cycle).

#![cfg(feature = "engine")]

use elidex_net::broker::RendererToNetwork;

use super::super::host_data::EventSourceState;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::events::install_ctor;

impl VmInner {
    /// Allocate `EventSource.prototype` chained to `EventTarget.prototype`,
    /// install the readyState / url / withCredentials accessors, the
    /// `close` method, 3 `on*` handler getter/setter pairs, the
    /// CONNECTING / OPEN / CLOSED IDL constants, and expose the
    /// user-callable `EventSource` constructor on `globalThis`.
    /// `addEventListener` / `removeEventListener` / `dispatchEvent` are
    /// inherited from `EventTarget.prototype`, not installed here.
    ///
    /// SSE lacks a `CLOSING` constant per WHATWG HTML §9.2 (3-state
    /// machine, not 4) — `CLOSING` is WebSocket-only.
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` is `None` — would mean the
    /// call-order invariant from `register_globals()` was violated.
    pub(in crate::vm) fn register_event_source_global(&mut self) {
        // `EventSource : EventTarget` (WHATWG HTML §9.2.2) — chain the
        // prototype to `EventTarget.prototype` so `addEventListener` /
        // `removeEventListener` / `dispatchEvent` are inherited and
        // route through the §2.9 VmObject dispatch core.  Named SSE
        // events (`event: foo`) and the §9.2.6 "message vs named"
        // fan-out then fall out of dispatching the right event type at
        // the shared core (`#11-realtime-event-listeners`).
        let event_target_proto = self
            .event_target_prototype
            .expect("register_event_source_global called before register_event_target_prototype");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.event_source_prototype = Some(proto_id);

        self.install_event_source_members(proto_id);

        let global_sid = self.well_known.event_source_global;
        install_ctor(
            self,
            proto_id,
            "EventSource",
            native_event_source_constructor,
            global_sid,
            super::super::value::CallShape::ConstructorOnly,
        );

        install_sse_readystate_constants(self, proto_id, global_sid);
    }

    fn install_event_source_members(&mut self, proto_id: ObjectId) {
        // Readonly accessors: readyState / url / withCredentials.
        let ro_accessors: [(StringId, NativeFn); 3] = [
            (self.well_known.ready_state, native_es_get_ready_state),
            (self.well_known.url, native_es_get_url),
            (
                self.well_known.with_credentials,
                native_es_get_with_credentials,
            ),
        ];
        for (name_sid, getter) in ro_accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Event handler attributes: onopen / onmessage / onerror, over
        // the shared VmObject EventHandler backend (WHATWG HTML
        // §8.1.8.1) — recorded in the unified `vm_event_listeners` home.
        self.install_vm_object_handler_attrs(proto_id, &["onopen", "onmessage", "onerror"]);

        // Methods: close.  `addEventListener` / `removeEventListener` /
        // `dispatchEvent` are inherited from `EventTarget.prototype`
        // (proto reparented above) — the bespoke per-instance
        // `event_listeners` registry + `native_es_add/remove_event_listener`
        // were removed by `#11-realtime-event-listeners`.
        self.install_native_method(
            proto_id,
            self.well_known.close,
            native_es_close,
            PropertyAttrs::METHOD,
        );
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
// Constructor — `new EventSource(url, init?)`
// ---------------------------------------------------------------------------

/// `new EventSource(url, init?)` per WHATWG HTML §9.2.1.
///
/// Steps:
/// 1. Coerce `url` arg via `ToString`; parse via
///    `vm.navigation.current_url.join(.)` so relative inputs like
///    `"/events"` resolve against the active document base URL
///    (the spec's "API base URL of the entry settings object" —
///    SSE explicitly supports this, in contrast with the WebSocket
///    ctor which uses absolute-URL parsing only).  SyntaxError on
///    parse failure.
/// 2. Read `init.withCredentials` dict member (WebIDL §3.10
///    dictionary semantics: undefined / null init → empty dict,
///    primitive non-Object init → TypeError; Object init →
///    `ToBoolean(Get(init, "withCredentials"))` defaulting to
///    `false`).
/// 3. Allocate `conn_id` via `HostData::alloc_sse_conn_id()`.
/// 4. Promote `this` to `ObjectKind::EventSource`; insert
///    `EventSourceState { ready_state: Connecting, url, origin_sid,
///    with_credentials, last_event_id: "", conn_id }` into
///    `host_data.event_source_states`; populate reverse map
///    `sse_conn_to_object[conn_id] = id`.  (Listeners live in the
///    shared `vm_event_listeners` home, not the state struct.)
/// 5. Send `RendererToNetwork::EventSourceOpen { conn_id, url,
///    last_event_id: None, origin: Some(page_origin), with_credentials }`
///    to the broker via `network_handle.send()`.  `origin` is the
///    document's origin via `VmInner::document_origin()` (the relevant
///    settings object's origin, HTML §9.2.2; opaque origins serialise as
///    `"null"`, so a sandboxed document does not leak its real origin).
///    A disconnected handle is a non-fatal config — the side-
///    table entry persists in CONNECTING and the broker's natural
///    reply path will eventually surface a `SseEvent::FatalError`
///    (or never, mirroring the WebSocket "no network" scenario).
#[allow(clippy::needless_pass_by_value)]
fn native_event_source_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Step 1: required `url` arg + parse against the document
    // base URL.
    let url_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to construct 'EventSource': 1 argument required, but only 0 present.",
        )
    })?;
    let url_sid = super::super::coerce::to_string(ctx.vm, url_arg)?;
    let url_str = ctx.vm.strings.get_utf8(url_sid);
    // SSE supports relative URLs against the document base — use
    // `Url::join` (which handles both absolute and relative inputs)
    // rather than `Url::parse` (absolute-only) per WHATWG HTML
    // §9.2.1 "Let urlRecord be the result of encoding-parsing a
    // URL given url, relative to settings".  Diverges from the WS
    // ctor (`websocket.rs`) which deliberately uses absolute-URL
    // parsing because the WS scheme has no document-base concept.
    let url = ctx.vm.navigation.current_url.join(&url_str).map_err(|e| {
        VmError::syntax_error(format!(
            "Failed to construct 'EventSource': The URL '{url_str}' is invalid: {e}"
        ))
    })?;

    // Step 2: `init.withCredentials` dict member read.
    let with_credentials = parse_event_source_init(ctx, args.get(1).copied())?;

    // Step 3+4: instance promotion + state install.
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::EventSource;

    // The EventSource request's `Origin` is the EventSource's **relevant**
    // settings object's origin (WHATWG HTML §9.2.2: the constructor sets the
    // request's client to `ev`'s relevant settings object) — opaque (`"null"`)
    // for a sandboxed doc.
    // Read the canonical `document_origin` resolver, not `current_url`
    // (S1b §5).  (`es_origin_string` below is the SSE *server* origin — a
    // distinct fact, unchanged.)
    let page_origin_str = ctx.vm.document_origin().serialize();
    let es_origin_string = url.origin().ascii_serialization();
    let origin_sid = ctx.vm.strings.intern(&es_origin_string);
    let url_serialized = url.as_str().to_owned();

    // Populate state + reverse map BEFORE the broker emit (mirror
    // the WS ctor's ordering: a broker that races us with an
    // immediate `FatalError` reply must find the wrapper through
    // the reverse map when `dispatch_realtime_event` routes the
    // event).
    let conn_id = {
        let hd = ctx.vm.host_data.as_deref_mut().ok_or_else(|| {
            VmError::type_error(
                "Failed to construct 'EventSource': VM is not bound to a HostData session",
            )
        })?;
        let conn_id = hd.alloc_sse_conn_id();
        hd.event_source_states.insert(
            inst_id,
            EventSourceState {
                ready_state: elidex_api_ws::SseReadyState::Connecting,
                url: url_serialized,
                origin_sid,
                with_credentials,
                last_event_id: String::new(),
                conn_id,
            },
        );
        hd.sse_conn_to_object.insert(conn_id, inst_id);
        conn_id
    };

    // Step 5: broker dispatch.  Disconnected / absent handle is
    // best-effort (same contract as the WS ctor).
    if let Some(handle) = ctx.vm.network_handle.as_ref() {
        let _ = handle.send(RendererToNetwork::EventSourceOpen {
            conn_id,
            url,
            last_event_id: None,
            origin: Some(page_origin_str),
            with_credentials,
        });
    }

    Ok(JsValue::Object(inst_id))
}

/// Parse the optional `init` dict argument per WHATWG HTML §9.2.1
/// + WebIDL §3.10 dictionary coercion.
///
/// - `undefined` / `null` / missing → default `EventSourceInit { withCredentials: false }`.
/// - Object → read `withCredentials` via the prototype chain
///   (`coerce::get_property`); apply `ToBoolean` (defaults to
///   `false` when the property is missing or `undefined`).
/// - Other primitive (Number / String / Boolean / Symbol) →
///   `TypeError` per WebIDL §3.10.6 "If V is not undefined and
///   V is not null and Type(V) is not Object, then throw a
///   TypeError exception."
fn parse_event_source_init(
    ctx: &mut NativeContext<'_>,
    init: Option<JsValue>,
) -> Result<bool, VmError> {
    let Some(val) = init else {
        return Ok(false);
    };
    match val {
        JsValue::Undefined | JsValue::Null => Ok(false),
        JsValue::Object(obj_id) => {
            let key = PropertyKey::String(ctx.vm.well_known.with_credentials);
            let raw = match super::super::coerce::get_property(ctx.vm, obj_id, key) {
                Some(super::super::coerce::PropertyResult::Data(v)) => v,
                Some(super::super::coerce::PropertyResult::Getter(g)) => {
                    // Honour user-side getter side-effects (matches
                    // WebIDL dictionary semantics + the EventInit
                    // precedent at `events.rs::parse_event_init`).
                    ctx.vm.call(g, JsValue::Object(obj_id), &[])?
                }
                None => JsValue::Undefined,
            };
            Ok(super::super::coerce::to_boolean(ctx.vm, raw))
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'EventSource': \
             EventSourceInit is not an object.",
        )),
    }
}

// ---------------------------------------------------------------------------
// Read-only accessors
// ---------------------------------------------------------------------------

fn native_es_get_ready_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_event_source_this(ctx, this, "readyState")?;
    let state = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.event_source_states.get(&id))
        .map_or(elidex_api_ws::SseReadyState::Closed, |s| s.ready_state);
    Ok(JsValue::Number(f64::from(state as u8)))
}

fn native_es_get_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_event_source_this(ctx, this, "url")?;
    let url_str: String = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.event_source_states.get(&id))
        .map(|s| s.url.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&url_str);
    Ok(JsValue::String(sid))
}

fn native_es_get_with_credentials(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_event_source_this(ctx, this, "withCredentials")?;
    let val = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.event_source_states.get(&id))
        .is_some_and(|s| s.with_credentials);
    Ok(JsValue::Boolean(val))
}

// ---------------------------------------------------------------------------
// `close()`
// ---------------------------------------------------------------------------

/// `EventSource.prototype.close()` per WHATWG HTML §9.2
/// "EventSource interface".
///
/// Idempotent: if already CLOSED, no-op.  Otherwise transitions
/// readyState to CLOSED and emits `EventSourceClose(conn_id)` so
/// the broker terminates the SSE I/O thread (no auto-reconnect
/// after a JS-initiated close, distinct from the transient-error
/// reconnect path).
///
/// No event fires for the JS-initiated close path (SSE has no
/// CloseEvent equivalent — `close` is purely a terminator).
fn native_es_close(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_event_source_this(ctx, this, "close")?;
    let (was_terminal, conn_id) = {
        let hd = ctx.vm.host_data.as_deref_mut().ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'close' on 'EventSource': VM is not bound to a session",
            )
        })?;
        let Some(state) = hd.event_source_states.get_mut(&id) else {
            // Already swept / unbound — idempotent no-op.
            return Ok(JsValue::Undefined);
        };
        let was_terminal = matches!(state.ready_state, elidex_api_ws::SseReadyState::Closed);
        if !was_terminal {
            let _ = state.transition_to(elidex_api_ws::SseReadyState::Closed);
        }
        (was_terminal, state.conn_id)
    };
    if was_terminal {
        return Ok(JsValue::Undefined);
    }
    if let Some(handle) = ctx.vm.network_handle.as_ref() {
        let _ = handle.send(RendererToNetwork::EventSourceClose(conn_id));
    }
    Ok(JsValue::Undefined)
}

// `addEventListener` / `removeEventListener` / `dispatchEvent` are
// inherited from `EventTarget.prototype` (the prototype is reparented in
// `register_event_source_global`) and route through the §2.9 VmObject
// dispatch core.  Server-named SSE events (`event: notification\n`)
// surface through `addEventListener("notification", cb)` as an emergent
// property of dispatching a `MessageEvent` of the parsed type at the
// shared core — the §9.2.6 "message vs named" fan-out is no longer
// hand-coded.  `on*` handlers install via the shared VmObject
// EventHandler backend (`install_vm_object_handler_attrs`).  The bespoke
// `event_listeners` registry + `es_on_handler!` macro were removed by
// `#11-realtime-event-listeners`.
