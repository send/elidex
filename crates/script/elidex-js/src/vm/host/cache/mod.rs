//! Cache API host bindings (WHATWG Service Workers §5; slot
//! `#11-cache-api-vm` / D-19 PR-1).
//!
//! ```text
//! caches  (ObjectKind::CacheStorage singleton) → CacheStorage.prototype → Object.prototype
//! Cache   (ObjectKind::Cache, vended by caches.open) → Cache.prototype → Object.prototype
//! ```
//!
//! ## Layering (CLAUDE.md Layering mandate)
//!
//! This module is marshalling plus the §5 Promise-delivery orchestration
//! ONLY.  Every CacheStorage open/has/delete/keys, every per-cache
//! match/put/delete/keys, and the matching/Vary algorithm live in the
//! engine-independent `elidex-cache-api` crate (`storage.rs` / `store.rs`
//! / `entry.rs`).  host/cache/ converts `JsValue` to/from `CachedEntry` /
//! `Response` / `Request`, holds the shared origin connection
//! ([`backend::CacheBackend`], DR-A), and settles the returned Promise on
//! the VM event loop — all engine-bound concerns.
//!
//! ## Async model (Cache API §5; DR-A.1)
//!
//! Each `caches.*` / `Cache.*` op runs its backend call **synchronously**
//! at the native call site (the SQLite store is local — no real
//! parallelism), marshals the result into an owned [`CacheDelivery`]
//! outcome, then settles its Promise via a queued
//! [`super::pending_tasks::PendingTask::CacheDeliver`] task drained at the
//! event-loop tail — never inline.  This matches the spec's "run these
//! steps in parallel … queue a task" ordering and reuses the single
//! VM async-delivery model (the IDB `IdbDeliver` mechanism + the
//! `PostMessage` owned-payload shape), not a second sync-settle path.
//!
//! ## Deferred surface
//!
//! `Cache.add` / `Cache.addAll` (§5.4.3 / §5.4.4) are **not installed** —
//! they fetch over the network then put, which needs a native
//! promise-continuation hooked into the fetch broker settlement
//! (`fetch_tick::settle_fetch`) that the VM does not have yet.  Faking
//! them (boa's synchronous empty-response) would corrupt the cache, so
//! the honest surface is their absence until the fetch-integration
//! tranche lands → slot `#11-cache-add-fetch-integration`.  All other 11
//! surfaces are real.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::value::{JsValue, ObjectId, VmError};
use super::super::VmInner;

mod backend;
mod marshal;
mod natives;
mod register;

pub(crate) use backend::CacheBackend;

/// Per-`Cache`-`ObjectId` handle state (Cache API §5.4) — the cache name
/// only.  Every op routes through the shared origin backend, so a `Cache`
/// wrapper carries no per-instance data beyond which named cache it
/// targets (StorageEvent / IDB side-store precedent; the brand stays
/// payload-free).
#[derive(Debug)]
pub(crate) struct CacheHandleState {
    pub(crate) cache_name: String,
}

/// Owned outcome staged in
/// [`super::pending_tasks::PendingTask::CacheDeliver`] (DR-A.1).  Built
/// synchronously at the native call site; settled at drain.  `Copy` (its
/// only payload is a `Copy` `JsValue`) so the deferred task hands it off
/// without a move-vs-borrow dance.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CacheDelivery {
    /// Fulfill the Promise with this already-marshalled value (a
    /// `Response` / `Request` / `Array` / boolean / `undefined`).
    Resolve(JsValue),
    /// Reject the Promise with this reason (a `TypeError` thrown value).
    Reject(JsValue),
}

impl VmInner {
    /// Return the Cache API backend, lazily minting an in-memory one when
    /// the shell installed none (boa `ensure_cache_backend` parity, DR-A).
    /// `None` only when the VM is unbound (no `HostData`) or in-memory
    /// SQLite creation fails — the caller surfaces that to JS.
    pub(crate) fn ensure_cache_backend(&mut self) -> Option<Arc<CacheBackend>> {
        let host = self.host_data.as_deref_mut()?;
        if host.cache_backend().is_none() {
            let backend = CacheBackend::in_memory().ok()?;
            host.install_cache_storage(Arc::new(backend));
        }
        host.cache_backend().cloned()
    }

    /// [`Self::ensure_cache_backend`] or a thrown `TypeError` — the
    /// backend-unavailable path is identical at every call site, so the
    /// message lives here once.
    pub(crate) fn require_cache_backend(&mut self) -> Result<Arc<CacheBackend>, VmError> {
        self.ensure_cache_backend()
            .ok_or_else(|| VmError::type_error("Cache storage backend unavailable"))
    }
}

/// Stage `outcome` on a fresh Pending Promise and queue its deferred
/// [`super::pending_tasks::PendingTask::CacheDeliver`] settle (DR-A.1).
/// Returns the Promise to hand back from the native synchronously.
pub(super) fn settle_async(vm: &mut VmInner, outcome: CacheDelivery) -> JsValue {
    let promise = super::super::natives_promise::create_promise(vm);
    vm.queue_task(super::pending_tasks::PendingTask::CacheDeliver {
        promise_id: promise,
        outcome,
    });
    JsValue::Object(promise)
}

/// Drain step for [`super::pending_tasks::PendingTask::CacheDeliver`]
/// (DR-A.1) — settle the Promise with the staged outcome.
pub(crate) fn dispatch_cache_deliver(
    vm: &mut VmInner,
    promise_id: ObjectId,
    outcome: CacheDelivery,
) {
    let (is_reject, value) = match outcome {
        CacheDelivery::Resolve(v) => (false, v),
        CacheDelivery::Reject(v) => (true, v),
    };
    let _ = super::super::natives_promise::settle_promise(vm, promise_id, is_reject, value);
}
