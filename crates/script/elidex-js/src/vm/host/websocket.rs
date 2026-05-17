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
//! ## Scope (Phase 1 + Phase 2)
//!
//! - `new WebSocket(url, protocols?)` — URL parse + WHATWG §9.3.1
//!   scheme promotion (`normalize_ws_url`) + validation
//!   (`validate_ws_url`) + RFC 7230 §3.2.6 token-ABNF protocols
//!   coercion + mixed-content gate (`is_mixed_content`) + eager
//!   `WebSocketOpen` to broker + side-table + reverse map insert.
//! - `readyState` accessor (CONNECTING=0 / OPEN=1 / CLOSING=2 /
//!   CLOSED=3).
//! - `send(data)` — full `(USVString | Blob | ArrayBuffer |
//!   ArrayBufferView)` IDL union.  Plain Object (no Blob / Buffer
//!   brand) raises TypeError per WebIDL §3.10.18.  State semantics:
//!   throw `InvalidStateError` ONLY on CONNECTING; CLOSING/CLOSED
//!   silently discard the transmission BUT still increment
//!   `bufferedAmount` by the encoded byte length (per WHATWG §9.3.4
//!   step 2-3).
//! - `close(code?, reason?)` — code u16 ∈ {1000} ∪ [3000,4999]
//!   (`InvalidAccessError` otherwise), reason ≤ 123 UTF-8 bytes
//!   (`SyntaxError` otherwise), idempotent if CLOSING/CLOSED.
//! - `binaryType` getter+setter — WebIDL `enum BinaryType { "blob",
//!   "arraybuffer" }`.  Setter ToString-coerces first (Symbol throws
//!   ECMA-262 TypeError), then enum-checks; any other string raises
//!   TypeError with the Chrome / Firefox parity message
//!   (spec-mandated; the boa reference silently ignored unknown
//!   strings).
//! - `onopen` / `onmessage` / `onerror` / `onclose` handler accessor
//!   pairs (FileReader callable-only retention precedent).
//! - `Connected` / `Closed` / `TextMessage` / `BinaryMessage` /
//!   `Error` / `BytesSent` broker events dispatch through
//!   `tick_network` → `dispatch_realtime_event` (see
//!   `vm/host/fetch_tick.rs` + `vm/host/websocket_dispatch.rs`).
//!
//! ### Asymmetry with EventSource: no `addEventListener` in this PR
//!
//! WebSocket inherits `EventTarget` per WHATWG §9.3, so spec-correct
//! `ws.addEventListener("message", cb)` should work — but this PR
//! ships the minimal shim ONLY on
//! [`super::event_source`] because SSE's named-event delivery
//! (`event: notification\ndata: ...`) is silently lost without it
//! (spec-violating user-data loss).  WebSocket's surface is
//! covered by `onopen` / `onmessage` / `onerror` / `onclose` alone
//! — losing the listener registry only drops the redundant
//! handler-mode access, not protocol-level data.  Full
//! `EventTarget` integration for both surfaces is tracked as
//! defer slot `#11-realtime-event-listeners`.
//!
//! Phase 3 (EventSource) and Phase 4 (constants / cross-cutting
//! polish) land in follow-up commits on the same branch.
//!
//! ### Spec divergence note: `close()` with no code argument
//!
//! WHATWG §9.3.4 specifies that calling `close()` with no `code`
//! argument should produce a close frame with no body (the remote
//! observes `CloseEvent.code == 1005`).  The broker's
//! `WsCommand::Close(u16, String)` requires a `u16`, so Phase 1
//! always sends `Close(1000, "")` for the bare `close()` case.  This
//! is a wire-visible divergence — the remote sees code 1000 instead
//! of "no status received".  Full §9.3.4 fidelity would require the
//! broker to support a no-body close variant; tracked separately if
//! WPT calls it out.

#![cfg(feature = "engine")]

use elidex_net::broker::RendererToNetwork;
use elidex_net::ws::WsCommand;

