//! `onmessage` / `onerror` dispatch (MessageEvent / plain Event).

use elidex_net::ws::WsEvent;

use crate::vm::tests::{assert_eval_bool, assert_eval_number, assert_eval_string};
use crate::vm::value::JsValue;

use super::{inject_ws_event_and_tick, open_ws, with_ws_vm};

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
    // No listener of any kind registered → silent no-op: the
    // `vm_path_has_listener` gate inside the shared §2.9 VmObject fire
    // (`#11-realtime-event-listeners`) skips event allocation entirely.
    with_ws_vm(false, |vm| {
        open_ws(vm);
        // Should not throw, should not mutate state.
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("nope".to_string()));
        assert_eval_number(vm, "s.readyState", 1.0);
    });
}

#[test]
fn binary_message_no_handler_does_not_fire_or_build_payload() {
    // Regression for the lazy-alloc invariant (`#11-realtime-event-listeners`,
    // Copilot R1): an unobserved socket builds NEITHER the Blob / ArrayBuffer
    // payload NOR the event.  `fire_vm_message_event` gates before running the
    // binary `build_data` thunk, so the (potentially large) payload is never
    // constructed.  Observable proxy for "nothing allocated": no throw + no
    // state mutation, in both binaryType modes (blob default + the costly
    // arraybuffer byte-copy path).
    with_ws_vm(false, |vm| {
        open_ws(vm);
        inject_ws_event_and_tick(vm, 0, WsEvent::BinaryMessage(vec![1, 2, 3, 4]));
        assert_eval_number(vm, "s.readyState", 1.0);
        vm.eval("s.binaryType = 'arraybuffer';").unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::BinaryMessage(vec![5, 6, 7, 8]));
        assert_eval_number(vm, "s.readyState", 1.0);
    });
}

#[test]
fn unobserved_ws_close_does_not_intern_reason() {
    // Regression (`#11-realtime-event-listeners`, Copilot R2 / IMPORTANT): the
    // server-controlled close `reason` is interned only PAST the listener gate,
    // so an unobserved socket never grows the permanent `StringPool` with a
    // unique reason.  `StringPool` entries are never freed, so a pre-gate
    // intern would be a permanent leak under server control.
    with_ws_vm(false, |vm| {
        open_ws(vm); // no `close` / `onclose` listener registered
        let before = vm.inner.strings.len();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: "a-unique-server-controlled-reason".to_string(),
                was_clean: true,
            },
        );
        assert_eq!(
            before,
            vm.inner.strings.len(),
            "unobserved WS close interned its reason into the permanent StringPool"
        );
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
