//! D-12 `#11-net-ws-sse` Phase 1 + Phase 2 — `WebSocket` JS thin
//! binding tests.
//!
//! Coverage:
//! - Constructor success path + WHATWG §9.3.1 validation (URL parse /
//!   scheme promotion / fragment / mixed-content / protocols token
//!   ABNF / duplicate).
//! - `readyState` accessor + CONNECTING-on-construction invariant.
//! - `send(data)` for the full `(USVString | Blob | ArrayBuffer |
//!   ArrayBufferView)` union + CRIT-2 state semantics (throw on
//!   CONNECTING, silent-discard on CLOSING/CLOSED with
//!   `bufferedAmount` increment) + WebIDL union-resolution dispatch
//!   matrix (plain Object → TypeError, primitive → ToString).
//! - `close(code?, reason?)` validation matrix + idempotency.
//! - `binaryType` enum getter / setter with IMP-2 TypeError on
//!   non-`{"blob","arraybuffer"}` strings.
//! - `onopen` / `onmessage` / `onerror` / `onclose` handler accessor
//!   pairs (callable-only retention).
//! - `Connected` / `Closed` / `TextMessage` / `BinaryMessage` /
//!   `Error` / `BytesSent` broker events dispatched through
//!   `tick_network` → `dispatch_realtime_event`.
//! - MessageEvent slot population (data / origin / lastEventId /
//!   source / ports) including MIN-5 regression: `origin` is the
//!   **ws-URL** origin, NOT the page origin.
//! - `addEventListener` (EventTarget) integration for the broker events.
//! - Brand checks: prototype methods and accessors on a non-WebSocket
//!   receiver throw TypeError.
//!
//! Split by section (pure move, R-0d-pre): [`ctor`], [`ready_state`],
//! [`send`], [`close`], [`handlers`], [`brand`], [`binary_type`],
//! [`messages`], [`events`].

#![cfg(feature = "engine")]

mod binary_type;
mod brand;
mod close;
mod ctor;
mod events;
mod handlers;
mod messages;
mod ready_state;
mod send;

use std::rc::Rc;

use elidex_ecs::{Attributes, EcsDom};
use elidex_net::broker::{NetworkHandle, NetworkToRenderer};
use elidex_net::ws::WsEvent;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::{assert_eval_bool, assert_eval_number, assert_eval_string};

/// Minimal `doc` fixture for tests requiring a bound HostData session
/// (the WebSocket constructor demands one for the side-table writes).
fn build_min_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    let body = dom.create_element_ownerless_fixture("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// Run `f` against a VM bound to a fresh `HostData` session with a
/// mock `NetworkHandle` installed and the navigation URL pointing at
/// either `http://example.com/page` (for protocols / happy-path
/// tests) or `https://example.com/page` (for the mixed-content
/// gate).  The `UnbindOnDrop` guard tears down the bind before the
/// `session` / `dom` owners go out of scope, satisfying `bind_vm`'s
/// outlives-this-call contract.
fn with_ws_vm<F, R>(https: bool, f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let scheme = if https { "https" } else { "http" };
    vm.inner.navigation.current_url =
        url::Url::parse(&format!("{scheme}://example.com/page")).expect("valid base URL");
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(vec![])));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    let result = f(guard.0);
    drop(guard);
    drop(session);
    drop(dom);
    result
}

/// Inject a `WsEvent` for `conn_id` and drive a network tick — the
/// dispatch helper inside `tick_network` will route the event to the
/// matching wrapper via the reverse map.
fn inject_ws_event_and_tick(vm: &mut Vm, conn_id: u64, ev: WsEvent) {
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    handle.rebuffer_events(vec![NetworkToRenderer::WebSocketEvent(conn_id, ev)]);
    vm.tick_network();
}

/// Helper: open a WS connection and transition it to OPEN so tests
/// can exercise the OPEN-state branch of `send`.
fn open_ws(vm: &mut Vm) {
    vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
        .unwrap();
    inject_ws_event_and_tick(
        vm,
        0,
        WsEvent::Connected {
            protocol: String::new(),
            extensions: String::new(),
        },
    );
}
