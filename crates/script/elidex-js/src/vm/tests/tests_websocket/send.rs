//! `send()` (USVString + Phase 2 union matrix) and `BytesSent` accounting.

use elidex_net::ws::WsEvent;

use crate::vm::tests::assert_eval_number;

use super::{inject_ws_event_and_tick, open_ws, with_ws_vm};

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
// Phase 2: send(Blob | ArrayBuffer | ArrayBufferView | primitive) +
// TypeError matrix
// ---------------------------------------------------------------------------

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
