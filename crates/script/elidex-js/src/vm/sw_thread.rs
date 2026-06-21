//! Service Worker thread entry + per-SW event loop (WHATWG Service Workers
//! §4; slot `#11-service-workers-vm` / D-19 PR-2).
//!
//! Runs **on the SW thread** as the body the shell coordinator spawns
//! ([`sw_thread_main`] — the VM twin of `elidex_js_boa::sw_thread::sw_thread_main`;
//! the D-26 PR7 cutover swaps the coordinator's `std::thread::spawn` from boa
//! to this fn).  It fetches + validates the SW script, builds a
//! `ServiceWorkerGlobalScope` VM bound to an empty [`EcsDom`], installs the
//! **shared** origin cache backend (DR-A) + a `Send` `NetworkHandle`, seeds
//! the client snapshot, evaluates the script, then drives the per-SW event
//! loop over `ContentToSw` messages.
//!
//! ## DR-C — the real `respondWith(promise)` drain
//!
//! The central novelty boa cannot do: after `dispatch_script_event` fires
//! `onfetch`, the loop (`pump_response`) drains microtasks + timers +
//! **network ticks** until the `respondWith` promise settles, so
//! `respondWith(fetch(req))` actually resolves (boa reads the response
//! synchronously right after the listener → a fetched Response never
//! resolves).  The same pump backs `waitUntil` for install/activate
//! (`ExtendableEvent` lifetime promises, §4.4.1).
//!
//! ## Lifecycle success (spec-faithful, parity-or-better)
//!
//! `dispatch_script_event` *reports* listener exceptions (it returns
//! `!defaultPrevented`, not a success flag), so lifecycle success is driven
//! **solely by the `waitUntil` lifetime promises** (WHATWG SW Install
//! `#install` / Activate `#activate`): a rejection or a timeout →
//! `LifecycleComplete{success:false}`; a synchronous handler throw *without*
//! `waitUntil` → success (the exception is merely reported — Chrome parity).
//! This is stricter-correct than boa's `dispatch_sw_event` (which fails the
//! lifecycle on any handler throw and stubs `waitUntil`).

#![cfg(feature = "engine")]

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;
use elidex_api_sw::{ClientSnapshot, ContentToSw, LifecycleEvent, SwResponse, SwToContent};
use elidex_ecs::EcsDom;
use elidex_net::broker::NetworkHandle;
use elidex_net::CredentialsMode;
use elidex_plugin::LocalChannel;
use elidex_script_session::SessionCore;
use elidex_storage_core::SqliteConnection;

use super::coroutine_types::PromiseStatus;
use super::host::cache::CacheBackend;
use super::host::service_worker::{event, marshal};
use super::host_data::HostData;
use super::value::{JsValue, ObjectId, ObjectKind};
use super::{Vm, VmInner};

/// 16 ms recv-timeout / drain cadence (matches the content thread).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// `SwHandle::DEFAULT_IDLE_TIMEOUT` (30 s, `elidex-api-sw`): the idle-exit
/// bound *and* the `respondWith` / `waitUntil` drain bound (SW §4.4.1
/// timed-out flag / §4.6.7).
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll cadence while waiting on an in-flight promise (network round-trip).
const POLL_INTERVAL: Duration = Duration::from_millis(2);

type SwChannel = LocalChannel<SwToContent, ContentToSw>;

