//! Event listener registration and lookup.
//!
//! `EventListeners` is an ECS component attached to entities that have
//! registered JS event listeners. It stores metadata only — the actual
//! JS function objects live in the JS engine layer (e.g. `HostBridge`).

use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for unique `ListenerId` values.
static NEXT_LISTENER_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identifier for a registered event listener.
///
/// Globally unique — each call to `EventListeners::add()` allocates a
/// new monotonic ID from a process-wide atomic counter.
///
/// Used as the key to look up the JS function object in the engine layer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ListenerId(u64);

impl ListenerId {
    /// Extract the raw `u64` value.
    #[must_use]
    pub fn to_raw(self) -> u64 {
        self.0
    }

    /// Create a `ListenerId` from a raw `u64` value.
    #[must_use]
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Metadata for a single registered event listener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenerEntry {
    /// Unique identifier for this listener.
    pub id: ListenerId,
    /// The event type (e.g. `"click"`, `"keydown"`).
    pub event_type: String,
    /// Whether this listener was registered for the capture phase.
    pub capture: bool,
    /// WHATWG DOM §2.6: if `true`, the listener is automatically removed
    /// after its first invocation (removed **before** the callback runs,
    /// per §2.10 step 15).
    pub once: bool,
    /// WHATWG DOM §2.6: if `true`, `preventDefault()` inside this listener
    /// is a silent no-op (the canceled flag is not set).
    pub passive: bool,
}

/// ECS component holding all event listeners for a single entity.
///
/// Listeners are stored in registration order. IDs are allocated from
/// a global atomic counter, ensuring uniqueness across all entities.
#[derive(Clone, Debug, Default)]
pub struct EventListeners {
    entries: Vec<ListenerEntry>,
}

impl EventListeners {
    /// Create a new, empty listener set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new listener and return its globally unique ID.
    pub fn add(&mut self, event_type: impl Into<String>, capture: bool) -> ListenerId {
        self.add_with_options(event_type, capture, false, false)
    }

    /// Register a new listener with `once`/`passive` options.
    pub fn add_with_options(
        &mut self,
        event_type: impl Into<String>,
        capture: bool,
        once: bool,
        passive: bool,
    ) -> ListenerId {
        let id = ListenerId(NEXT_LISTENER_ID.fetch_add(1, Ordering::Relaxed));
        self.entries.push(ListenerEntry {
            id,
            event_type: event_type.into(),
            capture,
            once,
            passive,
        });
        id
    }

    /// Remove a listener by its ID. Returns `true` if found and removed.
    pub fn remove(&mut self, id: ListenerId) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != len_before
    }

    /// Return all listener IDs matching the given event type and capture flag.
    #[must_use]
    pub fn matching(&self, event_type: &str, capture: bool) -> Vec<ListenerId> {
        self.entries
            .iter()
            .filter(|e| e.event_type == event_type && e.capture == capture)
            .map(|e| e.id)
            .collect()
    }

    /// Iterate over listener entries matching the given event type.
    pub fn iter_matching<'a>(
        &'a self,
        event_type: &'a str,
    ) -> impl Iterator<Item = &'a ListenerEntry> {
        self.entries
            .iter()
            .filter(move |e| e.event_type == event_type)
    }

    /// Return all listener entries matching the given event type (both capture and bubble).
    #[must_use]
    pub fn matching_all(&self, event_type: &str) -> Vec<&ListenerEntry> {
        self.entries
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }

    /// Return all listener IDs matching the given event type (both capture and bubble).
    #[must_use]
    pub fn matching_all_ids(&self, event_type: &str) -> Vec<ListenerId> {
        self.matching_all(event_type)
            .into_iter()
            .map(|e| e.id)
            .collect()
    }

    /// Find a listener entry by its ID.
    #[must_use]
    pub fn find_entry(&self, id: ListenerId) -> Option<&ListenerEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Returns `true` if there are no registered listeners.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of registered listeners.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_returns_unique_ids() {
        let mut listeners = EventListeners::new();
        let id1 = listeners.add("click", false);
        let id2 = listeners.add("click", false);
        assert_ne!(id1, id2);
    }

    #[test]
    fn remove_existing_listener() {
        let mut listeners = EventListeners::new();
        let id = listeners.add("click", false);
        assert!(listeners.remove(id));
        assert!(listeners.is_empty());
    }

    #[test]
    fn remove_nonexistent_listener() {
        let mut listeners = EventListeners::new();
        assert!(!listeners.remove(ListenerId(999)));
    }

    #[test]
    fn matching_filters_by_type_and_capture() {
        let mut listeners = EventListeners::new();
        let id1 = listeners.add("click", false);
        let _id2 = listeners.add("click", true);
        let _id3 = listeners.add("keydown", false);

        let result = listeners.matching("click", false);
        assert_eq!(result, vec![id1]);
    }

    #[test]
    fn matching_all_returns_both_phases() {
        let mut listeners = EventListeners::new();
        listeners.add("click", false);
        listeners.add("click", true);
        listeners.add("keydown", false);

        let result = listeners.matching_all("click");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn len_and_is_empty() {
        let mut listeners = EventListeners::new();
        assert!(listeners.is_empty());
        assert_eq!(listeners.len(), 0);

        listeners.add("click", false);
        assert!(!listeners.is_empty());
        assert_eq!(listeners.len(), 1);
    }

    #[test]
    fn listener_id_raw_roundtrip() {
        let id = ListenerId::from_raw(42);
        assert_eq!(id.to_raw(), 42);
    }

    #[test]
    fn remove_preserves_order_of_remaining() {
        let mut listeners = EventListeners::new();
        let id1 = listeners.add("click", false);
        let id2 = listeners.add("click", false);
        let id3 = listeners.add("click", false);

        listeners.remove(id2);
        let ids: Vec<_> = listeners.matching("click", false);
        assert_eq!(ids, vec![id1, id3]);
    }
}
