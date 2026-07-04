//! Dedicated worker thread entry + per-worker event loop (WHATWG HTML §10.2.4
//! "run a worker" + §10.2.2 worker event loop).
//!
//! Runs **on the worker thread** (invoked as the `elidex_api_workers::spawn_worker`
//! body). [`run_worker`] is the full entry: it fetches / decodes the worker
//! script ([`obtain_worker_source`] — `data:` inline-decode, else fetch +
//! validate via the supplied `network_handle`, WHATWG HTML §10.2.4 "fetch a
//! classic worker script"), then [`run_worker_with_source`] builds a fresh
//! worker-mode [`Vm`] bound to an empty [`EcsDom`], evaluates the script, and
//! drives the worker's own event loop: inbound `postMessage` → MessageEvent
//! dispatch, microtask + timer + network drain each tick, outgoing-message +
//! close/shutdown handling. ([`run_worker_with_source`] is also the unit-test
//! seam, exercised directly with already-fetched source.)
//!
//! The loop deliberately reuses `drain_microtasks` / `drain_timers` /
//! `tick_network` (all window/document-independent) and **not** the Window
//! `pending_tasks` queue.
//!
//! The `network_handle` is supplied by the caller (the main-side `Worker`
//! constructor, `host/worker.rs`): it mints a sibling on the **main** thread via
//! `NetworkHandle::create_sibling_handle()` and moves it here. `NetworkHandle`
//! is `Send` (its `RefCell` / `Arc` / crossbeam fields are all `Send`; it is
//! only `!Sync`), so the by-value move across the spawn boundary is sound — the
//! worker wraps it in a thread-local `Rc` here.

#![cfg(feature = "engine")]

use std::rc::Rc;
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;
use elidex_api_workers::{ParentToWorker, WorkerToParent};
use elidex_ecs::EcsDom;
use elidex_net::broker::NetworkHandle;
use elidex_plugin::LocalChannel;
use elidex_script_session::SessionCore;

use super::host_data::HostData;
use super::Vm;

/// 16 ms frame interval for the recv-timeout / timer-drain cadence (matches the
/// content thread).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

type WorkerChannel = LocalChannel<WorkerToParent, ParentToWorker>;

/// Worker thread entry (WHATWG HTML §10.2.4 "run a worker", from the "fetch a
/// classic worker script" step). Runs **on the worker thread** as the
/// `spawn_worker` closure body: obtains the script source, then hands off to
/// [`run_worker_with_source`].
///
/// `data:` URLs decode inline (no network); every other scheme fetches through
/// the `Send` sibling `NetworkHandle` minted on the main thread by the `Worker`
/// constructor (`NetworkHandle::create_sibling_handle()`, WHATWG HTML §10.2.6.3)
/// and validates the response per §10.2.4 ("fetch a classic worker script").
/// A missing handle for a non-`data:` URL, a fetch failure, or a validation
/// failure reports an `error` to the parent and ends the worker.
///
/// This is the production consumer of [`run_worker_with_source`] +
/// `elidex_api_workers` (it is what the main-side ctor passes to `spawn_worker`).
pub(crate) fn run_worker(
    script_url: &url::Url,
    name: String,
    is_secure_context: bool,
    credentials: elidex_net::CredentialsMode,
    network_handle: Option<NetworkHandle>,
    engine_mode: elidex_plugin::EngineMode,
    channel: &WorkerChannel,
) {
    let source = match obtain_worker_source(script_url, credentials, network_handle.as_ref()) {
        Ok(source) => source,
        Err(message) => {
            send_error(channel, message, script_url.as_str());
            let _ = channel.send(WorkerToParent::Closed);
            return;
        }
    };
    run_worker_with_source(
        &source,
        script_url,
        name,
        is_secure_context,
        credentials,
        network_handle,
        engine_mode,
        channel,
    );
}

