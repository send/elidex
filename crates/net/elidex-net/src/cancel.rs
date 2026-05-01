//! Cancellation primitive for in-flight fetch requests.
//!
//! [`CancelHandle`] is a cheap-to-clone, async-aware "go away"
//! signal used to abort an in-flight HTTP request before its
//! tokio future has resolved.  Modelled on
//! `tokio_util::sync::CancellationToken` but kept in-house to
//! avoid the extra workspace dep — the surface here is a small
//! subset (one-shot `cancel()`, async `cancelled()`).
//!
//! ## Why
//!
//! Without cancellation, the broker's `RendererToNetwork::CancelFetch`
//! handler can only synthesise an `Err("aborted")` reply on the
//! renderer side; the underlying tokio request continues to run
//! until network IO completes (success, error, or transport
//! timeout — typically 30s for a stalled connection).  This
//! leaks a `MAX_CONCURRENT_FETCHES` slot per cancel: a workload
//! that issues many fetches and cancels each one immediately can
//! saturate the global concurrency limit and starve subsequent
//! fetches until the cancelled IO drains.
//!
//! With a [`CancelHandle`] threaded through `NetClient::send`
//! and `HttpTransport::send`, `cancel()` drops the underlying
//! hyper future immediately via `tokio::select!`, the
//! `FetchInflightGuard` decrements the counter, and subsequent
//! fetches are unblocked.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

/// One-shot cancellation signal shared between the requester
/// (broker thread) and the in-flight HTTP request (transport
/// futures).  Cheap to clone (`Arc` internally) so each layer
/// can hold its own handle without `&` borrows fighting the
/// async lifetime.
#[derive(Clone, Debug, Default)]
pub struct CancelHandle(Arc<CancelInner>);

#[derive(Debug, Default)]
struct CancelInner {
    /// Set to `true` once `cancel()` has fired.  Read by
    /// [`CancelHandle::is_cancelled`] for a synchronous probe;
    /// `cancelled().await` polls this then awaits the `Notify`.
    cancelled: AtomicBool,
    /// Wakes any task currently parked on `cancelled().await`.
    /// `notify_waiters` is preferred over `notify_one` so every
    /// concurrent `cancelled()` future resolves on a single
    /// `cancel()` (multiple layers — `transport.send` +
    /// `redirect::follow_redirects` — may be racing each other
    /// for the abort).
    notify: Notify,
}

impl CancelHandle {
    /// Construct a fresh handle in the un-cancelled state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Trigger cancellation.  Idempotent: subsequent calls are
    /// no-ops.  Any task currently parked on
    /// [`CancelHandle::cancelled`] resolves; subsequent
    /// `cancelled()` futures resolve immediately on the
    /// `is_cancelled()` fast-path.
    pub fn cancel(&self) {
        // `Release` ensures the store is visible to any
        // subsequent `is_cancelled()` `Acquire` load.
        self.0.cancelled.store(true, Ordering::Release);
        self.0.notify.notify_waiters();
    }

    /// Synchronous probe.  Useful for opportunistic abort
    /// checks that don't want to await (e.g. before allocating
    /// a new connection).
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    /// Future that resolves once [`Self::cancel`] has fired.
    /// Designed to be `.select!`'d against the actual request
    /// future inside `HttpTransport::send`:
    ///
    /// ```ignore
    /// tokio::select! {
    ///     _ = cancel.cancelled() => Err(NetError::cancelled()),
    ///     res = do_fetch() => res,
    /// }
    /// ```
    ///
    /// Resolves immediately when the handle is already
    /// cancelled at the time of the call (avoids parking on the
    /// `Notify` for a wake that already happened).
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        // Race window: cancel() may have fired between the
        // probe and the `notified()` future construction.
        // `notify_waiters` only wakes parked-at-the-time
        // waiters, so we re-check after registering interest
        // to avoid losing a wake.
        let notified = self.0.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_resolves_after_cancel() {
        let h = CancelHandle::new();
        let h2 = h.clone();
        let join = tokio::spawn(async move {
            h2.cancelled().await;
        });
        // Yield once so the spawned task has a chance to park
        // on `notified()` before we fire the cancel.
        tokio::task::yield_now().await;
        h.cancel();
        join.await.expect("waiter resolved");
        assert!(h.is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_returns_immediately_if_already_cancelled() {
        let h = CancelHandle::new();
        h.cancel();
        // Should not park.
        h.cancelled().await;
        assert!(h.is_cancelled());
    }

    #[tokio::test]
    async fn multiple_clones_all_resolve_on_single_cancel() {
        let h = CancelHandle::new();
        let h2 = h.clone();
        let h3 = h.clone();
        let join = tokio::spawn(async move {
            tokio::join!(h2.cancelled(), h3.cancelled());
        });
        tokio::task::yield_now().await;
        h.cancel();
        join.await.expect("both waiters resolved");
    }

    #[test]
    fn cancel_is_idempotent() {
        let h = CancelHandle::new();
        h.cancel();
        h.cancel();
        h.cancel();
        assert!(h.is_cancelled());
    }
}