/// Service Worker thread entry (the D-26 cutover target — VM twin of
/// `elidex_js_boa::sw_thread::sw_thread_main`).
///
/// Fetches the SW script through the `Send` `network_handle` (non-`Option`:
/// a missing handle is a hard error here, *not* a silent in-loop fetch
/// timeout — the whole DR-C `respondWith(fetch(req))` path depends on it,
/// F1), validates it, then runs the realm.  `cache_conn` is the shared
/// origin `Arc<Mutex<SqliteConnection>>` (DR-A) both the window VM and this
/// SW observe; `initial_clients` seeds `clients.matchAll()`.
///
/// **Derives [`BrowserCompat`](elidex_plugin::EngineMode::BrowserCompat) — not an
/// embedder-selectable mode (F10).** A SW realm has no in-process parent VM to
/// inherit a mode from, so the mode is the embedder's to supply. Taking it as a
/// parameter here would let a production embedder select
/// [`BrowserCore`](elidex_plugin::EngineMode::BrowserCore) /
/// [`App`](elidex_plugin::EngineMode::App) and create the no-storage SW realm the
/// `#[cfg(test)]` gate on `Vm::new_with_mode` prevents on the main thread (a core
/// session is contracted to expose `elidex.storage`, design §14.4.3). Until
/// `#11-async-core-storage-cookiestore` makes non-compat modes production-
/// selectable, this spawn entry hard-derives `BrowserCompat`; that PR threads the
/// authorized embedder mode in. (Tests exercise non-compat SW realms via the
/// crate-internal `run_service_worker`, which keeps the explicit-mode parameter.)
#[allow(clippy::needless_pass_by_value)]
pub fn sw_thread_main(
    script_url: url::Url,
    scope: url::Url,
    channel: LocalChannel<SwToContent, ContentToSw>,
    network_handle: NetworkHandle,
    cache_conn: Arc<Mutex<SqliteConnection>>,
    initial_clients: Vec<ClientSnapshot>,
) {
    let source = match obtain_sw_source(&script_url, &network_handle) {
        Ok(source) => source,
        Err(message) => {
            let _ = channel.send(SwToContent::Error {
                message,
                filename: script_url.to_string(),
                lineno: 0,
                colno: 0,
            });
            let _ = channel.send(SwToContent::LifecycleComplete {
                event: LifecycleEvent::Install,
                success: false,
            });
            return;
        }
    };
    run_service_worker(
        &source,
        &script_url,
        &scope,
        &channel,
        network_handle,
        cache_conn,
        initial_clients,
        DEFAULT_IDLE_TIMEOUT,
        // No parent VM to inherit from; the SW realm runs BrowserCompat until
        // `#11-async-core-storage-cookiestore` lets the embedder supply a mode (F10).
        elidex_plugin::EngineMode::BrowserCompat,
    );
}

/// Fetch + validate the SW classic script (driven by the SW "Update"
/// algorithm; the fetch + MIME/status validation is HTML §10.2.4 "fetch a
/// classic worker script", reused via `elidex_api_workers`).
fn obtain_sw_source(script_url: &url::Url, handle: &NetworkHandle) -> Result<String, String> {
    let request = elidex_net::Request {
        method: "GET".to_string(),
        url: script_url.clone(),
        origin: Some(script_url.origin()),
        credentials: CredentialsMode::SameOrigin,
        mode: elidex_net::RequestMode::SameOrigin,
        headers: vec![("Service-Worker".to_string(), "script".to_string())],
        ..Default::default()
    };
    let response = handle
        .fetch_blocking(request)
        .map_err(|e| format!("NetworkError: failed to fetch SW script: {e}"))?;
    let content_type = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str());
    elidex_api_workers::validate_worker_script_response(
        content_type,
        response.status,
        &response.body,
        script_url,
    )
    .map_err(|e| format!("NetworkError: {e}"))
}

