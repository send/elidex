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

use std::rc::Rc;
use std::time::{Duration, Instant};

use elidex_api_workers::{spawn_worker, WorkerToParent};
use elidex_ecs::{EcsDom, Entity};
use elidex_net::broker::NetworkHandle;
use elidex_net::{CredentialsMode, HttpVersion, Response as NetResponse};
use elidex_script_session::SessionCore;
use url::Url;

use super::super::host_data::HostData;
use super::super::value::JsValue;
use super::super::worker_thread::{run_worker, run_worker_with_source};
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

/// Run `f` against a bound dedicated-worker VM. `session` / `dom` are declared
/// **before** `vm`, and `vm` is dropped first, satisfying the `Vm::bind_worker`
/// safety contract (the pointed-to `SessionCore` / `EcsDom` must outlive the
/// bound VM). `secure` is the creator-inherited `isSecureContext` flag.
fn with_worker_vm<F, R>(name: &str, url: &str, secure: bool, f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut vm = Vm::new_worker(
        name.to_string(),
        Url::parse(url).unwrap(),
        secure,
        CredentialsMode::SameOrigin,
        elidex_plugin::EngineMode::BrowserCompat,
    );
    unsafe { bind_worker_vm(&mut vm, &mut session, &mut dom, doc) };
    let result = f(&mut vm);
    // `vm` (whose `HostData` holds raw pointers into `session` / `dom`) drops
    // first, while both are still live.
    drop(vm);
    drop(session);
    drop(dom);
    result
}

// ---------------------------------------------------------------------------
// Worker global scope
// ---------------------------------------------------------------------------

#[test]
fn worker_self_is_global() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert!(eval_bool_on(vm, "self === globalThis"));
        assert!(eval_bool_on(vm, "typeof self.postMessage === 'function'"));
        assert!(eval_bool_on(vm, "typeof self.close === 'function'"));
        assert!(eval_bool_on(
            vm,
            "typeof self.addEventListener === 'function'"
        ));
    });
}

#[test]
fn worker_has_no_document_or_window() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert!(eval_bool_on(vm, "typeof document === 'undefined'"));
        assert!(eval_bool_on(vm, "typeof window === 'undefined'"));
    });
}

#[test]
fn worker_name_and_location_and_navigator() {
    with_worker_vm("my-worker", WORKER_URL, true, |vm| {
        assert_eq!(eval_str_on(vm, "self.name"), "my-worker");
        assert_eq!(eval_str_on(vm, "self.location.href"), WORKER_URL);
        assert_eq!(eval_str_on(vm, "self.location.protocol"), "https:");
        assert_eq!(eval_str_on(vm, "self.location.toString()"), WORKER_URL);
        assert!(eval_bool_on(
            vm,
            "typeof self.navigator.userAgent === 'string'"
        ));
        assert!(eval_bool_on(vm, "self.isSecureContext === true"));
    });
}

#[test]
fn worker_is_secure_context_inherits_from_creator_not_script_url() {
    // The flag is threaded from the creator, NOT derived from the worker script
    // URL (WHATWG HTML §8.1.3.5). An https script URL with a non-secure creator
    // reads `false`; a secure creator reads `true` regardless of the URL.
    with_worker_vm("", WORKER_URL, false, |vm| {
        assert!(eval_bool_on(vm, "self.isSecureContext === false"));
    });
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert!(eval_bool_on(vm, "self.isSecureContext === true"));
    });
}

// ---------------------------------------------------------------------------
// Inbound message delivery (dispatch_worker_message)
// ---------------------------------------------------------------------------

#[test]
fn onmessage_handler_receives_data_and_origin() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        vm.eval(
            "globalThis.got = null; globalThis.gotOrigin = null;
             self.onmessage = function(e) { globalThis.got = e.data; globalThis.gotOrigin = e.origin; };",
        )
        .unwrap();
        vm.inner
            .dispatch_worker_message("\"hello\"", "https://sender.example");

        assert_eq!(eval_str_on(vm, "globalThis.got"), "hello");
        assert_eq!(
            eval_str_on(vm, "globalThis.gotOrigin"),
            "https://sender.example"
        );
    });
}

