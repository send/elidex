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

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_ecs::{Attributes, EcsDom};
use elidex_net::broker::{NetworkHandle, NetworkToRenderer};
use elidex_net::ws::WsEvent;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Minimal `doc` fixture for tests requiring a bound HostData session
/// (the WebSocket constructor demands one for the side-table writes).
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

fn assert_eval_number(vm: &mut Vm, src: &str, expected: f64) {
    match vm.eval(src).unwrap() {
        JsValue::Number(n) => assert!(
            (n - expected).abs() < f64::EPSILON,
            "expected {expected}, got {n} (src: {src})"
        ),
        other => panic!("expected Number({expected}), got {other:?} (src: {src})"),
    }
}

fn assert_eval_string(vm: &mut Vm, src: &str, expected: &str) {
    match vm.eval(src).unwrap() {
        JsValue::String(id) => assert_eq!(vm.get_string(id), expected, "src: {src}"),
        other => panic!("expected String({expected:?}), got {other:?} (src: {src})"),
    }
}

fn assert_eval_bool(vm: &mut Vm, src: &str, expected: bool) {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => assert_eq!(b, expected, "src: {src}"),
        other => panic!("expected Boolean({expected}), got {other:?} (src: {src})"),
    }
}

// ---------------------------------------------------------------------------
// Constructor: success path + URL validation
// ---------------------------------------------------------------------------

#[test]
fn ctor_accepts_ws_url() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .expect("ctor succeeds");
        assert_eval_number(vm, "s.readyState", 0.0);
    });
}

#[test]
fn ctor_accepts_wss_url() {
    with_ws_vm(true, |vm| {
        vm.eval("globalThis.s = new WebSocket('wss://example.com/socket');")
            .expect("wss should succeed on https page");
    });
}

#[test]
fn ctor_promotes_http_to_ws() {
    // WHATWG §9.3.1 step 6: `http://` → `ws://`, `https://` → `wss://`.
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('http://example.com/socket');")
            .expect("http promotes to ws");
        match vm.eval("s.url").unwrap() {
            JsValue::String(id) => {
                let s = vm.get_string(id);
                assert!(s.starts_with("ws://"), "expected ws:// prefix, got: {s}");
            }
            other => panic!("url should be String, got {other:?}"),
        }
    });
}

#[test]
fn ctor_promotes_https_to_wss() {
    with_ws_vm(true, |vm| {
        vm.eval("globalThis.s = new WebSocket('https://example.com/socket');")
            .expect("https promotes to wss");
        match vm.eval("s.url").unwrap() {
            JsValue::String(id) => {
                let s = vm.get_string(id);
                assert!(s.starts_with("wss://"), "expected wss:// prefix, got: {s}");
            }
            other => panic!("url should be String, got {other:?}"),
        }
    });
}

#[test]
fn ctor_rejects_unsupported_scheme() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ftp://example.com/socket');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("scheme"),
            "expected SyntaxError mentioning scheme, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_fragment() {
    // WHATWG §9.3.1 step 7: SyntaxError on fragment.
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket#frag');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("fragment"),
            "expected SyntaxError mentioning fragment, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_garbage_url() {
    with_ws_vm(false, |vm| {
        let err = vm.eval("new WebSocket('not a url at all');").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("Syntax"), "expected SyntaxError, got: {msg}");
    });
}

#[test]
fn ctor_rejects_missing_url_arg() {
    with_ws_vm(false, |vm| {
        let err = vm.eval("new WebSocket();").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("argument") || msg.contains("required"),
            "expected arity error, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_mixed_content() {
    // https page + ws:// URL → SecurityError (DOMException).
    with_ws_vm(true, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Security") || msg.contains("insecure"),
            "expected SecurityError, got: {msg}"
        );
    });
}