/// Build the SW realm from already-fetched `source` and drive its event loop
/// until `Shutdown` / channel disconnect / idle timeout.  Also the unit-test
/// seam (`pump_timeout` is lowered in harness tests so the never-settle
/// branches don't wait the full 30 s).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_service_worker(
    source: &str,
    script_url: &url::Url,
    scope: &url::Url,
    channel: &SwChannel,
    network_handle: NetworkHandle,
    cache_conn: Arc<Mutex<SqliteConnection>>,
    initial_clients: Vec<ClientSnapshot>,
    pump_timeout: Duration,
    engine_mode: elidex_plugin::EngineMode,
) {
    // Declared before `vm` so they outlive it: the VM's `HostData` holds raw
    // pointers into `session` / `dom` and must drop first (reverse order).
    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    let mut session = SessionCore::new();

    let mut vm = Vm::new_service_worker(
        scope.clone(),
        script_url.clone(),
        true,
        CredentialsMode::SameOrigin,
        engine_mode,
    );
    vm.install_host_data(HostData::new());
    vm.install_network_handle(Rc::new(network_handle));
    // The SW's environment-settings API base URL is its script URL (HTML
    // §8.1.3.2 "Environment settings objects" — the API base URL):
    // `fetch()` resolves relative URLs + derives the request origin against
    // `navigation.current_url`, so seed it (else it defaults to opaque
    // `about:blank` and same-origin `respondWith(fetch(...))` misclassifies as
    // cross-origin).
    vm.inner.navigation.current_url = script_url.clone();
    // DR-A: install the SHARED origin cache backend so the window realm and
    // this SW observe one cache store (else `ensure_cache_backend` would mint
    // a private in-memory fallback and silently un-share).
    if let Some(hd) = vm.inner.host_data.as_deref_mut() {
        hd.install_cache_storage(Arc::new(CacheBackend::new(cache_conn)));
    }
    vm.inner.set_sw_clients(initial_clients);
    // SAFETY: `session` / `dom` outlive `vm` (declared before it) and are not
    // touched via any Rust reference while the VM is bound — all access goes
    // through the VM's raw pointers until it drops at function return.
    #[allow(unsafe_code)]
    unsafe {
        vm.bind_worker(
            std::ptr::from_mut(&mut session),
            std::ptr::from_mut(&mut dom),
            document,
        );
    }

    if let Err(e) = vm.eval(source) {
        let _ = channel.send(SwToContent::Error {
            message: e.message.clone(),
            filename: script_url.to_string(),
            lineno: 0,
            colno: 0,
        });
        let _ = channel.send(SwToContent::LifecycleComplete {
            event: LifecycleEvent::Install,
            success: false,
        });
        return;
    }
    drain_sw_outgoing(&mut vm, channel);

    let mut last_activity = Instant::now();
    loop {
        let timeout = FRAME_INTERVAL.min(
            DEFAULT_IDLE_TIMEOUT
                .checked_sub(last_activity.elapsed())
                .unwrap_or(Duration::ZERO),
        );
        match channel.recv_timeout(timeout) {
            Ok(msg) => {
                if !handle_message(&mut vm, channel, msg, pump_timeout) {
                    break; // Shutdown
                }
                // Reset AFTER handling (a respondWith / waitUntil pump can take
                // seconds): the idle window is measured from work *completion*,
                // not request receipt, so a heavy SW isn't torn down moments
                // after finishing a slow fetch.
                last_activity = Instant::now();
            }
            Err(RecvTimeoutError::Timeout) => {
                if last_activity.elapsed() >= DEFAULT_IDLE_TIMEOUT {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Post-tick drains (queued cache deliveries + microtask-driven late
        // `postMessage` / timers).
        pump_once(&mut vm.inner);
        drain_sw_outgoing(&mut vm, channel);
    }
}

/// Dispatch one inbound `ContentToSw`.  Returns `false` only on `Shutdown`
/// (loop exit).
fn handle_message(
    vm: &mut Vm,
    channel: &SwChannel,
    msg: ContentToSw,
    pump_timeout: Duration,
) -> bool {
    match msg {
        ContentToSw::Install => {
            let success = run_lifecycle(vm, "install", pump_timeout);
            drain_sw_outgoing(vm, channel);
            let _ = channel.send(SwToContent::LifecycleComplete {
                event: LifecycleEvent::Install,
                success,
            });
        }
        ContentToSw::Activate => {
            let success = run_lifecycle(vm, "activate", pump_timeout);
            drain_sw_outgoing(vm, channel);
            let _ = channel.send(SwToContent::LifecycleComplete {
                event: LifecycleEvent::Activate,
                success,
            });
        }
        ContentToSw::FetchEvent {
            fetch_id,
            request,
            client_id,
            resulting_client_id,
        } => {
            let response = run_fetch(vm, &request, &client_id, &resulting_client_id, pump_timeout);
            drain_sw_outgoing(vm, channel);
            let _ = match response {
                Some(response) => channel.send(SwToContent::FetchResponse { fetch_id, response }),
                None => channel.send(SwToContent::FetchPassthrough { fetch_id }),
            };
        }
        ContentToSw::PostMessage {
            data,
            origin,
            client_id: _,
        } => {
            vm.inner.dispatch_worker_message(&data, &origin);
            drain_sw_outgoing(vm, channel);
        }
        ContentToSw::ClientList { clients } => {
            vm.inner.set_sw_clients(clients);
        }
        // Sync / PeriodicSync / Notification event dispatch in the VM realm
        // is out of PR-2 scope (slot `#11-sw-push-notifications`, §G F5) — the
        // coordinator does not route these to the VM until D-26.  Ignored
        // (no silent reply: the coordinator awaits no completion here yet).
        ContentToSw::SyncEvent { .. }
        | ContentToSw::PeriodicSyncEvent { .. }
        | ContentToSw::NotificationEvent { .. } => {}
        ContentToSw::Shutdown => return false,
    }
    true
}

/// Fire an `install` / `activate` `ExtendableEvent` and wait (bounded) for
/// its `waitUntil` lifetime promises.  Returns `false` iff any rejected or
/// the bound elapsed (SW §4.4.1).
fn run_lifecycle(vm: &mut Vm, event_type: &str, pump_timeout: Duration) -> bool {
    let event_id = event::create_extendable_event(&mut vm.inner, event_type);
    // Root the event object on the VM operand stack across dispatch + pump:
    // GC runs during `drain_microtasks`, and the side-store key does not root
    // the object.  The lifetime promises themselves are rooted by the GC mark
    // phase (`gc/collect.rs` marks `extendable_event_states.lifetime_promises`
    // while the entry lives) — which is why the loop must NOT `mem::take` them
    // out of the side-store: the clone leaves them in place for the marker.
    let mut scope = vm.inner.push_stack_scope();
    scope.stack.push(JsValue::Object(event_id));
    let _ = event::dispatch_event_at_sw_scope(&mut scope, event_id);
    let promises: Vec<ObjectId> = scope
        .extendable_event_states
        .get(&event_id)
        .map(|s| s.lifetime_promises.clone())
        .unwrap_or_default();
    let deadline = Instant::now() + pump_timeout;
    let success = pump_lifetime(&mut scope, &promises, deadline);
    scope.extendable_event_states.remove(&event_id);
    drop(scope);
    success
}

/// Fire a `FetchEvent` and, if the handler called `respondWith`, drain its
/// promise (DR-C) into a [`SwResponse`].  `None` → network passthrough (no
/// `respondWith`, a rejected promise, a non-`Response` value, or a timeout —
/// SW §4.6.7).
fn run_fetch(
    vm: &mut Vm,
    request: &elidex_api_sw::SwRequest,
    client_id: &str,
    resulting_client_id: &str,
    pump_timeout: Duration,
) -> Option<SwResponse> {
    let request_url = request.url.clone();
    let request_obj = marshal::build_request_from_sw_request(&mut vm.inner, request);
    // Root the request + the event object on the VM operand stack across
    // `create_fetch_event` + dispatch + the DR-C pump (GC runs during
    // `drain_microtasks` / `tick_network`; the event roots `request_obj` as an
    // own prop).  The `respondWith` promise is rooted by the GC mark phase
    // (`gc/collect.rs` marks `fetch_event_states.response_promise` while the
    // entry lives), and its fulfilled value via the promise's `result`.
    let mut scope = vm.inner.push_stack_scope();
    scope.stack.push(JsValue::Object(request_obj));
    let event_id =
        event::create_fetch_event(&mut scope, request_obj, client_id, resulting_client_id);
    scope.stack.push(JsValue::Object(event_id));
    let _ = event::dispatch_event_at_sw_scope(&mut scope, event_id);

    let (responded, response_promise) = scope
        .fetch_event_states
        .get(&event_id)
        .map_or((false, None), |s| (s.responded, s.response_promise));

    let result = match (responded, response_promise) {
        (true, Some(promise)) => {
            let deadline = Instant::now() + pump_timeout;
            match pump_response(&mut scope, promise, deadline) {
                Some(value) => marshal::response_to_sw_response(&scope, value, request_url).ok(),
                None => None,
            }
        }
        // No `respondWith` (incl. a handler that threw before calling it) →
        // passthrough.
        _ => None,
    };

    // A FetchEvent's `waitUntil(f)` (FetchEvent : ExtendableEvent) is best-
    // effort in PR-2: any synchronous-ish `f` reactions already ran during the
    // response pump above (`pump_once` drains microtasks), but extending the
    // SW's lifetime to hold it alive *past* the response is deferred — the
    // entry is torn down here.  Impl-discovered gap (the plan covered only
    // `ExtendableEvent.waitUntil` for install/activate); a landing-time defer
    // candidate (4-question audit → `#11-sw-fetchevent-waituntil` if eligible).
    scope.fetch_event_states.remove(&event_id);
    scope.extendable_event_states.remove(&event_id);
    drop(scope);
    result
}

// ---------------------------------------------------------------------------
// Promise pumps (DR-C / D-2)
// ---------------------------------------------------------------------------

/// Drain microtasks + timers + network until **all** `promises` are settled
/// (returns `true`), any rejects (`false`), or `deadline` passes (`false`).
/// An empty list (no `waitUntil`) settles immediately → `true`.
fn pump_lifetime(vm: &mut VmInner, promises: &[ObjectId], deadline: Instant) -> bool {
    loop {
        pump_once(vm);
        let mut all_settled = true;
        for &p in promises {
            match promise_status(vm, p) {
                PromiseStatus::Rejected => return false,
                PromiseStatus::Pending => all_settled = false,
                PromiseStatus::Fulfilled => {}
            }
        }
        if all_settled {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Drain microtasks + timers + network until `promise` settles, returning the
/// fulfilled value (`Some`) or `None` on rejection / timeout.
fn pump_response(vm: &mut VmInner, promise: ObjectId, deadline: Instant) -> Option<JsValue> {
    loop {
        pump_once(vm);
        match promise_status(vm, promise) {
            PromiseStatus::Fulfilled => return Some(promise_result(vm, promise)),
            PromiseStatus::Rejected => return None,
            PromiseStatus::Pending => {}
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// One VM advance: drain queued tasks (Cache API `CacheDeliver` settlements,
/// so `caches.*` promises resolve in the SW realm — C12) + microtask
/// checkpoint + timer drain + network tick.  The shared per-iteration step of
/// both pumps + the loop body.
fn pump_once(vm: &mut VmInner) {
    vm.drain_tasks();
    vm.drain_microtasks();
    vm.drain_timers(Instant::now());
    vm.tick_network();
}

/// `[[PromiseState]]` of `promise`; a non-promise reads as `Rejected` (a
/// `respondWith`-wrapped value is always a promise, so this is defensive).
fn promise_status(vm: &VmInner, promise: ObjectId) -> PromiseStatus {
    match &vm.get_object(promise).kind {
        ObjectKind::Promise(state) => state.status,
        _ => PromiseStatus::Rejected,
    }
}

/// `[[PromiseResult]]` of a settled `promise`.
fn promise_result(vm: &VmInner, promise: ObjectId) -> JsValue {
    match &vm.get_object(promise).kind {
        ObjectKind::Promise(state) => state.result,
        _ => JsValue::Undefined,
    }
}

/// Forward every queued `SwToContent` (skipWaiting / claim / Client.postMessage)
/// to the coordinator.
fn drain_sw_outgoing(vm: &mut Vm, channel: &SwChannel) {
    if vm.inner.sw_outgoing.is_empty() {
        return;
    }
    for msg in std::mem::take(&mut vm.inner.sw_outgoing) {
        let _ = channel.send(msg);
    }
}
