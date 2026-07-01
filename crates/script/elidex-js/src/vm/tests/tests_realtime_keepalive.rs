//! S5-3b — the keepalive-predicate seam's `WebSocket` / `EventSource` arms
//! (`#11-eventtarget-keepalive-registrant-coverage`).
//!
//! A `WebSocket` / `EventSource` wrapper is a non-Node `EventTarget` anchored
//! only by its listeners (callbacks rooted in `listener_store`, the wrapper
//! itself NOT a GC root). Before this arm, the GC sweep
//! (`collect.rs`) pruned its state row AND emitted a broker
//! `WebSocketClose` / `EventSourceClose` — force-closing a connection a page
//! was still listening on. The seam (`gc::keepalive::keepalive_survivors`)
//! marks a listener-held (or, for WS, data-queued) non-CLOSED connection BEFORE
//! the sweep, so it survives and keeps delivering; the genuine orphan / CLOSED
//! wrapper is still swept + force-closed (the spec's GC-while-open close /
//! abort-fetch — WebSockets §7 / HTML §9.2.9). The tier rule is the
//! engine-independent `elidex_api_ws::{ws_keepalive, es_keepalive}`.
//!
//! Kept in a dedicated file (not `tests_websocket` / `tests_event_source`, both
//! already over the 1000-line convention) — the S5-3a
//! `tests_match_media_keepalive` split precedent; WS + ES keepalive share the
//! seam and the recorded-outgoing drain pattern.

#![cfg(feature = "engine")]

use std::rc::Rc;

use elidex_ecs::{Attributes, EcsDom};
use elidex_net::broker::{NetworkHandle, NetworkToRenderer};
use elidex_net::sse::SseEvent;
use elidex_net::ws::WsEvent;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::Vm;
use super::assert_eval_number;

fn build_min_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Run `body` against a VM bound to a fresh session with a mock
/// `NetworkHandle`, exposing the handle so the test can drain the recorded
/// `WebSocketClose` / `EventSourceClose` outgoing log. `https` selects the page
/// scheme (`false` = `http`, so `ws://` is not mixed-content blocked; `true` =
/// `https`, for `EventSource`). Mirrors `tests_websocket::with_ws_vm` +
/// `tests_realtime`'s handle-keeping setup.
fn with_realtime_vm(https: bool, body: impl FnOnce(&mut Vm, &Rc<NetworkHandle>)) {
    let mut vm = Vm::new();
    let scheme = if https { "https" } else { "http" };
    vm.inner.navigation.current_url =
        url::Url::parse(&format!("{scheme}://example.com/page/")).expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    body(&mut vm, &handle);
    vm.unbind();
    drop(session);
    drop(dom);
}

fn inject_ws(vm: &mut Vm, conn_id: u64, ev: WsEvent) {
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    handle.rebuffer_events(vec![NetworkToRenderer::WebSocketEvent(conn_id, ev)]);
    vm.tick_network();
}

fn inject_sse(vm: &mut Vm, conn_id: u64, ev: SseEvent) {
    let handle = vm.inner.network_handle.clone().expect("handle installed");
    handle.rebuffer_events(vec![NetworkToRenderer::EventSourceEvent(conn_id, ev)]);
    vm.tick_network();
}

fn ws_connected() -> WsEvent {
    WsEvent::Connected {
        protocol: String::new(),
        extensions: String::new(),
    }
}

fn sse_connected() -> SseEvent {
    SseEvent::Connected {
        final_url: url::Url::parse("https://example.com/events").expect("valid URL"),
    }
}

fn ws_state_count(vm: &Vm) -> usize {
    vm.inner
        .host_data
        .as_deref()
        .map_or(0, |hd| hd.websocket_states.len())
}

fn es_state_count(vm: &Vm) -> usize {
    vm.inner
        .host_data
        .as_deref()
        .map_or(0, |hd| hd.event_source_states.len())
}

/// Count outgoing `<prefix>(…)` messages in the mock handle's recorded log.
fn outgoing_count(handle: &Rc<NetworkHandle>, prefix: &str) -> usize {
    handle
        .drain_recorded_outgoing()
        .iter()
        .filter(|s| s.starts_with(prefix))
        .count()
}

// ---------------------------------------------------------------------------
// WebSocket — WebSockets §7
// ---------------------------------------------------------------------------