#[test]
fn ctor_requires_new_operator() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("WebSocket('ws://example.com/socket');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("new") || msg.contains("Type"),
            "expected TypeError for bare call, got: {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// Constructor: protocols arg
// ---------------------------------------------------------------------------

#[test]
fn ctor_accepts_single_string_protocol() {
    with_ws_vm(false, |vm| {
        vm.eval("new WebSocket('ws://example.com/socket', 'chat');")
            .expect("single string protocol ok");
    });
}

#[test]
fn ctor_accepts_array_of_protocols() {
    with_ws_vm(false, |vm| {
        vm.eval("new WebSocket('ws://example.com/socket', ['chat', 'superchat']);")
            .expect("array of distinct protocols ok");
    });
}

#[test]
fn ctor_rejects_empty_protocol() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket', '');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("empty"),
            "expected SyntaxError for empty protocol, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_protocol_with_separator() {
    // RFC 7230 §3.2.6: comma is a separator, not a token char.
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket', 'bad,protocol');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("token"),
            "expected SyntaxError for bad token, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_protocol_with_space() {
    // Space (0x20) is below the 0x21..=0x7E token range.
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket', 'bad protocol');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax"),
            "expected SyntaxError for space in protocol, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_duplicate_protocols() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket', ['chat', 'chat']);")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("duplicate"),
            "expected SyntaxError for duplicate, got: {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// readyState transitions via dispatch_realtime_event
// ---------------------------------------------------------------------------

#[test]
fn ready_state_transitions_connecting_to_open_via_connected_event() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Connected {
                protocol: "chat".to_string(),
                extensions: "permessage-deflate".to_string(),
            },
        );
        assert_eval_number(vm, "s.readyState", 1.0);
        assert_eval_string(vm, "s.protocol", "chat");
        assert_eval_string(vm, "s.extensions", "permessage-deflate");
    });
}

#[test]
fn ready_state_transitions_open_to_closed_via_closed_event() {
    with_ws_vm(false, |vm| {
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
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: "normal".to_string(),
                was_clean: true,
            },
        );
        assert_eval_number(vm, "s.readyState", 3.0);
    });
}

// ---------------------------------------------------------------------------
// send(USVString) — CRIT-2 state semantics
// ---------------------------------------------------------------------------

#[test]
fn send_during_connecting_throws_invalid_state_error() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        let err = vm.eval("s.send('hello');").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("InvalidState") || msg.contains("CONNECTING"),
            "expected InvalidStateError mentioning CONNECTING, got: {msg}"
        );
        // bufferedAmount must NOT have incremented.
        assert_eval_number(vm, "s.bufferedAmount", 0.0);
    });
}

#[test]
fn send_during_open_increments_buffered_amount() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.send('hello');").expect("send on OPEN succeeds");
        // "hello" = 5 UTF-8 bytes.
        assert_eval_number(vm, "s.bufferedAmount", 5.0);
        vm.eval("s.send('世界');").expect("multibyte send ok");
        // "世界" = 6 UTF-8 bytes (each char 3 bytes); total 5 + 6 = 11.
        assert_eval_number(vm, "s.bufferedAmount", 11.0);
    });
}

#[test]
fn send_during_closing_silent_discards_but_increments_buffered_amount() {
    // CRIT-2 regression test: CLOSING/CLOSED → silent + still
    // increment bufferedAmount per WHATWG §9.3.4 step 2-3.
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close();").unwrap();
        assert_eval_number(vm, "s.readyState", 2.0);
        vm.eval("s.send('x');")
            .expect("send during CLOSING must not throw");
        assert_eval_number(vm, "s.bufferedAmount", 1.0);
    });
}

#[test]
fn send_after_closed_silent_discards_but_increments_buffered_amount() {
    with_ws_vm(false, |vm| {
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
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: String::new(),
                was_clean: true,
            },
        );
        assert_eval_number(vm, "s.readyState", 3.0);
        vm.eval("s.send('x');")
            .expect("send after CLOSED must not throw");
        assert_eval_number(vm, "s.bufferedAmount", 1.0);
    });
}

// ---------------------------------------------------------------------------
// close(code?, reason?) — validation + idempotency
// ---------------------------------------------------------------------------

#[test]
fn close_with_no_args_transitions_to_closing() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close();").expect("close() succeeds");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

#[test]
fn close_with_code_1000_accepted() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close(1000, 'bye');").expect("1000 accepted");
    });
}

#[test]
fn close_with_code_3000_accepted() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close(3000);").expect("3000 accepted");
    });
}

