//! Background Sync (WICG proposal — Chrome/Edge implemented).
//!
//! One-shot sync: fires on network online transition when no foreground clients exist.
//! Periodic sync: fires on a browser-determined schedule (min 12h).
//!
//! Retry policy is UA-defined. elidex uses:
//! - Max 5 retries, exponential backoff (1min → 5min → 30min → 2h → 12h)
//! - lastChance=true on final retry

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Maximum retry attempts for one-shot sync.
const MAX_RETRIES: u32 = 5;

/// Backoff durations for each retry attempt.
const BACKOFF_DURATIONS: [Duration; 5] = [
    Duration::from_secs(60),    // 1 min
    Duration::from_secs(300),   // 5 min
    Duration::from_secs(1800),  // 30 min
    Duration::from_secs(7200),  // 2 hours
    Duration::from_secs(43200), // 12 hours
];

/// Minimum interval for periodic sync (browser enforced).
pub const PERIODIC_SYNC_MIN_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60); // 12 hours

/// State of a one-shot sync registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Waiting to be fired.
    Pending,
    /// Currently being processed by the SW.
    Firing,
    /// Re-registered while firing (will fire again after current attempt).
    ReregisteredWhileFiring,
}

/// A one-shot sync registration.
#[derive(Debug, Clone)]
pub struct SyncRegistration {
    pub tag: String,
    pub state: SyncState,
    pub retry_count: u32,
    pub next_retry: Option<Instant>,
}

/// A periodic sync registration.
#[derive(Debug, Clone)]
pub struct PeriodicSyncRegistration {
    pub tag: String,
    /// Requested min interval (browser treats as suggestion, enforces >= 12h).
    pub min_interval: Duration,
    pub last_fired: Option<Instant>,
}