use super::super::host_data::{BinaryType, WebSocketState};
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
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

        self.install_websocket_members(proto_id);

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

    fn install_websocket_members(&mut self, proto_id: ObjectId) {
        // Readonly accessors: readyState / url / protocol / extensions /
        // bufferedAmount.  `binaryType` is mutable and installed via
        // its own accessor pair below.
        let ro_accessors: [(StringId, NativeFn); 5] = [
            (self.well_known.ready_state, native_ws_get_ready_state),
            (self.well_known.url, native_ws_get_url),
            (self.well_known.protocol, native_ws_get_protocol),
            (self.well_known.ws_extensions, native_ws_get_extensions),
            (
                self.well_known.buffered_amount,
                native_ws_get_buffered_amount,
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

        // `binaryType` — getter + setter pair.  Setter throws TypeError
        // on any non-{"blob","arraybuffer"} value per WebIDL §3.10.21
        // enum coercion (spec-mandated; the boa reference silently
        // ignored unknown strings).
        self.install_accessor_pair(
            proto_id,
            self.well_known.binary_type,
            native_ws_get_binary_type,
            Some(native_ws_set_binary_type),
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Event handler attributes: 4 pairs (onopen / onmessage /
        // onerror / onclose).  WHATWG IDL EventHandler attribute —
        // callable-only retention (non-callable assignments null the
        // slot).  The macro factors the boilerplate for the 4
        // structurally-identical pairs.
        let on_handlers: [(StringId, NativeFn, NativeFn); 4] = [
            (
                self.well_known.onopen,
                native_ws_get_onopen,
                native_ws_set_onopen,
            ),
            (
                self.well_known.onmessage,
                native_ws_get_onmessage,
                native_ws_set_onmessage,
            ),
            (
                self.well_known.onerror,
                native_ws_get_onerror,
                native_ws_set_onerror,
            ),
            (
                self.well_known.onclose,
                native_ws_get_onclose,
                native_ws_set_onclose,
            ),
        ];
        for (name_sid, getter, setter) in on_handlers {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Methods: send + close.
        let methods: [(StringId, NativeFn); 2] = [
            (self.well_known.ws_send_method, native_ws_send),
            (self.well_known.close, native_ws_close),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
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
// Constructor — `new WebSocket(url, protocols?)`
// ---------------------------------------------------------------------------

/// `new WebSocket(url, protocols?)` per WHATWG WebSockets §9.3.1.
///
/// Steps:
/// 1. Coerce `url` arg via `ToString`; parse via `url::Url::parse` —
///    SyntaxError on parse failure (§9.3.1 "throw a SyntaxError").
/// 2. `elidex_api_ws::normalize_ws_url(&mut url)` — `http`→`ws` /
///    `https`→`wss` promotion (§9.3.1 step 6).
/// 3. `elidex_api_ws::validate_ws_url(&url)` — scheme check (ws/wss
///    only), fragment rejection (§9.3.1 step 7), engine-local SSRF
///    gate.  Any failure → SyntaxError.
/// 4. Parse `protocols` arg (`DOMString | sequence<DOMString>`) and
///    validate each against RFC 7230 §3.2.6 `<token>` ABNF.  Empty /
///    non-token / duplicate → SyntaxError.
/// 5. Mixed-content gate via `is_mixed_content(page_scheme, ws_url)`
///    — secure page + `ws://` target → `SecurityError`.
/// 6. Allocate `conn_id` via `HostData::alloc_ws_conn_id()`.
/// 7. Promote `this` to `ObjectKind::WebSocket`; insert
///    `WebSocketState { ready_state: Connecting, url, conn_id, ... }`
///    into `host_data.websocket_states`; populate reverse map
///    `ws_conn_to_object[conn_id] = id`.
/// 8. Send `RendererToNetwork::WebSocketOpen { conn_id, url,
///    protocols, origin }` to the broker via `network_handle.send()`.
///    Origin is `vm.navigation.current_url.origin().ascii_serialization()`
///    (opaque origins serialise as `"null"` per WHATWG URL §3.5).  A
///    disconnected handle (no `network_handle` installed OR `send`
///    returns false) is a non-fatal config — the side-table entry
///    persists in CONNECTING and the broker's natural reply path will
///    eventually surface a `WsEvent::Closed` (or never, if there's no
///    broker at all — JS sees a perpetually-CONNECTING socket which
///    matches a real-browser "no network" scenario).
#[allow(clippy::needless_pass_by_value)]
fn native_websocket_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'WebSocket': Please use the 'new' operator",
        ));
    }

    // Step 1: required `url` arg + parse.
    let url_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to construct 'WebSocket': 1 argument required, but only 0 present.",
        )
    })?;
    let url_sid = super::super::coerce::to_string(ctx.vm, url_arg)?;
    let url_str = ctx.vm.strings.get_utf8(url_sid);
    let mut url = url::Url::parse(&url_str).map_err(|e| {
        VmError::syntax_error(format!(
            "Failed to construct 'WebSocket': The URL '{url_str}' is invalid: {e}"
        ))
    })?;

    // Steps 2-3: scheme normalize → validate (scheme/fragment/SSRF).
    elidex_api_ws::normalize_ws_url(&mut url)
        .map_err(|e| VmError::syntax_error(format!("Failed to construct 'WebSocket': {e}")))?;
    elidex_api_ws::validate_ws_url(&url)
        .map_err(|e| VmError::syntax_error(format!("Failed to construct 'WebSocket': {e}")))?;

    // Step 4: protocols coercion + token-ABNF validation.
    let protocols_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let protocols = parse_protocols(ctx, protocols_arg)?;

    // Step 5: mixed-content gate.  Page scheme comes from the active
    // browsing-context URL; `is_mixed_content` returns true only when
    // page is `https` AND ws URL is `ws://` (not `wss://`).
    let page_scheme = ctx.vm.navigation.current_url.scheme().to_owned();
    if elidex_api_ws::is_mixed_content(&page_scheme, &url) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_security_error,
            "Failed to construct 'WebSocket': An insecure WebSocket connection \
             may not be initiated from a page loaded over HTTPS.",
        ));
    }

    // Step 6: instance promotion.  `this` was pre-allocated by `do_new`
    // with `WebSocket.prototype` in its proto chain so subclasses
    // (`class Sub extends WebSocket { ... }`) keep the right
    // `Sub.prototype` link.
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::WebSocket;

    // Steps 7-8: allocate conn_id, install state, emit WebSocketOpen.
    // Two distinct origins are involved:
    // - `page_origin_str` — the active browsing-context's WHATWG
    //   origin; sent to the broker as `WebSocketOpen.origin` per
    //   §9.3.1 step 13 ("origin of the entry settings object").
    //   Opaque origins serialise as `"null"`.
    // - `ws_origin_sid` — the SERVER's origin (derived from `url`);
    //   pre-interned here so per-message `MessageEvent.origin`
    //   dispatch reads a `StringId` without re-parsing per WHATWG
    //   §9.3.7.
    let page_origin_str = ctx.vm.navigation.current_url.origin().ascii_serialization();
    let ws_origin_string = url.origin().ascii_serialization();
    let ws_origin_sid = ctx.vm.strings.intern(&ws_origin_string);
    let url_serialized = url.as_str().to_owned();

    // Grab the conn_id + populate state-and-reverse-map BEFORE emitting
    // the broker message: if the broker thread races the renderer and
    // emits a `WsEvent::Closed` immediately (e.g. SSRF block on the
    // broker side post-open), the reverse map must already be live so
    // `dispatch_realtime_event` finds the wrapper.
    let conn_id = {
        let hd = ctx.vm.host_data.as_deref_mut().ok_or_else(|| {
            VmError::type_error(
                "Failed to construct 'WebSocket': VM is not bound to a HostData session",
            )
        })?;
        let conn_id = hd.alloc_ws_conn_id();
        hd.websocket_states.insert(
            inst_id,
            WebSocketState {
                ready_state: elidex_api_ws::WsReadyState::Connecting,
                url: url_serialized,
                origin_sid: ws_origin_sid,
                protocol: String::new(),
                extensions: String::new(),
                buffered_amount: 0,
                binary_type: super::super::host_data::BinaryType::default(),
                conn_id,
                onopen: None,
                onmessage: None,
                onerror: None,
                onclose: None,
            },
        );
        hd.ws_conn_to_object.insert(conn_id, inst_id);
        conn_id
    };

    // Broker dispatch.  A disconnected / absent handle is best-effort:
    // `send` returns false; the side-table entry stays in CONNECTING
    // and Vm-side close() / GC sweep will still emit a teardown
    // `WebSocketClose` later (a no-op on a dead handle).
    if let Some(handle) = ctx.vm.network_handle.as_ref() {
        let _ = handle.send(RendererToNetwork::WebSocketOpen {
            conn_id,
            url,
            protocols,
            origin: page_origin_str,
        });
    }

    Ok(JsValue::Object(inst_id))
}

