//! D-12 `#11-net-ws-sse` Phase 1 — `WebSocket` JS thin binding tests.
//!
//! Coverage:
//! - Constructor success path + WHATWG §9.3.1 validation (URL parse /
//!   scheme promotion / fragment / mixed-content / protocols token
//!   ABNF / duplicate).
//! - `readyState` accessor + CONNECTING-on-construction invariant.
//! - `send(USVString)` CRIT-2 state semantics: throw on CONNECTING,
//!   silent-discard on CLOSING/CLOSED with `bufferedAmount` increment.
//! - `close(code?, reason?)` validation matrix + idempotency.
//! - `onopen` / `onclose` handler accessor pairs (callable-only
//!   retention).
//! - `Connected` / `Closed` event dispatch through `tick_network` →
//!   `dispatch_realtime_event`.
//!
//! Constants verification + binary send (`binaryType` /
//! `onmessage` / `onerror`) land in Phase 2.

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