#[test]
fn close_with_code_4999_accepted() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close(4999);").expect("4999 accepted");
    });
}

#[test]
fn close_with_code_2999_rejected() {
    // Just below 3000 — DOMException with name "InvalidAccessError".
    // The Rust `Debug` format prints only the interned StringId, so
    // the test reads `.name` on the JS-materialised DOMException.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.close(2999); } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "InvalidAccessError");
    });
}

#[test]
fn close_with_code_5000_rejected() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.close(5000); } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "InvalidAccessError");
    });
}

#[test]
fn close_with_long_reason_rejected() {
    // 124 ASCII bytes → exceeds 123 byte cap.  SyntaxError.
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        let err = vm.eval("s.close(1000, 'x'.repeat(124));").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && (msg.contains("reason") || msg.contains("123")),
            "expected SyntaxError mentioning reason/123, got: {msg}"
        );
    });
}

#[test]
fn close_with_multibyte_reason_uses_byte_length() {
    // 41 chars × 3 bytes each = 123 bytes (boundary, ok).
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        vm.eval("s.close(1000, 'あ'.repeat(41));")
            .expect("41 × 3 = 123 bytes — at the boundary, must succeed");
    });
    // 42 chars × 3 = 126 bytes — above cap, must throw.  Use a
    // separate VM so the first close()'s state mutation doesn't
    // confound the second case.
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        let err = vm.eval("s.close(1000, 'あ'.repeat(42));").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax"),
            "expected SyntaxError for 42 × 3 = 126 bytes, got: {msg}"
        );
    });
}

#[test]
fn close_is_idempotent_when_already_closing() {
    with_ws_vm(false, |vm| {
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
        vm.eval("s.close();").unwrap();
        vm.eval("s.close();").expect("idempotent close ok");
        assert_eval_number(vm, "s.readyState", 2.0);
    });
}

#[test]
fn close_arg_validation_runs_before_state_idempotency() {
    // Even on a CLOSED socket, an invalid code arg surfaces an
    // InvalidAccessError.  Matches Chrome / Firefox semantics — the
    // WebIDL coercion happens before the §9.3.4 state check.
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: String::new(),
                was_clean: true,
            },
        );
        vm.eval(
            "globalThis.r = null; \
             try { s.close(123); } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "InvalidAccessError");
    });
}

// ---------------------------------------------------------------------------
// onopen / onclose handler attributes
// ---------------------------------------------------------------------------

#[test]
fn onopen_fires_with_open_event_after_connected_dispatch() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onopen = function(e) { globalThis._evts.push({t: e.type, tg: e.target === s}); };",
        )
        .unwrap();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Connected {
                protocol: "p".to_string(),
                extensions: "x".to_string(),
            },
        );
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "open");
        assert_eval_bool(vm, "_evts[0].tg", true);
    });
}

#[test]
fn onclose_fires_with_close_event_payload() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onclose = function(e) { \
               globalThis._evts.push({c: e.code, r: e.reason, w: e.wasClean, t: e.type}); \
             };",
        )
        .unwrap();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1006,
                reason: "abnormal".to_string(),
                was_clean: false,
            },
        );
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_number(vm, "_evts[0].c", 1006.0);
        assert_eval_string(vm, "_evts[0].r", "abnormal");
        assert_eval_bool(vm, "_evts[0].w", false);
        assert_eval_string(vm, "_evts[0].t", "close");
    });
}

#[test]
fn onopen_setter_only_retains_callable_values() {
    // WHATWG IDL EventHandler attribute: non-callable → nulls slot.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onopen = function() {}; \
             s.onopen = 42; /* nulls the slot */",
        )
        .unwrap();
        match vm.eval("s.onopen").unwrap() {
            JsValue::Null => {}
            other => panic!("onopen should be null after non-callable assignment, got {other:?}"),
        }
    });
}

#[test]
fn onopen_setter_round_trips_callable() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.fn = function() {}; \
             s.onopen = fn;",
        )
        .unwrap();
        assert_eval_bool(vm, "s.onopen === fn", true);
    });
}

// ---------------------------------------------------------------------------
// Brand check on prototype methods
// ---------------------------------------------------------------------------