/// Parse `protocols` argument per WebIDL `DOMString | sequence<DOMString>`.
///
/// - `undefined` / `null` / missing → empty list.
/// - `ObjectKind::Array` (fast path) → iterate the backing `elements`
///   `Vec` directly; matches the `blob.rs::collect_blob_parts_bytes`
///   precedent.  Note this is NOT a full iterator-protocol consumer
///   (Set / Map / user iterables fall through to the String branch
///   below — Phase 1 accepts Array literals + DOMString only, the
///   real-world case for `new WebSocket(url, protocols)`).
/// - Any other value (String, number, plain Object, etc.) → coerce
///   via `ToString` and treat as a single-element list, per WebIDL
///   union-type resolution against the `DOMString` arm.
///
/// Each element is validated against RFC 7230 §3.2.6 `<token>` ABNF
/// (`is_valid_protocol_token_char`); empty / non-token / duplicate
/// values throw `SyntaxError` per WHATWG WebSockets §9.3.1.
fn parse_protocols(ctx: &mut NativeContext<'_>, arg: JsValue) -> Result<Vec<String>, VmError> {
    let mut out: Vec<String> = Vec::new();

    match arg {
        JsValue::Undefined | JsValue::Null => return Ok(out),
        JsValue::Object(obj_id) => {
            // Array fast path — same shape as `blob.rs` collects
            // BlobParts.  `coerce::get_property` does NOT auto-index
            // ObjectKind::Array dense `elements`, so the
            // mutation_observer-style indexed-property loop would
            // silently read `undefined` for each slot.  Cloning the
            // `elements` Vec is necessary because we then call back
            // into the VM (to_string) which may mutate the heap.
            if let ObjectKind::Array { ref elements } = ctx.vm.get_object(obj_id).kind {
                let snapshot = elements.clone();
                for item in snapshot {
                    let s_sid = super::super::coerce::to_string(ctx.vm, item)?;
                    out.push(ctx.vm.strings.get_utf8(s_sid));
                }
            } else {
                // Plain Object (`{ length: 0 }` etc.) — WebIDL would
                // try the sequence arm via the iterator protocol.
                // Phase 1 falls back to ToString single-element to
                // keep the dependency surface small.
                let s_sid = super::super::coerce::to_string(ctx.vm, arg)?;
                out.push(ctx.vm.strings.get_utf8(s_sid));
            }
        }
        _ => {
            // Primitive (string / number / boolean) → DOMString
            // single-element per WebIDL union resolution.
            let s_sid = super::super::coerce::to_string(ctx.vm, arg)?;
            out.push(ctx.vm.strings.get_utf8(s_sid));
        }
    }

    // RFC 7230 §3.2.6 token-ABNF validation + duplicate check.  Done
    // post-collect so a SyntaxError surfaces the offending value
    // rather than failing mid-iteration.
    for s in &out {
        if s.is_empty() {
            return Err(VmError::syntax_error(
                "Failed to construct 'WebSocket': empty sub-protocol is not allowed",
            ));
        }
        if !s.chars().all(is_valid_protocol_token_char) {
            return Err(VmError::syntax_error(format!(
                "Failed to construct 'WebSocket': sub-protocol '{s}' \
                 contains a character not allowed by RFC 7230 token ABNF"
            )));
        }
    }
    // O(n²) duplicate detection — protocol lists are tiny (1-5 entries
    // in practice); a HashSet would burn more allocation than it saves.
    for (i, a) in out.iter().enumerate() {
        for b in &out[i + 1..] {
            if a == b {
                return Err(VmError::syntax_error(format!(
                    "Failed to construct 'WebSocket': duplicate sub-protocol '{a}'"
                )));
            }
        }
    }

    Ok(out)
}