#[test]
fn add_event_listener_message_receives_in_order() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        vm.eval(
            "globalThis.rx = [];
             self.addEventListener('message', function(e) { globalThis.rx.push(e.data); });",
        )
        .unwrap();
        vm.inner.dispatch_worker_message("\"a\"", "o");
        vm.inner.dispatch_worker_message("42", "o");

        assert_eq!(eval_str_on(vm, "globalThis.rx.join(',')"), "a,42");
    });
}

// ---------------------------------------------------------------------------
// importScripts (WHATWG HTML §10.3.1) — synchronous fetch + validate + run,
// driven through a mock NetworkHandle (no thread).
// ---------------------------------------------------------------------------

/// Build a 200 response for `url` with `content_type` and `body`.
fn js_response(url: &str, content_type: &str, body: &'static [u8]) -> NetResponse {
    let parsed = Url::parse(url).expect("valid URL");
    NetResponse {
        status: 200,
        headers: vec![("content-type".to_string(), content_type.to_string())],
        body: bytes::Bytes::from_static(body),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
        is_redirect_tainted: false,
        credentialed_network: false,
    }
}

#[test]
fn import_scripts_fetches_validates_and_runs() {
    let helper = "https://example.com/app/helper.js";
    let response = js_response(
        helper,
        "application/javascript",
        b"globalThis.imported = 42;",
    );
    with_worker_vm("", WORKER_URL, true, |vm| {
        let handle = Rc::new(NetworkHandle::mock_with_responses(vec![(
            Url::parse(helper).unwrap(),
            Ok(response),
        )]));
        vm.install_network_handle(handle.clone());
        // Relative URL resolves against the worker script URL base
        // (https://example.com/app/worker.js → .../app/helper.js).
        vm.eval("importScripts('helper.js');")
            .expect("importScripts");
        assert!(eval_bool_on(vm, "globalThis.imported === 42"));

        // The fetch must carry the worker's own origin + credentials mode
        // (F-R6-1/F-R6-3) — not `origin = None` (which would attach cookies
        // unconditionally).
        let recorded = handle.drain_recorded_requests();
        let req = recorded.last().expect("importScripts issued a request");
        assert_eq!(
            req.origin,
            Some(Url::parse(WORKER_URL).unwrap().origin()),
            "importScripts request must carry the worker origin"
        );
        assert_eq!(req.credentials, CredentialsMode::SameOrigin);
        // Explicit SameOrigin mode (not the NoCors default) so the broker
        // applies CORS/same-origin gating (F-R7-1).
        assert_eq!(req.mode, elidex_net::RequestMode::SameOrigin);
    });
}

#[test]
fn import_scripts_rejects_non_js_mime() {
    let helper = "https://example.com/app/helper.js";
    let response = js_response(helper, "text/html", b"globalThis.imported = 1;");
    with_worker_vm("", WORKER_URL, true, |vm| {
        vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(vec![(
            Url::parse(helper).unwrap(),
            Ok(response),
        )])));
        assert!(
            vm.eval("importScripts('helper.js');").is_err(),
            "non-JavaScript MIME must reject (WHATWG HTML §10.2.4)"
        );
    });
}

// ---------------------------------------------------------------------------
// Outbound message + close state
// ---------------------------------------------------------------------------

#[test]
fn post_message_queues_serialized_outgoing() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        vm.eval("postMessage({ a: 1 }); postMessage('hi');")
            .unwrap();
        assert_eq!(
            vm.inner.worker_outgoing,
            vec!["{\"a\":1}".to_string(), "\"hi\"".to_string()]
        );
    });
}

/// Regression (Copilot R12): `postMessage` serialization must NOT intern the
/// transient JSON blob into the GC-less `StringPool`. With numeric payloads
/// (no string literals to intern as live values), a high-frequency send loop
/// must leave the pool size effectively unchanged — pre-fix this grew by one
/// entry per message (`native_json_stringify` interned every `"0"`, `"1"`, …).
#[test]
fn post_message_does_not_intern_transient_json() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        // Warm up: compile the loop + intern its identifiers once.
        vm.eval("for (let i = 0; i < 8; i++) postMessage(i);")
            .unwrap();
        let before = vm.inner.strings.len();
        vm.eval("for (let i = 0; i < 500; i++) postMessage(i);")
            .unwrap();
        let grew = vm.inner.strings.len() - before;
        assert!(
            grew < 50,
            "StringPool grew by {grew} over 500 numeric postMessage sends \
             (expected ~0 — transient JSON must not be interned)"
        );
        assert!(vm.inner.worker_outgoing.len() >= 500);
    });
}