#[test]
fn ws_listener_held_open_survives_gc_no_force_close_and_keeps_delivering() {
    // Headline: a listener-only OPEN WebSocket (no retained reference) must
    // survive GC — NOT be swept + force-closed — and keep delivering.
    with_realtime_vm(false, |vm, handle| {
        vm.eval(
            "globalThis.msgs = 0; \
             new WebSocket('ws://example.com/socket') \
                 .addEventListener('message', function () { globalThis.msgs++; });",
        )
        .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing(); // clear ctor WebSocketOpen

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            1,
            "a listener-held OPEN WebSocket must survive GC via the keepalive seam",
        );
        assert_eq!(
            outgoing_count(handle, "WebSocketClose("),
            0,
            "a survived connection must NOT be force-closed",
        );

        // Still functional: a subsequent server frame is delivered.
        inject_ws(vm, 0, WsEvent::TextMessage("hi".to_string()));
        assert_eval_number(vm, "msgs", 1.0);
    });
}

#[test]
fn ws_orphan_open_collected_and_force_closed() {
    // Negative control: an OPEN WebSocket with no listener and no queued data is
    // a genuine orphan — collected AND force-closed (the §7 GC-while-open close).
    with_realtime_vm(false, |vm, handle| {
        vm.eval("new WebSocket('ws://example.com/socket');")
            .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            0,
            "an orphan OPEN WebSocket must be collected"
        );
        assert_eq!(
            outgoing_count(handle, "WebSocketClose("),
            1,
            "a swept orphan must be force-closed (WebSocketClose emitted)",
        );
    });
}

#[test]
fn ws_open_with_only_open_listener_collected() {
    // Tier proof (not any-listener): `open` is NOT in the OPEN tier
    // {message,error,close}, so an OPEN WebSocket whose only listener is `open`
    // is collected + force-closed.
    with_realtime_vm(false, |vm, handle| {
        vm.eval(
            "new WebSocket('ws://example.com/socket') \
                 .addEventListener('open', function () {});",
        )
        .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            0,
            "an OPEN WebSocket with only an out-of-tier `open` listener must be collected",
        );
        assert_eq!(outgoing_count(handle, "WebSocketClose("), 1);
    });
}

#[test]
fn ws_data_queued_no_listener_clause() {
    // §7 no-listener clause: an established connection (OPEN) with data still
    // queued to transmit (`buffered_amount > 0`) must survive GC even with zero
    // listeners — then once the bytes flush (`BytesSent`), it is collectible.
    with_realtime_vm(false, |vm, handle| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        vm.eval("s.send('payload-bytes'); globalThis.s = null;")
            .unwrap(); // buffered_amount > 0, drop the reference
        let _ = handle.drain_recorded_outgoing(); // clear ctor open + send command

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            1,
            "an OPEN WebSocket with queued data must survive GC (no-listener clause)",
        );
        assert_eq!(
            outgoing_count(handle, "WebSocketClose("),
            0,
            "a data-queued connection must NOT be force-closed",
        );

        // Bytes flushed → buffered_amount drops to 0 → now a genuine orphan.
        inject_ws(vm, 0, WsEvent::BytesSent(64));
        let _ = handle.drain_recorded_outgoing();
        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            0,
            "once the queued data flushes, a listener-less WebSocket is collectible",
        );
        assert_eq!(outgoing_count(handle, "WebSocketClose("), 1);
    });
}

#[test]
fn ws_closed_never_kept() {
    // CLOSED is never kept — even with a `close` listener registered.
    with_realtime_vm(false, |vm, handle| {
        vm.eval(
            "new WebSocket('ws://example.com/socket') \
                 .addEventListener('close', function () {});",
        )
        .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        inject_ws(
            vm,
            0,
            WsEvent::Closed {
                code: 1000,
                reason: "normal".to_string(),
                was_clean: true,
            },
        ); // → CLOSED
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            0,
            "a CLOSED WebSocket must be collected even with a close listener",
        );
    });
}

#[test]
fn ws_onmessage_handler_only_survives_gc() {
    // The on-handler path counts too: `ws.onmessage = cb` (no addEventListener,
    // no retained wrapper) keeps an OPEN WebSocket alive.
    with_realtime_vm(false, |vm, handle| {
        vm.eval("new WebSocket('ws://example.com/socket').onmessage = function () {};")
            .unwrap();
        inject_ws(vm, 0, ws_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            ws_state_count(vm),
            1,
            "an onmessage-handler-only OPEN WebSocket must survive GC",
        );
        assert_eq!(outgoing_count(handle, "WebSocketClose("), 0);
    });
}

// ---------------------------------------------------------------------------
// EventSource — HTML §9.2.9
// ---------------------------------------------------------------------------