/// RFC 7230 §3.2.6 `<token>` character predicate.
///
/// `tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" /
///          "." / "^" / "_" / `` ` `` / "|" / "~" / DIGIT / ALPHA`
///
/// Equivalently: visible ASCII (0x21..=0x7E) EXCLUDING the 16
/// separators `( ) < > @ , ; : \ " / [ ] ? = { }`.  Space (0x20) and
/// horizontal tab (0x09) are excluded automatically by the lower
/// bound 0x21.
fn is_valid_protocol_token_char(c: char) -> bool {
    let b = c as u32;
    (0x21..=0x7E).contains(&b)
        && !matches!(
            c,
            '"' | '('
                | ')'
                | ','
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '?'
                | '@'
                | '['
                | '\\'
                | ']'
                | '{'
                | '}'
        )
}

// ---------------------------------------------------------------------------
// Read-only accessors
// ---------------------------------------------------------------------------

fn native_ws_get_ready_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "readyState")?;
    let state = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map_or(elidex_api_ws::WsReadyState::Closed, |s| s.ready_state);
    Ok(JsValue::Number(f64::from(state as u8)))
}

fn native_ws_get_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "url")?;
    let url_str: String = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map(|s| s.url.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&url_str);
    Ok(JsValue::String(sid))
}

fn native_ws_get_protocol(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "protocol")?;
    let val: String = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map(|s| s.protocol.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

fn native_ws_get_extensions(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "extensions")?;
    let val: String = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map(|s| s.extensions.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

fn native_ws_get_buffered_amount(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "bufferedAmount")?;
    let n = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map_or(0_u64, |s| s.buffered_amount);
    // `unsigned long long` → JS Number.  Values above 2^53 lose
    // precision per the §9.3 IDL note ("the implementation might
    // approximate", same as Chrome/Firefox).  See defer slot
    // `#11-ws-buffered-amount-precision` for the BigInt path if WPT
    // ever fails on high-throughput streams.
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(n as f64))
}