#[test]
fn close_sets_close_requested() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert!(!vm.inner.worker_close_requested);
        vm.eval("close();").unwrap();
        assert!(vm.inner.worker_close_requested);
    });
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
            true,
            CredentialsMode::SameOrigin,
            None,
            elidex_plugin::EngineMode::BrowserCompat,
            &ch,
        );
    });

    handle.post_message("\"ping\"".to_string(), "https://example.com".to_string());

    match recv_within(&handle, Duration::from_secs(5)) {
        Some(WorkerToParent::PostMessage { data, .. }) => assert_eq!(data, "\"ping pong\""),
        other => panic!("expected echoed PostMessage, got {other:?}"),
    }
}

/// Proves the Q3 main-side mechanism end-to-end: a `Send` sibling
/// `NetworkHandle` minted on this (parent) thread crosses into the worker
/// thread's `spawn_worker` closure (compiles ⇒ `NetworkHandle: Send`) and is
/// installed, so `fetch` is available inside the worker. This is exactly the
/// path the main-side `Worker` constructor will use
/// (`self.network_handle.create_sibling_handle()`).
#[test]
fn worker_thread_accepts_sibling_network_handle() {
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let sibling = np.create_renderer_handle().create_sibling_handle();

    let url = Url::parse(WORKER_URL).unwrap();
    let body_url = url.clone();
    let handle = spawn_worker(String::new(), url, move |ch| {
        run_worker_with_source(
            "self.onmessage = function(e) { postMessage(typeof fetch); };",
            &body_url,
            String::new(),
            true,
            CredentialsMode::SameOrigin,
            Some(sibling),
            elidex_plugin::EngineMode::BrowserCompat,
            &ch,
        );
    });

    handle.post_message("\"probe\"".to_string(), "https://example.com".to_string());
    match recv_within(&handle, Duration::from_secs(5)) {
        Some(WorkerToParent::PostMessage { data, .. }) => assert_eq!(data, "\"function\""),
        other => panic!("expected `typeof fetch` reply, got {other:?}"),
    }
    drop(handle);
    // Keep the network process alive until the worker has exited.
    drop(np);
}

#[test]
fn worker_data_url_non_js_mime_rejected() {
    // `data:text/html,...` is not a JavaScript MIME essence — the fetch/decode
    // step must reject it (WHATWG HTML §10.2.4) rather than evaluate the HTML
    // as script (F-R9-1).
    let url = Url::parse("data:text/html,<h1>hi</h1>").unwrap();
    let body_url = url.clone();
    let handle = spawn_worker(String::new(), url, move |ch| {
        run_worker(
            &body_url,
            String::new(),
            true,
            CredentialsMode::SameOrigin,
            None,
            elidex_plugin::EngineMode::BrowserCompat,
            &ch,
        );
    });
    match recv_within(&handle, Duration::from_secs(5)) {
        Some(WorkerToParent::Error { .. } | WorkerToParent::Closed) => {}
        other => panic!("expected error/close for non-JS data: MIME, got {other:?}"),
    }
}

#[test]
fn worker_thread_close_sends_closed_and_exits() {
    let url = Url::parse(WORKER_URL).unwrap();
    let body_url = url.clone();
    let handle = spawn_worker(String::new(), url, move |ch| {
        run_worker_with_source(
            "close();",
            &body_url,
            String::new(),
            true,
            CredentialsMode::SameOrigin,
            None,
            elidex_plugin::EngineMode::BrowserCompat,
            &ch,
        );
    });

    assert!(
        matches!(
            recv_within(&handle, Duration::from_secs(5)),
            Some(WorkerToParent::Closed)
        ),
        "worker should report Closed after close()"
    );
}

