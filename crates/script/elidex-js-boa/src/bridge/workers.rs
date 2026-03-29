//! Worker-related methods for `HostBridge`.
//!
//! Parent-side methods manage the `WorkerRegistry` (register, terminate, drain,
//! shutdown). Worker-side methods manage the `WorkerBridgeState` (outgoing
//! messages, close flag, event listeners).

use boa_engine::JsObject;

use super::worker_state::OutgoingMessage;
use super::{worker_registry, worker_state, HostBridge};

impl HostBridge {
    // --- Workers (parent side) ---

    /// Register a new worker in the parent registry.
    pub fn register_worker(
        &self,
        handle: elidex_api_workers::WorkerHandle,
        js_object: JsObject,
    ) -> u64 {
        self.inner
            .borrow_mut()
            .worker_registry
            .create_worker(handle, js_object)
    }

    /// Terminate a specific worker by ID.
    pub fn terminate_worker(&self, id: u64) {
        self.inner.borrow_mut().worker_registry.terminate_worker(id);
    }

    /// Remove a worker entry (after it has closed).
    pub fn remove_worker(&self, id: u64) {
        self.inner.borrow_mut().worker_registry.remove_worker(id);
    }

    /// Drain all pending messages from all workers.
    pub fn drain_worker_messages(&self) -> Vec<(u64, elidex_api_workers::WorkerToParent)> {
        self.inner.borrow_mut().worker_registry.drain_all_messages()
    }

    /// Shut down all workers (page unload).
    pub fn shutdown_all_workers(&self) {
        self.inner.borrow_mut().worker_registry.shutdown_all();
    }

    /// Get the JS object for a worker by ID.
    pub fn get_worker_js_object(&self, id: u64) -> Option<JsObject> {
        self.inner
            .borrow()
            .worker_registry
            .get_js_object(id)
            .cloned()
    }

    /// Access the worker registry mutably via closure.
    pub(crate) fn with_worker_registry<R>(
        &self,
        f: impl FnOnce(&mut worker_registry::WorkerRegistry) -> R,
    ) -> R {
        f(&mut self.inner.borrow_mut().worker_registry)
    }

    /// Whether this bridge has any active workers.
    pub fn has_workers(&self) -> bool {
        !self.inner.borrow().worker_registry.is_empty()
    }

    /// Get all callbacks (IDL handler + addEventListener listeners) for a parent-side worker.
    pub(crate) fn get_parent_worker_callbacks(
        &self,
        worker_id: u64,
        event_type: &str,
    ) -> Vec<boa_engine::JsObject> {
        self.inner
            .borrow()
            .worker_registry
            .get_callbacks(worker_id, event_type)
    }

    // --- Workers (worker side) ---

    /// Initialize this bridge as a worker bridge.
    pub fn init_worker_state(&self, name: String, script_url: url::Url) {
        self.inner.borrow_mut().worker_state =
            Some(worker_state::WorkerBridgeState::new(name, script_url));
    }

    /// Whether this bridge belongs to a worker thread.
    #[must_use]
    pub fn is_worker(&self) -> bool {
        self.inner.borrow().worker_state.is_some()
    }

    /// Get the worker name (panics if not a worker bridge).
    #[must_use]
    pub fn worker_name(&self) -> String {
        self.inner
            .borrow()
            .worker_state
            .as_ref()
            .expect("worker_name() on non-worker bridge")
            .name
            .clone()
    }

    /// Get the worker script URL (panics if not a worker bridge).
    #[must_use]
    pub fn worker_script_url(&self) -> url::Url {
        self.inner
            .borrow()
            .worker_state
            .as_ref()
            .expect("worker_script_url() on non-worker bridge")
            .script_url
            .clone()
    }

    /// Queue an outgoing postMessage from the worker.
    pub(crate) fn worker_queue_message(&self, msg: OutgoingMessage) {
        if let Some(ref mut ws) = self.inner.borrow_mut().worker_state {
            ws.outgoing_messages.push(msg);
        }
    }

    /// Set the close flag on the worker bridge.
    pub fn worker_request_close(&self) {
        if let Some(ref mut ws) = self.inner.borrow_mut().worker_state {
            ws.close_requested = true;
        }
    }

    /// Check if the worker has requested close.
    #[must_use]
    pub fn worker_close_requested(&self) -> bool {
        self.inner
            .borrow()
            .worker_state
            .as_ref()
            .is_some_and(|ws| ws.close_requested)
    }

    /// Drain outgoing messages from the worker.
    pub(crate) fn worker_drain_messages(&self) -> Vec<OutgoingMessage> {
        self.inner
            .borrow_mut()
            .worker_state
            .as_mut()
            .map_or_else(Vec::new, |ws| std::mem::take(&mut ws.outgoing_messages))
    }

    /// Add an event listener to the worker global scope.
    pub(crate) fn worker_add_event_listener(&self, event_type: String, callback: JsObject) {
        if let Some(ref mut ws) = self.inner.borrow_mut().worker_state {
            ws.event_listeners
                .entry(event_type)
                .or_default()
                .push(callback);
        }
    }

    /// Remove an event listener from the worker global scope.
    pub(crate) fn worker_remove_event_listener(&self, event_type: &str, callback: &JsObject) {
        if let Some(ref mut ws) = self.inner.borrow_mut().worker_state {
            if let Some(listeners) = ws.event_listeners.get_mut(event_type) {
                listeners.retain(|cb| !JsObject::equals(cb, callback));
            }
        }
    }

    /// Set an IDL event handler attribute (e.g., `onmessage`).
    pub(crate) fn worker_set_event_handler(&self, event_type: String, callback: Option<JsObject>) {
        if let Some(ref mut ws) = self.inner.borrow_mut().worker_state {
            match callback {
                Some(cb) => {
                    ws.event_handlers.insert(event_type, cb);
                }
                None => {
                    ws.event_handlers.remove(&event_type);
                }
            }
        }
    }

    /// Get an IDL event handler attribute.
    pub(crate) fn worker_get_event_handler(&self, event_type: &str) -> Option<JsObject> {
        self.inner
            .borrow()
            .worker_state
            .as_ref()
            .and_then(|ws| ws.event_handlers.get(event_type).cloned())
    }

    /// Get all callbacks (handler + listeners) for a given event type.
    pub(crate) fn worker_get_callbacks(&self, event_type: &str) -> Vec<JsObject> {
        self.inner
            .borrow()
            .worker_state
            .as_ref()
            .map_or_else(Vec::new, |ws| ws.get_callbacks(event_type))
    }
}
