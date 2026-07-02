//! IPC message types for parent ↔ worker communication.

/// Messages sent from the parent thread to a dedicated worker thread.
///
/// Neither direction carries an origin: `Worker.postMessage` (WHATWG HTML
/// §10.2.6.3) and `DedicatedWorkerGlobalScope.postMessage` (§10.2.1.2) act
/// as if they invoked `postMessage` on the entangled port, and the
/// *message port post message steps* (§9.4.4,
/// `#message-port-post-message-steps`) step 7.7 fire the `message` event
/// initializing only `data` + `ports` — `MessageEvent.origin` stays the
/// `MessageEventInit` default `""` on both endpoints.
#[derive(Debug)]
pub enum ParentToWorker {
    /// Deliver a message (JSON-serialized) to the worker's `onmessage`.
    PostMessage {
        /// JSON-serialized message data.
        data: String,
    },
    /// Terminate the worker thread (from `worker.terminate()`).
    Shutdown,
}

/// Messages sent from a dedicated worker thread back to the parent.
///
/// Carries no origin — see [`ParentToWorker`] (WHATWG HTML §9.4.4 *message
/// port post message steps* step 7.7 / §10.2.1.2 port delegation).
#[derive(Debug)]
pub enum WorkerToParent {
    /// Worker called `postMessage(data)`.
    PostMessage {
        /// JSON-serialized message data.
        data: String,
    },
    /// An uncaught error occurred in the worker.
    Error {
        /// Error message string.
        message: String,
        /// Script URL where the error occurred.
        filename: String,
        /// Line number (0 if unavailable from boa).
        lineno: u32,
        /// Column number (0 if unavailable from boa).
        colno: u32,
        /// String representation of the error value.
        error_value: String,
    },
    /// Worker called `close()` — the worker thread will exit after sending this.
    Closed,
    /// JSON.stringify failed on postMessage data (circular reference, etc.).
    /// The parent should fire a `messageerror` event on the Worker object.
    MessageError,
}
