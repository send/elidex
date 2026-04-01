//! Dedicated Web Worker types and thread management for elidex.
//!
//! Provides IPC message types ([`ParentToWorker`], [`WorkerToParent`]) and the
//! parent-side [`WorkerHandle`] for managing worker threads. The actual worker
//! thread event loop and JS runtime setup live in `elidex-js-boa`.

mod handle;
mod types;

pub use handle::WorkerHandle;
pub use types::{ParentToWorker, WorkerToParent};