#[test]
fn send_on_non_websocket_throws_type_error() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("WebSocket.prototype.send.call({}, 'x');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-WebSocket") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

#[test]
fn close_on_non_websocket_throws_type_error() {
    with_ws_vm(false, |vm| {
        let err = vm.eval("WebSocket.prototype.close.call({});").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-WebSocket") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

#[test]
fn readystate_getter_on_non_websocket_throws() {
    // Reach the accessor through the prototype descriptor, then call
    // it with a plain Object receiver — brand check must reject.
    with_ws_vm(false, |vm| {
        let err = vm
            .eval(
                "let d = Object.getOwnPropertyDescriptor(WebSocket.prototype, 'readyState'); \
                 d.get.call({});",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-WebSocket") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

// ---------------------------------------------------------------------------
// Phase 2: send(Blob | ArrayBuffer | ArrayBufferView | primitive) +
// TypeError matrix
// ---------------------------------------------------------------------------

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

#[test]
fn send_blob_increments_buffered_amount_by_byte_length() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // `new Blob(["abc"])` → 3 bytes.
        vm.eval("s.send(new Blob(['abc']));")
            .expect("send(Blob) on OPEN succeeds");
        assert_eval_number(vm, "s.bufferedAmount", 3.0);
    });
}

#[test]
fn send_arraybuffer_increments_buffered_amount() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // 5-byte ArrayBuffer.
        vm.eval("s.send(new ArrayBuffer(5));")
            .expect("send(ArrayBuffer) on OPEN succeeds");
        assert_eval_number(vm, "s.bufferedAmount", 5.0);
    });
}

#[test]
fn send_typed_array_uses_view_byte_length() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // Uint8Array view over a 10-byte buffer, length 4 → 4 bytes.
        vm.eval("s.send(new Uint8Array([1, 2, 3, 4]));")
            .expect("send(Uint8Array) ok");
        assert_eval_number(vm, "s.bufferedAmount", 4.0);
    });
}

#[test]
fn send_uint16_typed_array_byte_length_doubles_element_count() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // Uint16Array with 4 elements → 8 bytes.
        vm.eval("s.send(new Uint16Array([1, 2, 3, 4]));")
            .expect("send(Uint16Array) ok");
        assert_eval_number(vm, "s.bufferedAmount", 8.0);
    });
}

#[test]
fn send_dataview_uses_byte_length() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // DataView over 6-byte buffer.
        vm.eval("s.send(new DataView(new ArrayBuffer(6)));")
            .expect("send(DataView) ok");
        assert_eval_number(vm, "s.bufferedAmount", 6.0);
    });
}

#[test]
fn send_plain_object_throws_type_error() {
    // WebIDL §3.10.18 union resolution: Object that doesn't brand as
    // Blob / ArrayBuffer / TypedArray / DataView → TypeError (NO
    // implicit fall-through to DOMString).
    with_ws_vm(false, |vm| {
        open_ws(vm);
        let err = vm.eval("s.send({a: 1});").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("not of type") || msg.contains("Type"),
            "expected TypeError for plain Object arg, got: {msg}"
        );
        // bufferedAmount must NOT have incremented on a throw.
        assert_eval_number(vm, "s.bufferedAmount", 0.0);
    });
}

#[test]
fn send_number_coerces_to_string() {
    // Primitives (number, bool, null, undefined) fall through to the
    // USVString arm via ToString — `123` → `"123"` = 3 bytes.
    with_ws_vm(false, |vm| {
        open_ws(vm);
        vm.eval("s.send(123);")
            .expect("send(number) ToString-coerces");
        assert_eval_number(vm, "s.bufferedAmount", 3.0);
    });
}

#[test]
fn send_boolean_coerces_to_string() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // `true` → `"true"` = 4 bytes.
        vm.eval("s.send(true);").expect("send(bool) ok");
        assert_eval_number(vm, "s.bufferedAmount", 4.0);
    });
}

#[test]
fn send_null_coerces_to_string() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // `null` → `"null"` = 4 bytes.
        vm.eval("s.send(null);").expect("send(null) ok");
        assert_eval_number(vm, "s.bufferedAmount", 4.0);
    });
}

