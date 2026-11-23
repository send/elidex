//! Brand checks on prototype methods and accessors.

use super::*;

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
