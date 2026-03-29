//! Worker-side bridge state for dedicated workers.
//!
//! When a `HostBridge` belongs to a worker thread, this state holds the
//! outgoing message queue, close flag, worker name, script URL, and
//! event listeners for the `WorkerGlobalScope`.

use std::collections::HashMap;

use boa_engine::JsObject;

/// State specific to a worker thread's `HostBridge`.
///
/// Present only when the bridge belongs to a dedicated worker
/// (`HostBridgeInner.worker_state` is `Some`).
pub(crate) struct WorkerBridgeState {
    /// Outgoing `postMessage` data queued by `self.postMessage()`.
    /// Drained by the worker event loop after each tick.
    pub(crate) outgoing_messages: Vec<OutgoingMessage>,
    /// Set to `true` when the worker calls `close()`.
    pub(crate) close_requested: bool,
    /// Worker name (from `new Worker(url, { name })` option).
    pub(crate) name: String,
    /// Worker script URL (for `WorkerLocation` and error reporting).
    pub(crate) script_url: url::Url,
    /// Event listeners for the worker global scope, keyed by event type.
    pub(crate) event_listeners: HashMap<String, Vec<JsObject>>,
    /// IDL event handler attributes (`onmessage`, `onerror`, `onmessageerror`).
    /// These replace (not append) on each set, per WHATWG HTML §8.1.3.
    pub(crate) event_handlers: HashMap<String, JsObject>,
}

/// A message queued by `self.postMessage(data)` inside a worker.
pub(crate) enum OutgoingMessage {
    /// Successfully JSON-stringified data.
    Data(String),
    /// JSON.stringify failed (circular reference, etc.) — fire `messageerror` on parent.
    SerializationError,
}

impl WorkerBridgeState {
    /// Create a new worker bridge state.
    pub(crate) fn new(name: String, script_url: url::Url) -> Self {
        Self {
            outgoing_messages: Vec::new(),
            close_requested: false,
            name,
            script_url,
            event_listeners: HashMap::new(),
            event_handlers: HashMap::new(),
        }
    }

    /// Get all callbacks (handler + listeners) for a given event type.
    pub(crate) fn get_callbacks(&self, event_type: &str) -> Vec<JsObject> {
        let mut result = Vec::new();
        // IDL event handler first.
        if let Some(handler) = self.event_handlers.get(event_type) {
            result.push(handler.clone());
        }
        // addEventListener listeners next.
        if let Some(listeners) = self.event_listeners.get(event_type) {
            result.extend(listeners.iter().cloned());
        }
        result
    }
}