#[test]
fn send_blob_during_closing_silent_discards_but_increments_buffered_amount() {
    // CRIT-2 must hold for binary too: CLOSING/CLOSED silent-discard,
    // bufferedAmount still increments.
    with_ws_vm(false, |vm| {
        open_ws(vm);
        vm.eval("s.close();").unwrap();
        assert_eval_number(vm, "s.readyState", 2.0);
        vm.eval("s.send(new Blob(['xyz']));")
            .expect("send(Blob) during CLOSING must not throw");
        assert_eval_number(vm, "s.bufferedAmount", 3.0);
    });
}

// ---------------------------------------------------------------------------
// Phase 2: binaryType getter + setter (IMP-2 TypeError on non-enum)
// ---------------------------------------------------------------------------

#[test]
fn binary_type_default_is_blob() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_accepts_arraybuffer() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.binaryType = 'arraybuffer';",
        )
        .unwrap();
        assert_eval_string(vm, "s.binaryType", "arraybuffer");
    });
}

#[test]
fn binary_type_setter_accepts_blob_round_trip() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.binaryType = 'arraybuffer'; \
             s.binaryType = 'blob';",
        )
        .unwrap();
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_rejects_other_string_with_type_error() {
    // Regression: the boa reference silently ignored unknown
    // strings; the spec-correct behaviour is TypeError per WebIDL
    // §3.10.21 enum coercion.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = 'foo'; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
        // The slot must NOT have changed.
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_rejects_number_with_type_error() {
    // `42` ToString → "42" → enum mismatch → TypeError.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = 42; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_with_undefined_throws_type_error() {
    // `undefined` ToString → "undefined" → enum mismatch → TypeError.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = undefined; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
    });
}

// ---------------------------------------------------------------------------
// Phase 2: onmessage + onerror dispatch (MessageEvent / plain Event)
// ---------------------------------------------------------------------------

#[test]
fn onmessage_fires_with_string_data_message_event() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onmessage = function(e) { \
               globalThis._evts.push({ \
                 t: e.type, d: e.data, lei: e.lastEventId, \
                 src: e.source, tg: e.target === s, \
                 portsLen: e.ports.length \
               }); \
             };",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("hello".to_string()));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "message");
        assert_eval_string(vm, "_evts[0].d", "hello");
        assert_eval_string(vm, "_evts[0].lei", "");
        assert_eval_bool(vm, "_evts[0].src === null", true);
        assert_eval_bool(vm, "_evts[0].tg", true);
        assert_eval_number(vm, "_evts[0].portsLen", 0.0);
    });
}

#[test]
fn onmessage_origin_is_ws_url_origin_not_page_origin() {
    // Regression: WHATWG §9.3.7 requires the MessageEvent.origin to
    // be the WebSocket URL's origin (the SERVER's origin), NOT the
    // page's origin.  The page is at http://example.com but the WS
    // is to ws://different.example.org so the two are visibly
    // distinct.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._origin = null; \
             globalThis.s = new WebSocket('ws://different.example.org:8081/socket'); \
             s.onmessage = function(e) { globalThis._origin = e.origin; };",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("x".to_string()));
        // `Url::origin().ascii_serialization()` for a non-default port
        // preserves the port → "ws://different.example.org:8081".
        assert_eval_string(vm, "_origin", "ws://different.example.org:8081");
    });
}

#[test]
fn onmessage_data_is_blob_when_binary_type_is_blob() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._dataKind = null; \
             globalThis._size = null; \
             globalThis._mime = null; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onmessage = function(e) { \
               globalThis._dataKind = e.data instanceof Blob; \
               globalThis._size = e.data.size; \
               globalThis._mime = e.data.type; \
             };",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::BinaryMessage(vec![1, 2, 3, 4]));
        assert_eval_bool(vm, "_dataKind", true);
        assert_eval_number(vm, "_size", 4.0);
        // WHATWG §9.3.7: Blob `type` is the empty string for WS
        // binary messages (no spec-mandated MIME).
        assert_eval_string(vm, "_mime", "");
    });
}

