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

/// Distinguishes an `addEventListener`-registered listener from one
/// backing an event-handler IDL attribute (`el.onclick = fn`).
///
/// WHATWG HTML §8.1.8.1: an event handler is a *special kind of event
/// listener*. It participates in dispatch in registration order like
/// any listener, but its callback is the internal "event handler
/// processing algorithm" — so `removeEventListener(type, fn)` and the
/// `addEventListener` duplicate check (both keyed on the raw callback
/// object) must **not** match it. Identity-match call sites filter to
/// [`ListenerKind::Normal`]; dispatch invokes both kinds uniformly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListenerKind {
    /// Registered via `addEventListener`. Subject to identity match.
    Normal,
    /// Backs an event-handler IDL attribute. `uncompiled` holds the
    /// inline content-attribute source (`<button onclick="...">`)
    /// pending lazy compilation (WHATWG HTML §8.1.8.1 "internal raw
    /// uncompiled handler"); `None` once compiled or set via the IDL
    /// setter. When `Some`, it takes precedence over any stale
    /// compiled callable in the engine's listener store (last-write-
    /// wins): the read/dispatch site recompiles and overwrites.
    EventHandler {
        /// Inline source awaiting lazy compile, or `None`.
        uncompiled: Option<UncompiledHandler>,
    },
}

/// An inline event-handler content attribute's uncompiled source
/// (WHATWG HTML §8.1.8.1). Engine-independent: holds the raw source
/// string only; compilation happens in the JS engine layer at first
/// invoke / on read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncompiledHandler {
    /// The attribute value (handler body source), e.g. `"alert('x')"`.
    pub source: String,
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
    /// `Normal` (addEventListener) vs `EventHandler` (IDL attribute).
    pub kind: ListenerKind,
}