// ---------------------------------------------------------------------------
// Main-side `Worker` IDL (constructor + postMessage / terminate / onmessage /
// onerror, end-to-end through a real worker thread). These exercise both
// endpoints: the `data:` worker script runs the runtime harness on its own
// thread, and `Vm::drain_worker_messages` converts the inbound frames into DOM
// events on the main VM.
// ---------------------------------------------------------------------------

const PAGE_URL: &str = "https://example.com/app/index.html";

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// Run `f` against a main-thread (Window) VM bound to a fresh session, with the
/// navigation URL at `https://example.com/app/` so relative / same-origin
/// worker scripts resolve. No network handle is installed — the tests use
/// `data:` worker scripts, which the runtime harness decodes inline.
fn with_main_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    vm.inner.navigation.current_url = Url::parse(PAGE_URL).expect("valid base URL");
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { super::super::test_helpers::bind_vm(&mut vm, &mut session, &mut dom, doc) };
    let guard = UnbindOnDrop(&mut vm);
    let result = f(guard.0);
    drop(guard);
    drop(session);
    drop(dom);
    result
}

/// Drive `vm.drain_worker_messages()` (the main event-loop step) until the JS
/// `predicate` evaluates truthy or `timeout` elapses. Returns whether the
/// predicate became true.
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

#[test]
fn main_worker_round_trip_via_onmessage() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.got = null;
               const w = new Worker("data:text/javascript,self.onmessage=function(e){postMessage(e.data+'-pong')}");
               w.onmessage = function(e) { globalThis.got = e.data; };
               w.postMessage("ping");"#,
        )
        .expect("ctor + postMessage succeed");

        assert!(
            pump_until(vm, "globalThis.got !== null", Duration::from_secs(5)),
            "worker reply never arrived"
        );
        assert_eq!(eval_str_on(vm, "globalThis.got"), "ping-pong");
    });
}

#[test]
fn main_worker_add_event_listener_receives_message() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.rx = [];
               const w = new Worker("data:text/javascript,self.onmessage=function(e){postMessage(e.data)}");
               w.addEventListener("message", function(e) { globalThis.rx.push(e.data); });
               w.postMessage(7);"#,
        )
        .expect("ctor + postMessage succeed");

        assert!(
            pump_until(vm, "globalThis.rx.length > 0", Duration::from_secs(5)),
            "worker reply never arrived"
        );
        assert_eq!(eval_str_on(vm, "globalThis.rx.join(',')"), "7");
    });
}

#[test]
fn main_worker_message_event_target_and_origin() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.targetIsWorker = null; globalThis.origin = null;
               const w = new Worker("data:text/javascript,self.onmessage=function(){postMessage('x')}");
               globalThis.w = w;
               w.onmessage = function(e) {
                 globalThis.targetIsWorker = (e.target === w);
                 globalThis.origin = e.origin;
               };
               w.postMessage(0);"#,
        )
        .expect("ctor + postMessage succeed");

        assert!(
            pump_until(
                vm,
                "globalThis.targetIsWorker !== null",
                Duration::from_secs(5)
            ),
            "worker reply never arrived"
        );
        assert!(eval_bool_on(vm, "globalThis.targetIsWorker === true"));
        // The worker scope's origin is the (opaque) `data:` URL origin.
        assert_eq!(eval_str_on(vm, "globalThis.origin"), "null");
    });
}

#[test]
fn main_worker_terminate_is_callable_and_idempotent() {
    with_main_vm(|vm| {
        vm.eval(
            r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");
               w.terminate();
               w.terminate();"#,
        )
        .expect("terminate is idempotent");
    });
}

#[test]
fn main_worker_terminate_uncaches_wrapper() {
    use super::super::host::worker::WorkerRef;
    with_main_vm(|vm| {
        vm.eval(r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}"); w.terminate();"#)
            .expect("ctor + terminate");
        // The `NodeKind::Worker` entity persists (brand-check home), but its
        // cached `Worker` wrapper must be dropped so it is no longer GC-rooted
        // (F-R2-2 — otherwise create/terminate cycles leak wrappers).
        let hd = vm.host_data().expect("bound");
        let entity = {
            let mut q = hd.dom_shared().world().query::<(Entity, &WorkerRef)>();
            q.iter().map(|(e, _)| e).next()
        }
        .expect("worker entity exists");
        assert!(
            hd.get_cached_wrapper(entity).is_none(),
            "terminated worker's wrapper must be uncached (not GC-rooted)"
        );
    });
}