#[test]
fn onmessage_data_is_arraybuffer_when_binary_type_is_arraybuffer() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._kind = null; \
             globalThis._size = null; \
             globalThis._byte0 = null; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.binaryType = 'arraybuffer'; \
             s.onmessage = function(e) { \
               globalThis._kind = e.data instanceof ArrayBuffer; \
               globalThis._size = e.data.byteLength; \
               globalThis._byte0 = new Uint8Array(e.data)[0]; \
             };",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::BinaryMessage(vec![42, 99, 100]));
        assert_eval_bool(vm, "_kind", true);
        assert_eval_number(vm, "_size", 3.0);
        assert_eval_number(vm, "_byte0", 42.0);
    });
}

#[test]
fn onmessage_text_does_not_fire_when_no_handler_registered() {
    // No `onmessage` set → silent-drop (no addEventListener path in
    // Phase 1/2 — that's `#11-realtime-event-listeners`).  This is
    // intentional WHATWG-compliant behaviour for the EventHandler-
    // only surface.
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // Should not throw, should not mutate state.
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("nope".to_string()));
        assert_eval_number(vm, "s.readyState", 1.0);
    });
}

#[test]
fn onerror_fires_plain_event() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onerror = function(e) { \
               globalThis._evts.push({t: e.type, tg: e.target === s, msg: e.message}); \
             };",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::Error("server crashed".to_string()));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_string(vm, "_evts[0].t", "error");
        assert_eval_bool(vm, "_evts[0].tg", true);
        // Plain Event has no `message` field — accessing it returns
        // undefined (NOT the broker's error string, intentionally
        // opaque per WHATWG §9.3.7).
        assert_eval_bool(vm, "_evts[0].msg === undefined", true);
    });
}

#[test]
fn onmessage_setter_round_trips_callable() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.fn = function() {}; \
             s.onmessage = fn;",
        )
        .unwrap();
        assert_eval_bool(vm, "s.onmessage === fn", true);
    });
}

#[test]
fn onerror_setter_non_callable_nulls_slot() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onerror = function() {}; \
             s.onerror = 'oops'; /* nulls slot */",
        )
        .unwrap();
        match vm.eval("s.onerror").unwrap() {
            JsValue::Null => {}
            other => panic!("onerror should be null after non-callable assignment, got {other:?}"),
        }
    });
}

// ---------------------------------------------------------------------------
// Phase 2: BytesSent — bufferedAmount saturating decrement
// ---------------------------------------------------------------------------

#[test]
fn bytes_sent_decrements_buffered_amount() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        vm.eval("s.send('hello world');").unwrap();
        assert_eval_number(vm, "s.bufferedAmount", 11.0);
        inject_ws_event_and_tick(vm, 0, WsEvent::BytesSent(5));
        assert_eval_number(vm, "s.bufferedAmount", 6.0);
    });
}

#[test]
fn bytes_sent_saturating_subtracts_without_underflow() {
    // Broker over-reports (e.g. fragmentation accounting drift) →
    // saturating arithmetic clamps at 0 instead of underflowing.
    with_ws_vm(false, |vm| {
        open_ws(vm);
        vm.eval("s.send('hi');").unwrap();
        assert_eval_number(vm, "s.bufferedAmount", 2.0);
        inject_ws_event_and_tick(vm, 0, WsEvent::BytesSent(999));
        assert_eval_number(vm, "s.bufferedAmount", 0.0);
    });
}

// ---------------------------------------------------------------------------
// Phase 2: brand check on new accessor + send-arg-extension surface
// ---------------------------------------------------------------------------

#[test]
fn binary_type_getter_on_non_websocket_throws() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval(
                "let d = Object.getOwnPropertyDescriptor(WebSocket.prototype, 'binaryType'); \
                 d.get.call({});",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-WebSocket") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}

#[test]
fn binary_type_setter_on_non_websocket_throws() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval(
                "let d = Object.getOwnPropertyDescriptor(WebSocket.prototype, 'binaryType'); \
                 d.set.call({}, 'blob');",
            )
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("non-WebSocket") || msg.contains("Type"),
            "expected brand-check TypeError, got: {msg}"
        );
    });
}
