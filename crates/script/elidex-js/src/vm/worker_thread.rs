//! Dedicated worker thread entry + per-worker event loop (WHATWG HTML §10.2.4
//! "run a worker" + §10.2.2 worker event loop).
//!
//! Runs **on the worker thread** (invoked as the `elidex_api_workers::spawn_worker`
//! body). Builds a fresh worker-mode [`Vm`] bound to an empty [`EcsDom`],
//! evaluates the worker script, and drives the worker's own event loop: inbound
//! `postMessage` → MessageEvent dispatch, microtask + timer + network drain
//! each tick, outgoing-message + close/shutdown handling.
//!
//! The loop deliberately reuses `drain_microtasks` / `drain_timers` /
//! `tick_network` (all window/document-independent) and **not** the Window
//! `pending_tasks` queue.
//!
//! The `network_handle` is supplied by the caller (the main-side `Worker`
//! constructor, PR-B): it mints a sibling on the **main** thread via
//! `NetworkHandle::create_sibling_handle()` and moves it here. `NetworkHandle`
//! is `Send` (its `RefCell` / `Arc` / crossbeam fields are all `Send`; it is
//! only `!Sync`), so the by-value move across the spawn boundary is sound — the
//! worker wraps it in a thread-local `Rc` here. The worker-script *fetch +
//! validate* step that precedes this entry also lives with that constructor;
//! this module owns the post-fetch runtime harness, the seam tested in
//! isolation.

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
pub(crate) fn run_worker_with_source(
    source: &str,
    script_url: &url::Url,
    name: String,
    network_handle: Option<NetworkHandle>,
    channel: &WorkerChannel,
) {
    // Declared before `vm` so they outlive it: `vm`'s `HostData` holds raw
    // pointers into `session` / `dom` and must drop first (reverse declaration
    // order). The VM is never `unbind`-ed — `HostData::drop` does not deref the
    // pointers, so dropping in this order is sound.
    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    let mut session = SessionCore::new();

    let mut vm = Vm::new_worker(name, script_url.clone());
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
        send_error(channel, e.message, script_url.as_str());
        let _ = channel.send(WorkerToParent::Closed);
        return;
    }
    drain_outgoing(&mut vm, channel, script_url);

    loop {
        if vm.inner.worker_close_requested {
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }

        match channel.recv_timeout(FRAME_INTERVAL) {
            Ok(ParentToWorker::PostMessage { data, origin }) => {
                vm.inner.dispatch_worker_message(&data, &origin);
            }
            // `terminate()` (Shutdown) and a dropped parent channel both end
            // the worker (WHATWG HTML §10.2.4 "terminate a worker").
            Ok(ParentToWorker::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        vm.inner.drain_microtasks();
        vm.inner.drain_timers(Instant::now());
        vm.tick_network();
        drain_outgoing(&mut vm, channel, script_url);

        if vm.inner.worker_close_requested {
            let _ = channel.send(WorkerToParent::Closed);
            break;
        }
    }
}

/// Forward any `self.postMessage()` data queued during the last tick to the
/// parent, stamping each with the worker scope's origin (the script URL's
/// origin per WHATWG HTML §10.2.1.2).
fn drain_outgoing(vm: &mut Vm, channel: &WorkerChannel, script_url: &url::Url) {
    if vm.inner.worker_outgoing.is_empty() {
        return;
    }
    let origin = script_url.origin().ascii_serialization();
    for data in std::mem::take(&mut vm.inner.worker_outgoing) {
        let _ = channel.send(WorkerToParent::PostMessage {
            data,
            origin: origin.clone(),
        });
    }
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
