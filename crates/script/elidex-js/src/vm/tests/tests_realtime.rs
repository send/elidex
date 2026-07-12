//! D-12 `#11-net-ws-sse` Phase 4 — cross-cutting tests covering the
//! shared surface between [`WebSocket`] and [`EventSource`]:
//!
//! - WebIDL `const unsigned short` install discipline — constants
//!   live on the constructor object AND the prototype, NOT as
//!   instance own-properties (instances inherit through the proto
//!   chain).  WebSocket: CONNECTING=0 / OPEN=1 / CLOSING=2 /
//!   CLOSED=3; EventSource: CONNECTING=0 / OPEN=1 / CLOSED=2
//!   (no CLOSING in the SSE 3-state machine).
//! - `instanceof` parity for both interfaces.
//! - `structuredClone` raises `DataCloneError` for either kind
//!   (per WHATWG HTML §2.8 "Cloneable objects" — WS / SSE are not
//!   on the cloneable list).
//! - `Vm::unbind` CRIT-A teardown — every active WebSocket /
//!   EventSource conn_id emits `WebSocketClose` / `EventSourceClose`
//!   to the outgoing handle BEFORE the side-tables are cleared,
//!   matching the [`super::super::host::fetch_tick`]
//!   `reject_pending_fetches_with_error` shape.
//!
//! Test infra notes: shares the [`build_min_doc`] / `UnbindOnDrop`
//! / `with_realtime_vm` pattern with `tests_websocket.rs` and
//! `tests_event_source.rs`; deduplication is deferred until a
//! third sibling lands (current 2-site precedent doesn't warrant
//! hoisting).

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_ecs::{Attributes, EcsDom};
use elidex_net::broker::NetworkHandle;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::{assert_eval_bool, assert_eval_number};

fn build_min_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
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

/// Bind a VM with a mock NetworkHandle + `http://example.com/page`
/// navigation URL and run `f`.  The `UnbindOnDrop` guard tears
/// down the bind on scope exit so the test never panics in
/// `bind_vm`'s outlives-this-call contract.  Returns whatever
/// `f` returns.
fn with_realtime_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page").expect("valid base URL");
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

// ---------------------------------------------------------------------------
// Constants: installed on ctor + prototype, NOT on instances
// ---------------------------------------------------------------------------

#[test]
fn ws_constants_on_constructor() {
    with_realtime_vm(|vm| {
        assert_eval_number(vm, "WebSocket.CONNECTING", 0.0);
        assert_eval_number(vm, "WebSocket.OPEN", 1.0);
        assert_eval_number(vm, "WebSocket.CLOSING", 2.0);
        assert_eval_number(vm, "WebSocket.CLOSED", 3.0);
    });
}

#[test]
fn ws_constants_on_prototype() {
    with_realtime_vm(|vm| {
        assert_eval_number(vm, "WebSocket.prototype.CONNECTING", 0.0);
        assert_eval_number(vm, "WebSocket.prototype.OPEN", 1.0);
        assert_eval_number(vm, "WebSocket.prototype.CLOSING", 2.0);
        assert_eval_number(vm, "WebSocket.prototype.CLOSED", 3.0);
    });
}

#[test]
fn ws_constants_inherited_by_instance_via_proto_chain() {
    // Instances see the constants via prototype lookup but do
    // NOT carry own-property copies — MIN-3 fold per plan v1.1.
    with_realtime_vm(|vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        assert_eval_number(vm, "s.CONNECTING", 0.0);
        assert_eval_number(vm, "s.OPEN", 1.0);
        assert_eval_number(vm, "s.CLOSING", 2.0);
        assert_eval_number(vm, "s.CLOSED", 3.0);
        // hasOwnProperty must be false for every constant — the
        // instance shape inherits, doesn't duplicate.
        assert_eval_bool(vm, "s.hasOwnProperty('CONNECTING')", false);
        assert_eval_bool(vm, "s.hasOwnProperty('OPEN')", false);
        assert_eval_bool(vm, "s.hasOwnProperty('CLOSING')", false);
        assert_eval_bool(vm, "s.hasOwnProperty('CLOSED')", false);
    });
}

#[test]
fn es_constants_on_constructor() {
    with_realtime_vm(|vm| {
        assert_eval_number(vm, "EventSource.CONNECTING", 0.0);
        assert_eval_number(vm, "EventSource.OPEN", 1.0);
        assert_eval_number(vm, "EventSource.CLOSED", 2.0);
        // SSE 3-state machine: NO CLOSING.
        assert_eval_bool(vm, "EventSource.CLOSING === undefined", true);
    });
}

#[test]
fn es_constants_on_prototype() {
    with_realtime_vm(|vm| {
        assert_eval_number(vm, "EventSource.prototype.CONNECTING", 0.0);
        assert_eval_number(vm, "EventSource.prototype.OPEN", 1.0);
        assert_eval_number(vm, "EventSource.prototype.CLOSED", 2.0);
        assert_eval_bool(vm, "EventSource.prototype.CLOSING === undefined", true);
    });
}

#[test]
fn es_constants_inherited_by_instance_via_proto_chain() {
    with_realtime_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events');")
            .unwrap();
        assert_eval_number(vm, "s.CONNECTING", 0.0);
        assert_eval_number(vm, "s.OPEN", 1.0);
        assert_eval_number(vm, "s.CLOSED", 2.0);
        assert_eval_bool(vm, "s.hasOwnProperty('CONNECTING')", false);
        assert_eval_bool(vm, "s.hasOwnProperty('OPEN')", false);
        assert_eval_bool(vm, "s.hasOwnProperty('CLOSED')", false);
    });
}