// ---------------------------------------------------------------------------
// `binaryType` accessor — getter + setter
// ---------------------------------------------------------------------------

/// `WebSocket.prototype.binaryType` getter — read the current
/// [`BinaryType`] from the side-table and return the matching IDL
/// enum string (`"blob"` or `"arraybuffer"`).
fn native_ws_get_binary_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "binaryType")?;
    let bt = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.websocket_states.get(&id))
        .map_or(BinaryType::default(), |s| s.binary_type);
    let sid = match bt {
        BinaryType::Blob => ctx.vm.well_known.binary_type_blob,
        BinaryType::ArrayBuffer => ctx.vm.well_known.binary_type_arraybuffer,
    };
    Ok(JsValue::String(sid))
}

/// `WebSocket.prototype.binaryType` setter per WHATWG WebSockets
/// §9.3 + WebIDL §3.10.21 enum setter:
///
/// 1. `ToString(v)` — propagates ECMA-262 TypeError for `Symbol`
///    values (the `?` on `to_string` does this BEFORE the enum
///    check fires, which is the spec-mandated ordering).
/// 2. Compare the resulting `StringId` against the pre-interned
///    `binary_type_blob` / `binary_type_arraybuffer` slots (cheap
///    integer equality — no UTF-8 walk).
/// 3. Any other string → `TypeError` with the Chrome / Firefox
///    parity wording (spec-mandated; the boa reference silently
///    ignored unknown values).
fn native_ws_set_binary_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "binaryType")?;
    let new_val = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL §3.10.21 step 1: ToString first.  Symbol throws here
    // (correct spec ordering — enum-check TypeError must not pre-empt
    // the ToString TypeError).
    let s_sid = super::super::coerce::to_string(ctx.vm, new_val)?;
    let bt = if s_sid == ctx.vm.well_known.binary_type_blob {
        BinaryType::Blob
    } else if s_sid == ctx.vm.well_known.binary_type_arraybuffer {
        BinaryType::ArrayBuffer
    } else {
        // Echo the bad string in the error message per WebIDL
        // convention.  Allocates a String — only on the error path
        // (rare).
        let raw = ctx.vm.strings.get_utf8(s_sid);
        return Err(VmError::type_error(format!(
            "Failed to set the 'binaryType' property on 'WebSocket': \
             The provided value '{raw}' is not a valid enum value of type BinaryType."
        )));
    };
    if let Some(hd) = ctx.vm.host_data.as_deref_mut() {
        if let Some(state) = hd.websocket_states.get_mut(&id) {
            state.binary_type = bt;
        }
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// `send(data)` — `(USVString or Blob or ArrayBuffer or ArrayBufferView)`
// ---------------------------------------------------------------------------

/// `WebSocket.prototype.send(data)` per WHATWG WebSockets §9.3.4.
///
/// Accepts the full IDL union `(USVString | Blob | ArrayBuffer |
/// ArrayBufferView)`; non-Object primitives (`number` / `boolean` /
/// `null` / `undefined`) coerce to `USVString` via the union's
/// DOMString arm.  Plain `Object` values (not branding as Blob /
/// ArrayBuffer / TypedArray / DataView) raise a `TypeError` —
/// WebIDL §3.10.18 union resolution does NOT have an implicit
/// Object → DOMString fall-through.
///
/// State semantics per WHATWG §9.3.4:
/// - `CONNECTING` → throw `InvalidStateError` BEFORE any coercion
///   (spec step 1 runs before step 2 "extract bytes").
/// - `OPEN` → transmit + `bufferedAmount.saturating_add(byte_len)`.
/// - `CLOSING` / `CLOSED` → silently DROP the transmission but STILL
///   increment `bufferedAmount` per spec §9.3.4 step 2-3 (the spec
///   says the data is "queued" even when no longer transmitted, so
///   observers can detect undelivered bytes).
fn native_ws_send(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "send")?;
    let data_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to execute 'send' on 'WebSocket': 1 argument required, but only 0 present.",
        )
    })?;

    // State check FIRST per WHATWG §9.3.4 step 1 — before any
    // coercion that may throw (e.g. `ToString` on `Symbol`).
    let ready_state = {
        let hd = ctx.vm.host_data.as_deref().ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'send' on 'WebSocket': VM is not bound to a session",
            )
        })?;
        hd.websocket_states
            .get(&id)
            .map_or(elidex_api_ws::WsReadyState::Closed, |s| s.ready_state)
    };
    if matches!(ready_state, elidex_api_ws::WsReadyState::Connecting) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'send' on 'WebSocket': Still in CONNECTING state.",
        ));
    }

    // Extract bytes + frame kind per WebIDL union resolution.  Order
    // mirrors WebIDL §3.10.18 distinguishability for `(USVString or
    // Blob or ArrayBuffer or ArrayBufferView)`:
    //   1. Object brand-test: Blob / File → Blob arm; ArrayBuffer /
    //      TypedArray / DataView → BufferSource arm.
    //   2. Plain Object (no matching brand) → TypeError (NO implicit
    //      fall-through to DOMString — spec §3.10.18 step 8).
    //   3. String → USVString.
    //   4. Other primitive (number / bool / null / undefined / Symbol)
    //      → USVString via `ToString` (Symbol throws TypeError here,
    //      matching Chrome / Firefox).
    let frame = match data_arg {
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::Blob | ObjectKind::File => {
                // Infallible `Arc<[u8]>` clone — Blob bytes are
                // fully in-memory (see `#11-body-data-arc-cow`
                // defer slot for zero-copy backing).
                let bytes = super::blob::blob_bytes(ctx.vm, obj_id).to_vec();
                Frame::Binary(bytes)
            }
            ObjectKind::ArrayBuffer
            | ObjectKind::TypedArray { .. }
            | ObjectKind::DataView { .. } => {
                let bytes = super::text_encoding::extract_buffer_source_bytes(
                    ctx,
                    data_arg,
                    "Failed to execute 'send' on 'WebSocket'",
                    1,
                    false,
                )?;
                Frame::Binary(bytes)
            }
            _ => {
                return Err(VmError::type_error(
                    "Failed to execute 'send' on 'WebSocket': \
                     The provided value is not of type \
                     '(Blob or ArrayBuffer or ArrayBufferView or USVString)'.",
                ));
            }
        },
        JsValue::String(sid) => Frame::Text(ctx.vm.strings.get_utf8(sid)),
        _ => {
            let sid = super::super::coerce::to_string(ctx.vm, data_arg)?;
            Frame::Text(ctx.vm.strings.get_utf8(sid))
        }
    };

    let byte_len = frame.byte_len();

    // Snapshot conn_id and apply bufferedAmount delta UNCONDITIONALLY
    // (CLOSING/CLOSED keep the increment even though no transmission
    // occurs — see state-semantics note in the function's doc-
    // comment).  Done BEFORE the broker send so a disconnected
    // handle still leaves the observer-visible counter consistent.
    let conn_id = {
        let hd = ctx.vm.host_data.as_deref_mut().ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'send' on 'WebSocket': VM is not bound to a session",
            )
        })?;
        if let Some(state) = hd.websocket_states.get_mut(&id) {
            state.buffered_amount = state.buffered_amount.saturating_add(byte_len);
            Some(state.conn_id)
        } else {
            None
        }
    };

    // Only OPEN actually transmits.  CLOSING/CLOSED keep the
    // bufferedAmount increment above but do NOT emit a broker command
    // (matches §9.3.4 step 3: "do not queue/transmit, but DO
    // increase bufferedAmount").
    if matches!(ready_state, elidex_api_ws::WsReadyState::Open) {
        if let (Some(conn_id), Some(handle)) = (conn_id, ctx.vm.network_handle.as_ref()) {
            let cmd = match frame {
                Frame::Text(s) => WsCommand::SendText(s),
                Frame::Binary(bytes) => WsCommand::SendBinary(bytes),
            };
            let _ = handle.send(RendererToNetwork::WebSocketSend(conn_id, cmd));
        }
    }

    Ok(JsValue::Undefined)
}

