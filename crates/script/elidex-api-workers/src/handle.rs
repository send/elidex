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
    ///
    /// Returns `Ok(msg)` if a message is available, `Err(Empty)` if the channel
    /// is empty, or `Err(Disconnected)` if the worker thread has exited.
    pub fn try_recv(&self) -> Result<WorkerToParent, crossbeam_channel::TryRecvError> {
        self.channel.try_recv()
    }

    /// Terminate the worker thread.
    ///
    /// Sends `Shutdown` and drops the `JoinHandle` to detach the thread.
    /// The worker will exit when it processes `Shutdown` or detects channel
    /// disconnect.
    pub fn terminate(&mut self) {
        let _ = self.channel.send(ParentToWorker::Shutdown);
        // Drop the JoinHandle to detach the worker thread.
        self.thread.take();
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.terminate();
        }
    }
}