impl ListenerEntry {
    /// Construct a `Normal` (addEventListener-registered) entry.
    ///
    /// Builder used by every Normal-listener construction site so the
    /// `kind` field stays additive at call sites (a bare struct literal
    /// would force every site to spell out `kind: ListenerKind::Normal`).
    #[must_use]
    pub fn normal(
        id: ListenerId,
        event_type: String,
        capture: bool,
        once: bool,
        passive: bool,
    ) -> Self {
        Self {
            id,
            event_type,
            capture,
            once,
            passive,
            kind: ListenerKind::Normal,
        }
    }
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
        self.entries.push(ListenerEntry::normal(
            id,
            event_type.into(),
            capture,
            once,
            passive,
        ));
        id
    }

    /// Register an event-handler IDL attribute listener (WHATWG HTML
    /// §8.1.8.1 "activate an event handler"). Capture is always `false`
    /// per spec. The compiled callable lives in the engine's listener
    /// store keyed by the returned [`ListenerId`]; `uncompiled` starts
    /// `None` (IDL setter path) and is populated via [`Self::set_uncompiled`]
    /// for inline content-attribute handlers.
    pub fn add_event_handler(&mut self, event_type: impl Into<String>) -> ListenerId {
        let id = ListenerId(NEXT_LISTENER_ID.fetch_add(1, Ordering::Relaxed));
        self.entries.push(ListenerEntry {
            id,
            event_type: event_type.into(),
            capture: false,
            once: false,
            passive: false,
            kind: ListenerKind::EventHandler { uncompiled: None },
        });
        id
    }

    /// Find the single event-handler listener for `event_type`, if any
    /// (WHATWG HTML §8.1.8.1 — at most one handler per (target, event
    /// type)). This is the activation-idempotency lookup: a hit means a
    /// handler listener already exists for this attribute. Engine-
    /// independent single source of truth (no VM-side reverse map).
    #[must_use]
    pub fn find_event_handler(&self, event_type: &str) -> Option<ListenerId> {
        self.entries
            .iter()
            .find(|e| {
                e.event_type == event_type && matches!(e.kind, ListenerKind::EventHandler { .. })
            })
            .map(|e| e.id)
    }

    /// Set the uncompiled inline source for an event-handler listener
    /// (content-attribute write path). No-op if `id` is absent or not an
    /// event-handler listener.
    pub fn set_uncompiled(&mut self, id: ListenerId, source: impl Into<String>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            if let ListenerKind::EventHandler { uncompiled } = &mut entry.kind {
                *uncompiled = Some(UncompiledHandler {
                    source: source.into(),
                });
            }
        }
    }

    /// Drain the uncompiled inline source for an event-handler listener,
    /// leaving `uncompiled = None`. Used by the lazy-compile step at
    /// read/dispatch time (WHATWG HTML §8.1.8.1 "get the current value").
    pub fn take_uncompiled(&mut self, id: ListenerId) -> Option<UncompiledHandler> {
        let entry = self.entries.iter_mut().find(|e| e.id == id)?;
        if let ListenerKind::EventHandler { uncompiled } = &mut entry.kind {
            uncompiled.take()
        } else {
            None
        }
    }

    /// Clear an event-handler listener's uncompiled source without
    /// returning it (IDL setter / null-clear path: a fresh compiled
    /// callable supersedes any pending inline source).
    pub fn clear_uncompiled(&mut self, id: ListenerId) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            if let ListenerKind::EventHandler { uncompiled } = &mut entry.kind {
                *uncompiled = None;
            }
        }
    }

    /// Read an event-handler listener's pending uncompiled source
    /// (without draining). `None` if absent, not a handler, or already
    /// compiled.
    #[must_use]
    pub fn uncompiled_source(&self, id: ListenerId) -> Option<&str> {
        self.entries.iter().find(|e| e.id == id).and_then(|e| {
            if let ListenerKind::EventHandler {
                uncompiled: Some(u),
            } = &e.kind
            {
                Some(u.source.as_str())
            } else {
                None
            }
        })
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
    ///
    /// The `event_type` lifetime is tied to `self` because the returned
    /// iterator's closure captures the reference. Callers needing a
    /// temporary `String` should bind it to a local variable first.
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
    fn normal_builder_sets_normal_kind() {
        let mut listeners = EventListeners::new();
        let id = listeners.add("click", false);
        let entry = listeners.find_entry(id).unwrap();
        assert_eq!(entry.kind, ListenerKind::Normal);
    }

    #[test]
    fn add_event_handler_is_event_handler_kind_capture_false() {
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        let entry = listeners.find_entry(id).unwrap();
        assert!(matches!(entry.kind, ListenerKind::EventHandler { .. }));
        assert!(!entry.capture);
    }

    #[test]
    fn find_event_handler_matches_only_handler_kind() {
        let mut listeners = EventListeners::new();
        let normal = listeners.add("click", false);
        // A Normal listener for "click" must NOT be returned.
        assert_eq!(listeners.find_event_handler("click"), None);
        let handler = listeners.add_event_handler("click");
        assert_eq!(listeners.find_event_handler("click"), Some(handler));
        assert_ne!(handler, normal);
        // Different event type → no match.
        assert_eq!(listeners.find_event_handler("keydown"), None);
    }

    #[test]
    fn set_take_uncompiled_roundtrip() {
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        assert_eq!(listeners.uncompiled_source(id), None);
        listeners.set_uncompiled(id, "window.x = 1");
        assert_eq!(listeners.uncompiled_source(id), Some("window.x = 1"));
        let taken = listeners.take_uncompiled(id).unwrap();
        assert_eq!(taken.source, "window.x = 1");
        // Drained.
        assert_eq!(listeners.uncompiled_source(id), None);
        assert_eq!(listeners.take_uncompiled(id), None);
    }

    #[test]
    fn clear_uncompiled_resets() {
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        listeners.set_uncompiled(id, "a()");
        listeners.clear_uncompiled(id);
        assert_eq!(listeners.uncompiled_source(id), None);
    }

    #[test]
    fn uncompiled_ops_noop_on_normal_listener() {
        let mut listeners = EventListeners::new();
        let id = listeners.add("click", false);
        // Setting uncompiled on a Normal listener is a no-op.
        listeners.set_uncompiled(id, "a()");
        assert_eq!(listeners.uncompiled_source(id), None);
        assert_eq!(listeners.take_uncompiled(id), None);
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