#[test]
fn drain_live_set_drops_terminated_workers() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.a = new Worker("data:text/javascript,self.onmessage=function(){}");
               globalThis.b = new Worker("data:text/javascript,self.onmessage=function(){}");
               globalThis.a.terminate();"#,
        )
        .expect("two ctors + one terminate");
        // The drain iterates `worker_entities` (live workers only), NOT a full
        // `WorkerRef` world scan — so a terminated worker is dropped from the
        // set and the per-frame cost stays O(live workers) (F-R4-1).
        assert_eq!(
            vm.inner.worker_entities.len(),
            1,
            "terminated worker must be removed from the live drain set"
        );
    });
}

#[test]
fn worker_wrappers_uncached_on_unbind() {
    use super::super::host::worker::WorkerRef;
    let mut vm = Vm::new();
    vm.inner.navigation.current_url = Url::parse(PAGE_URL).unwrap();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { super::super::test_helpers::bind_vm(&mut vm, &mut session, &mut dom, doc) };

    vm.eval(r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");"#)
        .expect("ctor");
    let entity = {
        let hd = vm.host_data().expect("bound");
        let mut q = hd.dom_shared().world().query::<(Entity, &WorkerRef)>();
        q.iter().map(|(e, _)| e).next()
    }
    .expect("worker entity");

    vm.unbind();

    // After unbind, the `Worker` wrapper must be uncached (F-R3-2): the drain
    // early-returns on the now-empty registry, so an un-removed wrapper would
    // stay GC-rooted across navigation.
    let hd = vm.host_data().expect("host data installed");
    assert!(
        hd.get_cached_wrapper(entity).is_none(),
        "Worker wrapper must be uncached on unbind"
    );

    drop(vm);
    drop(session);
    drop(dom);
}

#[test]
fn main_worker_cross_origin_url_throws_security_error() {
    with_main_vm(|vm| {
        let name = eval_str_on(
            vm,
            "try { new Worker('https://evil.example/w.js'); 'no-throw' } \
             catch (e) { e.name }",
        );
        assert_eq!(name, "SecurityError");
    });
}

#[test]
fn main_worker_module_type_throws() {
    with_main_vm(|vm| {
        let threw = eval_bool_on(
            vm,
            "try { new Worker('data:text/javascript,', { type: 'module' }); false } \
             catch (e) { true }",
        );
        assert!(threw, "{{ type: 'module' }} must be rejected");
    });
}

#[test]
fn main_worker_is_not_a_node() {
    with_main_vm(|vm| {
        // A `Worker` object is a `HostObject` over a `NodeKind::Worker` entity,
        // but Worker is NOT a Node — Node-argument coercion must reject it so it
        // can't be grafted into the DOM tree (F-R8-1).
        let threw = eval_bool_on(
            vm,
            r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");
               try { document.appendChild(w); false } catch (e) { e instanceof TypeError }"#,
        );
        assert!(threw, "Worker must be rejected as a Node argument");
    });
}

#[test]
fn main_worker_blob_url_rejected() {
    with_main_vm(|vm| {
        // `blob:` worker scripts are rejected until a blob-URL source loader
        // exists (F-R6-4 — no `URL.createObjectURL`; defer `#11-worker-blob-script`).
        let threw = eval_bool_on(
            vm,
            "try { new Worker('blob:https://example.com/uuid-1'); false } catch (e) { true }",
        );
        assert!(threw, "blob: worker URL must be rejected");
    });
}

#[test]
fn main_worker_post_message_circular_throws_data_clone() {
    with_main_vm(|vm| {
        // A circular structure rejects as a `DataCloneError` DOMException —
        // `e.name` must read "DataCloneError" and `e instanceof DOMException`
        // hold (WHATWG WebIDL §3.14 / HTML §10.2.6.3), not a bare TypeError.
        let name = eval_str_on(
            vm,
            r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");
               const o = {}; o.self = o;
               try { w.postMessage(o); 'no-throw' } catch (e) { e.name }"#,
        );
        assert_eq!(name, "DataCloneError");
        assert!(eval_bool_on(
            vm,
            r#"const w = new Worker("data:text/javascript,self.onmessage=function(){}");
               const o = {}; o.self = o;
               try { w.postMessage(o); false } catch (e) { e instanceof DOMException }"#,
        ));
    });
}

#[test]
fn worker_scope_post_message_circular_throws_data_clone() {
    // Worker-side `self.postMessage` of a circular structure rejects as a
    // `DataCloneError` DOMException (WHATWG HTML §10.2.1.2).
    with_worker_vm("", WORKER_URL, true, |vm| {
        assert_eq!(
            eval_str_on(
                vm,
                "const o = {}; o.self = o; try { postMessage(o); 'no-throw' } catch (e) { e.name }",
            ),
            "DataCloneError"
        );
    });
}

/// Regression (Copilot R13): a user exception thrown *during* serialization (a
/// throwing `toJSON` / property getter) must propagate unchanged — NOT be
/// masked as `DataCloneError`. WHATWG HTML postMessage runs StructuredSerialize,
/// whose `[[Get]]` propagates getter throws; only structural "cannot represent"
/// failures (circular / BigInt / depth) map to `DataCloneError`.
#[test]
fn post_message_throwing_getter_propagates_original_error() {
    with_worker_vm("", WORKER_URL, true, |vm| {
        // A throwing getter surfaces the user's TypeError verbatim, not a
        // DataCloneError DOMException.
        assert!(eval_bool_on(
            vm,
            r"const o = { get x() { throw new TypeError('boom'); } };
               try { postMessage(o); false }
               catch (e) { e instanceof TypeError && e.message === 'boom' }",
        ));
        // Sanity: the structural circular case still maps to DataCloneError.
        assert_eq!(
            eval_str_on(
                vm,
                "const c = {}; c.self = c; try { postMessage(c); 'no-throw' } catch (e) { e.name }",
            ),
            "DataCloneError"
        );
    });
}

#[test]
fn main_worker_uncaught_error_fires_onerror() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.errMsg = null; globalThis.errVal = null;
               const w = new Worker("data:text/javascript,throw new Error('boom')");
               w.onerror = function(e) { globalThis.errMsg = e.message; globalThis.errVal = e.error; };"#,
        )
        .expect("ctor succeeds even though the worker script throws");

        assert!(
            pump_until(vm, "globalThis.errMsg !== null", Duration::from_secs(5)),
            "worker error never propagated"
        );
        assert!(
            eval_str_on(vm, "globalThis.errMsg").contains("boom"),
            "error message should carry the worker error"
        );
        // ErrorEvent.error carries the (cross-thread string) error value, not
        // null (F-R10-1).
        assert!(
            eval_str_on(vm, "globalThis.errVal").contains("boom"),
            "ErrorEvent.error should carry the error value"
        );
    });
}

#[test]
fn main_concurrent_workers_each_receive_their_own_reply() {
    with_main_vm(|vm| {
        vm.eval(
            r#"globalThis.a = null; globalThis.b = null;
               const wa = new Worker("data:text/javascript,self.onmessage=function(e){postMessage('a:'+e.data)}");
               const wb = new Worker("data:text/javascript,self.onmessage=function(e){postMessage('b:'+e.data)}");
               wa.onmessage = function(e){ globalThis.a = e.data; };
               wb.onmessage = function(e){ globalThis.b = e.data; };
               wa.postMessage(1); wb.postMessage(2);"#,
        )
        .expect("two ctors + posts succeed");

        assert!(
            pump_until(
                vm,
                "globalThis.a !== null && globalThis.b !== null",
                Duration::from_secs(5)
            ),
            "both worker replies never arrived"
        );
        assert_eq!(eval_str_on(vm, "globalThis.a"), "a:1");
        assert_eq!(eval_str_on(vm, "globalThis.b"), "b:2");
    });
}

#[test]
fn worker_ctor_requires_new() {
    super::assert_ctor_requires_new("Worker('foo.js')", "Worker");
}
