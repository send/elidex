//! Parent-side worker registry for tracking active dedicated workers.

use std::collections::HashMap;

use boa_engine::JsObject;
use elidex_api_workers::{WorkerHandle, WorkerToParent};

/// Registry of active dedicated workers owned by a parent document.
///
/// Lives in the parent's `HostBridgeInner`. The content thread drains
/// messages from all workers each event loop iteration.
pub(crate) struct WorkerRegistry {
    /// Active workers, keyed by worker ID.
    workers: HashMap<u64, WorkerEntry>,
    /// Monotonic ID counter for new workers.
    next_id: u64,
}

/// A tracked worker with its handle and JS-side references.
pub(crate) struct WorkerEntry {
    /// The parent-side handle (channel + thread).
    pub(crate) handle: WorkerHandle,
    /// The JS `Worker` object returned by `new Worker()`.
    pub(crate) js_object: JsObject,
    /// `onmessage` event handler attribute.
    pub(crate) onmessage: Option<JsObject>,
    /// `onerror` event handler attribute.
    pub(crate) onerror: Option<JsObject>,
    /// `onmessageerror` event handler attribute.
    pub(crate) onmessageerror: Option<JsObject>,
    /// Event listeners registered via `addEventListener`.
    pub(crate) event_listeners: HashMap<String, Vec<JsObject>>,
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self {
            workers: HashMap::new(),
            next_id: 1,
        }
    }
}

impl WorkerRegistry {
    /// Register a new worker and return its unique ID.
    pub(crate) fn create_worker(&mut self, handle: WorkerHandle, js_object: JsObject) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.workers.insert(
            id,
            WorkerEntry {
                handle,
                js_object,
                onmessage: None,
                onerror: None,
                onmessageerror: None,
                event_listeners: HashMap::new(),
            },
        );
        id
    }

    /// Terminate a specific worker by ID.
    pub(crate) fn terminate_worker(&mut self, id: u64) {
        if let Some(mut entry) = self.workers.remove(&id) {
            entry.handle.terminate();
        }
    }

    /// Remove a worker entry (e.g. after `WorkerToParent::Closed`).
    pub(crate) fn remove_worker(&mut self, id: u64) {
        self.workers.remove(&id);
    }

    /// Drain all pending messages from all workers.
    ///
    /// Returns `(worker_id, message)` pairs. Workers whose channels are
    /// disconnected (thread exited without sending `Closed`) are automatically
    /// removed from the registry.
    pub(crate) fn drain_all_messages(&mut self) -> Vec<(u64, WorkerToParent)> {
        if self.workers.is_empty() {
            return Vec::new();
        }
        let mut messages = Vec::new();
        let mut disconnected = Vec::new();
        for (&id, entry) in &mut self.workers {
            loop {
                match entry.handle.try_recv() {
                    Ok(msg) => messages.push((id, msg)),
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        disconnected.push(id);
                        break;
                    }
                }
            }
        }
        for id in disconnected {
            self.workers.remove(&id);
        }
        messages
    }

    /// Shut down all workers (called on page unload).
    pub(crate) fn shutdown_all(&mut self) {
        let ids: Vec<u64> = self.workers.keys().copied().collect();
        for id in ids {
            if let Some(mut entry) = self.workers.remove(&id) {
                entry.handle.terminate();
            }
        }
    }

    /// Get the JS object for a worker by ID.
    pub(crate) fn get_js_object(&self, id: u64) -> Option<&JsObject> {
        self.workers.get(&id).map(|e| &e.js_object)
    }

    /// Get a mutable reference to a worker entry by ID.
    pub(crate) fn get_entry_mut(&mut self, id: u64) -> Option<&mut WorkerEntry> {
        self.workers.get_mut(&id)
    }

    /// Set `onmessage` handler for a worker.
    pub(crate) fn set_onmessage(&mut self, id: u64, callback: Option<JsObject>) {
        if let Some(entry) = self.workers.get_mut(&id) {
            entry.onmessage = callback;
        }
    }

    /// Set `onerror` handler for a worker.
    pub(crate) fn set_onerror(&mut self, id: u64, callback: Option<JsObject>) {
        if let Some(entry) = self.workers.get_mut(&id) {
            entry.onerror = callback;
        }
    }

    /// Set `onmessageerror` handler for a worker.
    pub(crate) fn set_onmessageerror(&mut self, id: u64, callback: Option<JsObject>) {
        if let Some(entry) = self.workers.get_mut(&id) {
            entry.onmessageerror = callback;
        }
    }

    /// Get the IDL event handler attribute for a worker.
    pub(crate) fn get_event_handler(&self, id: u64, event_type: &str) -> Option<JsObject> {
        let entry = self.workers.get(&id)?;
        match event_type {
            "message" => entry.onmessage.clone(),
            "error" => entry.onerror.clone(),
            "messageerror" => entry.onmessageerror.clone(),
            _ => None,
        }
    }

    /// Add an event listener for a worker.
    pub(crate) fn add_event_listener(&mut self, id: u64, event_type: String, callback: JsObject) {
        if let Some(entry) = self.workers.get_mut(&id) {
            entry
                .event_listeners
                .entry(event_type)
                .or_default()
                .push(callback);
        }
    }

    /// Remove an event listener for a worker by reference identity.
    pub(crate) fn remove_event_listener(&mut self, id: u64, event_type: &str, callback: &JsObject) {
        if let Some(entry) = self.workers.get_mut(&id) {
            if let Some(listeners) = entry.event_listeners.get_mut(event_type) {
                listeners.retain(|cb| !JsObject::equals(cb, callback));
            }
        }
    }

    /// Check if the registry has any workers.
    pub(crate) fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    /// Get all callbacks (IDL handler + addEventListener listeners) for a worker.
    pub(crate) fn get_callbacks(&self, id: u64, event_type: &str) -> Vec<JsObject> {
        let Some(entry) = self.workers.get(&id) else {
            return Vec::new();
        };
        let mut cbs = Vec::new();
        let handler = match event_type {
            "message" => &entry.onmessage,
            "error" => &entry.onerror,
            "messageerror" => &entry.onmessageerror,
            _ => &None,
        };
        if let Some(h) = handler {
            cbs.push(h.clone());
        }
        if let Some(ls) = entry.event_listeners.get(event_type) {
            cbs.extend(ls.iter().cloned());
        }
        cbs
    }
}
