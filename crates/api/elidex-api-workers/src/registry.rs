//! Main-thread registry of running dedicated workers + thread-spawn scaffolding.
//!
//! Engine-independent cross-thread infrastructure. The registry holds only
//! transport handles ([`WorkerHandle`] = channel endpoint + `JoinHandle` +
//! metadata) keyed by an opaque worker id; it deliberately holds **no** JS
//! value / listener state. Per the elidex ECS-native design, a worker's
//! `onmessage`/`onerror`/`addEventListener` state lives in the engine-bound
//! `EventListeners` ECS component on the `Worker` entity, not here.

use std::collections::HashMap;
use std::thread::JoinHandle;

use elidex_plugin::{channel_pair, LocalChannel};
use url::Url;

use crate::handle::WorkerHandle;
use crate::types::{ParentToWorker, WorkerToParent};

/// Opaque identifier for a registered worker. Ids start at 1 and are never
/// reused for the lifetime of a registry, so a stale id never aliases a live
/// worker.
pub type WorkerId = u64;

/// Main-thread registry mapping [`WorkerId`] → [`WorkerHandle`].
///
/// The binding layer keys the brand-checked `Worker` HostObject by [`WorkerId`]
/// and routes `postMessage` / `terminate` / message-drain through this table.
#[derive(Default)]
pub struct WorkerRegistry {
    next_id: WorkerId,
    workers: HashMap<WorkerId, WorkerHandle>,
}

impl WorkerRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a running worker, returning its freshly minted id.
    pub fn register(&mut self, handle: WorkerHandle) -> WorkerId {
        self.next_id += 1;
        let id = self.next_id;
        self.workers.insert(id, handle);
        id
    }

    /// Borrow a worker handle by id.
    #[must_use]
    pub fn get(&self, id: WorkerId) -> Option<&WorkerHandle> {
        self.workers.get(&id)
    }

    /// Mutably borrow a worker handle by id.
    pub fn get_mut(&mut self, id: WorkerId) -> Option<&mut WorkerHandle> {
        self.workers.get_mut(&id)
    }

    /// Terminate a worker (sends `Shutdown`, detaches the thread) and drop its
    /// handle from the registry.
    pub fn terminate(&mut self, id: WorkerId) {
        if let Some(mut handle) = self.workers.remove(&id) {
            handle.terminate();
        }
    }

    /// Remove and return a worker handle (e.g. when the worker has exited).
    pub fn remove(&mut self, id: WorkerId) -> Option<WorkerHandle> {
        self.workers.remove(&id)
    }

    /// Terminate every registered worker (WHATWG HTML §10.2.4 "terminate a
    /// worker", invoked en masse at document teardown). Dropping each
    /// [`WorkerHandle`] runs its `Drop` (sends `Shutdown`, detaches the
    /// thread); the id counter is preserved so a later worker never aliases a
    /// terminated one.
    pub fn terminate_all(&mut self) {
        self.workers.clear();
    }

    /// Ids of all currently registered workers, for the main-loop message drain.
    pub fn ids(&self) -> Vec<WorkerId> {
        self.workers.keys().copied().collect()
    }

    /// Number of registered workers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.workers.len()
    }

    /// Whether the registry holds no workers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }
}

/// Spawn a dedicated worker thread and return the parent-side [`WorkerHandle`].
///
/// `body` runs **on the worker thread** and is supplied by the binding layer:
/// it builds + runs the worker JS runtime (engine-bound, `!Send` internally,
/// but constructed inside the thread so nothing `!Send` crosses the boundary).
/// Only the worker-side channel endpoint is handed to `body`; everything the
/// runtime needs (e.g. a `Send` network factory) must be captured by `body`
/// before this call.
///
/// This is the engine-independent half of the spawn: channel-pair creation,
/// thread spawn, and handle assembly. No JS-runtime type appears in the
/// signature — `body` is an opaque `FnOnce`, not a runtime trait.
pub fn spawn_worker<F>(name: String, script_url: Url, body: F) -> WorkerHandle
where
    F: FnOnce(LocalChannel<WorkerToParent, ParentToWorker>) + Send + 'static,
{
    let (parent_channel, worker_channel) = channel_pair::<ParentToWorker, WorkerToParent>();
    let thread: JoinHandle<()> = std::thread::spawn(move || body(worker_channel));
    WorkerHandle::new(name, script_url, parent_channel, thread)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_url() -> Url {
        Url::parse("https://example.com/w.js").expect("valid")
    }

    #[test]
    fn ids_are_monotonic_and_unique() {
        let mut reg = WorkerRegistry::new();
        let a = spawn_worker("a".into(), dummy_url(), |_ch| {});
        let b = spawn_worker("b".into(), dummy_url(), |_ch| {});
        let id_a = reg.register(a);
        let id_b = reg.register(b);
        assert_eq!(id_a, 1);
        assert_eq!(id_b, 2);
        assert_eq!(reg.len(), 2);
        reg.terminate(id_a);
        assert!(reg.get(id_a).is_none());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn spawn_runs_body_with_worker_channel() {
        let (tx, rx) = crossbeam_channel::unbounded::<String>();
        let handle = spawn_worker("echo".into(), dummy_url(), move |ch| {
            // Worker side: wait for one PostMessage, echo its data back out.
            if let Ok(ParentToWorker::PostMessage { data, .. }) =
                ch.recv_timeout(std::time::Duration::from_secs(2))
            {
                let _ = tx.send(data);
            }
        });
        handle.post_message("ping".into());
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(2))
                .as_deref(),
            Ok("ping")
        );
    }
}
