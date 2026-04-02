//! Service Worker FetchEvent relay.
//!
//! Manages pending fetch requests that are routed through the browser thread
//! to a Service Worker thread, then back to the originating content thread.
//!
//! Flow: content thread → browser thread (here) → SW thread → response back.

use std::collections::HashMap;
use std::time::Instant;

use super::tab::TabId;
use elidex_api_sw::{ContentToSw, SwHandle, SwResponse};

/// Timeout for pending SW fetch requests (30 seconds, matching SW idle timeout).
const SW_FETCH_TIMEOUT_SECS: u64 = 30;

/// A pending fetch awaiting a SW response.
struct PendingFetch {
    tab_id: TabId,
    initiated_at: Instant,
    #[allow(dead_code)]
    scope: url::Url,
}

/// Manages pending SW fetch interception requests.
///
/// Owned by the browser thread `App`. The lifecycle is:
/// 1. `initiate()` — content thread requests fetch interception
/// 2. SW thread processes FetchEvent and responds
/// 3. `resolve()` — browser thread routes response back to content thread
/// 4. `check_timeouts()` — periodic cleanup of stale requests
pub struct SwFetchRelay {
    pending: HashMap<u64, PendingFetch>,
    next_id: u64,
}

impl SwFetchRelay {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            next_id: 1,
        }
    }

    /// Initiate a fetch interception request.
    ///
    /// Sends `ContentToSw::FetchEvent` to the SW handle and registers the
    /// pending fetch for response routing. Returns the fetch_id.
    pub fn initiate(
        &mut self,
        tab_id: TabId,
        scope: &url::Url,
        handle: &SwHandle,
        request: elidex_api_sw::SwRequest,
        client_id: String,
        resulting_client_id: String,
    ) -> u64 {
        let fetch_id = self.next_id;
        self.next_id += 1;

        handle.send(ContentToSw::FetchEvent {
            fetch_id,
            request: Box::new(request),
            client_id,
            resulting_client_id,
        });

        self.pending.insert(
            fetch_id,
            PendingFetch {
                tab_id,
                initiated_at: Instant::now(),
                scope: scope.clone(),
            },
        );

        fetch_id
    }

    /// Resolve a fetch response from the SW thread.
    ///
    /// Returns `Some((tab_id, response))` if the fetch was pending,
    /// `None` if the fetch_id was already resolved or timed out.
    pub fn resolve(
        &mut self,
        fetch_id: u64,
        response: Option<SwResponse>,
    ) -> Option<(TabId, Option<SwResponse>)> {
        let pending = self.pending.remove(&fetch_id)?;
        Some((pending.tab_id, response))
    }

    /// Check for timed-out pending fetches.
    ///
    /// Returns a list of (tab_id, fetch_id) pairs that have exceeded the timeout.
    /// These should be resolved with a passthrough (None response).
    pub fn check_timeouts(&mut self) -> Vec<(TabId, u64)> {
        let now = Instant::now();
        let timeout = std::time::Duration::from_secs(SW_FETCH_TIMEOUT_SECS);
        let mut timed_out = Vec::new();

        self.pending.retain(|&fetch_id, pending| {
            if now.duration_since(pending.initiated_at) > timeout {
                timed_out.push((pending.tab_id, fetch_id));
                false
            } else {
                true
            }
        });

        timed_out
    }

    /// Number of pending fetches (for debugging).
    #[allow(dead_code)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for SwFetchRelay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initiate_and_resolve() {
        let mut relay = SwFetchRelay::new();

        // We can't easily create a real SwHandle in tests, so test
        // the resolve/timeout logic directly.
        relay.pending.insert(
            1,
            PendingFetch {
                tab_id: TabId(42),
                initiated_at: Instant::now(),
                scope: url::Url::parse("https://example.com/").unwrap(),
            },
        );

        let result = relay.resolve(1, None);
        assert!(result.is_some());
        let (tab_id, resp) = result.unwrap();
        assert_eq!(tab_id, TabId(42));
        assert!(resp.is_none());

        // Double-resolve returns None.
        assert!(relay.resolve(1, None).is_none());
    }

    #[test]
    fn timeout_detection() {
        let mut relay = SwFetchRelay::new();

        // Insert an already-timed-out entry.
        relay.pending.insert(
            1,
            PendingFetch {
                tab_id: TabId(1),
                initiated_at: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(60))
                    .unwrap(),
                scope: url::Url::parse("https://example.com/").unwrap(),
            },
        );

        // Insert a fresh entry.
        relay.pending.insert(
            2,
            PendingFetch {
                tab_id: TabId(2),
                initiated_at: Instant::now(),
                scope: url::Url::parse("https://example.com/").unwrap(),
            },
        );

        let timed_out = relay.check_timeouts();
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0], (TabId(1), 1));
        assert_eq!(relay.pending_count(), 1); // only fresh entry remains
    }
}
