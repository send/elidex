//! Deferred event queue for events that cannot be dispatched synchronously.
//!
//! Used by `checkValidity()` (boa re-entrancy constraint) and future
//! `MutationObserver` callback delivery.

use elidex_ecs::Entity;
use elidex_plugin::EventPayload;

/// Maximum pending events to prevent unbounded growth.
const MAX_PENDING_EVENTS: usize = 10_000;

/// A queued event awaiting dispatch.
#[derive(Clone, Debug)]
pub struct QueuedEvent {
    /// The event type string (e.g. "invalid", "change").
    pub event_type: String,
    /// The target entity for the event.
    pub target: Entity,
    /// Whether the event bubbles.
    pub bubbles: bool,
    /// Whether the event is cancelable.
    pub cancelable: bool,
    /// Event payload (mouse, keyboard, etc.).
    pub payload: EventPayload,
}

/// A queue for deferred event dispatch.
///
/// Events enqueued here are dispatched after the current JS execution completes,
/// avoiding re-entrancy issues with boa.
#[derive(Clone, Debug, Default)]
pub struct EventQueue {
    pending: Vec<QueuedEvent>,
}

impl EventQueue {
    /// Create a new empty event queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue an event for later dispatch.
    ///
    /// Drops the event with a warning if the queue exceeds `MAX_PENDING_EVENTS`,
    /// to prevent unbounded memory growth from runaway event generation.
    pub fn enqueue(&mut self, event: QueuedEvent) {
        if self.pending.len() < MAX_PENDING_EVENTS {
            self.pending.push(event);
        } else {
            eprintln!(
                "EventQueue: dropping '{}' event (queue full at {MAX_PENDING_EVENTS})",
                event.event_type
            );
        }
    }

    /// Drain all pending events, returning them in FIFO order.
    #[must_use]
    pub fn drain(&mut self) -> Vec<QueuedEvent> {
        std::mem::take(&mut self.pending)
    }

    /// Returns `true` if there are no pending events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Returns the number of pending events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_entity() -> Entity {
        // Spawn a real hecs entity to obtain valid Entity bits
        let mut world = hecs::World::new();
        world.spawn(())
    }

    #[test]
    fn enqueue_and_drain() {
        let mut q = EventQueue::new();
        assert!(q.is_empty());

        q.enqueue(QueuedEvent {
            event_type: "invalid".to_string(),
            target: dummy_entity(),
            bubbles: false,
            cancelable: true,
            payload: EventPayload::default(),
        });
        assert_eq!(q.len(), 1);

        let events = q.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "invalid");
        assert!(q.is_empty());
    }

    #[test]
    fn drain_returns_fifo_order() {
        let mut q = EventQueue::new();
        q.enqueue(QueuedEvent {
            event_type: "first".to_string(),
            target: dummy_entity(),
            bubbles: false,
            cancelable: false,
            payload: EventPayload::default(),
        });
        q.enqueue(QueuedEvent {
            event_type: "second".to_string(),
            target: dummy_entity(),
            bubbles: false,
            cancelable: false,
            payload: EventPayload::default(),
        });
        let events = q.drain();
        assert_eq!(events[0].event_type, "first");
        assert_eq!(events[1].event_type, "second");
    }

    #[test]
    fn max_pending_limit() {
        let mut q = EventQueue::new();
        for _ in 0..MAX_PENDING_EVENTS + 100 {
            q.enqueue(QueuedEvent {
                event_type: "test".to_string(),
                target: dummy_entity(),
                bubbles: false,
                cancelable: false,
                payload: EventPayload::default(),
            });
        }
        assert_eq!(q.len(), MAX_PENDING_EVENTS);
    }
}
