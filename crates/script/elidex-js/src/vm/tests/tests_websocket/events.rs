//! `addEventListener` / EventTarget integration (`#11-realtime-event-listeners`).

use elidex_net::ws::WsEvent;

use crate::vm::tests::{assert_eval_bool, assert_eval_number, assert_eval_string};

use super::{inject_ws_event_and_tick, open_ws, with_ws_vm};

// ---------------------------------------------------------------------------
// addEventListener / EventTarget integration (#11-realtime-event-listeners)
//
// WebSocket gains real `addEventListener` / `removeEventListener` /
// `dispatchEvent` by inheriting `EventTarget.prototype` and routing
// through the shared §2.9 VmObject dispatch core; `on*` handlers now
// live in the same `vm_event_listeners` home.
// ---------------------------------------------------------------------------

#[test]
fn add_event_listener_message_delivers_message_event() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._evts = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.addEventListener('message', function(e) { \
               globalThis._evts.push({ \
                 mev: e instanceof MessageEvent, t: e.type, d: e.data, \
                 origin: e.origin, tg: e.target === s \
               }); \
             });",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("hi".to_string()));
        assert_eval_number(vm, "_evts.length", 1.0);
        assert_eval_bool(vm, "_evts[0].mev", true);
        assert_eval_string(vm, "_evts[0].t", "message");
        assert_eval_string(vm, "_evts[0].d", "hi");
        assert_eval_string(vm, "_evts[0].origin", "ws://example.com");
        assert_eval_bool(vm, "_evts[0].tg", true);
    });
}

#[test]
fn onmessage_and_add_event_listener_both_fire_one_frame() {
    // A single incoming frame fires BOTH the `onmessage` EventHandler
    // listener and every `addEventListener('message')` listener, in
    // registration order, off one event object.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._log = []; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.onmessage = function(e) { globalThis._log.push('on:' + e.data); }; \
             s.addEventListener('message', function(e) { globalThis._log.push('ael:' + e.data); });",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("x".to_string()));
        assert_eval_number(vm, "_log.length", 2.0);
        assert_eval_bool(vm, "_log.indexOf('on:x') !== -1", true);
        assert_eval_bool(vm, "_log.indexOf('ael:x') !== -1", true);
    });
}

#[test]
fn close_delivers_close_event_to_add_event_listener() {
    with_ws_vm(false, |vm| {
        open_ws(vm);
        vm.eval(
            "globalThis._c = null; \
             s.addEventListener('close', function(e) { \
               globalThis._c = { ce: e instanceof CloseEvent, code: e.code, \
                                 reason: e.reason, clean: e.wasClean }; \
             });",
        )
        .unwrap();
        inject_ws_event_and_tick(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: "bye".to_string(),
                was_clean: true,
            },
        );
        assert_eval_bool(vm, "_c.ce", true);
        assert_eval_number(vm, "_c.code", 1000.0);
        assert_eval_string(vm, "_c.reason", "bye");
        assert_eval_bool(vm, "_c.clean", true);
    });
}

#[test]
fn add_event_listener_once_fires_exactly_once() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._n = 0; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.addEventListener('message', function() { globalThis._n++; }, { once: true });",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("a".to_string()));
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("b".to_string()));
        assert_eval_number(vm, "_n", 1.0);
    });
}

#[test]
fn add_event_listener_capture_true_still_fires_at_target() {
    // A flat VmObject EventTarget has no ancestors, but a `{capture:true}`
    // listener on the target itself must still fire at-target — pins the
    // `vm_path_has_listener` `is_target` arm (#264 R2 lesson) for a flat
    // target.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._n = 0; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.addEventListener('message', function() { globalThis._n++; }, { capture: true });",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("a".to_string()));
        assert_eval_number(vm, "_n", 1.0);
    });
}

#[test]
fn add_event_listener_signal_abort_detaches() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._n = 0; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.ac = new AbortController(); \
             s.addEventListener('message', function() { globalThis._n++; }, { signal: ac.signal }); \
             ac.abort();",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("a".to_string()));
        assert_eval_number(vm, "_n", 0.0);
    });
}

#[test]
fn remove_event_listener_stops_delivery() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._n = 0; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.h = function() { globalThis._n++; }; \
             s.addEventListener('message', h); \
             s.removeEventListener('message', h);",
        )
        .unwrap();
        inject_ws_event_and_tick(vm, 0, WsEvent::TextMessage("a".to_string()));
        assert_eval_number(vm, "_n", 0.0);
    });
}

#[test]
fn dispatch_event_synthetic_walks_listeners() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis._n = 0; \
             globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.addEventListener('custom', function() { globalThis._n++; }); \
             globalThis._ret = s.dispatchEvent(new Event('custom'));",
        )
        .unwrap();
        assert_eval_number(vm, "_n", 1.0);
        assert_eval_bool(vm, "_ret", true);
    });
}

#[test]
fn websocket_add_event_listener_is_inherited_not_shadowed() {
    // F10 structural single-home guard: WebSocket must NOT carry a
    // bespoke own `addEventListener` shadowing the inherited
    // EventTarget.prototype one (that would split the listener home).
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        assert_eval_bool(vm, "typeof s.addEventListener === 'function'", true);
        // WebSocket.prototype must NOT own `addEventListener` — it is
        // inherited from EventTarget.prototype.  A surviving bespoke own
        // method would shadow the shared one and split the listener home
        // (the one split-brain vector not caught at compile time).
        assert_eval_bool(
            vm,
            "Object.getOwnPropertyNames(Object.getPrototypeOf(s))\
             .indexOf('addEventListener') === -1",
            true,
        );
    });
}