/// Resolved `send(data)` payload — Text drives `WsCommand::SendText`,
/// Binary drives `WsCommand::SendBinary`.  Pre-computed in the union-
/// resolution match so the OPEN-state emit is a single branch on a
/// fully-decoded payload (no re-inspection of `data_arg`).
enum Frame {
    Text(String),
    Binary(Vec<u8>),
}

impl Frame {
    fn byte_len(&self) -> u64 {
        match self {
            Self::Text(s) => s.len() as u64,
            Self::Binary(b) => b.len() as u64,
        }
    }
}

// ---------------------------------------------------------------------------
// `close(code?, reason?)`
// ---------------------------------------------------------------------------

/// `WebSocket.prototype.close(code?, reason?)` per WHATWG WebSockets
/// §9.3.4.
///
/// Validation (runs BEFORE the state check so a CLOSING/CLOSED
/// receiver still surfaces a bad-arg `InvalidAccessError` /
/// `SyntaxError` — matches Chrome/Firefox):
/// - `code` `u16`, if present, must equal 1000 or fall in 3000..=4999.
///   Anything else → `InvalidAccessError` (per §9.3.4 step 1).
/// - `reason` `USVString`, if present, must be ≤ 123 **UTF-8 bytes**
///   (NOT char count) → `SyntaxError` otherwise.
///
/// After validation:
/// - If already `CLOSING` or `CLOSED`, no-op (idempotent).
/// - Otherwise, transition to `CLOSING` and emit
///   `WebSocketSend(conn_id, WsCommand::Close(code, reason))` so the
///   broker drives the close-handshake frame and ultimately surfaces
///   `WsEvent::Closed{code, reason, was_clean}` back through
///   `dispatch_realtime_event`.
///
/// Spec divergence: when `code` is omitted, this Phase 1 impl sends
/// `Close(1000, "")` (see module-level doc).  Wire-observable code on
/// the remote will be 1000, not the spec's "no status received"
/// (1005).
fn native_ws_close(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_websocket_this(ctx, this, "close")?;

    // ---- Code arg coercion + range check ----
    // WebIDL `unsigned short` semantics — `ToNumber` then modulo-2^16
    // truncation, no `[EnforceRange]`.  But the spec close algorithm
    // says: "If code is present, but is neither an integer equal to
    // 1000 nor an integer in the range 3000 to 4999, inclusive, throw
    // an InvalidAccessError DOMException."  Coerce to u16 via
    // ToUint16, then check the literal value (post-mod-2^16) against
    // the spec's allowed ranges.
    let (code, code_provided) = match args.first().copied() {
        None | Some(JsValue::Undefined) => (1000_u16, false),
        Some(v) => (super::super::coerce::to_uint16(ctx.vm, v)?, true),
    };
    if code_provided && !matches!(code, 1000 | 3000..=4999) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_access_error,
            format!(
                "Failed to execute 'close' on 'WebSocket': \
                 The close code must be 1000 or in the range 3000-4999, got {code}."
            ),
        ));
    }

    // ---- Reason arg coercion + 123-UTF-8-byte cap ----
    let reason = match args.get(1).copied() {
        None | Some(JsValue::Undefined) => String::new(),
        Some(v) => {
            let sid = super::super::coerce::to_string(ctx.vm, v)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    // Spec: "must be no longer than 123 BYTES when encoded in UTF-8."
    // Rust `String::len()` returns byte length — the right metric.
    if reason.len() > 123 {
        return Err(VmError::syntax_error(
            "Failed to execute 'close' on 'WebSocket': \
             The close reason must not be longer than 123 UTF-8 bytes.",
        ));
    }

    // ---- State transition + broker emit ----
    // Snapshot current state + conn_id, transition, drop borrow before
    // touching `network_handle` (which is a separate field on
    // `VmInner`) — keeps the borrow ordering obvious.
    let (was_terminal, conn_id) = {
        let hd = ctx.vm.host_data.as_deref_mut().ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'close' on 'WebSocket': VM is not bound to a session",
            )
        })?;
        let Some(state) = hd.websocket_states.get_mut(&id) else {
            // Side-table entry missing — already swept or never
            // populated.  Idempotent no-op matches the
            // already-CLOSED branch below.
            return Ok(JsValue::Undefined);
        };
        let was_terminal = matches!(
            state.ready_state,
            elidex_api_ws::WsReadyState::Closing | elidex_api_ws::WsReadyState::Closed
        );
        if !was_terminal {
            // `transition_to` accepts Connecting/Open → Closing per the
            // state-machine matrix at `host_data.rs::WebSocketState::
            // transition_to`.  Both source states are legal here.
            let _ = state.transition_to(elidex_api_ws::WsReadyState::Closing);
        }
        (was_terminal, state.conn_id)
    };
    if was_terminal {
        // Idempotent close — already in CLOSING/CLOSED, drop the
        // broker emit and return.
        return Ok(JsValue::Undefined);
    }

    if let Some(handle) = ctx.vm.network_handle.as_ref() {
        let _ = handle.send(RendererToNetwork::WebSocketSend(
            conn_id,
            WsCommand::Close(code, reason),
        ));
    }

    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Event handler attribute accessors — `onopen` / `onmessage` /
