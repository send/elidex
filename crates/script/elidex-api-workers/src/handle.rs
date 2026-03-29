//! Worker handle — parent-side reference to a running worker thread.

use std::thread::JoinHandle;

use elidex_plugin::LocalChannel;

use crate::types::{ParentToWorker, WorkerToParent};

/// Parent-side handle to a dedicated worker thread.
///
/// Owns the channel endpoint and the thread join handle. Dropping the handle
/// without calling [`terminate()`](Self::terminate) will send `Shutdown` and
/// detach the thread.
pub struct WorkerHandle {
    /// Channel to communicate with the worker thread.
    pub(crate) channel: LocalChannel<ParentToWorker, WorkerToParent>,
    /// Thread join handle (consumed on terminate/drop).
    pub(crate) thread: Option<JoinHandle<()>>,
    /// Worker name (from `new Worker(url, { name })` option).
    pub(crate) name: String,
    /// Worker script URL (for error reporting and `WorkerLocation`).
    pub(crate) script_url: url::Url,
}

impl WorkerHandle {
    /// Create a new worker handle.
    pub fn new(
        name: String,
        script_url: url::Url,
        channel: LocalChannel<ParentToWorker, WorkerToParent>,
        thread: JoinHandle<()>,
    ) -> Self {
        Self {
            channel,
            thread: Some(thread),
            name,
            script_url,
        }
    }

    /// The worker's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The worker's script URL.
    #[must_use]
    pub fn script_url(&self) -> &url::Url {
        &self.script_url
    }

    /// Send a `postMessage` to the worker.
    pub fn post_message(&self, data: String, origin: String) {
        let _ = self
            .channel
            .send(ParentToWorker::PostMessage { data, origin });
    }

    /// Try to receive a message from the worker without blocking.
    pub fn try_recv(&self) -> Option<WorkerToParent> {
        self.channel.try_recv().ok()
    }

    /// Terminate the worker thread.
    ///
    /// Sends `Shutdown`, then joins the thread with a 1-second timeout.
    /// If the thread doesn't finish in time, it is detached.
    pub fn terminate(&mut self) {
        let _ = self.channel.send(ParentToWorker::Shutdown);
        if let Some(handle) = self.thread.take() {
            // Park the current thread for up to 1s waiting for the worker.
            // std::thread::JoinHandle has no timeout API, so we use a
            // crossbeam channel as a signaling mechanism.
            let (done_tx, done_rx) = crossbeam_channel::bounded(1);
            std::thread::spawn(move || {
                let _ = handle.join();
                let _ = done_tx.send(());
            });
            let _ = done_rx.recv_timeout(std::time::Duration::from_secs(1));
            // If the thread didn't finish, it's now detached (the spawned
            // helper thread will join it eventually).
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.terminate();
        }
    }
}
