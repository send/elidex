//! Dedicated Web Worker tests (slot `#11-web-workers-vm`, PR-A).
//!
//! Two layers:
//! 1. Direct worker-VM assertions (no thread / no network) — worker globals,
//!    `self.onmessage` / `addEventListener` delivery via
//!    `dispatch_worker_message`, and `close()` / `postMessage` state.
//! 2. Thread-level round-trip + close via `elidex_api_workers::spawn_worker` +
//!    `run_worker_with_source` (the post-fetch runtime-harness seam).

#![cfg(feature = "engine")]
// Worker VMs are bound via the `unsafe` `Vm::bind_worker` raw-pointer contract
// (identical to `Vm::bind`); the binding is scoped to each test's locals.
#![allow(unsafe_code)]

use std::time::{Duration, Instant};

use elidex_api_workers::{spawn_worker, WorkerToParent};
use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;
use url::Url;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::worker_thread::run_worker_with_source;
use super::super::Vm;

const WORKER_URL: &str = "https://example.com/app/worker.js";

/// Bind `vm` (a worker-mode VM) against `session` / `dom`. The caller must keep
/// both alive and untouched for the VM's lifetime (raw-pointer aliasing
/// contract — see [`Vm::bind_worker`]).
#[allow(unsafe_code)]
unsafe fn bind_worker_vm(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, doc: Entity) {
    if vm.host_data().is_none() {
        vm.install_host_data(HostData::new());
    }
    unsafe {
        vm.bind_worker(std::ptr::from_mut(session), std::ptr::from_mut(dom), doc);
    }
}

fn eval_str_on(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).expect("eval succeeds") {
        JsValue::String(sid) => vm.get_string(sid),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_bool_on(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).expect("eval succeeds") {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// Receive the first matching `WorkerToParent` within `timeout`, polling the
/// handle (the worker runs on its own thread).
fn recv_within(
    handle: &elidex_api_workers::WorkerHandle,
    timeout: Duration,
) -> Option<WorkerToParent> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match handle.try_recv() {
            Ok(msg) => return Some(msg),
            Err(_) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Worker global scope
// ---------------------------------------------------------------------------

#[test]
fn worker_self_is_global() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    assert!(eval_bool_on(&mut vm, "self === globalThis"));
    assert!(eval_bool_on(
        &mut vm,
        "typeof self.postMessage === 'function'"
    ));
    assert!(eval_bool_on(&mut vm, "typeof self.close === 'function'"));
    assert!(eval_bool_on(
        &mut vm,
        "typeof self.addEventListener === 'function'"
    ));
}

#[test]
fn worker_has_no_document_or_window() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    assert!(eval_bool_on(&mut vm, "typeof document === 'undefined'"));
    assert!(eval_bool_on(&mut vm, "typeof window === 'undefined'"));
}

#[test]
fn worker_name_and_location_and_navigator() {
    let mut vm = Vm::new_worker("my-worker".to_string(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    assert_eq!(eval_str_on(&mut vm, "self.name"), "my-worker");
    assert_eq!(eval_str_on(&mut vm, "self.location.href"), WORKER_URL);
    assert_eq!(eval_str_on(&mut vm, "self.location.protocol"), "https:");
    assert_eq!(eval_str_on(&mut vm, "self.location.toString()"), WORKER_URL);
    assert!(eval_bool_on(
        &mut vm,
        "typeof self.navigator.userAgent === 'string'"
    ));
    assert!(eval_bool_on(&mut vm, "self.isSecureContext === true"));
}

// ---------------------------------------------------------------------------
// Inbound message delivery (dispatch_worker_message)
// ---------------------------------------------------------------------------

#[test]
fn onmessage_handler_receives_data_and_origin() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    vm.eval(
        "globalThis.got = null; globalThis.gotOrigin = null;
         self.onmessage = function(e) { globalThis.got = e.data; globalThis.gotOrigin = e.origin; };",
    )
    .unwrap();
    vm.inner
        .dispatch_worker_message("\"hello\"", "https://sender.example");

    assert_eq!(eval_str_on(&mut vm, "globalThis.got"), "hello");
    assert_eq!(
        eval_str_on(&mut vm, "globalThis.gotOrigin"),
        "https://sender.example"
    );
}

#[test]
fn add_event_listener_message_receives_in_order() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    vm.eval(
        "globalThis.rx = [];
         self.addEventListener('message', function(e) { globalThis.rx.push(e.data); });",
    )
    .unwrap();
    vm.inner.dispatch_worker_message("\"a\"", "o");
    vm.inner.dispatch_worker_message("42", "o");

    assert_eq!(eval_str_on(&mut vm, "globalThis.rx.join(',')"), "a,42");
}

// ---------------------------------------------------------------------------
// Outbound message + close state
// ---------------------------------------------------------------------------

#[test]
fn post_message_queues_serialized_outgoing() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    vm.eval("postMessage({ a: 1 }); postMessage('hi');")
        .unwrap();
    assert_eq!(
        vm.inner.worker_outgoing,
        vec!["{\"a\":1}".to_string(), "\"hi\"".to_string()]
    );
}

#[test]
fn close_sets_close_requested() {
    let mut vm = Vm::new_worker(String::new(), Url::parse(WORKER_URL).unwrap());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };

    assert!(!vm.inner.worker_close_requested);
    vm.eval("close();").unwrap();
    assert!(vm.inner.worker_close_requested);
}

// ---------------------------------------------------------------------------
// Thread-level round-trip (spawn_worker + run_worker_with_source)
// ---------------------------------------------------------------------------

#[test]
fn worker_thread_round_trip_echo() {
    let url = Url::parse(WORKER_URL).unwrap();
    let body_url = url.clone();
    let handle = spawn_worker(String::new(), url, move |ch| {
        run_worker_with_source(
            "self.onmessage = function(e) { postMessage(e.data + ' pong'); };",
            &body_url,
            String::new(),
            None,
            &ch,
        );
    });

    handle.post_message("\"ping\"".to_string(), "https://example.com".to_string());

    match recv_within(&handle, Duration::from_secs(5)) {
        Some(WorkerToParent::PostMessage { data, .. }) => assert_eq!(data, "\"ping pong\""),
        other => panic!("expected echoed PostMessage, got {other:?}"),
    }
}

#[test]
fn worker_thread_close_sends_closed_and_exits() {
    let url = Url::parse(WORKER_URL).unwrap();
    let body_url = url.clone();
    let handle = spawn_worker(String::new(), url, move |ch| {
        run_worker_with_source("close();", &body_url, String::new(), None, &ch);
    });

    assert!(
        matches!(
            recv_within(&handle, Duration::from_secs(5)),
            Some(WorkerToParent::Closed)
        ),
        "worker should report Closed after close()"
    );
}
