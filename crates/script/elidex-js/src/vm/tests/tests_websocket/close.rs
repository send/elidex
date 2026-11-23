//! `close(code?, reason?)` validation + idempotency.

use super::*;

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
    // InvalidAccessError: close() steps 1–2 validate before the
    // step 3 CLOSING/CLOSED check.
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
