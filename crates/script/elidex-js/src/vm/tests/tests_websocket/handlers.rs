//! `onopen` / `onclose` handler attributes.

use super::*;

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
