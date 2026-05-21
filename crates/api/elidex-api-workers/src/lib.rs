//! Dedicated Web Worker types, validation, and thread management for elidex.
//!
//! Engine-independent worker infrastructure shared by the JS runtimes:
//! - IPC message types ([`ParentToWorker`], [`WorkerToParent`]) and the
//!   parent-side [`WorkerHandle`].
//! - Pure worker-script validation + URL resolution ([`validate`]).
//! - The main-thread [`WorkerRegistry`] and the [`spawn_worker`] thread-spawn
//!   scaffolding (parameterized over an opaque `FnOnce` body the runtime
//!   supplies — no JS-runtime type crosses this crate's boundary).
//!
//! The worker JS runtime construction + event loop is engine-bound and lives in
//! the consuming runtime crate (`elidex-js` for the VM); only the
//! `!Send`-free transport + pure algorithm lives here.

mod handle;
mod registry;
mod types;
pub mod validate;

pub use handle::WorkerHandle;
pub use registry::{spawn_worker, WorkerId, WorkerRegistry};
pub use types::{ParentToWorker, WorkerToParent};
pub use validate::{
    resolve_worker_script_url, validate_credentials, validate_worker_script_response,
    validate_worker_type, WorkerScriptError, JS_MIME_TYPES,
};
