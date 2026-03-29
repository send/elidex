//! Timer queue for `setTimeout`, `setInterval`, and `requestAnimationFrame`.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::time::{Duration, Instant};

/// Opaque timer identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerId(u64);

impl TimerId {
    /// Returns the raw numeric ID (exposed to JS).
    pub fn to_raw(self) -> u64 {
        self.0
    }

    /// Create a `TimerId` from a raw `u64` (for timer cancellation from JS).
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// A pending timer entry.
///
/// # Phase 2 limitation
///
/// Callbacks are stored as source code strings, not JS function objects.
/// This means `setTimeout(() => { ... }, 100)` converts the function via
/// `toString()` and re-evaluates the source text, which won't call the
/// function. Only string-form callbacks work: `setTimeout("code()", 100)`.
/// A future phase should store `JsFunction` values for proper callback support.
#[derive(Debug)]
struct TimerEntry {
    id: TimerId,
    fire_at: Instant,
    /// JS source code to evaluate when the timer fires (Phase 2 limitation:
    /// function callbacks are stringified, not stored as closures).
    callback: String,
    /// `Some(duration)` for `setInterval`, `None` for `setTimeout`.
    interval: Option<Duration>,
}

impl Eq for TimerEntry {}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.fire_at.cmp(&other.fire_at)
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Min-heap timer queue managing `setTimeout`, `setInterval`, and `requestAnimationFrame`.
pub struct TimerQueue {
    next_id: u64,
    pending: BinaryHeap<Reverse<TimerEntry>>,
    cancelled: HashSet<TimerId>,
}

impl TimerQueue {
    /// Create a new empty timer queue.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            pending: BinaryHeap::new(),
            cancelled: HashSet::new(),
        }
    }

    /// Schedule a `setTimeout` callback.
    pub fn set_timeout(&mut self, callback: String, delay_ms: u64) -> TimerId {
        let id = self.alloc_id();
        self.pending.push(Reverse(TimerEntry {
            id,
            fire_at: Instant::now() + Duration::from_millis(delay_ms),
            callback,
            interval: None,
        }));
        id
    }

    /// Schedule a `setInterval` callback.
    pub fn set_interval(&mut self, callback: String, interval_ms: u64) -> TimerId {
        let id = self.alloc_id();
        let interval = Duration::from_millis(interval_ms);
        self.pending.push(Reverse(TimerEntry {
            id,
            fire_at: Instant::now() + interval,
            callback,
            interval: Some(interval),
        }));
        id
    }

    /// Schedule a `requestAnimationFrame` callback (fires at next drain).
    pub fn request_animation_frame(&mut self, callback: String) -> TimerId {
        let id = self.alloc_id();
        self.pending.push(Reverse(TimerEntry {
            id,
            fire_at: Instant::now(), // fires immediately on next drain
            callback,
            interval: None,
        }));
        id
    }

    /// Cancel a timer by ID (`clearTimeout`, `clearInterval`, `cancelAnimationFrame`).
    ///
    /// IDs of 0 or IDs that were never allocated are ignored, preventing
    /// unbounded growth of the cancelled set from bogus JS values.
    pub fn clear_timer(&mut self, id: TimerId) {
        if id.0 != 0 && id.0 < self.next_id {
            self.cancelled.insert(id);
        }
    }

    /// Drain all timers that are ready to fire.
    ///
    /// Returns `(TimerId, callback_source)` pairs for ready timers.
    /// Interval timers are automatically re-scheduled.
    ///
    /// To prevent infinite loops with 0ms intervals, each interval fires
    /// at most once per `drain_ready` call.
    pub fn drain_ready(&mut self) -> Vec<(TimerId, String)> {
        let now = Instant::now();
        let mut ready = Vec::new();
        let mut reschedule = Vec::new();

        while let Some(Reverse(entry)) = self.pending.peek() {
            if entry.fire_at > now {
                break;
            }
            let Reverse(entry) = self.pending.pop().unwrap();

            // Skip cancelled timers.
            if self.cancelled.remove(&entry.id) {
                continue;
            }

            ready.push((entry.id, entry.callback.clone()));

            // Collect interval timers for re-scheduling after the loop.
            if let Some(interval) = entry.interval {
                let min_interval = interval.max(Duration::from_millis(1));
                reschedule.push(TimerEntry {
                    id: entry.id,
                    fire_at: now + min_interval,
                    callback: entry.callback,
                    interval: Some(interval),
                });
            }
        }

        // Re-schedule interval timers after draining.
        for entry in reschedule {
            self.pending.push(Reverse(entry));
        }

        // Compact: remove stale cancelled entries that no longer match any
        // pending timer, preventing unbounded growth.
        if !self.cancelled.is_empty() {
            let pending_ids: HashSet<TimerId> =
                self.pending.iter().map(|Reverse(e)| e.id).collect();
            self.cancelled.retain(|id| pending_ids.contains(id));
        }

        ready
    }

    /// Returns the deadline of the next timer, if any.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.pending.peek().map(|Reverse(entry)| entry.fire_at)
    }

    /// Returns the number of pending (non-cancelled) timers.
    ///
    /// Note: cancelled timers are lazily removed, so this may over-count.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Returns `true` if there are no pending timers.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Clear all pending timers (`window.stop()` support).
    pub fn clear_all(&mut self) {
        self.pending.clear();
        self.cancelled.clear();
    }

    fn alloc_id(&mut self) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Default for TimerQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn set_timeout_fires() {
        let mut q = TimerQueue::new();
        q.set_timeout("cb()".into(), 0);
        let ready = q.drain_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].1, "cb()");
    }

    #[test]
    fn set_timeout_delayed() {
        let mut q = TimerQueue::new();
        q.set_timeout("cb()".into(), 1000);
        // Not ready yet.
        let ready = q.drain_ready();
        assert!(ready.is_empty());
    }

    #[test]
    fn clear_timeout() {
        let mut q = TimerQueue::new();
        let id = q.set_timeout("cb()".into(), 0);
        q.clear_timer(id);
        let ready = q.drain_ready();
        assert!(ready.is_empty());
    }

    #[test]
    fn set_interval_repeats() {
        let mut q = TimerQueue::new();
        q.set_interval("tick()".into(), 0);

        let ready1 = q.drain_ready();
        assert_eq!(ready1.len(), 1);

        // After drain, the interval is re-scheduled with min 1ms delay.
        // Sleep to let it become ready.
        thread::sleep(Duration::from_millis(5));
        let ready2 = q.drain_ready();
        assert_eq!(ready2.len(), 1);
    }

    #[test]
    fn clear_interval() {
        let mut q = TimerQueue::new();
        let id = q.set_interval("tick()".into(), 0);
        q.drain_ready(); // First fire.
        q.clear_timer(id);
        // Even after sleeping, cancelled timer should not fire.
        thread::sleep(Duration::from_millis(5));
        let ready = q.drain_ready();
        assert!(ready.is_empty());
    }

    #[test]
    fn request_animation_frame_fires_immediately() {
        let mut q = TimerQueue::new();
        q.request_animation_frame("raf()".into());
        let ready = q.drain_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].1, "raf()");
        // rAF does not repeat.
        let ready2 = q.drain_ready();
        assert!(ready2.is_empty());
    }

    #[test]
    fn cancel_animation_frame() {
        let mut q = TimerQueue::new();
        let id = q.request_animation_frame("raf()".into());
        q.clear_timer(id);
        let ready = q.drain_ready();
        assert!(ready.is_empty());
    }

    #[test]
    fn ordering_by_fire_time() {
        let mut q = TimerQueue::new();
        q.set_timeout("late()".into(), 50);
        thread::sleep(Duration::from_millis(1));
        q.set_timeout("early()".into(), 0);

        let ready = q.drain_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].1, "early()");
    }

    #[test]
    fn next_deadline() {
        let mut q = TimerQueue::new();
        assert!(q.next_deadline().is_none());
        q.set_timeout("cb()".into(), 100);
        assert!(q.next_deadline().is_some());
    }

    #[test]
    fn timer_id_uniqueness() {
        let mut q = TimerQueue::new();
        let id1 = q.set_timeout("a()".into(), 0);
        let id2 = q.set_timeout("b()".into(), 0);
        assert_ne!(id1, id2);
    }
}
