//! S5-6b — "survival across per-turn `unbind`" boundary.
//!
//! The FLIP splits `Vm::unbind` into a per-turn `unbind` (which NO LONGER
//! force-closes `WebSocket` / `EventSource` connections nor terminates
//! dedicated workers) plus a per-document `Vm::teardown_document`. These two
//! tests pin the load-bearing invariant: a realtime connection / a dedicated
//! worker must SURVIVE repeated per-turn `unbind` (an unbind→bind cycle is the
//! per-turn re-establishment against the same session/dom/doc) and stay
//! deliverable afterwards.
//!
//! Distinct concern from the within-bind-cycle GC keepalive
//! (`tests_realtime_keepalive`) and the worker round-trip
//! (`tests_worker`) suites; kept in a dedicated file so neither of those
//! (already near the 1000-line convention) grows further.

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use std::rc::Rc;
use std::time::{Duration, Instant};

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_net::broker::{NetworkHandle, NetworkToRenderer};
use elidex_net::sse::SseEvent;
use elidex_net::ws::WsEvent;
use elidex_script_session::SessionCore;
use url::Url;

use super::super::host::worker::WorkerRef;
use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::assert_eval_bool;
use super::assert_eval_number;

const PAGE_URL: &str = "https://example.com/app/index.html";

fn build_min_doc(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
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
        final_url: Url::parse("http://example.com/events").expect("valid URL"),
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

fn eval_bool_on(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).expect("eval succeeds") {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// Drive `vm.drain_worker_messages()` until `predicate` evaluates truthy or
/// `timeout` elapses; returns whether the predicate became true. Mirror of
/// `tests_worker::pump_until`.
fn pump_until(vm: &mut Vm, predicate: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        vm.drain_worker_messages();
        if eval_bool_on(vm, predicate) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ---------------------------------------------------------------------------
// Realtime (WebSocket + EventSource) — survives per-turn unbind
// ---------------------------------------------------------------------------

#[test]
fn realtime_survives_per_turn_unbind_and_keeps_delivering() {
    let mut vm = Vm::new();
    // `http` page scheme so `ws://` is not mixed-content blocked; the
    // `EventSource` target is same-scheme `http://`.
    vm.inner.navigation.current_url =
        Url::parse("http://example.com/page/").expect("valid base URL");
    let handle = Rc::new(NetworkHandle::mock_with_responses(vec![]));
    vm.install_network_handle(Rc::clone(&handle));

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_doc(&mut dom);
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Both wrappers are hard-rooted via `globalThis.*` and listener-anchored.
    vm.eval(
        "globalThis.wsMsgs = 0; globalThis.sseMsgs = 0; \
         globalThis.ws = new WebSocket('ws://example.com/socket'); \
         globalThis.ws.addEventListener('message', function () { globalThis.wsMsgs++; }); \
         globalThis.es = new EventSource('http://example.com/events'); \
         globalThis.es.addEventListener('message', function () { globalThis.sseMsgs++; });",
    )
    .expect("realtime ctors + listeners");

    inject_ws(&mut vm, 0, ws_connected()); // ws id-space conn 0 → OPEN
    inject_sse(&mut vm, 0, sse_connected()); // sse id-space conn 0 → OPEN
    let _ = handle.drain_recorded_outgoing(); // clear ctor Open records

    // Two per-turn re-establishments: each `unbind` must leave both connection
    // state rows intact and emit NO force-close.
    for _ in 0..2 {
        vm.unbind();
        assert_eq!(
            ws_state_count(&vm),
            1,
            "WebSocket state must survive per-turn unbind",
        );
        assert_eq!(
            es_state_count(&vm),
            1,
            "EventSource state must survive per-turn unbind",
        );
        let outgoing = handle.drain_recorded_outgoing();
        let ws_closes = outgoing
            .iter()
            .filter(|s| s.starts_with("WebSocketClose("))
            .count();
        let es_closes = outgoing
            .iter()
            .filter(|s| s.starts_with("EventSourceClose("))
            .count();
        assert_eq!(
            ws_closes, 0,
            "per-turn unbind must NOT force-close the WebSocket",
        );
        assert_eq!(
            es_closes, 0,
            "per-turn unbind must NOT force-close the EventSource",
        );
        unsafe {
            bind_vm(&mut vm, &mut session, &mut dom, doc);
        }
    }

    // Deliverable after ≥2 unbinds: a real data frame reaches the listeners.
    inject_ws(&mut vm, 0, WsEvent::TextMessage("hi".to_string()));
    inject_sse(
        &mut vm,
        0,
        SseEvent::Event {
            event_type: "message".to_string(),
            data: "hi".to_string(),
            last_event_id: String::new(),
        },
    );
    assert_eval_number(&mut vm, "wsMsgs", 1.0);
    assert_eval_number(&mut vm, "sseMsgs", 1.0);

    vm.unbind();
    drop(session);
    drop(dom);
}

// ---------------------------------------------------------------------------
// Dedicated Worker — survives per-turn unbind
// ---------------------------------------------------------------------------

#[test]
fn worker_survives_per_turn_unbind_and_target_identity_holds() {
    let mut vm = Vm::new();
    vm.inner.navigation.current_url = Url::parse(PAGE_URL).expect("valid base URL");

    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Hard-root the worker via `globalThis.w`; record its identity in the
    // `onmessage` handler (do NOT postMessage yet).
    vm.eval(
        r#"globalThis.rx = 0; globalThis.targetOk = false;
           const w = new Worker("data:text/javascript,self.onmessage=function(){postMessage('x')}");
           globalThis.w = w;
           w.onmessage = function(e) { globalThis.rx++; globalThis.targetOk = (e.target === w); };"#,
    )
    .expect("worker ctor + onmessage");

    let entity = {
        let hd = vm.host_data().expect("bound");
        let mut q = hd.dom_shared().world().query::<(Entity, &WorkerRef)>();
        q.iter().map(|(e, _)| e).next()
    }
    .expect("worker entity");
    assert_eq!(
        vm.inner.worker_entities.len(),
        1,
        "exactly one live worker registered",
    );

    // Two per-turn re-establishments: the worker registry survives (NOT
    // terminated) and the Node-kind Worker wrapper is retained by the per-turn
    // `retain(kind == Node)` sweep.
    for _ in 0..2 {
        vm.unbind();
        assert_eq!(
            vm.inner.worker_entities.len(),
            1,
            "worker registry must survive per-turn unbind (not terminated)",
        );
        assert!(
            vm.host_data()
                .expect("host data installed")
                .get_cached_wrapper(entity)
                .is_some(),
            "Node-kind Worker wrapper must be retained across per-turn unbind",
        );
        unsafe {
            bind_vm(&mut vm, &mut session, &mut dom, doc);
        }
    }

    // Deliverable after ≥2 unbinds: the worker round-trip completes and the
    // reply's `event.target` is still the same JS-held `Worker` object.
    vm.eval("w.postMessage(0)").expect("postMessage");
    assert!(
        pump_until(&mut vm, "globalThis.rx > 0", Duration::from_secs(5)),
        "worker reply never arrived after per-turn unbinds",
    );
    assert_eval_bool(&mut vm, "globalThis.targetOk === true", true);

    vm.unbind();
    drop(session);
    drop(dom);
}