// `onerror` / `onclose`.
//
// The macro pattern mirrors FileReader's `fr_on_handler!` at
// `file_reader.rs:384-420`.  Four structurally-identical pairs at
// this scope; the macro avoids ~200 lines of duplicated body
// without sacrificing call-site visibility (the expansion is
// single-screen).
// ---------------------------------------------------------------------------

macro_rules! ws_on_handler {
    ($getter:ident, $setter:ident, $field:ident, $name:literal) => {
        fn $getter(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let id = require_websocket_this(ctx, this, $name)?;
            Ok(ctx
                .vm
                .host_data
                .as_deref()
                .and_then(|hd| hd.websocket_states.get(&id))
                .and_then(|s| s.$field)
                .map_or(JsValue::Null, JsValue::Object))
        }
        fn $setter(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let id = require_websocket_this(ctx, this, $name)?;
            let new_val = args.first().copied().unwrap_or(JsValue::Undefined);
            // Per WHATWG EventHandler IDL §8.1.7.2: only callable
            // values are retained; any other value nulls the slot
            // (matches Chrome/Firefox).
            let stored = match new_val {
                JsValue::Object(obj_id) if ctx.vm.get_object(obj_id).kind.is_callable() => {
                    Some(obj_id)
                }
                _ => None,
            };
            if let Some(hd) = ctx.vm.host_data.as_deref_mut() {
                if let Some(state) = hd.websocket_states.get_mut(&id) {
                    state.$field = stored;
                }
            }
            Ok(JsValue::Undefined)
        }
    };
}

ws_on_handler!(native_ws_get_onopen, native_ws_set_onopen, onopen, "onopen");
ws_on_handler!(
    native_ws_get_onmessage,
    native_ws_set_onmessage,
    onmessage,
    "onmessage"
);
ws_on_handler!(
    native_ws_get_onerror,
    native_ws_set_onerror,
    onerror,
    "onerror"
);
ws_on_handler!(
    native_ws_get_onclose,
    native_ws_set_onclose,
    onclose,
    "onclose"
);