// ---------------------------------------------------------------------------
// instanceof parity
// ---------------------------------------------------------------------------

#[test]
fn ws_instanceof_ws_holds() {
    with_realtime_vm(|vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        assert_eval_bool(vm, "s instanceof WebSocket", true);
        assert_eval_bool(vm, "s instanceof EventSource", false);
    });
}

#[test]
fn es_instanceof_es_holds() {
    with_realtime_vm(|vm| {
        vm.eval("globalThis.s = new EventSource('https://example.com/events');")
            .unwrap();
        assert_eval_bool(vm, "s instanceof EventSource", true);
        assert_eval_bool(vm, "s instanceof WebSocket", false);
    });
}

// ---------------------------------------------------------------------------
// structuredClone → DataCloneError
// ---------------------------------------------------------------------------

#[test]
fn structured_clone_websocket_throws_data_clone_error() {
    with_realtime_vm(|vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { structuredClone(s); } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        match vm.eval("r").unwrap() {
            JsValue::String(id) => assert_eq!(vm.get_string(id), "DataCloneError"),
            other => panic!("expected DataCloneError name, got {other:?}"),
        }
    });
}

#[test]
fn structured_clone_event_source_throws_data_clone_error() {
    with_realtime_vm(|vm| {
        vm.eval(
            "globalThis.s = new EventSource('https://example.com/events'); \
             globalThis.r = null; \
             try { structuredClone(s); } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        match vm.eval("r").unwrap() {
            JsValue::String(id) => assert_eq!(vm.get_string(id), "DataCloneError"),
            other => panic!("expected DataCloneError name, got {other:?}"),
        }
    });
}

// ---------------------------------------------------------------------------
// `Vm::teardown_document` CRIT-A teardown — broker receives Close per conn_id
// ---------------------------------------------------------------------------

#[test]
fn teardown_document_emits_websocket_close_per_active_connection() {
    // CRIT-A regression: every active WebSocket conn_id must be
    // surfaced to the broker via `WebSocketClose(conn_id)` BEFORE
    // the renderer-side side-tables are cleared.  The mock handle
    // captures every `send()` call in `recorded_outgoing`; verify
    // the per-conn Close messages appear there after unbind.
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page").expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "globalThis.a = new WebSocket('ws://example.com/a'); \
         globalThis.b = new WebSocket('ws://example.com/b');",
    )
    .unwrap();
    // Drain the ctor-time WebSocketOpen sends; we're interested
    // only in the teardown wave.
    let _ = handle.drain_recorded_outgoing();

    vm.teardown_document();
    let observed = handle.drain_recorded_outgoing();
    let close_count = observed
        .iter()
        .filter(|s| s.starts_with("WebSocketClose("))
        .count();
    assert_eq!(
        close_count, 2,
        "expected 2 WebSocketClose messages (one per active conn), got: {observed:?}"
    );
    drop(session);
    drop(dom);
}

#[test]
fn teardown_document_emits_event_source_close_per_active_connection() {
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("https://example.com/page/").expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "globalThis.a = new EventSource('/feed-a'); \
         globalThis.b = new EventSource('/feed-b'); \
         globalThis.c = new EventSource('/feed-c');",
    )
    .unwrap();
    let _ = handle.drain_recorded_outgoing();

    vm.teardown_document();
    let observed = handle.drain_recorded_outgoing();
    let close_count = observed
        .iter()
        .filter(|s| s.starts_with("EventSourceClose("))
        .count();
    assert_eq!(
        close_count, 3,
        "expected 3 EventSourceClose messages (one per active conn), got: {observed:?}"
    );
    drop(session);
    drop(dom);
}

#[test]
fn teardown_document_emits_close_for_mixed_websocket_and_event_source() {
    // Ensures both side-tables drain in the same unbind pass —
    // mirror of [`super::super::host_data::HostData::
    // drain_realtime_for_unbind`] returning the `(ws, sse)`
    // tuple.
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("https://example.com/page/").expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "globalThis.w = new WebSocket('wss://example.com/socket'); \
         globalThis.e = new EventSource('/events');",
    )
    .unwrap();
    let _ = handle.drain_recorded_outgoing();

    vm.teardown_document();
    let observed = handle.drain_recorded_outgoing();
    let ws_close = observed
        .iter()
        .filter(|s| s.starts_with("WebSocketClose("))
        .count();
    let es_close = observed
        .iter()
        .filter(|s| s.starts_with("EventSourceClose("))
        .count();
    assert_eq!(ws_close, 1, "1 WebSocketClose expected, got: {observed:?}");
    assert_eq!(
        es_close, 1,
        "1 EventSourceClose expected, got: {observed:?}"
    );
    drop(session);
    drop(dom);
}

#[test]
fn unbind_with_no_active_realtime_emits_no_close() {
    // Negative test: a clean teardown with nothing open emits
    // nothing — the drain helper still runs but observes empty
    // side-tables.
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page").expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = handle.drain_recorded_outgoing();

    vm.unbind();
    let observed = handle.drain_recorded_outgoing();
    let close_count = observed
        .iter()
        .filter(|s| s.starts_with("WebSocketClose(") || s.starts_with("EventSourceClose("))
        .count();
    assert_eq!(close_count, 0, "no Close expected, got: {observed:?}");
    drop(session);
    drop(dom);
}