/// Fetch (or inline-decode) the worker script source (WHATWG HTML §10.2.4
/// "fetch a classic worker script"). Returns the decoded source text or a
/// human-readable error message for the parent's `error` event.
///
/// The fetch carries the worker's own origin (the script URL's origin — the
/// top-level script is same-origin), `RequestMode::SameOrigin`, and the
/// `WorkerOptions.credentials` mode, so the broker gates cookie attachment
/// correctly (a bare `..Default::default()` would leave `origin = None` and
/// attach cookies unconditionally).
fn obtain_worker_source(
    script_url: &url::Url,
    credentials: elidex_net::CredentialsMode,
    network_handle: Option<&NetworkHandle>,
) -> Result<String, String> {
    if script_url.scheme() == "data" {
        let data = elidex_net::data_url::parse_data_url(script_url)
            .map_err(|e| format!("NetworkError: invalid worker data: URL: {e}"))?;
        // A `data:` worker script must still carry a JavaScript MIME essence
        // (WHATWG HTML §10.2.4) — `data:text/html,...` must fail validation, not
        // be evaluated as script. Reuse the same essence check as the network
        // path (status is synthesised as 200 for the inline body).
        return elidex_api_workers::validate_worker_script_response(
            Some(&data.media_type),
            200,
            &data.body,
            script_url,
        )
        .map_err(|e| format!("NetworkError: {e}"));
    }

    let handle = network_handle.ok_or_else(|| {
        "NetworkError: no network handle available to fetch the worker script".to_string()
    })?;
    let request = elidex_net::Request {
        method: "GET".to_string(),
        url: script_url.clone(),
        origin: Some(elidex_plugin::SecurityOrigin::from_url(script_url)),
        credentials,
        mode: elidex_net::RequestMode::SameOrigin,
        ..Default::default()
    };
    let response = handle
        .fetch_blocking(request)
        .map_err(|e| format!("NetworkError: failed to fetch worker script: {e}"))?;
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

/// Build + run a worker VM from already-fetched `source` (WHATWG HTML §10.2.4
/// from the "run the worker" step onward). Constructs a worker-mode VM bound to
/// an empty `EcsDom`, evaluates the script, then drives the per-worker event
/// loop until `close()` / `terminate()` / channel disconnect.
///
/// `network_handle = None` leaves `fetch` / `importScripts` unavailable (fine
/// for messaging-only scenarios + isolation tests); the `Worker` constructor
/// passes a `Send` sibling minted on the main thread
/// (`NetworkHandle::create_sibling_handle()`), which is wrapped in a
/// thread-local `Rc` here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_worker_with_source(
    source: &str,
    script_url: &url::Url,
    name: String,
    is_secure_context: bool,
    credentials: elidex_net::CredentialsMode,
    network_handle: Option<NetworkHandle>,
    engine_mode: elidex_plugin::EngineMode,
    channel: &WorkerChannel,
) {
    // Declared before `vm` so they outlive it: `vm`'s `HostData` holds raw
    // pointers into `session` / `dom` and must drop first (reverse declaration
    // order). The VM is never `unbind`-ed — `HostData::drop` does not deref the
    // pointers, so dropping in this order is sound.
    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    let mut session = SessionCore::new();

    let mut vm = Vm::new_worker(
        name,
        script_url.clone(),
        is_secure_context,
        credentials,
        engine_mode,
    );
    vm.install_host_data(HostData::new());
    if let Some(handle) = network_handle {
        vm.install_network_handle(Rc::new(handle));
    }
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
        let message = uncaught_error_message(&mut vm, &e);
        send_error(channel, message, script_url.as_str());
        let _ = channel.send(WorkerToParent::Closed);
        return;
    }
    drain_outgoing(&mut vm, channel);

    loop {
        if vm.inner.worker_close_requested {
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }

        match channel.recv_timeout(FRAME_INTERVAL) {
            // origin = "" per the message-port post-message steps — see
            // `elidex_api_workers::ParentToWorker`.
            Ok(ParentToWorker::PostMessage { data }) => {
                vm.inner.dispatch_worker_message(&data, "");
            }
            // `terminate()` (Shutdown) and a dropped parent channel both end
            // the worker (WHATWG HTML §10.2.4 "terminate a worker").
            Ok(ParentToWorker::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        vm.inner.drain_microtasks();
        vm.inner.drain_timers(Instant::now());
        vm.tick_network();
        drain_outgoing(&mut vm, channel);

        if vm.inner.worker_close_requested {
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }
    }
}

/// Forward any `self.postMessage()` data queued during the last tick to the
/// parent. No origin is stamped — origin = `""` per the message-port
/// post-message steps, see [`elidex_api_workers::WorkerToParent`].
fn drain_outgoing(vm: &mut Vm, channel: &WorkerChannel) {
    if vm.inner.worker_outgoing.is_empty() {
        return;
    }
    for data in std::mem::take(&mut vm.inner.worker_outgoing) {
        let _ = channel.send(WorkerToParent::PostMessage { data });
    }
}

/// Resolve a human-readable message for an uncaught worker-script error
/// (WHATWG HTML §10.2.5 — the `message` of the `error` event). For a thrown
/// `Error` instance, read its `message` property (the VM is still live after
/// the failed `eval`); for everything else fall back to the [`VmError`]'s own
/// diagnostic message.
fn uncaught_error_message(vm: &mut Vm, error: &super::value::VmError) -> String {
    use super::value::{JsValue, PropertyKey, VmErrorKind};
    if let VmErrorKind::ThrowValue(JsValue::Object(id)) = error.kind {
        let key = PropertyKey::String(vm.inner.strings.intern("message"));
        if let Ok(JsValue::String(sid)) = vm.inner.get_property_value(id, key) {
            let message = vm.inner.strings.get_utf8(sid);
            if !message.is_empty() {
                return message;
            }
        }
    }
    error.message.clone()
}

/// Report an uncaught worker error to the parent (WHATWG HTML §10.2.5).
fn send_error(channel: &WorkerChannel, message: String, filename: &str) {
    let _ = channel.send(WorkerToParent::Error {
        message: message.clone(),
        filename: filename.to_string(),
        lineno: 0,
        colno: 0,
        error_value: message,
    });
}
