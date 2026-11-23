//! `readyState` transitions via `dispatch_realtime_event`.

use elidex_net::ws::WsEvent;

use crate::vm::tests::{assert_eval_number, assert_eval_string};

use super::{inject_ws_event_and_tick, with_ws_vm};

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
