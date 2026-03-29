//! IPC message types for parent ↔ worker communication.

/// Messages sent from the parent thread to a dedicated worker thread.
#[derive(Debug)]
pub enum ParentToWorker {
    /// Deliver a message (JSON-serialized) to the worker's `onmessage`.
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Origin of the sending context (for `MessageEvent.origin`).
        origin: String,
    },
    /// Terminate the worker thread (from `worker.terminate()`).
    Shutdown,
}

/// Messages sent from a dedicated worker thread back to the parent.
#[derive(Debug)]
pub enum WorkerToParent {
    /// Worker called `postMessage(data)`.
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Origin of the worker's global scope (worker script URL's origin).
        origin: String,
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