#[test]
fn es_listener_held_open_survives_gc_no_force_close_and_keeps_delivering() {
    // Headline: a listener-only OPEN EventSource (no retained reference) must
    // survive GC, NOT be force-closed, and keep delivering.
    with_realtime_vm(true, |vm, handle| {
        vm.eval(
            "globalThis.n = 0; \
             new EventSource('/events') \
                 .addEventListener('message', function () { globalThis.n++; });",
        )
        .unwrap();
        inject_sse(vm, 0, sse_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing(); // clear ctor EventSourceOpen

        vm.inner.collect_garbage();
        assert_eq!(
            es_state_count(vm),
            1,
            "a listener-held OPEN EventSource must survive GC via the keepalive seam",
        );
        assert_eq!(
            outgoing_count(handle, "EventSourceClose("),
            0,
            "a survived connection must NOT be force-closed",
        );

        inject_sse(
            vm,
            0,
            SseEvent::Event {
                event_type: "message".to_string(),
                data: "hello".to_string(),
                last_event_id: String::new(),
            },
        );
        assert_eval_number(vm, "n", 1.0);
    });
}

#[test]
fn es_orphan_open_collected_and_force_closed() {
    // Negative control: an OPEN EventSource with no listener is collected AND
    // force-closed (§9.2.9 GC-while-open ⇒ abort fetch). The spec's
    // task-queued no-listener clause is vacuous under elidex's inline drain, so
    // a no-listener OPEN source has no keepalive.
    with_realtime_vm(true, |vm, handle| {
        vm.eval("new EventSource('/events');").unwrap();
        inject_sse(vm, 0, sse_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            es_state_count(vm),
            0,
            "an orphan OPEN EventSource must be collected"
        );
        assert_eq!(
            outgoing_count(handle, "EventSourceClose("),
            1,
            "a swept orphan must be force-closed (EventSourceClose emitted)",
        );
    });
}

#[test]
fn es_open_with_only_open_listener_collected() {
    // Tier proof: `open` is NOT in the ES OPEN tier {message,error}, so an OPEN
    // EventSource whose only listener is `open` is collected + force-closed.
    with_realtime_vm(true, |vm, handle| {
        vm.eval("new EventSource('/events').addEventListener('open', function () {});")
            .unwrap();
        inject_sse(vm, 0, sse_connected()); // → OPEN
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            es_state_count(vm),
            0,
            "an OPEN EventSource with only an out-of-tier `open` listener must be collected",
        );
        assert_eq!(outgoing_count(handle, "EventSourceClose("), 1);
    });
}

#[test]
fn es_connecting_open_listener_survives_gc() {
    // CONNECTING tier includes `open` (unlike OPEN): a CONNECTING EventSource
    // with an `open` listener survives a GC (e.g. fired before the handshake
    // completes). Proves the per-state tier, not a single flat type-set.
    with_realtime_vm(true, |vm, handle| {
        vm.eval("new EventSource('/events').addEventListener('open', function () {});")
            .unwrap();
        // No Connected injected → still CONNECTING.
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            es_state_count(vm),
            1,
            "a CONNECTING EventSource with an `open` listener must survive GC",
        );
        assert_eq!(outgoing_count(handle, "EventSourceClose("), 0);
    });
}

#[test]
fn es_closed_never_kept() {
    // CLOSED is never kept — even with a message listener. A fatal error closes
    // the source; the next GC collects it.
    with_realtime_vm(true, |vm, handle| {
        vm.eval(
            "new EventSource('/events') \
                 .addEventListener('message', function () {});",
        )
        .unwrap();
        inject_sse(vm, 0, sse_connected()); // → OPEN
        inject_sse(vm, 0, SseEvent::FatalError("server gone".to_string())); // → CLOSED
        let _ = handle.drain_recorded_outgoing();

        vm.inner.collect_garbage();
        assert_eq!(
            es_state_count(vm),
            0,
            "a CLOSED EventSource must be collected even with a message listener",
        );
    });
}

#[test]
fn unbind_force_closes_even_listener_held_connection() {
    // Regression guard for the §8.4 distinction: the GC keepalive keeps a
    // listener-held connection, but `Vm::unbind` (the spec's "Document goes
    // away ⇒ make disappear / forcibly close") force-closes EVERY connection,
    // listener-held or not — so a connection the keepalive just kept across a GC
    // is still force-closed on unbind. (The general per-conn unbind teardown is
    // covered in `tests_realtime`; here we assert it holds for a connection the
    // S5-3b keepalive arm actively kept alive.) Manual setup so the single
    // `unbind` + post-unbind drain ordering is observable.
    let mut vm = Vm::new();
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page/").expect("valid base URL");
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
        "new WebSocket('ws://example.com/socket') \
             .addEventListener('message', function () {});",
    )
    .unwrap();
    inject_ws(&mut vm, 0, ws_connected()); // → OPEN
    vm.inner.collect_garbage(); // keepalive keeps it
    assert_eq!(
        ws_state_count(&vm),
        1,
        "keepalive kept the listener-held connection"
    );
    let _ = handle.drain_recorded_outgoing();

    vm.unbind();
    assert_eq!(
        outgoing_count(&handle, "WebSocketClose("),
        1,
        "unbind must force-close even a keepalive-kept connection (document teardown)",
    );
    drop(session);
    drop(dom);
}