/// Manages background sync registrations for an origin.
#[derive(Debug, Default)]
pub struct SyncManager {
    registrations: HashMap<String, SyncRegistration>,
    periodic: HashMap<String, PeriodicSyncRegistration>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self::default()
    }

    // -- One-shot sync --

    /// Register a one-shot sync. If tag exists, resets to Pending.
    pub fn register(&mut self, tag: String) {
        let entry = self.registrations.entry(tag.clone());
        match entry {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let reg = e.get_mut();
                if reg.state == SyncState::Firing {
                    reg.state = SyncState::ReregisteredWhileFiring;
                } else {
                    reg.state = SyncState::Pending;
                    reg.retry_count = 0;
                    reg.next_retry = None;
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(SyncRegistration {
                    tag,
                    state: SyncState::Pending,
                    retry_count: 0,
                    next_retry: None,
                });
            }
        }
    }

    /// Get tags of all registered (non-firing) syncs.
    pub fn get_tags(&self) -> Vec<String> {
        self.registrations.keys().cloned().collect()
    }

    /// Get sync registrations ready to fire.
    ///
    /// Returns tags of Pending registrations whose retry backoff has elapsed.
    pub fn ready_to_fire(&self) -> Vec<(String, bool)> {
        let now = Instant::now();
        self.registrations
            .iter()
            .filter(|(_, reg)| {
                reg.state == SyncState::Pending && reg.next_retry.is_none_or(|t| now >= t)
            })
            .map(|(tag, reg)| {
                let last_chance = reg.retry_count >= MAX_RETRIES - 1;
                (tag.clone(), last_chance)
            })
            .collect()
    }

    /// Mark a sync as firing.
    pub fn mark_firing(&mut self, tag: &str) {
        if let Some(reg) = self.registrations.get_mut(tag) {
            reg.state = SyncState::Firing;
        }
    }

    /// Report sync event result.
    pub fn sync_completed(&mut self, tag: &str, success: bool) {
        let Some(reg) = self.registrations.get_mut(tag) else {
            return;
        };

        if success {
            // If re-registered while firing, go back to Pending
            if reg.state == SyncState::ReregisteredWhileFiring {
                reg.state = SyncState::Pending;
                reg.retry_count = 0;
                reg.next_retry = None;
            } else {
                // Success — remove registration
                self.registrations.remove(tag);
            }
            return;
        }

        // Failure — schedule retry or remove
        reg.retry_count += 1;
        if reg.retry_count >= MAX_RETRIES {
            self.registrations.remove(tag);
            return;
        }

        let backoff_idx = (reg.retry_count - 1).min(BACKOFF_DURATIONS.len() as u32 - 1) as usize;
        reg.next_retry = Some(Instant::now() + BACKOFF_DURATIONS[backoff_idx]);

        reg.state = SyncState::Pending;
    }

    // -- Periodic sync --

    /// Register a periodic sync.
    pub fn register_periodic(&mut self, tag: String, min_interval: Duration) {
        let effective_interval = min_interval.max(PERIODIC_SYNC_MIN_INTERVAL);
        self.periodic.insert(
            tag.clone(),
            PeriodicSyncRegistration {
                tag,
                min_interval: effective_interval,
                last_fired: None,
            },
        );
    }

    /// Unregister a periodic sync.
    pub fn unregister_periodic(&mut self, tag: &str) -> bool {
        self.periodic.remove(tag).is_some()
    }

    /// Get tags of all periodic sync registrations.
    pub fn get_periodic_tags(&self) -> Vec<String> {
        self.periodic.keys().cloned().collect()
    }

    /// Get periodic sync registrations ready to fire.
    pub fn periodic_ready_to_fire(&self) -> Vec<String> {
        let now = Instant::now();
        self.periodic
            .iter()
            .filter(|(_, reg)| {
                reg.last_fired
                    .is_none_or(|t| now.duration_since(t) >= reg.min_interval)
            })
            .map(|(tag, _)| tag.clone())
            .collect()
    }

    /// Record that a periodic sync fired.
    pub fn periodic_fired(&mut self, tag: &str) {
        if let Some(reg) = self.periodic.get_mut(tag) {
            reg.last_fired = Some(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get_tags() {
        let mut sm = SyncManager::new();
        sm.register("sync-data".into());
        sm.register("sync-analytics".into());

        let mut tags = sm.get_tags();
        tags.sort();
        assert_eq!(tags, vec!["sync-analytics", "sync-data"]);
    }

    #[test]
    fn register_duplicate_resets() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());
        sm.mark_firing("tag");
        sm.register("tag".into()); // re-register while firing

        let reg = sm.registrations.get("tag").unwrap();
        assert_eq!(reg.state, SyncState::ReregisteredWhileFiring);
    }

    #[test]
    fn ready_to_fire() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());

        let ready = sm.ready_to_fire();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, "tag");
        assert!(!ready[0].1); // not last chance
    }

    #[test]
    fn sync_success_removes() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());
        sm.mark_firing("tag");
        sm.sync_completed("tag", true);

        assert!(sm.registrations.is_empty());
    }

    #[test]
    fn sync_failure_retries() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());
        sm.mark_firing("tag");
        sm.sync_completed("tag", false);

        let reg = sm.registrations.get("tag").unwrap();
        assert_eq!(reg.retry_count, 1);
        assert_eq!(reg.state, SyncState::Pending);
        assert!(reg.next_retry.is_some());
    }

    #[test]
    fn sync_max_retries_removes() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());

        for _ in 0..MAX_RETRIES {
            sm.mark_firing("tag");
            sm.sync_completed("tag", false);
            // Reset state for next iteration if still exists
            if sm.registrations.contains_key("tag") {
                sm.registrations.get_mut("tag").unwrap().next_retry = None;
            }
        }

        assert!(sm.registrations.is_empty());
    }

    #[test]
    fn reregistered_while_firing_success() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());
        sm.mark_firing("tag");
        sm.register("tag".into()); // re-register during fire
        sm.sync_completed("tag", true);

        // Should go back to Pending (re-registered)
        let reg = sm.registrations.get("tag").unwrap();
        assert_eq!(reg.state, SyncState::Pending);
    }

    #[test]
    fn last_chance_on_final_retry() {
        let mut sm = SyncManager::new();
        sm.register("tag".into());

        // Simulate retries up to MAX - 1
        for _ in 0..MAX_RETRIES - 1 {
            sm.mark_firing("tag");
            sm.sync_completed("tag", false);
            sm.registrations.get_mut("tag").unwrap().next_retry = None;
        }

        let ready = sm.ready_to_fire();
        assert_eq!(ready.len(), 1);
        assert!(ready[0].1); // last_chance = true
    }

    #[test]
    fn periodic_sync_min_interval_enforced() {
        let mut sm = SyncManager::new();
        // Request 1 minute — should be raised to 12 hours
        sm.register_periodic("analytics".into(), Duration::from_secs(60));

        let reg = sm.periodic.get("analytics").unwrap();
        assert!(reg.min_interval >= PERIODIC_SYNC_MIN_INTERVAL);
    }

    #[test]
    fn periodic_sync_lifecycle() {
        let mut sm = SyncManager::new();
        sm.register_periodic("tag".into(), PERIODIC_SYNC_MIN_INTERVAL);

        assert_eq!(sm.get_periodic_tags(), vec!["tag"]);

        // Should be ready (never fired)
        assert_eq!(sm.periodic_ready_to_fire().len(), 1);

        sm.periodic_fired("tag");

        // Should NOT be ready (just fired)
        assert!(sm.periodic_ready_to_fire().is_empty());

        sm.unregister_periodic("tag");
        assert!(sm.get_periodic_tags().is_empty());
    }
}
