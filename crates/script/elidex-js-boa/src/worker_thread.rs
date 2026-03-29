//! Worker thread event loop.
//!
//! Runs a `JsRuntime::for_worker()` in a dedicated thread, communicating with
//! the parent via `crossbeam_channel`. Follows the same pattern as iframe OOP
//! threads in `elidex-shell`.

use std::rc::Rc;
use std::time::Duration;

use crossbeam_channel::RecvTimeoutError;
use elidex_api_workers::{ParentToWorker, WorkerToParent};
use elidex_ecs::EcsDom;
use elidex_net::FetchHandle;
use elidex_plugin::LocalChannel;
use elidex_script_session::SessionCore;

use crate::JsRuntime;

/// 16ms frame interval for timer drain (matches content thread).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Send a `WorkerToParent::Error` message over the channel.
fn send_worker_error(
    channel: &LocalChannel<WorkerToParent, ParentToWorker>,
    message: String,
    filename: &str,
) {
    let _ = channel.send(WorkerToParent::Error {
        message: message.clone(),
        filename: filename.to_string(),
        lineno: 0,
        colno: 0,
        error_value: message,
    });
}

/// Entry point for the worker thread (WHATWG HTML §10.1.3).
///
/// - Fetches the worker script from `script_url` asynchronously.
/// - On fetch failure, sends `WorkerToParent::Error` and exits.
/// - On success, evaluates the script in a fresh `JsRuntime::for_worker()`.
/// - Runs an event loop receiving messages from the parent and draining timers.
/// - Sends outgoing messages back to the parent.
/// - Exits on `Shutdown`, channel disconnect, or `close()`.
#[allow(clippy::needless_pass_by_value)]
pub fn worker_thread_main(
    script_url: url::Url,
    name: String,
    channel: LocalChannel<WorkerToParent, ParentToWorker>,
) {
    // 1. Fetch the worker script.
    let fetch_handle = FetchHandle::with_default_client();
    let request = elidex_net::Request {
        method: "GET".to_string(),
        url: script_url.clone(),
        headers: Vec::new(),
        body: bytes::Bytes::new(),
    };

    let response = match fetch_handle.send_blocking(request) {
        Ok(resp) => resp,
        Err(e) => {
            send_worker_error(
                &channel,
                format!("Failed to fetch worker script: {e}"),
                script_url.as_ref(),
            );
            return;
        }
    };

    // 2. Validate MIME type and HTTP status (WHATWG HTML §10.1.3).
    let script_source = match crate::globals::worker_constructor::validate_worker_script_response(
        &response,
        &script_url,
    ) {
        Ok(source) => source,
        Err(msg) => {
            send_worker_error(&channel, msg, script_url.as_ref());
            return;
        }
    };

    // 5. Run the worker with the fetched script.
    worker_thread_main_with_source(script_source, script_url, name, channel);
}

/// Entry point for the worker thread with pre-fetched script source.
///
/// Used by tests and potentially by blob: URLs where the script content is
/// already available without a network fetch.
#[allow(clippy::needless_pass_by_value)]
pub fn worker_thread_main_with_source(
    script_source: String,
    script_url: url::Url,
    name: String,
    channel: LocalChannel<WorkerToParent, ParentToWorker>,
) {
    // 1. Create independent FetchHandle for this worker.
    let fetch_handle = Rc::new(FetchHandle::with_default_client());

    // 2. Create worker JsRuntime.
    let mut runtime = JsRuntime::for_worker(Some(fetch_handle), name, script_url.clone());

    // 3. Create empty EcsDom + SessionCore (required for bridge bind).
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut session = SessionCore::new();

    // 4. Evaluate the worker script.
    let eval_result = runtime.eval(&script_source, &mut session, &mut dom, doc);
    if !eval_result.success {
        if let Some(ref err) = eval_result.error {
            send_worker_error(&channel, err.clone(), script_url.as_ref());
        }
    }

    // 5. Event loop.
    loop {
        // Check close flag (set by worker's close() call).
        if runtime.bridge().worker_close_requested() {
            runtime.bridge().clear_all_timers();
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }

        // Receive messages from parent with timeout.
        match channel.recv_timeout(FRAME_INTERVAL) {
            Ok(msg) => match msg {
                ParentToWorker::PostMessage { data, origin } => {
                    runtime.dispatch_worker_message(&mut session, &mut dom, doc, &data, &origin);
                }
                ParentToWorker::Shutdown => {
                    runtime.bridge().clear_all_timers();
                    break;
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                // Normal — drain timers and continue.
            }
            Err(RecvTimeoutError::Disconnected) => {
                // Parent dropped the channel — exit.
                break;
            }
        }

        // Drain timers.
        runtime.drain_timers(&mut session, &mut dom, doc);

        // Drain any messages queued during timer/event callbacks.
        drain_outgoing(&runtime, &channel);

        // Re-check close flag after processing.
        if runtime.bridge().worker_close_requested() {
            runtime.bridge().clear_all_timers();
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }
    }
}

/// Drain outgoing messages from the worker runtime and send them to the parent.
fn drain_outgoing(runtime: &JsRuntime, channel: &LocalChannel<WorkerToParent, ParentToWorker>) {
    for msg in runtime.drain_worker_outgoing() {
        let _ = channel.send(msg);
    }
}
