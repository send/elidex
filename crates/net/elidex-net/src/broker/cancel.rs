//! Per-fetch cancellation token map (`CancelMap`) + the panic-safe
//! RAII guards that keep the inflight counter and the cancel-token
//! map bounded.
//!
//! Worker threads spawned by [`super::dispatch::NetworkProcessState::handle_fetch`]
//! own a [`FetchInflightGuard`] (decrements `inflight_fetches` on
//! drop) and a [`CancelMapEntryGuard`] (removes the
//! `(client_id, FetchId)` entry on drop, including unwind paths).
//! Without those guards a worker panic anywhere in the hot path
//! would leak its slot or its cancel-token entry, eventually
//! starving the broker (Copilot R2 / R5 findings).

use std::collections::HashMap;
use std::sync::Arc;

use super::FetchId;

/// Per-fetch cancellation token map.  Worker threads drop their
/// entry on completion (regardless of outcome) so the map is
/// bounded by `MAX_CONCURRENT_FETCHES` rather than the total
/// fetch count.  Wrapped in `Arc<Mutex<...>>` because the broker
/// thread inserts/cancels and worker threads remove on
/// completion.
///
/// **Key shape**: `(client_id, FetchId)` — keying on `FetchId`
/// alone would let one renderer cancel another renderer's
/// in-flight fetch by guessing/observing its id (the broker's
/// synthetic `Err("aborted")` reply would also be misrouted to
/// the *cancelling* client while the original client's promise
/// stays unresolved).  Pairing with `client_id` mirrors the
/// `ws_handles` / `sse_handles` ownership convention (Copilot
/// R1).
pub(super) type CancelMap = Arc<std::sync::Mutex<HashMap<(u64, FetchId), crate::CancelHandle>>>;

/// RAII guard that decrements the inflight fetch counter on drop.
/// Ensures the counter is decremented even if the fetch thread panics.
pub(super) struct FetchInflightGuard(pub(super) Arc<std::sync::atomic::AtomicUsize>);

impl Drop for FetchInflightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// RAII guard that removes the worker's `(cid, fetch_id)` entry
/// from [`CancelMap`] on drop — including unwind paths.  Without
/// this, a panic anywhere in the worker (tokio runtime build,
/// `block_on`, future internals, downstream `.expect`s) would
/// leave the entry in the map; over time those orphan entries
/// would grow the map beyond the documented `MAX_CONCURRENT_FETCHES`
/// bound (Copilot R2).  Uses `unwrap_or_else(into_inner)` so a
/// poisoned mutex during panic teardown still releases the
/// entry rather than double-panicking.
pub(super) struct CancelMapEntryGuard {
    pub(super) map: CancelMap,
    pub(super) key: (u64, FetchId),
}

impl Drop for CancelMapEntryGuard {
    fn drop(&mut self) {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for Copilot R2 (PR-true-request-cancellation,
    /// PR #136 — `cancel_map` leak on worker panic): the worker
    /// thread must remove its
    /// `(cid, fetch_id)` entry from [`CancelMap`] even on
    /// unwind, otherwise an `expect()` panic anywhere in the
    /// hot path leaks the entry and grows the map past its
    /// `MAX_CONCURRENT_FETCHES` bound.
    ///
    /// We can't easily force a panic inside the live worker
    /// without test-only inject points, so we exercise the
    /// guard directly: insert an entry, drop the guard via
    /// `catch_unwind` after deliberately panicking, then
    /// verify the map is empty.
    #[test]
    fn cancel_map_entry_guard_removes_on_panic() {
        let map: CancelMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let key = (42_u64, FetchId::next());
        map.lock().unwrap().insert(key, crate::CancelHandle::new());
        assert_eq!(map.lock().unwrap().len(), 1);

        let map_for_worker = Arc::clone(&map);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _entry = CancelMapEntryGuard {
                map: map_for_worker,
                key,
            };
            panic!("simulated worker panic");
        }));
        assert!(result.is_err(), "panic was caught");
        // Guard's Drop ran during unwind → entry removed.
        assert!(
            map.lock().unwrap().is_empty(),
            "CancelMapEntryGuard leaked entry on panic"
        );
    }

    /// Sibling assertion: the guard removes the entry on
    /// normal scope exit too (the success path).
    #[test]
    fn cancel_map_entry_guard_removes_on_normal_drop() {
        let map: CancelMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let key = (7_u64, FetchId::next());
        map.lock().unwrap().insert(key, crate::CancelHandle::new());
        {
            let _entry = CancelMapEntryGuard {
                map: Arc::clone(&map),
                key,
            };
            // No panic — guard drops at end of block.
        }
        assert!(map.lock().unwrap().is_empty());
    }
}
